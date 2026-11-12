//! A jar object's own `equals` and `compareTo` where a host collection or a
//! modelled comparator compares it.
//!
//! The host's collections hold their elements as markers and find one by
//! identity. That is Java's answer only while an element's class keeps
//! `Object.equals`. A record, or any jar class that overrides `equals`, is
//! looked up by value on a JVM: `map.get(new Key(1))` finds the entry another
//! `new Key(1)` was put under. The host found nothing, uncounted, and
//! `Set.add` stored a duplicate. Likewise the natural order of two jar objects
//! (`stream.sorted()`, `List.sort(null)`, `Comparator.naturalOrder()`) was
//! taken from their marker strings, which is the order they were constructed
//! in, not the order their `compareTo` defines.
//!
//! Both now run the jar's body through the interpreter:
//!
//! * [`Runtime::canonicalize_equal_key`]: before a host collection answers an
//!   equality-keyed member (`contains`, `indexOf`, `remove(Object)`, a set's
//!   `add`, a map's `get`/`put`/`containsKey`/`computeIfAbsent`...), the key
//!   argument is compared, with its own `equals` as `HashMap` and
//!   `ArrayList.indexOf` call it (`key.equals(element)`), against the
//!   collection's elements (a map's keys, or values for `containsValue`) in
//!   iteration order. The first equal element replaces the argument, so the
//!   host's identity lookup lands where the JVM's equality lookup does: `get`
//!   finds the entry, `put` replaces its value and keeps the original key (as
//!   `HashMap.put` does), and `Set.add` answers `false`.
//! * [`Runtime::natural_order`]: `a.compareTo(b)` runs the jar's
//!   `compareTo(Object)` (javac's bridge for `Comparable<T>`), an enum
//!   constant compares by ordinal (`Enum.compareTo`), and a jar object with no
//!   body for it is counted as `Comparable#compareTo` and answers `0`.
//!
//! The lookup assumes `hashCode` agrees with `equals`, as the `Object`
//! contract requires. A class that breaks the contract can differ from a JVM's
//! `HashMap`, which compares only within a hash bucket. That is a named
//! departure.

use super::*;

/// `equals(Object)`.
const EQUALS: &str = "(Ljava/lang/Object;)Z";
/// `Comparable.compareTo(Object)`, the erased bridge every implementer has.
const COMPARE_TO: &str = "(Ljava/lang/Object;)I";

/// Which of a collection's stored values an equality-keyed member compares
/// its argument with.
#[derive(Clone, Copy, PartialEq)]
enum Candidates {
    /// Every element of a list, set or queue.
    Elements,
    /// A map's keys (the even positions of its stored pairs).
    Keys,
    /// A map's values (the odd positions).
    Values,
}

/// The candidates `name descriptor` on a collection of kind `owner` compares
/// its first argument with, or `None` when the member does not look the
/// argument up by equality.
fn equality_lookup(
    owner: &str,
    name: &str,
    descriptor: &str,
    set_like: bool,
) -> Option<Candidates> {
    if is_map_owner(owner) {
        return match (name, descriptor) {
            ("containsValue", "(Ljava/lang/Object;)Z") => Some(Candidates::Values),
            ("get" | "remove", "(Ljava/lang/Object;)Ljava/lang/Object;")
            | ("containsKey", "(Ljava/lang/Object;)Z")
            | (
                "put" | "putIfAbsent" | "getOrDefault" | "replace",
                "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
            )
            | ("remove", "(Ljava/lang/Object;Ljava/lang/Object;)Z")
            | ("replace", "(Ljava/lang/Object;Ljava/lang/Object;Ljava/lang/Object;)Z")
            | (
                "computeIfAbsent",
                "(Ljava/lang/Object;Ljava/util/function/Function;)Ljava/lang/Object;",
            )
            | (
                "computeIfPresent" | "compute",
                "(Ljava/lang/Object;Ljava/util/function/BiFunction;)Ljava/lang/Object;",
            )
            | (
                "merge",
                "(Ljava/lang/Object;Ljava/lang/Object;Ljava/util/function/BiFunction;)Ljava/lang/Object;",
            ) => Some(Candidates::Keys),
            _ => None,
        };
    }
    if !is_collection_owner(owner) {
        return None;
    }
    match (name, descriptor) {
        ("contains" | "remove", "(Ljava/lang/Object;)Z")
        | ("indexOf" | "lastIndexOf", "(Ljava/lang/Object;)I") => Some(Candidates::Elements),
        ("add", "(Ljava/lang/Object;)Z") if set_like => Some(Candidates::Elements),
        _ => None,
    }
}

impl<H: Host> Runtime<'_, H> {
    /// The jar class of an object this run built whose class the interpreter
    /// runs (not a host- or library-modelled copy).
    fn interpreted_object_class(&self, value: &Value) -> Option<String> {
        let class = jvm_dispatch::constructed_object_class(expect_ref(value)?)?;
        (!self.host.models_class(class)
            && !jvm_dispatch::library_modelled(class)
            && self.class(class).is_some())
        .then(|| class.to_owned())
    }

    /// The jar class whose `equals(Object)` body `value` runs, when it
    /// overrides `Object.equals`.
    fn jar_equals_owner(&self, value: &Value) -> Option<String> {
        let class = self.interpreted_object_class(value)?;
        let (owner, true) = self.member_owner(&class, "equals", EQUALS)? else {
            return None;
        };
        self.class(&owner)?
            .method("equals", EQUALS)?
            .code
            .is_some()
            .then_some(owner)
    }

    /// Replace `args[0]` with the first element of the host collection
    /// `receiver` it `equals`, for a member that looks its argument up by
    /// equality. A no-op for every other call, for keys that keep identity
    /// equality, and for a call the jar's own body answers.
    pub(crate) fn canonicalize_equal_key(
        &mut self,
        r: &MethodRef,
        opcode: u8,
        receiver: Option<&Value>,
        args: &mut [Value],
    ) -> Result<(), NativeError> {
        if !matches!(opcode, 0xb6 | 0xb9) {
            return Ok(());
        }
        let (Some(receiver), Some(key)) = (receiver, args.first()) else {
            return Ok(());
        };
        let Some(equals_owner) = self.jar_equals_owner(key) else {
            return Ok(());
        };
        if self
            .jvm_selected_jar_body(r, opcode, Some(receiver))
            .is_some()
        {
            return Ok(());
        }
        // The JDK class whose contract answers: the call site's owner, or
        // the JDK superclass a jar subclass (`extends HashMap`) inherits it from.
        let owner = if is_collection_owner(&r.owner) {
            r.owner.clone()
        } else {
            match self.member_owner(&r.owner, &r.name, &r.descriptor) {
                Some((owner, false)) => owner,
                _ => return Ok(()),
            }
        };
        let Some(stored) = self.host.mutable_collection_values(receiver) else {
            return Ok(());
        };
        let receiver_class = expect_ref(receiver)
            .and_then(|marker| marker.split_once('#'))
            .map(|(class, _)| class);
        let set_like = is_set_owner(&owner)
            || receiver_class.is_some_and(is_set_owner)
            || expect_ref(receiver).is_some_and(|marker| marker.starts_with("set"));
        let Some(candidates) = equality_lookup(&owner, &r.name, &r.descriptor, set_like) else {
            return Ok(());
        };
        let key = key.clone();
        for (position, marker) in stored.iter().enumerate() {
            let wanted = match candidates {
                Candidates::Elements => true,
                Candidates::Keys => position % 2 == 0,
                Candidates::Values => position % 2 == 1,
            };
            if !wanted {
                continue;
            }
            let Ok(element) = parse_collection_value(marker) else {
                continue;
            };
            let equal = element == key
                || matches!(
                    self.invoke(
                        &equals_owner,
                        "equals",
                        EQUALS,
                        Some(key.clone()),
                        std::slice::from_ref(&element),
                    )?,
                    Value::Int(flag) if flag != 0
                );
            if equal {
                args[0] = element;
                break;
            }
        }
        Ok(())
    }

    /// `((Comparable) a).compareTo(b)`, the natural order a modelled
    /// comparator, `sorted()` or `List.sort(null)` applies.
    pub(crate) fn natural_order(&mut self, a: &Value, b: &Value) -> Result<i32, NativeError> {
        let Some(class) = self.interpreted_object_class(a) else {
            if self.interpreted_object_class(b).is_some() {
                // A platform value's `compareTo` of a jar object: a JVM throws
                // `ClassCastException` or runs code the host does not hold.
                return self.count_compare_to();
            }
            return jdk_stream::natural_compare(a, b);
        };
        if let Some((owner, true)) = self.member_owner(&class, "compareTo", COMPARE_TO)
            && self
                .class(&owner)
                .and_then(|file| file.method("compareTo", COMPARE_TO))
                .is_some_and(|method| method.code.is_some())
        {
            return match self.invoke(
                &owner,
                "compareTo",
                COMPARE_TO,
                Some(a.clone()),
                std::slice::from_ref(b),
            )? {
                Value::Int(order) => Ok(order),
                other => Err(NativeError::Metadata(format!(
                    "{owner}.compareTo answered {}",
                    describe_value(&other)
                ))),
            };
        }
        // `Enum.compareTo` is `ordinal - other.ordinal`.
        let ordinal = |value: &Value| {
            expect_ref(value)
                .and_then(|marker| self.enum_constants.get(marker))
                .map(|(_, ordinal)| *ordinal)
        };
        if let (Some(left), Some(right)) = (ordinal(a), ordinal(b)) {
            return Ok(left - right);
        }
        self.count_compare_to()
    }

    /// Count `Comparable.compareTo` as unknown and answer "equal".
    fn count_compare_to(&mut self) -> Result<i32, NativeError> {
        let compare_to = MethodRef {
            owner: "java/lang/Comparable".into(),
            name: "compareTo".into(),
            descriptor: COMPARE_TO.into(),
        };
        self.host.unknown_operand_call(&compare_to)?;
        Ok(0)
    }
}

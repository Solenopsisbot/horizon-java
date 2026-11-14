//! Iterators, enumerations and list iterators as host handles (#156 P14f).
//!
//! They were the text `iterator/<n>`, `resource-enumeration/<n>` and `list-iterator/<n>`, which a
//! `String` could spell: `"iterator/0"` reached the host's iterator table. Each is now a handle
//! whose key is that same table key (`iterator/<n>`, `enumeration/<n>`, `list-iterator/<n>`), so
//! the host's tables (`StdHost::iterators`, `list_iterators`, the fail-fast guards of
//! `iteration_guard.rs`) and heap tracing (`heap.rs`) keep their rows; only a handle reaches them,
//! through [`marker`], and a text spelling one is a `String` (`value_class::RETIRED_PREFIXES`).
//!
//! Java 25:
//!
//! * **Every `iterator()`/`listIterator()`/enumeration call makes a new object** with its own
//!   cursor: a per-allocation key is one object (canonical), so `==`, `IdentityHashMap` and
//!   `equals` (each class keeps `Object.equals`) are exact.
//! * **`Collections.emptyIterator()`/`emptyEnumeration()`** are shared singletons
//!   (`EmptyIterator.EMPTY_ITERATOR`, `EmptyEnumeration.EMPTY_ENUMERATION`): one key each.
//! * **`Collections.enumeration(c)`** is an anonymous `Enumeration` over `c.iterator()`, so it
//!   fails fast as that iterator does: this module keeps the iterator and runs its `hasNext`/
//!   `next` through the dispatch ([`EnumerationOver::Iterator`]).
//! * **`Vector.elements()`** reads the vector live by index (`count < elementCount`), never
//!   failing fast; past the end it throws `NoSuchElementException("Vector Enumeration")`.
//! * **`ClassLoader.getResources`** is a new enumeration per call over the resources found.
//!
//! Their classes are the collections' inner classes (`ArrayList$Itr`, `HashMap$KeyIterator`,
//! `Collections$3`, `Vector$1`...), which this model does not name: the class is unknown
//! (`ClassRule::Unknown`, `getClass()` counted), the interface declared.

use crate::runtime::host_ref::{HostKind, HostRef, Methods, instance_of_class};
use crate::runtime::jdk_call::Site;
use crate::runtime::{Host, MethodRef, NativeError, Runtime, Value};

/// `Collections.emptyIterator()`'s key.
pub(crate) const EMPTY_ITERATOR: &str = "iterator/empty";
/// `Collections.emptyEnumeration()`'s key.
pub(crate) const EMPTY_ENUMERATION: &str = "enumeration/empty";

/// The handle of the host iterator `marker` (`iterator/<n>`).
pub(crate) fn iterator(marker: String) -> Value {
    Value::Host(HostRef::new(HostKind::Iterator, marker))
}

/// The handle of the host enumeration `marker` (`enumeration/<n>`).
pub(crate) fn enumeration(marker: String) -> Value {
    Value::Host(HostRef::new(HostKind::Enumeration, marker))
}

/// The handle of the host list iterator `marker` (`list-iterator/<n>`).
pub(crate) fn list_iterator(marker: String) -> Value {
    Value::Host(HostRef::new(HostKind::ListIterator, marker))
}

/// The host table key of an iterator, enumeration or list iterator; `None` for any other value
/// (a `String` spelling one included).
pub(crate) fn marker(value: &Value) -> Option<&String> {
    match value {
        Value::Host(handle)
            if matches!(
                handle.kind(),
                HostKind::Iterator | HostKind::Enumeration | HostKind::ListIterator
            ) =>
        {
            Some(handle.key_text())
        }
        _ => None,
    }
}

pub(crate) fn iterator_types(_: &str) -> &'static [&'static str] {
    &["java/util/Iterator"]
}

pub(crate) fn enumeration_types(_: &str) -> &'static [&'static str] {
    &["java/util/Enumeration"]
}

pub(crate) fn list_iterator_types(_: &str) -> &'static [&'static str] {
    &["java/util/ListIterator", "java/util/Iterator"]
}

/// Every iterator and enumeration class keeps `Object`'s members: `equals` is identity (one key
/// per object), `hashCode`/`toString` an identity hash (counted).
pub(crate) const ITERATOR_METHODS: Methods = Methods {
    to_string: |_, _| Ok(None),
    hash_code: |_, _| Ok(None),
    equals: |handle, other, _| {
        Ok(Some(
            matches!(other, Value::Host(other) if handle.same_object(other) == Some(true)),
        ))
    },
    instance_of: instance_of_class,
    // An iterator walks this jar's host's collection.
    crosses_jars: |_, _| false,
    compare_to: None,
};

/// What a modelled `Enumeration` reads.
#[derive(Clone, Debug, PartialEq)]
pub enum EnumerationOver {
    /// `Collections.enumeration(c)`: `c.iterator()`, run through the dispatch.
    Iterator(Value),
    /// `Vector.elements()`: the vector, read live at `count`.
    Vector { vector: Value, count: usize },
}

impl EnumerationOver {
    /// The values it holds (heap reachability).
    pub(crate) fn held(&self) -> &Value {
        match self {
            EnumerationOver::Iterator(value) | EnumerationOver::Vector { vector: value, .. } => {
                value
            }
        }
    }
}

fn no_such_element(message: Option<&str>) -> NativeError {
    NativeError::UncaughtException {
        class: "java/util/NoSuchElementException".into(),
        message: message.map(str::to_owned),
        frame: None,
    }
}

impl<H: Host> Runtime<'_, H> {
    /// `Collections.enumeration`, `Vector.elements()`, `Collections.emptyIterator()`/
    /// `emptyEnumeration()`, and the members of the enumerations they make. `Ok(None)` leaves the
    /// call to the rest of the chain.
    pub(crate) fn invoke_enumeration(
        &mut self,
        r: &MethodRef,
        opcode: u8,
        receiver: Option<&Value>,
        args: &[Value],
    ) -> Result<Option<Value>, NativeError> {
        match (
            opcode,
            r.owner.as_str(),
            r.name.as_str(),
            r.descriptor.as_str(),
        ) {
            (0xb8, "java/util/Collections", "emptyIterator", "()Ljava/util/Iterator;") => {
                return Ok(Some(self.host.empty_iterator()));
            }
            (0xb8, "java/util/Collections", "emptyEnumeration", "()Ljava/util/Enumeration;") => {
                return Ok(Some(self.host.empty_enumeration()));
            }
            (
                0xb8,
                "java/util/Collections",
                "enumeration",
                "(Ljava/util/Collection;)Ljava/util/Enumeration;",
            ) => {
                // The anonymous class's field initialiser: `i = c.iterator()`.
                let collection = args.first().cloned().unwrap_or(Value::Ref(None));
                let iterator = self.call_virtual(
                    Site::Interface,
                    "java/util/Collection",
                    "iterator",
                    "()Ljava/util/Iterator;",
                    &collection,
                    &[],
                )?;
                return match self
                    .host
                    .new_enumeration(EnumerationOver::Iterator(iterator))
                {
                    Some(enumeration) => Ok(Some(enumeration)),
                    None => self.host.unknown_operand_call(r).map(Some),
                };
            }
            (
                0xb6,
                "java/util/Vector" | "java/util/Stack",
                "elements",
                "()Ljava/util/Enumeration;",
            ) if receiver.is_some_and(|vector| {
                matches!(
                    self.host_collection_class(vector).as_deref(),
                    Some("java/util/Vector" | "java/util/Stack")
                )
            }) =>
            {
                let vector = receiver.cloned().unwrap_or(Value::Ref(None));
                return match self
                    .host
                    .new_enumeration(EnumerationOver::Vector { vector, count: 0 })
                {
                    Some(enumeration) => Ok(Some(enumeration)),
                    None => self.host.unknown_operand_call(r).map(Some),
                };
            }
            _ => {}
        }
        if !matches!(opcode, 0xb6 | 0xb9) {
            return Ok(None);
        }
        let Some(key) = receiver.and_then(marker).cloned() else {
            return Ok(None);
        };
        let Some(over) = self.host.enumeration_over(&key) else {
            return Ok(None);
        };
        Ok(Some(
            match ((r.name.as_str(), r.descriptor.as_str()), over) {
                (("hasMoreElements", "()Z"), EnumerationOver::Iterator(iterator)) => self
                    .call_virtual(
                        Site::Interface,
                        "java/util/Iterator",
                        "hasNext",
                        "()Z",
                        &iterator,
                        &[],
                    )?,
                (("nextElement", "()Ljava/lang/Object;"), EnumerationOver::Iterator(iterator)) => {
                    self.call_virtual(
                        Site::Interface,
                        "java/util/Iterator",
                        "next",
                        "()Ljava/lang/Object;",
                        &iterator,
                        &[],
                    )?
                }
                (("hasMoreElements", "()Z"), EnumerationOver::Vector { vector, count }) => {
                    let size = self.collection_elements(&vector)?.len();
                    Value::Int(i32::from(count < size))
                }
                (
                    ("nextElement", "()Ljava/lang/Object;"),
                    EnumerationOver::Vector { vector, count },
                ) => {
                    let elements = self.collection_elements(&vector)?;
                    let value = elements
                        .get(count)
                        .cloned()
                        .ok_or_else(|| no_such_element(Some("Vector Enumeration")))?;
                    self.host.store_enumeration(
                        &key,
                        EnumerationOver::Vector {
                            vector,
                            count: count + 1,
                        },
                    );
                    value
                }
                // `asIterator` and the rest: counted by the chain.
                _ => return Ok(None),
            },
        ))
    }
}

/// `Joiner`'s class (Guava).
const JOINER: &str = "com/google/common/base/Joiner";

/// Parts one `Joiner.join(Iterator)` may drain before the call is counted instead.
const MAX_JOINED_PARTS: usize = 65_536;

impl<H: Host> Runtime<'_, H> {
    /// Guava's `Joiner.join(Iterator)` on a plain joiner (the host's model): `appendTo` walks the
    /// iterator through its own `hasNext`/`next`, so it consumes it and fails fast as that
    /// iterator does (#225). The parts it drew are then joined by the host's `join(Iterable)`. A
    /// `null` part stops the walk where Java's `toString(part)` throws, after consuming it.
    /// `Ok(None)` leaves any other call (and any other joiner) to the chain.
    pub(crate) fn invoke_joiner_iterator(
        &mut self,
        r: &MethodRef,
        opcode: u8,
        receiver: Option<&Value>,
        args: &[Value],
    ) -> Result<Option<Value>, NativeError> {
        let (0xb6, true, Some(joiner), [iterator]) = (
            opcode,
            r.owner == JOINER
                && r.name == "join"
                && r.descriptor == "(Ljava/util/Iterator;)Ljava/lang/String;",
            receiver,
            args,
        ) else {
            return Ok(None);
        };
        // A configured joiner (`skipNulls`, `useForNull`) is the host's to count; a `null`
        // iterator its NPE; an iterator that is neither the host's nor a jar object is counted.
        if crate::runtime::host_guava::separator(joiner, true).is_none()
            || marker(iterator).is_none() && crate::runtime::jar_object::object_key(iterator).is_none()
        {
            return Ok(None);
        }
        let mut parts = Vec::new();
        loop {
            let more = self.call_virtual(
                Site::Interface,
                "java/util/Iterator",
                "hasNext",
                "()Z",
                iterator,
                &[],
            )?;
            if more != Value::Int(1) {
                break;
            }
            if parts.len() >= MAX_JOINED_PARTS {
                return self.host.unknown_operand_call(r).map(Some);
            }
            let part = self.call_virtual(
                Site::Interface,
                "java/util/Iterator",
                "next",
                "()Ljava/lang/Object;",
                iterator,
                &[],
            )?;
            let null = part == Value::Ref(None);
            parts.push(part);
            if null {
                break;
            }
        }
        let parts = self.new_array_list(parts);
        let join = MethodRef {
            owner: JOINER.into(),
            name: "join".into(),
            descriptor: "(Ljava/lang/Iterable;)Ljava/lang/String;".into(),
        };
        self.host.invoke_instance(&join, joiner, &[parts]).map(Some)
    }
}

#[cfg(test)]
#[path = "host_iterator_tests.rs"]
mod tests;

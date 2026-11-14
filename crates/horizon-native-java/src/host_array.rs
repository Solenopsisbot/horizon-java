//! Java arrays as host handles (#156 P11).
//!
//! An array used to be marker text, `array/<class>/<method>/<n>`, the same
//! `Value::Ref` a `String` is, so a jar string spelled like one was that
//! array to every reader that checked the prefix (`System.arraycopy`, the
//! heap, `Arrays.asList`), and an array was text to any reader that did not.
//! Now an array is a [`HostKind::Array`] handle, and its elements stay where
//! they were: the runtime's `arrays` table, keyed by the handle's key.
//!
//! **Key:** `<array class>#<n>`, the class in descriptor form (`[I`,
//! `[Ljava/lang/String;`, `[[I`) and `n` the runtime's array counter, so the
//! key names one allocation and carries the array's class: `getClass()`,
//! `instanceof` and `checkcast` read it from the handle, for every array
//! however it was built (the class used to be a side record that only some
//! producers wrote). A key ends in its allocation number, like a jar object's
//! `class#n`, so the resident heap's collector finds it wherever it is held.
//!
//! The one key that is not one allocation is `<array class>#empty`
//! ([`shared_empty`]): the empty array the runtime answers for a call it does
//! not model whose declared result is an array. It has no table row (it has
//! no elements to keep) and stands for whatever array that call would have
//! returned, so its class is the call's declared result type, which the real
//! array may be a subclass of: `instanceof` that type or a supertype is
//! `true`, and anything else needing the exact class (`getClass()`, a
//! narrowing cast) is unknown and counted.
//!
//! **Identity:** an array is an object with identity (`new int[0]` twice is
//! two objects), and its `equals`/`hashCode` are `Object`'s. Every
//! allocation mints its own key and no key is ever minted twice, so two
//! handles with one allocation key are that one array, however they got
//! there (a local, a field, a collection, a capture read back from text):
//! the key is canonical ([`canonical`]). Two separate arrays always have two
//! keys, so `new int[0] == new int[0]` is `false` without counting. The one
//! exception is the shared empty array: one key standing for every array an
//! unmodelled call returned, so two of them are one object only when the
//! same construction flows to both sides (`HostRef::same_object`), and
//! identity on them is counted otherwise.
//!
//! **Text:** `toString()` is `getClass().getName() + "@" +
//! Integer.toHexString(hashCode())` and `hashCode()` is the identity hash: a
//! number the VM picks, which Java does not define and the runtime does not
//! have. Both answer unknown, and the caller counts the call; neither is ever
//! invented. `Arrays.toString`/`hashCode`/`equals` read the elements, not the
//! handle.

use crate::host_ref::{ClassHierarchy, HostKind, HostRef, Methods, Nested};
use crate::{NativeError, Value};

/// Whether `class` (the part of a `class#n` text before the `#`) names an
/// object the runtime now holds only as a handle: an array (`[I#0`) or a
/// `ColorCollection` (`color-collection#0`). No such text is an object any
/// more, so it is a `String` (until P17 retires the text classifiers).
pub(crate) fn handle_only_class(class: &str) -> bool {
    class.starts_with('[') || class == "color-collection"
}

/// The suffix of [`shared_empty`]'s key.
const SHARED_EMPTY: &str = "empty";

/// A new array's key: its class and allocation number.
pub(crate) fn key(class: &str, number: u64) -> String {
    format!("{class}#{number}")
}

/// The handle of the array whose elements are at `key` in the `arrays` table.
/// Call it once per allocation and clone the value from then on: two calls
/// are two constructions, which identity cannot tell apart.
pub(crate) fn handle(key: impl Into<String>) -> Value {
    Value::Host(HostRef::new(HostKind::Array, key))
}

/// The empty array of `class` (`[Ljava/lang/String;`) the runtime answers
/// where it has no array of its own to give (see the module notes).
pub(crate) fn shared_empty(class: &str) -> Value {
    Value::Host(HostRef::new(HostKind::UnknownArray, key_of_empty(class)))
}

/// The types a shared empty array surely is, whatever its runtime class
/// ([`HostKind::UnknownArray`]'s `ClassRule::Unknown`): every array's.
/// Its declared array class is in the key, and the kind's `instance_of`
/// reads it from there.
pub(crate) fn surely(_key: &str) -> &'static [&'static str] {
    &[
        "java/lang/Object",
        "java/lang/Cloneable",
        "java/io/Serializable",
    ]
}

fn key_of_empty(class: &str) -> String {
    format!("{class}#{SHARED_EMPTY}")
}

/// The kind's canonical predicate: an allocation key is one array; the
/// shared empty key is not one object.
pub(crate) fn canonical(key: &str) -> bool {
    !is_shared_empty(key)
}

/// Whether `key` is a [`shared_empty`] array's.
pub(crate) fn is_shared_empty(key: &str) -> bool {
    key.rsplit_once('#')
        .is_some_and(|(_, number)| number == SHARED_EMPTY)
}

/// The `arrays`-table key of an array handle; `None` for any other value (a
/// `String` spelled like an array is no array).
pub(crate) fn array_key(value: &Value) -> Option<&str> {
    match value {
        Value::Host(handle)
            if matches!(handle.kind(), HostKind::Array | HostKind::UnknownArray) =>
        {
            Some(handle.key())
        }
        _ => None,
    }
}

/// The array class a key carries: `[I` of `[I#3`.
pub(crate) fn class_of_key(key: &str) -> &str {
    key.rsplit_once('#').map_or(key, |(class, _)| class)
}

/// The component of an array class, borrowed (`java/lang/String` of
/// `[Ljava/lang/String;`, `[I` of `[[I`, `int` of `[I`): `None` for any
/// other text. Nothing is allocated: it runs on every array store.
pub(crate) fn component_of(class: &str) -> Option<&str> {
    let element = class.strip_prefix('[')?;
    Some(match element {
        "Z" => "boolean",
        "B" => "byte",
        "C" => "char",
        "S" => "short",
        "I" => "int",
        "J" => "long",
        "F" => "float",
        "D" => "double",
        nested if nested.starts_with('[') => nested,
        object => object.strip_prefix('L')?.strip_suffix(';')?,
    })
}

/// The exact class of an array handle: `None` for any other value and for
/// a shared empty array, whose exact class is unknown.
pub(crate) fn exact_class(value: &Value) -> Option<&str> {
    array_key(value)
        .filter(|key| !is_shared_empty(key))
        .map(class_of_key)
}

/// `Object.equals`: `this == obj`. Another array is this one only when
/// identity can tell; two shared empty arrays from separate constructions
/// are unknown.
fn equals(
    handle: &HostRef,
    other: &Value,
    _nested: &mut dyn Nested,
) -> Result<Option<bool>, NativeError> {
    Ok(match other {
        Value::Host(other) => handle.same_object(other),
        _ => Some(false),
    })
}

/// `array instanceof wanted`: an array class is assignable to `Object`,
/// `Cloneable` and `Serializable`, and to another array class when its
/// components are (JLS 4.10.3: `String[]` is an `Object[]`, `int[]` is not).
fn instance_of(handle: &HostRef, wanted: &str, hierarchy: &dyn ClassHierarchy) -> Option<bool> {
    let answer = class_assignable(class_of_key(handle.key()), wanted, hierarchy);
    if is_shared_empty(handle.key()) {
        // Only the declared type is known, not the exact class.
        return answer.filter(|fits| *fits);
    }
    answer
}

/// Whether an object of class `class` is a `wanted` (each an internal name
/// or an array descriptor), `None` where the hierarchy cannot tell. JLS
/// 4.10.3: an array class is an `Object`, `Cloneable` and `Serializable`,
/// and another array class when its components are (`String[]` is an
/// `Object[]`, `int[]` is not). Allocates nothing: it runs on array stores.
pub(crate) fn class_assignable(
    class: &str,
    wanted: &str,
    hierarchy: &dyn ClassHierarchy,
) -> Option<bool> {
    if class == wanted || wanted == "java/lang/Object" {
        return Some(true);
    }
    match (class.strip_prefix('['), wanted.strip_prefix('[')) {
        (Some(from), Some(to)) => match (element(from)?, element(to)?) {
            (Element::Primitive(a), Element::Primitive(b)) => Some(a == b),
            (Element::Primitive(_), _) | (_, Element::Primitive(_)) => Some(false),
            (Element::Reference(from), Element::Reference(to)) => {
                class_assignable(from, to, hierarchy)
            }
        },
        (Some(_), None) => Some(matches!(
            wanted,
            "java/lang/Cloneable" | "java/io/Serializable"
        )),
        (None, Some(_)) => Some(false),
        (None, None) => {
            if class == "java/lang/String" {
                // A `String[]` is a `CharSequence[]`, `Comparable[]`, ...
                return Some(crate::value_class::STRING_TYPES.contains(&wanted));
            }
            hierarchy.is_instance(class, wanted)
        }
    }
}

/// An array's element type, from its descriptor after the `[`.
enum Element<'a> {
    Primitive(u8),
    /// An internal class name, or an array class's descriptor.
    Reference(&'a str),
}

fn element(descriptor: &str) -> Option<Element<'_>> {
    match descriptor.as_bytes() {
        [code @ (b'Z' | b'B' | b'C' | b'S' | b'I' | b'J' | b'F' | b'D')] => {
            Some(Element::Primitive(*code))
        }
        [b'[', ..] => Some(Element::Reference(descriptor)),
        _ => descriptor
            .strip_prefix('L')?
            .strip_suffix(';')
            .map(Element::Reference),
    }
}

pub(crate) const ARRAY: Methods = Methods {
    // `[I@1b6d3586`: the identity hash only the VM has.
    to_string: |_, _| Ok(None),
    hash_code: |_, _| Ok(None),
    equals,
    instance_of,
    // An array is one jar runtime's mutable state.
    crosses_jars: |_, _| false,
    // Not `Comparable`.
    compare_to: None,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host_ref::JdkHierarchy;

    fn array(class: &str) -> HostRef {
        HostRef::new(HostKind::Array, key(class, 0))
    }

    /// JLS 4.10.3 array subtyping, against what Java 25 answers for
    /// `instanceof` (the fixture checks the same through bytecode).
    #[test]
    fn array_classes_are_assignable_as_the_jls_says() {
        let strings = array("[Ljava/lang/String;");
        let ints = array("[I");
        let grid = array("[[I");
        for (handle, wanted, expected) in [
            (&strings, "[Ljava/lang/Object;", true),
            (&strings, "[Ljava/lang/CharSequence;", true),
            (&strings, "[Ljava/lang/Integer;", false),
            (&strings, "java/lang/Cloneable", true),
            (&strings, "java/io/Serializable", true),
            (&strings, "java/lang/Comparable", false),
            (&ints, "[Ljava/lang/Object;", false),
            (&ints, "[J", false),
            (&ints, "[I", true),
            (&ints, "java/lang/Object", true),
            (&grid, "[Ljava/lang/Object;", true),
            (&grid, "[Ljava/lang/Cloneable;", true),
            (&grid, "[[J", false),
            (&grid, "[I", false),
        ] {
            assert_eq!(
                handle.instance_of(wanted, &JdkHierarchy),
                Some(expected),
                "{} instanceof {wanted}",
                class_of_key(handle.key())
            );
        }
    }

    /// A key holds a jar's class name, which may contain any character but
    /// `.;[/` (JVMS 4.2.1): `#`, `,`, `\\`, `:`, `empty`, digits. Each key reads
    /// back as its own class, never a shared empty, never another array's
    /// key, and round-trips through a list of markers (a collection, a
    /// capture) as the same handle.
    #[test]
    fn class_names_with_every_separator_never_alias() {
        let names = [
            "demo/A",
            "demo/A#1",
            "demo/A#empty",
            "demo/A,1",
            "demo/A\\",
            "demo/A:I:25#0",
            "demo/#",
            "demo/A#1#2",
            "demo/empty",
            "demo/h:arr",
            "demo/A\\,#3",
        ];
        let mut keys = std::collections::BTreeSet::new();
        for (index, name) in names.iter().enumerate() {
            for class in [format!("[L{name};"), format!("[[L{name};")] {
                for number in [0, 1, 12, index as u64] {
                    let key = key(&class, number);
                    assert_eq!(class_of_key(&key), class, "{key}");
                    assert!(!is_shared_empty(&key), "{key}");
                    assert!(keys.insert(key.clone()) || number == index as u64, "{key}");
                    let value = handle(key.clone());
                    let list = [value.clone(), Value::Ref(Some(key.clone()))]
                        .iter()
                        .map(crate::value_marker)
                        .collect::<Vec<_>>()
                        .join(",");
                    let back = crate::split_capture_markers(&list)
                        .unwrap()
                        .iter()
                        .map(|marker| crate::parse_value_marker(marker).unwrap())
                        .collect::<Vec<_>>();
                    // The handle comes back as the same array; the string
                    // spelled like its key stays a string.
                    assert_eq!(back, vec![value, Value::Ref(Some(key.clone()))]);
                    assert_eq!(array_key(&back[0]), Some(key.as_str()));
                    assert_eq!(array_key(&back[1]), None);
                }
            }
            let empty = shared_empty(&format!("[L{name};"));
            assert_eq!(
                array_key(&empty).map(class_of_key),
                Some(format!("[L{name};").as_str())
            );
            assert!(array_key(&empty).is_some_and(is_shared_empty));
        }
    }

    /// Every key, shared empties too, carries its class; only the shared
    /// empties are marked as such.
    #[test]
    fn a_key_carries_its_class() {
        assert_eq!(
            class_of_key(&key("[Ljava/lang/String;", 12)),
            "[Ljava/lang/String;"
        );
        assert_eq!(
            array_key(&shared_empty("[[I")).map(class_of_key),
            Some("[[I")
        );
        assert!(is_shared_empty(&key_of_empty("[I")));
        // A shared empty array is its declared type, maybe a subclass of it.
        let Value::Host(objects) = shared_empty("[Ljava/lang/Object;") else {
            unreachable!()
        };
        assert_eq!(
            objects.instance_of("[Ljava/lang/Object;", &JdkHierarchy),
            Some(true)
        );
        assert_eq!(
            objects.instance_of("java/lang/Cloneable", &JdkHierarchy),
            Some(true)
        );
        assert_eq!(
            objects.instance_of("[Ljava/lang/String;", &JdkHierarchy),
            None
        );
        assert_eq!(objects.instance_of("[I", &JdkHierarchy), None);
        assert!(!is_shared_empty(&key("[I", 0)));
        assert_eq!(array_key(&Value::Ref(Some(key("[I", 0)))), None);
    }
}

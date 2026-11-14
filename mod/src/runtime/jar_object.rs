//! Objects the runtime allocated (`new`, `Runtime::new_object`): the one place
//! that reads one (#156 P12).
//!
//! An object is a handle, `Value::Host` of [`HostKind::JarObject`], whose key
//! is `<internal class name>#<n>`: its class and the runtime's allocation
//! number. Every allocation is one (a jar class's, a JDK class's a jar `new`s,
//! a model's own), so no object is text a `String` can spell (#156 P12b): the
//! accessors below answer for handles only. (A `Value::Ref` spelling a key
//! still reads as it did where a reader reads text; #156 P12b2 makes it a
//! `String` to every reader.) The key is the text the object used to be, so every
//! table keyed by an object (instance fields, the host's throwable messages and
//! collections, the resident heap) keeps its rows, and `expect_ref` answers a
//! handle's key to the readers that look one up.
//!
//! * **Class**: [`class_of_key`], from the key (`ClassRule::FromKey`); the
//!   class file the run holds (or Java 25's table for a JDK class) answers the
//!   rest of the `Class` surface.
//! * **Identity**: one key per allocation, never re-keyed, so the kind is
//!   canonical: `==` is key equality.
//! * **`toString`/`hashCode`/`equals`**: the class's own, which only the
//!   runtime can run; the row hands the handle back to the caller's
//!   [`Nested`], which runs them (the runtime), or answers what the host
//!   models and counts the rest. `Object`'s identity versions are counted.
//!
//! **Parsing a key.** A key ends in the allocation number, which the runtime
//! wrote, so the class is everything before the **last** `#` and the number
//! is the digits after it. A class name may itself hold `#`, `$` or `/` (the
//! JVM allows `#` in a binary name), which splitting at the first `#` misread.
//! A key is never built from text a jar chose apart from the class name the
//! class file declares, and no other separator is read back out of it.
//!
//! Not objects here: a handle-only class's text (an array, `[I#0`), and the
//! host's other `#` texts (`…#immutable3`, `…#neutral12`: collections, P14;
//! a member reference `owner#name`), whose suffix is not a number.

use crate::runtime::Value;
use crate::runtime::host_ref::{HostKind, HostRef, Methods, instance_of_class};

/// A jar object's handle for an allocation key `class#n`.
pub(crate) fn handle(key: String) -> Value {
    debug_assert!(split_key(&key).is_some(), "{key:?} is no allocation key");
    Value::Host(HostRef::new(HostKind::JarObject, key))
}

/// The object a table row of the runtime's own names by its key (a model's
/// field holding a child's key). Never for a jar's text: a key is a handle's
/// only, and a `String` spelling one is no object. The kind is canonical, so
/// this is the same object the key was read from.
pub(crate) fn stored(key: &str) -> Value {
    handle(key.to_owned())
}

/// A value a model kept as its text (a guard's source collection, a view's
/// map, a proxy's own marker): an object's key is that object's handle, any
/// other text the `Ref` it was. Only for text the runtime stored from a value
/// (`expect_ref` of one), never a jar's: no `Ref` is an object, so a key the
/// runtime holds is an object's.
pub(crate) fn stored_value(text: &str) -> Value {
    if parse_key(text).is_some() {
        stored(text)
    } else {
        Value::Ref(Some(text.to_owned()))
    }
}

/// Whether `value` is an allocated object (a [`HostKind::JarObject`]).
pub(crate) fn is_object(value: &Value) -> bool {
    object_key(value).is_some()
}

/// Whether `handle` is an allocated object.
pub(crate) fn is_object_handle(handle: &HostRef) -> bool {
    handle.kind() == HostKind::JarObject
}

/// `ClassRule::FromKey` of a jar object: the class before the allocation
/// number (a key always has one; [`handle`] is its only producer).
pub(crate) fn class_of_key(key: &str) -> &str {
    key_class(key).unwrap_or(key)
}

/// The kind's row: each method is the class's own, so the row asks the
/// caller's [`Nested`] about the object itself, as it asks about a value a
/// handle holds. The runtime's runs the jar's body (or counts `Object`'s
/// identity hash); the host's answers what it models and leaves the rest
/// unknown, so the caller counts it.
pub(crate) const JAR_OBJECT: Methods = Methods {
    to_string: |handle, nested| nested.text(&Value::Host(handle.clone())),
    hash_code: |handle, nested| nested.hash(&Value::Host(handle.clone())),
    equals: |handle, other, nested| nested.equals(&Value::Host(handle.clone()), other),
    instance_of: instance_of_class,
    // An object is its own jar's: another jar's runtime holds none of it.
    crosses_jars: |_, _| false,
    // `compareTo` dispatches to the class's own body like any method; a
    // caller that only has the row (a host sort) counts it.
    compare_to: None,
};

/// A handle's key `class#n`: `(class, n)`. The runtime wrote it
/// (`Runtime::new_object`), so the split is all there is to it.
fn split_key(key: &str) -> Option<(&str, &str)> {
    let (class, number) = key.rsplit_once('#')?;
    (!class.is_empty() && !number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit()))
        .then_some((class, number))
}

/// Whether a reference's *text* spells an object's key, `(class, n)`: what
/// decides that a `Ref` is a `String` and no host marker. Stricter than a
/// handle's key: not an array's (`[I#0`), no `:`, space, `.` or `,`, and
/// not a host collection marker.
pub(crate) fn parse_key(text: &str) -> Option<(&str, &str)> {
    let (class, number) = split_key(text)?;
    (!crate::runtime::host_array::handle_only_class(class)
        && !class.contains([':', ' ', '.', ','])
        && !is_collection_marker(text))
    .then_some((class, number))
}

/// A host collection's text marker (`list/a,b`, `set/x#3`, `list#i1/…`,
/// collections #156 P14), whose last element may end in `#<digits>` like a
/// key does.
fn is_collection_marker(text: &str) -> bool {
    let head = text.split_once('/').map_or(text, |(head, _)| head);
    let kind = head.split_once('#').map_or(head, |(kind, _)| kind);
    matches!(kind, "list" | "set" | "map")
}

/// The class of a handle's key (`class#n`), `None` for a text with no
/// allocation number.
pub(crate) fn key_class(key: &str) -> Option<&str> {
    split_key(key).map(|(class, _)| class)
}

/// The class any `#` marker names: an object's (split at its allocation
/// number), or, for the host models' own collection and view markers
/// (`java/util/ArrayList#neutral0`, `…ListN#immutable3`, collections P14),
/// the text before the first `#`, as their readers have always taken it.
pub(crate) fn marker_class(text: &str) -> Option<&str> {
    parse_key(text)
        .map(|(class, _)| class)
        .or_else(|| text.split_once('#').map(|(class, _)| class))
}

/// [`marker_class`], or the text itself when it has no `#`.
pub(crate) fn marker_class_or_text(text: &str) -> &str {
    marker_class(text).unwrap_or(text)
}

/// [`marker_class`] of a value: an object's class, or a host marker's.
pub(crate) fn value_marker_class(value: &Value) -> Option<&str> {
    object_class(value).or_else(|| ref_marker_class(value))
}

/// The key of the object `value` is, `None` for any other value (a
/// `String` spelling a key included).
pub(crate) fn object_key(value: &Value) -> Option<&str> {
    match value {
        Value::Host(handle) if is_object_handle(handle) => Some(handle.key()),
        _ => None,
    }
}

/// The text an instance field of `value` is filed under
/// (`<receiver>|<owner>#<name><descriptor>`): an object's key, or the text of
/// any other reference with fields (a jar enum constant's `enum/…`).
pub(crate) fn instance_key(value: &Value) -> Option<&str> {
    object_key(value).or(match value {
        Value::Ref(Some(text)) => Some(text.as_str()),
        _ => None,
    })
}

/// [`marker_class`] of a `Ref`'s text.
fn ref_marker_class(value: &Value) -> Option<&str> {
    match value {
        Value::Ref(Some(text)) => marker_class(text),
        _ => None,
    }
}

/// The class of the object `value` is, `None` for any other value.
pub(crate) fn object_class(value: &Value) -> Option<&str> {
    key_class(object_key(value)?)
}

/// [`object_class`] of a class in a package (`a/B`, not `B`): the
/// `class#n` objects `new` and the models build in this run, as
/// `jar_text`/`jdk_model` have always read them.
pub(crate) fn packaged_object_class(value: &Value) -> Option<&str> {
    object_class(value).filter(|class| class.contains('/'))
}

/// [`key_class`] of a class in a package.
pub(crate) fn packaged_key_class(text: &str) -> Option<&str> {
    key_class(text).filter(|class| class.contains('/'))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The class is everything before the last `#`, whatever the class name
    /// holds; a suffix that is no allocation number is no object.
    #[test]
    fn keys_split_at_the_allocation_number() {
        assert_eq!(parse_key("demo/Foo#3"), Some(("demo/Foo", "3")));
        assert_eq!(
            parse_key("demo/Outer$Inner#12"),
            Some(("demo/Outer$Inner", "12"))
        );
        assert_eq!(parse_key("demo/We#ird#7"), Some(("demo/We#ird", "7")));
        assert_eq!(parse_key("Foo#0"), Some(("Foo", "0")));
        assert_eq!(packaged_key_class("Foo#0"), None);
        for text in [
            "demo/Foo#",
            "#3",
            "demo/Foo#3a",
            "java/util/ArrayList#immutable3",
            "demo/Foo#bar",
            "[I#0",
            "a b#3",
            "minecraft:stone#3",
            "demo.Foo#3",
            "plain text",
            // A host collection marker whose element ends like a key.
            "list/r#1,r#2,r#3",
            "set/r#3",
            "list#i1/a#3",
        ] {
            assert_eq!(parse_key(text), None, "{text:?}");
        }
        assert_eq!(marker_class_or_text("demo/Foo"), "demo/Foo");
        assert_eq!(marker_class_or_text("demo/Foo#4"), "demo/Foo");
        assert_eq!(marker_class_or_text("demo/We#ird#4"), "demo/We#ird");
        // A host model's own collection marker keeps its old reading.
        assert_eq!(
            marker_class("java/util/ArrayList#neutral0"),
            Some("java/util/ArrayList")
        );
        assert_eq!(key_class("java/util/ArrayList#neutral0"), None);
        assert_eq!(object_class(&handle("demo/Foo#4".into())), Some("demo/Foo"));
        // A handle's key is the runtime's: its class is the split, even in a
        // package named like a host collection.
        assert_eq!(object_class(&handle("map/Foo#3".into())), Some("map/Foo"));
        // A host collection marker a model kept stays that marker.
        assert_eq!(
            stored_value("list/r#3"),
            Value::Ref(Some("list/r#3".into()))
        );
        // Text is no object.
        assert_eq!(object_class(&Value::Ref(Some("demo/Foo#4".into()))), None);
        assert_eq!(object_class(&Value::Int(4)), None);
    }
}

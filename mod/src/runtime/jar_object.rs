//! Objects the runtime allocated (`new`, `Runtime::new_object`): the one place
//! that reads one (#156 P12).
//!
//! An object is named by its key, `<internal class name>#<n>`: its class and
//! the runtime's allocation number. Today the key is also the object's value,
//! a `Value::Ref` text that a `String` could spell; #156 P12b makes an object
//! of a jar class a typed handle whose key is this same text, so every table
//! keyed by an object (instance fields, the host's throwable messages, the
//! resident heap) keeps its rows. Every reader goes through this module, so
//! the switch changes [`object_key`] and the producers, nothing else.
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

/// `class#n` text: `(class, n)`, or `None` for any other text.
pub(crate) fn parse_key(text: &str) -> Option<(&str, &str)> {
    let (class, number) = text.rsplit_once('#')?;
    (!class.is_empty()
        && !crate::runtime::host_array::handle_only_class(class)
        && !class.contains([':', ' ', '.'])
        && !number.is_empty()
        && number.bytes().all(|byte| byte.is_ascii_digit()))
    .then_some((class, number))
}

/// The class of an object's key text, `None` for any other text.
pub(crate) fn key_class(text: &str) -> Option<&str> {
    parse_key(text).map(|(class, _)| class)
}

/// The class any `#` marker names: an object's (split at its allocation
/// number), or, for the host models' own collection and view markers
/// (`java/util/ArrayList#neutral0`, `…ListN#immutable3`, collections P14),
/// the text before the first `#`, as their readers have always taken it.
pub(crate) fn marker_class(text: &str) -> Option<&str> {
    key_class(text).or_else(|| text.split_once('#').map(|(class, _)| class))
}

/// [`marker_class`], or the text itself when it has no `#`.
pub(crate) fn marker_class_or_text(text: &str) -> &str {
    marker_class(text).unwrap_or(text)
}

/// [`marker_class`] of a value: an object's class, or a host marker's.
pub(crate) fn value_marker_class(value: &Value) -> Option<&str> {
    object_class(value).or_else(|| marker_class(crate::runtime::expect_ref(value)?))
}

/// The key of the object `value` is, `None` for any other value.
pub(crate) fn object_key(value: &Value) -> Option<&str> {
    match value {
        Value::Ref(Some(text)) if parse_key(text).is_some() => Some(text),
        _ => None,
    }
}

/// The text an instance field of `value` is filed under
/// (`<receiver>|<owner>#<name><descriptor>`): an object's key, or the text of
/// any other reference with fields (a jar enum constant's `enum/…`).
pub(crate) fn instance_key(value: &Value) -> Option<&str> {
    object_key(value).or_else(|| crate::runtime::expect_ref(value).map(String::as_str))
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
        assert_eq!(
            object_class(&Value::Ref(Some("demo/Foo#4".into()))),
            Some("demo/Foo")
        );
        assert_eq!(object_class(&Value::Int(4)), None);
    }
}

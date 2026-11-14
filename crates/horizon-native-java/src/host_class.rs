//! `java.lang.Class` objects (#156 P13): the one place that reads one.
//!
//! A `Class` is a singleton per (loader, name). A run is one jar's view of
//! its loaders, and `class_precedence` (#124) already picks one class file per
//! name (the jar's own copy or a linked sibling's), so one name is one object
//! in a run: [`HostKind::Class`], keyed by the class's runtime name in
//! internal form (`demo/Foo`, `java/lang/String`, a primitive's `int`, an
//! array's descriptor `[I`, `[Ljava/lang/String;`), is canonical, and
//! `Foo.class == obj.getClass() == Class.forName("demo.Foo")` is key equality.
//!
//! Until #156 P13 a `Class` was text: the internal name, the same `Value::Ref`
//! a `String` is. P13 moves it in steps (`hj-notes/p13-plan.md`):
//!
//! * **P13a (this layer):** every reader of a `Class` value goes through
//!   [`class_name`] and [`is_class`], which accept a handle and, until P13c,
//!   the class-marker text every producer still answers. No answer changes.
//! * **P13b:** the producers (`ldc`, `getClass`, `forName`, the reflective
//!   getters, type nodes, annotation values, `MethodType`) answer handles.
//! * **P13c:** text is never a `Class`: a `String` spelled like a class name
//!   is that `String` everywhere, and [`class_name`] stops reading text.
//!
//! The row: `Class` is final and its supertypes are Java 25's
//! (`jdk_closure.rs`). `equals` is identity, exact between handles (one name,
//! one object); against text it is counted until P13c (a class marker may be
//! the same `Class`, a `String` spelled like it is not). `hashCode` is an
//! identity hash (counted) and `toString` (`class X`, `interface X`, `int`)
//! needs the class file, which the row does not have: counted until P13b
//! answers it in the runtime.

use crate::host_ref::{HostKind, HostRef, Methods, instance_of_class};
use crate::{ClassFile, LinkedClassTable, Value};
use std::collections::BTreeMap;

pub(crate) const CLASS: &str = "java/lang/Class";

/// The kind's `ClassRule::FromKey`: every key's class is `java.lang.Class`.
pub(crate) fn class_of_class(_: &str) -> &str {
    CLASS
}

/// The `Class` object of the class named `name` (internal form).
#[cfg_attr(not(test), allow(dead_code))] // P13b's producers
pub(crate) fn class_value(name: &str) -> Value {
    Value::Host(HostRef::new(HostKind::Class, name))
}

/// The name of the class `value` is: a `Class` handle's key, or (until
/// #156 P13c) the class-marker text a producer answered. `None` for any other
/// value. The text is taken as every reader took it before P13: whether it
/// names a class is [`is_class`]'s question.
pub(crate) fn class_name(value: &Value) -> Option<&str> {
    match value {
        Value::Host(handle) if handle.kind() == HostKind::Class => Some(handle.key()),
        Value::Ref(Some(text)) => Some(text),
        _ => None,
    }
}

/// Whether `value` is a `Class`: a handle, or (until #156 P13c) text shaped
/// like a class marker (`class_marker_like`: a held or linked class, a
/// primitive, an array descriptor, an internal name that is no host marker).
pub(crate) fn is_class(
    value: &Value,
    classes: &BTreeMap<String, ClassFile>,
    linked: &LinkedClassTable,
) -> bool {
    match value {
        Value::Host(handle) => handle.kind() == HostKind::Class,
        Value::Ref(Some(text)) => crate::class_marker_like(text, classes, linked),
        _ => false,
    }
}

/// The row. See the module docs.
pub(crate) const CLASS_METHODS: Methods = Methods {
    to_string: |_, _| Ok(None),
    hash_code: |_, _| Ok(None),
    equals: |handle, other, _| {
        Ok(match other {
            Value::Host(other) => handle.same_object(other),
            // A class marker (text until P13c) may be this `Class`.
            Value::Ref(Some(_)) => None,
            _ => Some(false),
        })
    },
    instance_of: instance_of_class,
    // A JDK class is one object in every jar's runtime (the boot and
    // platform loaders); a jar's or a game's class is not known to be.
    crosses_jars: |handle, _| crate::jdk_closure::in_jdk_package(handle.key()),
    compare_to: None,
};

#[cfg(test)]
mod tests {
    use super::*;

    /// One name is one object; text is read as a name until P13c, never as a
    /// handle.
    #[test]
    fn a_class_is_its_name_and_one_object() {
        let a = class_value("demo/Foo");
        let b = class_value("demo/Foo");
        let (Value::Host(left), Value::Host(right)) = (&a, &b) else {
            unreachable!()
        };
        assert_eq!(left.same_object(right), Some(true));
        assert_eq!(left.class(), Some(CLASS));
        assert_eq!(class_name(&a), Some("demo/Foo"));
        let text = Value::Ref(Some("demo/Foo".into()));
        assert_eq!(class_name(&text), Some("demo/Foo"));
        assert_eq!(class_name(&Value::Int(1)), None);
        assert_eq!(
            class_name(&class_value("[Ljava/lang/String;")),
            Some("[Ljava/lang/String;")
        );
    }
}

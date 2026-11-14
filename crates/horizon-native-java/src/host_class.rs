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
//! * **P13a:** every reader of a `Class` value went through [`class_name`]
//!   and [`is_class`].
//! * **P13b1 (#215):** every producer answers a handle: `ldc`, `getClass`,
//!   `forName`/`loadClass`, the wrappers' `TYPE`, the `Class` members, the
//!   reflective getters, type nodes, annotation values and `annotationType`,
//!   `MethodType`, `StackWalker`, proxies, Guava's and Gson's `TypeToken`.
//! * **P13c (this layer):** text is never a `Class`. [`class_name`] and
//!   [`is_class`] read handles only, `expect_ref` no longer reads a handle's
//!   key, and a `String` spelled like a class name (`java/lang/String`,
//!   `[I`, `int`) is that `String` to every reader: `instanceof`, casts,
//!   `equals`, collections, type switches, reflection. Only `Class.forName`
//!   turns text (a binary name) into a class, as Java does.
//!
//! The row: `Class` is final and its supertypes are Java 25's
//! (`jdk_closure.rs`). `equals` is identity: exact between handles (one
//! name, one object), `false` against anything else. `hashCode` is an
//! identity hash (counted). `toString` (`class X`, `interface X`, `int`) needs
//! to know whether X is an interface: the row hands the handle back to the
//! caller's `Nested` (as `jar_object.rs` does), and the runtime answers from
//! the class file it holds or Java 25's table ([`class_text`]); a class
//! neither knows is counted.

use crate::host_ref::{HostKind, HostRef, Methods, instance_of_class};
use crate::{ClassFile, Value};

pub(crate) const CLASS: &str = "java/lang/Class";

/// The primitive classes' names, `void` included (`int.class`, `Void.TYPE`).
const PRIMITIVES: &[&str] = &[
    "boolean", "byte", "char", "short", "int", "long", "float", "double", "void",
];

/// Whether `name` is a primitive class's (`int`, `void`).
pub(crate) fn is_primitive(name: &str) -> bool {
    PRIMITIVES.contains(&name)
}

/// `Class.getName()`: the binary name, an array's descriptor with dots
/// (`[Ljava.lang.String;`), a primitive's keyword.
pub(crate) fn binary_name(name: &str) -> String {
    name.replace('/', ".")
}

/// An array class's dimensions and its element class (`[[I` → `(2, "int")`),
/// `None` for any other class.
pub(crate) fn array_element(name: &str) -> Option<(usize, String)> {
    let dimensions = name.bytes().take_while(|byte| *byte == b'[').count();
    (dimensions > 0).then(|| (dimensions, crate::erased_field_type(&name[dimensions..])))
}

/// `Class.getTypeName()`: an array is its element's name and one `[]` per
/// dimension (`java.lang.String[][]`); any other class its binary name.
pub(crate) fn type_name(name: &str) -> String {
    match array_element(name) {
        Some((dimensions, element)) => {
            format!("{}{}", binary_name(&element), "[]".repeat(dimensions))
        }
        None => binary_name(name),
    }
}

/// `Class.toString()` (JDK 25): a primitive's keyword, else `interface ` or
/// `class ` and the binary name (an array is a class). `None` when whether the
/// class is an interface is not known (`interface`): the caller counts it.
pub(crate) fn class_text(name: &str, interface: Option<bool>) -> Option<String> {
    if is_primitive(name) {
        return Some(name.to_owned());
    }
    let interface = if name.starts_with('[') {
        false
    } else {
        interface?
    };
    let kind = if interface { "interface" } else { "class" };
    Some(format!("{kind} {}", binary_name(name)))
}

/// Whether a class nobody holds the file of is an interface: never for a
/// primitive or an array; Java 25's table for a JDK class; `None` otherwise.
pub(crate) fn table_is_interface(name: &str) -> Option<bool> {
    if is_primitive(name) || name.starts_with('[') {
        return Some(false);
    }
    crate::jdk_closure::class(name).map(|row| row.is_interface)
}

/// The `Class` handle `value` is: its name ([`class_name`]).
pub(crate) fn handle_name(value: &Value) -> Option<&str> {
    class_name(value)
}

/// The kind's `ClassRule::FromKey`: every key's class is `java.lang.Class`.
pub(crate) fn class_of_class(_: &str) -> &str {
    CLASS
}

/// The `Class` object of the class named `name` (internal form).
pub(crate) fn class_value(name: &str) -> Value {
    Value::Host(HostRef::new(HostKind::Class, name))
}

/// The name of the class `value` is: a `Class` handle's key. `None` for any
/// other value: a `String` spelled like a class name is that `String`
/// (#156 P13c), and only `Class.forName` resolves text to a class.
pub(crate) fn class_name(value: &Value) -> Option<&str> {
    match value {
        Value::Host(handle) if handle.kind() == HostKind::Class => Some(handle.key()),
        _ => None,
    }
}

/// Whether `value` is a `Class`: a handle, never text (#156 P13c).
pub(crate) fn is_class(value: &Value) -> bool {
    class_name(value).is_some()
}

/// The row. See the module docs.
pub(crate) const CLASS_METHODS: Methods = Methods {
    // `class X`/`interface X`: whether X is an interface is its class file's
    // (or Java 25's table's), which only the caller's `Nested` holds.
    to_string: |handle, nested| nested.text(&Value::Host(handle.clone())),
    hash_code: |_, _| Ok(None),
    equals: |handle, other, _| {
        Ok(match other {
            Value::Host(other) => handle.same_object(other),
            // Text is a `String` however it is spelled (#156 P13c).
            _ => Some(false),
        })
    },
    instance_of: instance_of_class,
    // A JDK class is one object in every jar's runtime (the boot and
    // platform loaders); a jar's or a game's class is not known to be.
    crosses_jars: |handle, _| crate::jdk_closure::in_jdk_package(handle.key()),
    compare_to: None,
};

impl<H: crate::Host> crate::Runtime<'_, H> {
    /// Whether the class `name` is an interface: its class file's flags (a
    /// jar's own copy first, `class_precedence.rs`), else
    /// [`table_is_interface`]. `None` when neither knows the class.
    pub(crate) fn class_is_interface(&self, name: &str) -> Option<bool> {
        match self.class(name) {
            Some(file) => {
                crate::jdk_model::class_access_flags(file).map(|flags| flags & 0x0200 != 0)
            }
            None => table_is_interface(name),
        }
    }

    /// `Class.toString()` of the class `name` ([`class_text`]); `None`
    /// (counted) for a class the run neither holds nor finds in Java 25's
    /// table.
    pub(crate) fn class_object_text(&self, name: &str) -> Option<String> {
        class_text(name, self.class_is_interface(name))
    }

    /// `Class.getSimpleName()` of an array class (JDK 25: its element's
    /// simple name and one `[]` per dimension). A nested element's simple name
    /// is its `InnerClasses` row's, which only a held class file has: one the
    /// run does not hold, named with a `$`, is `None` (counted).
    pub(crate) fn array_simple_name(&self, name: &str) -> Option<String> {
        let (dimensions, element) = array_element(name)?;
        let simple = if is_primitive(&element) {
            element
        } else {
            let top_level = || element.rsplit('/').next().unwrap_or(&element).to_owned();
            match self.class(&element).map(ClassFile::nested_simple_name) {
                Some(Some(nested)) => nested.unwrap_or_default(),
                Some(None) => top_level(),
                None if element.contains('$') => return None,
                None => top_level(),
            }
        };
        Some(format!("{simple}{}", "[]".repeat(dimensions)))
    }
}

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
        assert_eq!(class_name(&Value::Ref(Some("demo/Foo".into()))), None);
        assert_eq!(class_name(&Value::Int(1)), None);
        assert_eq!(
            class_name(&class_value("[Ljava/lang/String;")),
            Some("[Ljava/lang/String;")
        );
    }

    /// `getName`/`getTypeName`/`toString` as JDK 25 prints them
    /// (`fixture_class_objects.rs` runs the same through the interpreter).
    #[test]
    fn names_and_text_follow_jdk_25() {
        assert_eq!(binary_name("[Ljava/lang/String;"), "[Ljava.lang.String;");
        assert_eq!(type_name("[[Ljava/lang/String;"), "java.lang.String[][]");
        assert_eq!(type_name("[I"), "int[]");
        assert_eq!(type_name("demo/Probe$Foo"), "demo.Probe$Foo");
        assert_eq!(class_text("int", None).as_deref(), Some("int"));
        assert_eq!(class_text("[I", None).as_deref(), Some("class [I"));
        assert_eq!(
            class_text("java/util/List", table_is_interface("java/util/List")).as_deref(),
            Some("interface java.util.List")
        );
        assert_eq!(class_text("demo/Unknown", None), None);
    }

    /// Text is never a `Class`, however it is spelled (#156 P13c).
    #[test]
    fn text_is_never_a_class() {
        for text in [
            "demo/Foo",
            "java/lang/String",
            "int",
            "[I",
            "java.lang.String",
        ] {
            let text = Value::Ref(Some(text.into()));
            assert_eq!(class_name(&text), None);
            assert!(!is_class(&text));
        }
        assert!(is_class(&class_value("demo/Foo")));
    }
}

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
//! * **P13a (done):** every reader of a `Class` value goes through
//!   [`class_name`] and [`is_class`], which accept a handle and, until P13c,
//!   the class-marker text every producer still answers. No answer changes.
//! * **P13b1 (done):** the producers config code compares with `X.class`
//!   answer handles: `ldc` (`Foo.class`), `getClass`,
//!   `Class.forName`/`ClassLoader.loadClass`, the wrappers' `TYPE`, the
//!   `Class` members that answer a class (`getSuperclass`, `getInterfaces`,
//!   `getComponentType`, `getEnclosingClass`), the reflective getters
//!   (`Field.getType`/`getDeclaringClass`, `Method.getReturnType`/
//!   `getParameterTypes`/`getDeclaringClass`), type nodes that are classes
//!   (`getGenericType`, `getRawType`, arguments, bounds,
//!   `getGenericDeclaration`), commons-lang's `ClassUtils` boxing,
//!   `ServiceLoader.Provider.type` and Fabric's `getEntrypointClass`; and,
//!   pulled forward from P13b2 by #215's review, annotation `Class` values
//!   and `annotationType`, `MethodType` return and parameter types,
//!   `StackWalker` caller and frame classes, a proxy's `getClass`, Guava's
//!   and Gson's `TypeToken` classes. Every `Class` a jar can obtain is a
//!   handle (#216 lists the members still counted). Until P13c `expect_ref`
//!   answers a handle's key, as it does a jar object's, so a reader not yet
//!   moved to [`class_name`] keeps the text it read.
//! * **P13c:** text is never a `Class`: a `String` spelled like a class name
//!   is that `String` everywhere, and [`class_name`] stops reading text.
//!
//! The row: `Class` is final and its supertypes are Java 25's
//! (`jdk_closure.rs`). `equals` is identity, exact between handles (one name,
//! one object) and against text of another name; against text of its own
//! name it is counted until P13c, as `==` is (a class marker may be the same
//! `Class`, a `String` spelled like it is not: [`handle_against_same_text`]). `hashCode` is an
//! identity hash (counted). `toString` (`class X`, `interface X`, `int`) needs
//! to know whether X is an interface: the row hands the handle back to the
//! caller's `Nested` (as `jar_object.rs` does), and the runtime answers from
//! the class file it holds or Java 25's table ([`class_text`]); a class
//! neither knows is counted.

use crate::runtime::host_ref::{HostKind, HostRef, Methods, instance_of_class};
use crate::runtime::{ClassFile, LinkedClassTable, Value};
use std::collections::BTreeMap;

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
    (dimensions > 0).then(|| (dimensions, crate::runtime::erased_field_type(&name[dimensions..])))
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
    crate::runtime::jdk_closure::class(name).map(|row| row.is_interface)
}

/// The `Class` handle `value` is: its name. Not class-marker text (that is
/// [`class_name`]'s, until #156 P13c).
pub(crate) fn handle_name(value: &Value) -> Option<&str> {
    match value {
        Value::Host(handle) if handle.kind() == HostKind::Class => Some(handle.key()),
        _ => None,
    }
}

/// A `Class` handle against class-marker text of the same name: one class's
/// object and a value a producer #156 P13b has not moved yet, or a `String`
/// spelled like the name. Until P13c no reader can tell which, so `==` and
/// `equals` between them are counted, never a silent `false`.
pub(crate) fn handle_against_same_text(a: &Value, b: &Value) -> bool {
    let same = |handle: &Value, text: &Value| matches!((handle_name(handle), text), (Some(name), Value::Ref(Some(text))) if name == text);
    same(a, b) || same(b, a)
}

/// The kind's `ClassRule::FromKey`: every key's class is `java.lang.Class`.
pub(crate) fn class_of_class(_: &str) -> &str {
    CLASS
}

/// The `Class` object of the class named `name` (internal form).
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
        Value::Ref(Some(text)) => crate::runtime::class_marker_like(text, classes, linked),
        _ => false,
    }
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
            // A class marker of the same name (text until P13c) may be this
            // `Class`, or a `String` spelled like it: counted. Any other text
            // is another class's marker or a `String`, never this object.
            Value::Ref(Some(text)) if text == handle.key() => None,
            _ => Some(false),
        })
    },
    instance_of: instance_of_class,
    // A JDK class is one object in every jar's runtime (the boot and
    // platform loaders); a jar's or a game's class is not known to be.
    crosses_jars: |handle, _| crate::runtime::jdk_closure::in_jdk_package(handle.key()),
    compare_to: None,
};

impl<H: crate::runtime::Host> crate::runtime::Runtime<'_, H> {
    /// Whether the class `name` is an interface: its class file's flags (a
    /// jar's own copy first, `class_precedence.rs`), else
    /// [`table_is_interface`]. `None` when neither knows the class.
    pub(crate) fn class_is_interface(&self, name: &str) -> Option<bool> {
        match self.class(name) {
            Some(file) => {
                crate::runtime::jdk_model::class_access_flags(file).map(|flags| flags & 0x0200 != 0)
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
        let text = Value::Ref(Some("demo/Foo".into()));
        assert_eq!(class_name(&text), Some("demo/Foo"));
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

    /// A handle against the same name's text is the one undecidable pair.
    #[test]
    fn a_handle_against_its_names_text_is_undecided() {
        let handle = class_value("demo/Foo");
        assert!(handle_against_same_text(
            &handle,
            &Value::Ref(Some("demo/Foo".into()))
        ));
        assert!(handle_against_same_text(
            &Value::Ref(Some("demo/Foo".into())),
            &handle
        ));
        assert!(!handle_against_same_text(
            &handle,
            &Value::Ref(Some("demo/Bar".into()))
        ));
        assert!(!handle_against_same_text(&handle, &class_value("demo/Foo")));
    }
}

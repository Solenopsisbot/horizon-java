//! Gson's `FormattingStyle` and a builder's options as host handles (#156 P5b). They were the
//! text `formatting-style/<newline>/<indent>/<0|1>` and `gson-style/<options>`, which a String
//! could spell: `setFormattingStyle` read any String of that shape as a style.
//!
//! **`FormattingStyle`** (gson 2.14.0, `javap -p`): a class with a private constructor, so every
//! instance is exactly a `FormattingStyle`; it extends `Object`, implements nothing and overrides
//! none of `equals`/`hashCode`/`toString` (identity; an identity hash and `Class@hash` text:
//! counted). `COMPACT` and `PRETTY` are the class's two constants, and each `withNewline`/
//! `withIndent`/`withSpaceAfterSeparators` makes a new object. The key (`HostRef::of_values`,
//! which escapes) is `[id, newline, indent, space]`, `id` the constant's name or a serial minted
//! per allocation: one key, one object. `instanceof` follows the other library classes: an
//! interface a loaded Mixin plan adds is `true`, another interface counted, a class other than
//! its own and `Object` a proved `false`.
//!
//! **Builder options** (`GsonOptions`): the newline, indent, flags and policies a `GsonBuilder`
//! or `Gson` was made with, kept in the object's synthetic `#gsonStyle` field (`gson_model.rs`).
//! No jar holds one: its class is unknown (`Object` declared) and only a handle of this kind is
//! read back, so a String written into that field is not an option set.

use std::sync::atomic::{AtomicU64, Ordering};

use crate::host_ref::{
    ClassHierarchy, ClassSide, HostKind, HostRef, Methods, Nested, exact_class_rules_out,
};
use crate::{NativeError, Value};

pub(crate) const FORMATTING_STYLE: &str = "com/google/gson/FormattingStyle";
const OBJECT: &str = "java/lang/Object";

static NEXT_STYLE: AtomicU64 = AtomicU64::new(0);

/// A `FormattingStyle`'s fields.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Style {
    pub(crate) newline: String,
    pub(crate) indent: String,
    pub(crate) space_after_separators: bool,
}

/// The handle of `style`: the constant `name` (`COMPACT`, `PRETTY`), or a new object.
pub(crate) fn formatting_style(style: &Style, constant: Option<&str>) -> Value {
    let id = match constant {
        Some(name) => Value::Ref(Some(name.to_owned())),
        None => Value::Long(NEXT_STYLE.fetch_add(1, Ordering::Relaxed) as i64),
    };
    Value::Host(HostRef::of_values(
        HostKind::FormattingStyle,
        &[
            id,
            Value::Ref(Some(style.newline.clone())),
            Value::Ref(Some(style.indent.clone())),
            Value::Int(i32::from(style.space_after_separators)),
        ],
    ))
}

/// The fields of a `FormattingStyle` handle, `None` for any other value.
pub(crate) fn style_of(value: &Value) -> Option<Style> {
    let Value::Host(handle) = value else {
        return None;
    };
    if handle.kind() != HostKind::FormattingStyle {
        return None;
    }
    match handle.values()?.as_slice() {
        [
            _,
            Value::Ref(Some(newline)),
            Value::Ref(Some(indent)),
            Value::Int(space),
        ] => Some(Style {
            newline: newline.clone(),
            indent: indent.clone(),
            space_after_separators: *space != 0,
        }),
        _ => None,
    }
}

/// A builder's option set as its handle; `key` is the runtime's own encoding (`gson_model.rs`:
/// hex text and digits).
pub(crate) fn options(key: String) -> Value {
    Value::Host(HostRef::new(HostKind::GsonOptions, key))
}

/// The key of an option-set handle, `None` for any other value.
pub(crate) fn options_key(value: &Value) -> Option<&str> {
    match value {
        Value::Host(handle) if handle.kind() == HostKind::GsonOptions => Some(handle.key()),
        _ => None,
    }
}

pub(crate) fn options_types(_: &str) -> &'static [&'static str] {
    &[OBJECT]
}

/// `class instanceof wanted` for an object of exactly `FormattingStyle`.
fn class_instance_of(class: &str, wanted: &str, hierarchy: &dyn ClassHierarchy) -> Option<bool> {
    if wanted == OBJECT || wanted == class || hierarchy.is_instance(class, wanted) == Some(true) {
        return Some(true);
    }
    (hierarchy.is_interface(wanted) == Some(false) || exact_class_rules_out(class, wanted))
        .then_some(false)
}

fn identity_equals(
    handle: &HostRef,
    other: &Value,
    _: &mut dyn Nested,
) -> Result<Option<bool>, NativeError> {
    Ok(match other {
        Value::Host(other) => handle.same_object(other),
        _ => Some(false),
    })
}

pub(crate) const STYLE_METHODS: Methods = Methods {
    to_string: |_, _| Ok(None),
    hash_code: |_, _| Ok(None),
    equals: identity_equals,
    instance_of: |_, wanted, hierarchy| class_instance_of(FORMATTING_STYLE, wanted, hierarchy),
    crosses_jars: |_, _| false,
    compare_to: None,
};

pub(crate) const STYLE_CLASS_SIDE: ClassSide = ClassSide {
    owns: |class| class == FORMATTING_STYLE,
    instance_of: class_instance_of,
    superclass: |_| Some(OBJECT.to_owned()),
    is_interface: |_| Some(false),
    classes: || vec![FORMATTING_STYLE.to_owned()],
};

#[cfg(test)]
#[path = "host_gson_tests.rs"]
mod tests;

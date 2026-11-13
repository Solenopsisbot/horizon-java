//! NeoForge's attachment types and their builders as host handles (#156 P8c), the objects
//! `AttachmentType.builder(..)`/`serializable(..)` and `Builder#build()` answer
//! (`neoforge_attachments.rs`). They were the text `nf-attachment/<class>/<n>`, which a String could
//! spell: `setData("nf-attachment/…/AttachmentType/0", v)` reached the type's store.
//!
//! **Class.** NeoForge 26.3.0.101 (`javap -p`): `AttachmentType` is `final` with a private
//! constructor, `AttachmentType$Builder` has a private constructor, so each handle's class is exact.
//! Both extend `Object` and implement nothing, and neither overrides `toString`/`hashCode`/`equals`
//! (an identity hash: counted; `equals` is identity). Both are NeoForge classes a Mixin may target:
//! `instanceof` an interface a loaded Mixin plan adds is `true`, another interface counted, a class
//! other than their own and `Object` a proved `false` (a Mixin adds no superclass). Their class side
//! (`host_ref::ClassSide`) answers the `Class` surface the same way (`owned_class.rs`).
//!
//! **Identity.** A key is a serial minted once per allocation, process-wide, so one key is one
//! object (canonical); a builder's methods answer the same handle (`return this`).

use std::sync::atomic::{AtomicU64, Ordering};

use crate::host_ref::{
    ClassHierarchy, ClassSide, HostKind, HostRef, Methods, Nested, exact_class_rules_out,
};
use crate::{NativeError, Value};

pub(crate) const TYPE: &str = "net/neoforged/neoforge/attachment/AttachmentType";
pub(crate) const BUILDER: &str = "net/neoforged/neoforge/attachment/AttachmentType$Builder";
const OBJECT: &str = "java/lang/Object";

static NEXT_SERIAL: AtomicU64 = AtomicU64::new(0);

/// A new handle of `kind` (an `AttachmentType` or a builder), its key a fresh serial.
pub(crate) fn mint(kind: HostKind) -> Value {
    let serial = NEXT_SERIAL.fetch_add(1, Ordering::Relaxed);
    Value::Host(HostRef::new(kind, serial.to_string()))
}

/// The handle of `kind` whose key is `key` (a table key `mint` gave): the same object, the kinds
/// being canonical.
pub(crate) fn handle(kind: HostKind, key: &str) -> Value {
    Value::Host(HostRef::new(kind, key))
}

/// The key of `value` when it is a handle of `kind`.
pub(crate) fn key_of(value: &Value, kind: HostKind) -> Option<&str> {
    match value {
        Value::Host(handle) if handle.kind() == kind => Some(handle.key()),
        _ => None,
    }
}

/// `class instanceof wanted` for an object of exactly `class` (`AttachmentType` or its builder).
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

fn unknown_text(_: &HostRef, _: &mut dyn Nested) -> Result<Option<String>, NativeError> {
    Ok(None)
}

fn unknown_hash(_: &HostRef, _: &mut dyn Nested) -> Result<Option<i32>, NativeError> {
    Ok(None)
}

fn stays_in_its_jar(_: &HostRef, _: &dyn Fn(&Value) -> bool) -> bool {
    false
}

pub(crate) const TYPE_METHODS: Methods = Methods {
    to_string: unknown_text,
    hash_code: unknown_hash,
    equals: identity_equals,
    instance_of: |_, wanted, hierarchy| class_instance_of(TYPE, wanted, hierarchy),
    crosses_jars: stays_in_its_jar,
    compare_to: None,
};

pub(crate) const BUILDER_METHODS: Methods = Methods {
    instance_of: |_, wanted, hierarchy| class_instance_of(BUILDER, wanted, hierarchy),
    ..TYPE_METHODS
};

pub(crate) fn type_class(_: &str) -> &str {
    TYPE
}

pub(crate) fn builder_class(_: &str) -> &str {
    BUILDER
}

fn object_superclass(_: &str) -> Option<String> {
    Some(OBJECT.to_owned())
}

fn a_class(_: &str) -> Option<bool> {
    Some(false)
}

pub(crate) const TYPE_CLASS_SIDE: ClassSide = ClassSide {
    owns: |class| class == TYPE,
    instance_of: class_instance_of,
    superclass: object_superclass,
    is_interface: a_class,
    classes: || vec![TYPE.to_owned()],
};

pub(crate) const BUILDER_CLASS_SIDE: ClassSide = ClassSide {
    owns: |class| class == BUILDER,
    classes: || vec![BUILDER.to_owned()],
    ..TYPE_CLASS_SIDE
};

#[cfg(test)]
#[path = "host_attachment_tests.rs"]
mod tests;

//! The config libraries' objects as host handles (#156 P7): cloth-config's
//! `AutoConfig` holders, YetAnotherConfigLib's handlers and their builders,
//! and SuperMartijn642's configlib builders and the values they define
//! (`client_config.rs` answers their calls).
//!
//! They used to be text a jar `String` could spell: `config-holder/<class>`,
//! `config-builder/<class>`, the bare word `configlib-builder` and
//! `configlib-value/<n>`, so the String `"config-holder/demo/Cfg"` was a
//! holder to `getConfig()`, and every configlib builder was one shared word.
//! Each is now a [`HostRef`]; a key holds jar text (a config class's name)
//! only through the escaping encoder, so no class name can spell another key.
//!
//! What each is on a JVM (`javap` of cloth-config 26.3.159,
//! yet_another_config_lib_v3 3.9.7, supermartijn642configlib 1.1.8a):
//!
//! | kind | key | surely a | identity |
//! |---|---|---|---|
//! | [`HostKind::AutoConfigHolder`] | the config class | `ConfigHolder` (a `ConfigManager`) | one per class: `AutoConfig` keeps them in a map |
//! | [`HostKind::YaclHandler`] | the config class, a serial | `ConfigClassHandler` (a `ConfigClassHandlerImpl`) | a fresh object per `build()` |
//! | [`HostKind::YaclBuilder`] | the config class, a serial | `ConfigClassHandler$Builder` | a fresh object per `createBuilder` |
//! | [`HostKind::ConfiglibBuilder`] | a serial | `IConfigBuilder` (a `TomlConfigBuilder`/`JsonConfigBuilder`) | a fresh object per `new*Config`; its steps return `this` |
//! | [`HostKind::ConfiglibValue`] | the value's index | `Supplier` (the library's value object) | a fresh object per `define` |
//!
//! Every key names exactly one object, so every kind is canonical: a fresh
//! object's key holds a serial from the runtime's object counter (which only
//! grows, kept with the resident heap), a value's its index among the values
//! the jar's statics keep, and a holder its class (`AutoConfig.register`
//! refuses a class twice). Two objects never share a key, so equal keys are
//! one object and different keys two, in `==`, `IdentityHashMap` and every
//! hash collection alike.
//!
//! The runtime classes are the libraries' implementation classes, which the
//! runtime does not model, so every kind is `ClassRule::Unknown`: `getClass()`
//! is counted, and `instanceof` is exact for the interface the call answered
//! and its supertypes. None overrides `toString`/`hashCode` (both identity
//! based, counted) or `equals` (identity: known through
//! [`HostRef::same_object`], counted otherwise). None crosses to another jar.

use crate::runtime::host_ref::{HostKind, HostRef, Methods, Nested, instance_of_class};
use crate::runtime::{NativeError, Value};

/// A handle of `kind` whose key holds `values` through the escaping encoder.
pub(crate) fn handle(kind: HostKind, values: &[Value]) -> Value {
    Value::Host(HostRef::of_values(kind, values))
}

/// A handle of `kind` for a config class (its internal name): `serial` tells
/// apart the objects of a kind that makes a new one per call (`0` for an
/// `AutoConfig` holder, one per class).
pub(crate) fn of_class(kind: HostKind, class: &str, serial: i32) -> Value {
    handle(
        kind,
        &[Value::Ref(Some(class.to_owned())), Value::Int(serial)],
    )
}

/// The config class a handle of one of `kinds` names.
pub(crate) fn class_of(value: &Value, kinds: &[HostKind]) -> Option<String> {
    match value {
        Value::Host(handle) if kinds.contains(&handle.kind()) => {
            match handle.values()?.as_slice() {
                [Value::Ref(Some(class)), Value::Int(_)] => Some(class.clone()),
                _ => None,
            }
        }
        _ => None,
    }
}

/// Whether `value` is a handle of `kind`.
pub(crate) fn is(value: &Value, kind: HostKind) -> bool {
    matches!(value, Value::Host(handle) if handle.kind() == kind)
}

/// The index a configlib value handle names.
pub(crate) fn configlib_index(value: &Value) -> Option<i32> {
    match value {
        Value::Host(handle) if handle.kind() == HostKind::ConfiglibValue => {
            match handle.values()?.as_slice() {
                [Value::Int(index)] => Some(*index),
                _ => None,
            }
        }
        _ => None,
    }
}

pub(crate) fn auto_config_types(_key: &str) -> &'static [&'static str] {
    &["me/shedaniel/autoconfig/ConfigHolder"]
}

pub(crate) fn yacl_handler_types(_key: &str) -> &'static [&'static str] {
    &["dev/isxander/yacl3/config/v2/api/ConfigClassHandler"]
}

pub(crate) fn yacl_builder_types(_key: &str) -> &'static [&'static str] {
    &["dev/isxander/yacl3/config/v2/api/ConfigClassHandler$Builder"]
}

pub(crate) fn configlib_builder_types(_key: &str) -> &'static [&'static str] {
    &["com/supermartijn642/configlib/api/IConfigBuilder"]
}

pub(crate) fn configlib_value_types(_key: &str) -> &'static [&'static str] {
    &["java/util/function/Supplier"]
}

/// `Object.equals`: the same object, where identity can tell.
fn identity_equals(
    handle: &HostRef,
    other: &Value,
    _nested: &mut dyn Nested,
) -> Result<Option<bool>, NativeError> {
    Ok(match other {
        Value::Host(other) => handle.same_object(other),
        _ => Some(false),
    })
}

/// The `Object` methods of every config kind: identity `toString` and
/// `hashCode` (counted), identity `equals`.
pub(crate) const CONFIG_METHODS: Methods = Methods {
    to_string: |_, _| Ok(None),
    hash_code: |_, _| Ok(None),
    equals: identity_equals,
    instance_of: instance_of_class,
    crosses_jars: |_, _| false,
    compare_to: None,
};

#[cfg(test)]
mod tests {
    use super::*;

    /// A config class's name reaches a key only through the escaping
    /// encoder: names with every separator read back as themselves and never
    /// alias another class's object.
    #[test]
    fn config_class_names_with_every_separator_never_alias() {
        let classes = [
            "demo/Cfg",
            "demo/Cfg,0",
            "demo/C\\fg",
            "hacfg:rdemo/Cfg",
            "i0",
            "",
            ",",
            "\\",
        ];
        let mut keys = std::collections::BTreeSet::new();
        for class in classes {
            for serial in [0, 1, 12] {
                let holder = of_class(HostKind::YaclBuilder, class, serial);
                let Value::Host(handle) = &holder else {
                    unreachable!()
                };
                assert!(keys.insert(handle.key().to_owned()), "{class:?} {serial}");
                assert_eq!(
                    class_of(&holder, &[HostKind::YaclBuilder]).as_deref(),
                    Some(class)
                );
                assert_eq!(class_of(&holder, &[HostKind::YaclHandler]), None);
            }
        }
        assert_eq!(
            class_of(
                &Value::Ref(Some("config-builder/demo/Cfg".into())),
                &[HostKind::YaclBuilder]
            ),
            None
        );
    }
}

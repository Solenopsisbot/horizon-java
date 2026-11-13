//! Mixin's objects as host handles (#156 P7): the `CallbackInfo` an injection
//! point hands its callbacks, the `ClassNode`/`IMixinInfo` stand-ins a config
//! plugin's `preApply`/`postApply` receives, `MixinEnvironment` and its
//! `Side` constants.
//!
//! They used to be text a jar `String` could spell
//! (`mixin/callback-info/<owner>#<name>`, `mixin-plugin/class-node/<class>`,
//! `mixin-plugin/mixin-info/<class>`, `mixin-environment`,
//! `mixin-side/<NAME>`): the String `"mixin/callback-info/demo/T#run"` was
//! that callback info to `cancel()`. A key holds jar text (a class or method
//! name) only through the escaping encoder (`HostRef::of_values`), so no name
//! can spell another object's key; the old marker joined owner and name with
//! `#`, so an owner `a#b` with name `c` and an owner `a` with name `b#c` were
//! one object.
//!
//! What each is on a JVM (`javap` of sponge-mixin 0.17.5+mixin.0.8.7, the
//! Mixin Fabric Loader 0.19.5 ships):
//!
//! | kind | key | surely a | identity |
//! |---|---|---|---|
//! | [`HostKind::CallbackInfo`] | target owner, name, returns a value | `CallbackInfoReturnable` for a target with a result, else `CallbackInfo`; both a `Cancellable` | a new object per injection point call |
//! | [`HostKind::MixinClassNode`] | the target class | `org.objectweb.asm.tree.ClassNode` | one per apply |
//! | [`HostKind::MixinInfo`] | the Mixin class | `IMixinInfo` (Mixin's `MixinInfo`) | one per Mixin |
//! | [`HostKind::MixinEnvironment`] | none | `MixinEnvironment` (final), an `ITokenProvider` | one per phase |
//! | [`HostKind::MixinSide`] | the constant's name | `MixinEnvironment$Side`, an `Enum` (each constant has a body: `Side$1`...) | one per constant |
//!
//! `CallbackInfo`/`CallbackInfoReturnable` are the classes Mixin constructs,
//! but an exact `getClass()` needs an exact `Class` surface (#174), so every
//! kind is `ClassRule::Unknown` for now: `getClass()` is counted, and
//! `instanceof` is exact for the declared types and their supertypes.
//! `CallbackInfo.toString()` prints its `cancellable` flag, which the handle
//! does not keep, and the others are identity based: all counted. `equals` is
//! identity (`Enum.equals` for a side). A side compares by ordinal
//! (`Enum.compareTo`).

use crate::runtime::host_ref::{HostKind, HostRef, Methods, Nested, instance_of_class};
use crate::runtime::{NativeError, Value};

const CALLBACK_INFO: &str = "org/spongepowered/asm/mixin/injection/callback/CallbackInfo";
const CALLBACK_INFO_RETURNABLE: &str =
    "org/spongepowered/asm/mixin/injection/callback/CallbackInfoReturnable";
const CANCELLABLE: &str = "org/spongepowered/asm/mixin/injection/callback/Cancellable";
/// `MixinEnvironment.Side`'s constants in declaration order (their ordinals).
pub(crate) const SIDES: [&str; 3] = ["UNKNOWN", "CLIENT", "SERVER"];

fn text(text: &str) -> Value {
    Value::Ref(Some(text.to_owned()))
}

fn values_of(value: &Value, kind: HostKind) -> Option<Vec<Value>> {
    match value {
        Value::Host(handle) if handle.kind() == kind => handle.values(),
        _ => None,
    }
}

/// The key of the `CallbackInfo` an injection point on `owner.name` hands
/// its callbacks; `returns` when the target has a result (Mixin then makes a
/// `CallbackInfoReturnable`). The runtime's callback-info table is keyed by it.
pub(crate) fn callback_info_key(owner: &str, name: &str, returns: bool) -> String {
    HostRef::of_values(
        HostKind::CallbackInfo,
        &[text(owner), text(name), Value::Int(i32::from(returns))],
    )
    .key()
    .to_owned()
}

/// A handle on the callback info with `key` ([`callback_info_key`]).
pub(crate) fn callback_info(key: &str) -> Value {
    Value::Host(HostRef::new(HostKind::CallbackInfo, key))
}

/// The table key of a callback info handle; `None` for any other value (a
/// woven class's own `CallbackInfo` is a jar object, and a string spelled
/// like the old marker is a string).
pub(crate) fn callback_info_key_of(value: &Value) -> Option<&str> {
    match value {
        Value::Host(handle) if handle.kind() == HostKind::CallbackInfo => Some(handle.key()),
        _ => None,
    }
}

pub(crate) fn callback_info_types(key: &str) -> &'static [&'static str] {
    let returns = HostRef::new(HostKind::CallbackInfo, key)
        .values()
        .is_some_and(|values| values.get(2) == Some(&Value::Int(1)));
    if returns {
        &[CALLBACK_INFO_RETURNABLE, CALLBACK_INFO, CANCELLABLE]
    } else {
        &[CALLBACK_INFO, CANCELLABLE]
    }
}

/// `instanceof` on a callback info: its declared types, and a proven `false`
/// for `CallbackInfoReturnable` on a plain `CallbackInfo` (Mixin builds
/// exactly one of the two classes, and a Mixin adds interfaces, never a
/// superclass); anything else as the declared types decide.
fn callback_info_instance_of(
    handle: &HostRef,
    wanted: &str,
    hierarchy: &dyn crate::runtime::host_ref::ClassHierarchy,
) -> Option<bool> {
    if wanted == CALLBACK_INFO_RETURNABLE {
        return Some(callback_info_types(handle.key()).contains(&wanted));
    }
    instance_of_class(handle, wanted, hierarchy)
}

/// A callback info's `Object` methods, and its `instanceof`.
pub(crate) const CALLBACK_INFO_METHODS: Methods = Methods {
    instance_of: callback_info_instance_of,
    ..MIXIN_METHODS
};

/// The `ClassNode` stand-in for `target` a plugin's apply hooks receive.
pub(crate) fn class_node(target: &str) -> Value {
    Value::Host(HostRef::of_values(
        HostKind::MixinClassNode,
        &[text(target)],
    ))
}

/// The `IMixinInfo` stand-in for `mixin`.
pub(crate) fn mixin_info(mixin: &str) -> Value {
    Value::Host(HostRef::of_values(HostKind::MixinInfo, &[text(mixin)]))
}

/// Whether `value` is a plugin apply hook's stand-in, on which every call is
/// refused (the hook tier applies no bytecode).
pub(crate) fn is_apply_stand_in(value: &Value) -> bool {
    matches!(value, Value::Host(handle)
        if matches!(handle.kind(), HostKind::MixinClassNode | HostKind::MixinInfo))
}

pub(crate) fn class_node_types(_key: &str) -> &'static [&'static str] {
    &["org/objectweb/asm/tree/ClassNode"]
}

pub(crate) fn mixin_info_types(_key: &str) -> &'static [&'static str] {
    &["org/spongepowered/asm/mixin/extensibility/IMixinInfo"]
}

/// The environment `getCurrentEnvironment()`/`getDefaultEnvironment()` answer.
pub(crate) fn environment() -> Value {
    Value::Host(HostRef::of_values(HostKind::MixinEnvironment, &[]))
}

pub(crate) fn is_environment(value: &Value) -> bool {
    matches!(value, Value::Host(handle) if handle.kind() == HostKind::MixinEnvironment)
}

pub(crate) fn environment_types(_key: &str) -> &'static [&'static str] {
    &[
        "org/spongepowered/asm/mixin/MixinEnvironment",
        "org/spongepowered/asm/util/ITokenProvider",
    ]
}

/// The `MixinEnvironment.Side` constant `name` (one of [`SIDES`]).
pub(crate) fn side(name: &str) -> Value {
    Value::Host(HostRef::of_values(HostKind::MixinSide, &[text(name)]))
}

/// The name of a side constant handle.
pub(crate) fn side_name(value: &Value) -> Option<String> {
    match values_of(value, HostKind::MixinSide)?.as_slice() {
        [Value::Ref(Some(name))] => Some(name.clone()),
        _ => None,
    }
}

fn side_ordinal(handle: &HostRef) -> Option<i32> {
    match handle.values()?.as_slice() {
        [Value::Ref(Some(name))] => SIDES.iter().position(|side| side == name).map(|i| i as i32),
        _ => None,
    }
}

pub(crate) fn side_types(_key: &str) -> &'static [&'static str] {
    &[
        "org/spongepowered/asm/mixin/MixinEnvironment$Side",
        "java/lang/Enum",
        "java/lang/Comparable",
        "java/io/Serializable",
        "java/lang/constant/Constable",
    ]
}

/// `Object.equals`/`Enum.equals`: the same object, where identity can tell.
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

/// Identity `toString`/`hashCode` (counted), identity `equals`.
pub(crate) const MIXIN_METHODS: Methods = Methods {
    to_string: |_, _| Ok(None),
    hash_code: |_, _| Ok(None),
    equals: identity_equals,
    instance_of: instance_of_class,
    crosses_jars: |_, _| false,
    compare_to: None,
};

/// A side: `Enum.toString()` is its name (no constant overrides it),
/// `hashCode` is the identity hash (counted), `compareTo` the ordinals'
/// difference against another side.
pub(crate) const SIDE_METHODS: Methods = Methods {
    to_string: |handle, _| {
        Ok(match handle.values().as_deref() {
            Some([Value::Ref(Some(name))]) => Some(name.clone()),
            _ => None,
        })
    },
    compare_to: Some(|handle, other, _| {
        Ok(match other {
            Value::Host(other) if other.kind() == HostKind::MixinSide => side_ordinal(handle)
                .zip(side_ordinal(other))
                .map(|(a, b)| a - b),
            _ => None,
        })
    }),
    ..MIXIN_METHODS
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::host_ref::JdkHierarchy;

    /// A callback info's key holds the target's owner and name through the
    /// escaping encoder: names containing every separator (`#` that the old
    /// marker joined them with, the encoder's `,` and `\`, a tag's `:`) each
    /// give their own key, never another target's.
    #[test]
    fn target_names_with_every_separator_never_alias() {
        let names = [
            "a", "a#b", "b#c", "c", "a,b", "a\\", "\\,", "hmci:r", "i1", ",i1", "", "#",
        ];
        let mut keys = std::collections::BTreeSet::new();
        for owner in names {
            for name in names {
                for returns in [false, true] {
                    let key = callback_info_key(owner, name, returns);
                    assert!(keys.insert(key.clone()), "{owner:?} {name:?} {returns}");
                    let parts = HostRef::new(HostKind::CallbackInfo, key.as_str())
                        .values()
                        .unwrap();
                    assert_eq!(
                        parts,
                        [text(owner), text(name), Value::Int(i32::from(returns))]
                    );
                    let handle = callback_info(&key);
                    assert_eq!(callback_info_key_of(&handle), Some(key.as_str()));
                    // The declared types follow the target's result only.
                    let Value::Host(info) = handle else {
                        unreachable!()
                    };
                    assert_eq!(
                        info.instance_of(CALLBACK_INFO_RETURNABLE, &JdkHierarchy),
                        Some(returns)
                    );
                }
            }
        }
        // A string spelled like a key is no callback info.
        let key = callback_info_key("demo/T", "run", false);
        assert_eq!(callback_info_key_of(&Value::Ref(Some(key))), None);
    }
}

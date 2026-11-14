//! Fabric Loader's and Fabric API's library singletons as host handles (#156
//! P6c): the loader itself, its `MappingResolver`, the `EnvType` constants,
//! the entrypoint containers and the entrypoint instances they hold, the
//! four `PayloadTypeRegistry`s and the `ResourceLoader`s. Their behaviour
//! stays in `lib.rs` and `jdk_model.rs`; this module is their keys and rows.
//!
//! These were text a jar `String` could spell: `fabric-loader`,
//! `fabric-mapping-resolver`, the plain `CLIENT`/`SERVER` of `EnvType` (any
//! `"SERVER"` string *was* `EnvType.SERVER`), `fabric-entrypoint/<kind>/<class>/<provider>`
//! (split back at `/`), `fabric-entrypoint-instance/<class>`,
//! `fabric-payload-types/<direction>` and `fabric-resource-loader/<kind>`.
//!
//! What Fabric Loader 0.19.5 and Fabric API 0.165.0 do (bytecode, and the
//! oracle in `tests/fixture_host_fabric_runtime.rs`):
//!
//! * **loader**: `FabricLoader.getInstance()` is `FabricLoaderImpl.INSTANCE`,
//!   a final class: one object, canonical (key `""`).
//! * **mapping resolver**: `getMappingResolver()` makes one
//!   `LazyMappingResolver` on first use and keeps it: canonical.
//! * **`EnvType`**: a final enum of `CLIENT`, `SERVER`, no constant bodies:
//!   one canonical key per constant; `toString` its name, `compareTo` the
//!   ordinals' difference, and its `instanceof` exact.
//! * **entrypoint container**: `getEntrypointContainers` builds a *new*
//!   `EntrypointContainerImpl` per entry on every call: not canonical (key
//!   the kind, class and provider; two calls' containers are counted).
//! * **entrypoint instance**: what `getEntrypoint()`/`getEntrypoints` hand
//!   out, an object of the jar's own entrypoint class, which
//!   `Entry.getOrCreate(type)` caches per requested type: the host keys it by
//!   class and provider without the type, so it is not canonical either.
//!   Its class is the jar's class: `instanceof` reads the jar's hierarchy.
//! * **payload type registry**: `PayloadTypeRegistry.clientboundPlay()` and
//!   its three siblings answer `PayloadTypeRegistryImpl`'s four static
//!   constants: canonical, keyed by direction.
//! * **resource loader**: `ResourceLoader.get(type)` is
//!   `IMPL_MAP.computeIfAbsent(type, …)`: one per `PackType`, canonical.
//!
//! None of the implementations overrides `equals`/`hashCode`/`toString`:
//! identity `equals`, identity hash and `Object.toString` counted (the host
//! used to print `ResourceLoader[SERVER_DATA]`, which no class prints). Every
//! class but `EnvType` is an implementation the host does not hold:
//! [`ClassRule::Unknown`], so `getClass()` is counted. Jar text (a class
//! name, a mod id) enters keys only through [`HostRef::of_values`].
//!
//! [`ClassRule::Unknown`]: crate::host_ref::ClassRule

use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};

use crate::host_ref::{ClassHierarchy, HostKind, HostRef, Methods, Nested, instance_of_class};
use crate::{NativeError, Value};

const ENV_TYPE: &str = "net/fabricmc/api/EnvType";
/// `EnvType`'s constants, in declaration order.
const ENV_TYPES: &[&str] = &["CLIENT", "SERVER"];

/// The four `PayloadTypeRegistry` factories and the direction each names.
pub(crate) const PAYLOAD_DIRECTIONS: &[(&str, &str)] = &[
    ("clientboundPlay", "playToClient"),
    ("serverboundPlay", "playToServer"),
    ("clientboundConfiguration", "configurationToClient"),
    ("serverboundConfiguration", "configurationToServer"),
];

/// `PackType`'s constants (the game's), each with its own `ResourceLoader`.
const PACK_TYPES: &[&str] = &["CLIENT_RESOURCES", "SERVER_DATA"];

fn text(value: &str) -> Value {
    Value::Ref(Some(value.to_owned()))
}

fn texts(value: &Value, kind: HostKind) -> Option<Vec<String>> {
    match value {
        Value::Host(handle) if handle.kind() == kind => handle
            .values()?
            .into_iter()
            .map(|value| match value {
                Value::Ref(Some(text)) => Some(text),
                _ => None,
            })
            .collect(),
        _ => None,
    }
}

fn single(value: &Value, kind: HostKind) -> Option<String> {
    match texts(value, kind)?.as_slice() {
        [one] => Some(one.clone()),
        _ => None,
    }
}

/// `FabricLoader.getInstance()`.
pub(crate) fn loader() -> Value {
    Value::Host(HostRef::new(HostKind::FabricLoaderInstance, ""))
}

/// `FabricLoader.getMappingResolver()`.
pub(crate) fn mapping_resolver() -> Value {
    Value::Host(HostRef::new(HostKind::FabricMappingResolver, ""))
}

/// The `EnvType` constant `name` (`CLIENT` or `SERVER`); `None` for any
/// other name.
pub(crate) fn env_type(name: &str) -> Option<Value> {
    ENV_TYPES
        .contains(&name)
        .then(|| Value::Host(HostRef::of_values(HostKind::FabricEnvType, &[text(name)])))
}

/// An `EnvType` handle's constant name.
pub(crate) fn env_type_name(value: &Value) -> Option<String> {
    single(value, HostKind::FabricEnvType)
}

/// An `EnvType` handle's ordinal.
pub(crate) fn env_type_ordinal(value: &Value) -> Option<i32> {
    let name = env_type_name(value)?;
    ENV_TYPES
        .iter()
        .position(|constant| *constant == name)
        .map(|ordinal| ordinal as i32)
}

/// `values()` of an enum whose constants are this module's handles.
pub(crate) fn enum_values(owner: &str) -> Option<Vec<Value>> {
    (owner == ENV_TYPE).then(|| ENV_TYPES.iter().filter_map(|name| env_type(name)).collect())
}

/// The entrypoint container for entry `class` (internal name) of kind
/// `kind`, provided by mod `provider`.
pub(crate) fn entrypoint_container(kind: &str, class: &str, provider: &str) -> Value {
    Value::Host(HostRef::of_values(
        HostKind::FabricEntrypointContainer,
        &[text(kind), text(class), text(provider)],
    ))
}

/// A container handle's kind, class and provider.
pub(crate) fn entrypoint_container_parts(value: &Value) -> Option<(String, String, String)> {
    match texts(value, HostKind::FabricEntrypointContainer)?.as_slice() {
        [kind, class, provider] => Some((kind.clone(), class.clone(), provider.clone())),
        _ => None,
    }
}

/// The entrypoint instance of `class` (internal name) that mod `provider`
/// declared.
pub(crate) fn entrypoint_instance(class: &str, provider: &str) -> Value {
    Value::Host(HostRef::of_values(
        HostKind::FabricEntrypointInstance,
        &[text(class), text(provider)],
    ))
}

/// The `PayloadTypeRegistry` of `direction` (`playToClient`, ...).
pub(crate) fn payload_types(direction: &str) -> Value {
    Value::Host(HostRef::of_values(
        HostKind::FabricPayloadTypes,
        &[text(direction)],
    ))
}

/// A payload type registry handle's direction.
pub(crate) fn payload_types_direction(value: &Value) -> Option<String> {
    single(value, HostKind::FabricPayloadTypes)
}

/// `ResourceLoader.get(type)` for the game's `PackType` constant `pack_type`
/// (a host enum marker): `None` for anything else.
pub(crate) fn resource_loader(pack_type: &Value) -> Option<Value> {
    let name = match pack_type {
        Value::Ref(Some(marker)) => {
            marker.strip_prefix("enum/net/minecraft/server/packs/PackType/")?
        }
        _ => return None,
    };
    PACK_TYPES.contains(&name).then(|| {
        Value::Host(HostRef::of_values(
            HostKind::FabricResourceLoader,
            &[text(name)],
        ))
    })
}

/// Whether `value` is a `ResourceLoader` handle.
pub(crate) fn is_resource_loader(value: &Value) -> bool {
    single(value, HostKind::FabricResourceLoader).is_some()
}

pub(crate) fn loader_types(_: &str) -> &'static [&'static str] {
    &[
        "net/fabricmc/loader/impl/FabricLoaderImpl",
        "net/fabricmc/loader/FabricLoader",
        "net/fabricmc/loader/api/FabricLoader",
    ]
}
pub(crate) fn mapping_resolver_types(_: &str) -> &'static [&'static str] {
    &["net/fabricmc/loader/api/MappingResolver"]
}
pub(crate) fn env_type_types(_: &str) -> &'static [&'static str] {
    &[
        ENV_TYPE,
        "java/lang/Enum",
        "java/lang/Comparable",
        "java/io/Serializable",
        "java/lang/constant/Constable",
    ]
}
pub(crate) fn entrypoint_container_types(_: &str) -> &'static [&'static str] {
    &["net/fabricmc/loader/api/entrypoint/EntrypointContainer"]
}
pub(crate) fn payload_types_types(_: &str) -> &'static [&'static str] {
    &["net/fabricmc/fabric/api/networking/v1/PayloadTypeRegistry"]
}
pub(crate) fn resource_loader_types(_: &str) -> &'static [&'static str] {
    &["net/fabricmc/fabric/api/resource/v1/ResourceLoader"]
}

/// [`crate::host_ref::ClassRule::Unknown`]'s declared types of an
/// entrypoint instance: its own class (the jar's, from the key), interned
/// once per class (a bounded set: the entrypoints the run declared).
pub(crate) fn entrypoint_instance_types(key: &str) -> &'static [&'static str] {
    static CLASSES: OnceLock<Mutex<HashSet<&'static [&'static str]>>> = OnceLock::new();
    let handle = Value::Host(HostRef::new(HostKind::FabricEntrypointInstance, key));
    let Some(class) = texts(&handle, HostKind::FabricEntrypointInstance)
        .and_then(|parts| parts.into_iter().next())
    else {
        return &[];
    };
    let mut classes = CLASSES
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(known) = classes.iter().find(|known| known[0] == class) {
        return known;
    }
    let interned: &'static str = Box::leak(class.into_boxed_str());
    let types: &'static [&'static str] = Box::leak(Box::new([interned]));
    classes.insert(types);
    types
}

/// `Object.equals`/`Enum.equals`: identity, as far as it is known.
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

/// `Object`'s `toString` and `hashCode` (counted), identity `equals`.
pub(crate) const OBJECT_METHODS: Methods = Methods {
    to_string: |_, _| Ok(None),
    hash_code: |_, _| Ok(None),
    equals: identity_equals,
    instance_of: instance_of_class,
    crosses_jars: |_, _| false,
    compare_to: None,
};

/// `instanceof` of an `EnvType`: exactly `EnvType` (final, no constant
/// bodies), whose supertypes are its declared types.
fn env_type_instance_of(_: &HostRef, wanted: &str, _: &dyn ClassHierarchy) -> Option<bool> {
    Some(wanted == "java/lang/Object" || env_type_types("").contains(&wanted))
}

/// `Enum`: `toString` the name, `compareTo` the ordinals' difference,
/// identity `equals`, an identity `hashCode` (counted).
pub(crate) const ENV_TYPE_METHODS: Methods = Methods {
    to_string: |handle, _| Ok(env_type_name(&Value::Host(handle.clone()))),
    instance_of: env_type_instance_of,
    compare_to: Some(|handle, other, _| {
        Ok(match other {
            Value::Host(other) if other.kind() == HostKind::FabricEnvType => {
                env_type_ordinal(&Value::Host(handle.clone()))
                    .zip(env_type_ordinal(&Value::Host(other.clone())))
                    .map(|(a, b)| a - b)
            }
            _ => None,
        })
    }),
    ..OBJECT_METHODS
};

#[cfg(test)]
mod tests {
    use super::*;

    /// Class names and mod ids (jar text) with every separator a layer uses
    /// read back as themselves and never alias: the old
    /// `fabric-entrypoint/<kind>/<class>/<provider>` text split at `/`, so a
    /// provider id with a `/` named another class ("keys never parse jar
    /// text", #156).
    #[test]
    fn jar_text_round_trips_and_never_aliases() {
        let alphabet = ["a", "/", ",", "\\", ":", "|", "#", "r", "fepc:", ""];
        let texts: Vec<String> = alphabet
            .iter()
            .flat_map(|a| alphabet.iter().map(move |b| format!("{a}{b}")))
            .collect();
        let mut seen = std::collections::BTreeMap::new();
        for (index, class) in texts.iter().enumerate() {
            let provider = &texts[(index * 7 + 3) % texts.len()];
            let container = entrypoint_container("main", class, provider);
            assert_eq!(
                entrypoint_container_parts(&container),
                Some(("main".into(), class.clone(), provider.clone()))
            );
            assert_eq!(
                payload_types_direction(&payload_types(class)),
                Some(class.clone())
            );
            let Value::Host(handle) = entrypoint_instance(class, provider) else {
                unreachable!()
            };
            assert_eq!(entrypoint_instance_types(handle.key()), [class.as_str()]);
            let Value::Host(handle) = container else {
                unreachable!()
            };
            let parts = (class.clone(), provider.clone());
            if let Some(other) = seen.insert(handle.key().to_owned(), parts.clone()) {
                assert_eq!(other, parts);
            }
        }
        for name in ENV_TYPES {
            assert_eq!(
                env_type_name(&env_type(name).unwrap()).as_deref(),
                Some(*name)
            );
        }
        assert_eq!(env_type("SERVERS"), None);
        assert_eq!(
            resource_loader(&text(
                "enum/net/minecraft/server/packs/PackType/SERVER_DATA/x"
            )),
            None
        );
    }
}

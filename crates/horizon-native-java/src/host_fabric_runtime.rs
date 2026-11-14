//! Fabric Loader's and Fabric API's library singletons as host handles (#156
//! P6c): the loader itself, its `MappingResolver`, the `EnvType` constants,
//! the entrypoint containers, the four `PayloadTypeRegistry`s and the
//! `ResourceLoader`s. Their behaviour
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
//! * **entrypoint container**: `getEntrypointContainers(key, type)` builds a
//!   *new* `EntrypointContainerImpl` per entry on every call (in a new
//!   `ArrayList`): not canonical, keyed by the kind, class, provider and the
//!   requested type, which its lazy `getEntrypoint()` checks.
//! * **entrypoint instance**: no kind of its own. `Entry.getOrCreate(type)`
//!   caches one instance per requested type, and the loader's own
//!   `invokeEntrypoints` made the one for the kind's initializer interface:
//!   that one is the object the runtime constructed and ran, so the host
//!   answers the runtime's own object (`lib.rs`), and counts any other type.
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
//! [`ClassRule::Unknown`], so `getClass()` is counted. `EnvType`'s class is
//! exact (`ClassRule::FromKey`), with its own class side: superclass
//! `java.lang.Enum`, not an interface, its supertypes. Jar text (a class
//! name, a mod id) enters keys only through [`HostRef::of_values`].
//!
//! [`ClassRule::Unknown`]: crate::host_ref::ClassRule

use crate::host_ref::{
    ClassHierarchy, ClassSide, HostKind, HostRef, Methods, Nested, instance_of_class,
};
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

/// One entrypoint container handle's facts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ContainerParts {
    pub(crate) kind: String,
    /// The entry as `fabric.mod.json` names it.
    pub(crate) class: String,
    pub(crate) provider: String,
    /// The type `getEntrypointContainers` was asked for (internal name).
    pub(crate) requested: Option<String>,
}

/// The entrypoint container for entry `class` of kind `kind`, provided by
/// mod `provider`, from a call that asked for `requested`.
pub(crate) fn entrypoint_container(parts: &ContainerParts) -> Value {
    Value::Host(HostRef::of_values(
        HostKind::FabricEntrypointContainer,
        &[
            text(&parts.kind),
            text(&parts.class),
            text(&parts.provider),
            Value::Ref(parts.requested.clone()),
        ],
    ))
}

/// A container handle's facts.
pub(crate) fn entrypoint_container_parts(value: &Value) -> Option<ContainerParts> {
    match value {
        Value::Host(handle) if handle.kind() == HostKind::FabricEntrypointContainer => {
            match handle.values()?.as_slice() {
                [
                    Value::Ref(Some(kind)),
                    Value::Ref(Some(class)),
                    Value::Ref(Some(provider)),
                    Value::Ref(requested),
                ] => Some(ContainerParts {
                    kind: kind.clone(),
                    class: class.clone(),
                    provider: provider.clone(),
                    requested: requested.clone(),
                }),
                _ => None,
            }
        }
        _ => None,
    }
}

/// The interface Fabric Loader runs entrypoint `kind` with
/// (`invokeEntrypoints(kind, type, …)`): the type whose cached instance is
/// the one the runtime constructed and ran.
pub(crate) fn invocation_type(kind: &str) -> Option<&'static str> {
    match kind {
        "main" => Some("net/fabricmc/api/ModInitializer"),
        "client" => Some("net/fabricmc/api/ClientModInitializer"),
        "server" => Some("net/fabricmc/api/DedicatedServerModInitializer"),
        _ => None,
    }
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
    &[
        "net/fabricmc/loader/impl/LazyMappingResolver",
        "net/fabricmc/loader/api/MappingResolver",
    ]
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

/// `instanceof` of the loader or its mapping resolver: their whole type is
/// known (`FabricLoaderImpl` is final; the loader makes its resolver with
/// `new LazyMappingResolver`; neither interface extends another, and Fabric
/// Loader's classes are no Mixin target), so the declared types are all.
fn exact_instance_of(handle: &HostRef, wanted: &str, _: &dyn ClassHierarchy) -> Option<bool> {
    let declared = match handle.kind() {
        HostKind::FabricLoaderInstance => loader_types(""),
        _ => mapping_resolver_types(""),
    };
    Some(wanted == "java/lang/Object" || declared.contains(&wanted))
}

/// The loader's and its resolver's rows: `Object`'s methods, exact
/// `instanceof`.
pub(crate) const EXACT_OBJECT_METHODS: Methods = Methods {
    instance_of: exact_instance_of,
    ..OBJECT_METHODS
};

/// `instanceof` of an `EnvType`: exactly `EnvType` (final, no constant
/// bodies), whose supertypes are its declared types.
fn env_type_instance_of(_: &HostRef, wanted: &str, _: &dyn ClassHierarchy) -> Option<bool> {
    Some(wanted == "java/lang/Object" || env_type_types("").contains(&wanted))
}

/// [`crate::host_ref::ClassRule::FromKey`] of an `EnvType`: always `EnvType`.
pub(crate) fn env_type_class(_: &str) -> &str {
    ENV_TYPE
}

/// `EnvType`'s class side: a final enum class whose superclass is
/// `java.lang.Enum` and whose supertypes are `Enum`'s.
pub(crate) const ENV_TYPE_CLASS_SIDE: ClassSide = ClassSide {
    owns: |class| class == ENV_TYPE,
    instance_of: |class, wanted, _| {
        (class == ENV_TYPE)
            .then(|| wanted == "java/lang/Object" || env_type_types("").contains(&wanted))
    },
    superclass: |class| (class == ENV_TYPE).then(|| "java/lang/Enum".to_owned()),
    is_interface: |class| (class == ENV_TYPE).then_some(false),
    classes: || vec![ENV_TYPE.to_owned()],
};

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
            let parts = ContainerParts {
                kind: "main".into(),
                class: class.clone(),
                provider: provider.clone(),
                requested: (index % 2 == 0).then(|| provider.clone()),
            };
            let container = entrypoint_container(&parts);
            assert_eq!(entrypoint_container_parts(&container), Some(parts.clone()));
            assert_eq!(
                payload_types_direction(&payload_types(class)),
                Some(class.clone())
            );
            let Value::Host(handle) = container else {
                unreachable!()
            };
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

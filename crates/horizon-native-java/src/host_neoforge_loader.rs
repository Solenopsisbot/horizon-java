//! NeoForge's loader objects as host handles (#156 P6, second half): a mod's
//! `ModContainer`, its `IModInfo`, a `ModFileInfo`, the `ModConfig.Type`
//! constants and the game's detected `WorldVersion`. Their behaviour stays in
//! `neoforge_loader.rs`, `neoforge_lifecycle.rs` and `mod_config_register.rs`;
//! this module is their keys and rows.
//!
//! These were text a jar `String` could spell: `neoforge-mod-container/<id>/<version>`,
//! `mod-info/<id>/<version>` (both split back at the first `/`, so an id with
//! a `/` named another mod), `mod-file-info/<id>`, `fml-config-type/<NAME>`
//! and `world-version/<name>`.
//!
//! What FancyModLoader 12.0.12 and the 26.3 game do (bytecode, and the
//! oracle on the real classes in `tests/fixture_host_neoforge_loader.rs`):
//!
//! * **container**: `ModList.getModContainerById(id)` is
//!   `Optional.ofNullable(indexedMods.get(id))`, so there is one container
//!   per mod, the one FML also hands the `@Mod` constructor and each loading
//!   event: canonical, keyed by the id. Its class is `FMLModContainer` for a
//!   `javafml` mod, `MinecraftModContainer` for `minecraft` (the
//!   `mclanguageprovider` builds it, a `ModContainer` and nothing more), and
//!   unknown otherwise; that class is part of the key, so the runtime's
//!   `instanceof` reads it from the handle. `ModContainer` keeps `Object`'s
//!   `equals`/`hashCode`/`toString`.
//! * **mod info**: `ModContainer.getModInfo()` is the container's final
//!   `modInfo` field: one `ModInfo` per mod, canonical, keyed by the id.
//! * **mod file info**: `LoadingModList.getModFileById(id)` is
//!   `fileById.get(id)`, the one `ModFileInfo` of the *file* holding mod
//!   `id`: two mods of one file share it. The key is the file, named by its
//!   first `[[mods]]` entry: canonical. The host knows the files of the
//!   running jar, the linked NeoForge jars, `minecraft` and `neoforge`; a
//!   lookup of a mod it only assumes present (a dependency) is counted, never
//!   given a handle of its own.
//! * **config type**: `ModConfig.Type` is a final enum of `LOCAL`, `CLIENT`,
//!   `SYNCED`, `STARTUP` (in that order). One canonical key per constant;
//!   `toString` its name, `compareTo` the ordinals' difference. The host also
//!   answers the `COMMON` and `SERVER` constants older NeoForge declared,
//!   whose ordinals 12.0.12 does not know: their comparison is counted.
//! * **world version**: `DetectedVersion.tryDetectVersion()` parses
//!   `/version.json` into a new `WorldVersion.Simple` record each call: not
//!   canonical (two detections are not `==`), but record-equal, so two
//!   detections in one run (one `version.json`, the seeded game version) are
//!   `equals`. Its `toString` and `hashCode` cover every component (a
//!   `Date`, pack formats): counted.
//!
//! Every class is an implementation the host does not hold, so every kind is
//! [`ClassRule::Unknown`]: `getClass()` is counted. Ids and names enter keys
//! only through [`HostRef::of_values`].
//!
//! [`ClassRule::Unknown`]: crate::host_ref::ClassRule

use crate::host_ref::{ClassHierarchy, HostKind, HostRef, Methods, Nested, instance_of_class};
use crate::{NativeError, Value};

const MOD_CONTAINER: &str = "net/neoforged/fml/ModContainer";
const FML_MOD_CONTAINER: &str = "net/neoforged/fml/javafmlmod/FMLModContainer";
const MINECRAFT_MOD_CONTAINER: &str = "net/neoforged/fml/mclanguageprovider/MinecraftModContainer";
const CONFIG_TYPE: &str = "net/neoforged/fml/config/ModConfig$Type";

/// `ModConfig.Type`'s constants in 12.0.12, in declaration order.
const CONFIG_TYPES: &[&str] = &["LOCAL", "CLIENT", "SYNCED", "STARTUP"];
/// The constants a config registration may name: 12.0.12's and the
/// `COMMON`/`SERVER` of older NeoForge.
pub(crate) const CONFIG_TYPE_NAMES: &[&str] =
    &["LOCAL", "CLIENT", "SYNCED", "STARTUP", "COMMON", "SERVER"];

/// What a container's runtime class is known to be.
const UNKNOWN_LOADER: i32 = 0;
const JAVAFML: i32 = 1;
const MINECRAFT: i32 = 2;

fn text(value: &str) -> Value {
    Value::Ref(Some(value.to_owned()))
}

fn values_of(value: &Value, kind: HostKind) -> Option<Vec<Value>> {
    match value {
        Value::Host(handle) if handle.kind() == kind => handle.values(),
        _ => None,
    }
}

/// The one text a one-value handle of `kind` holds.
fn held_text(value: &Value, kind: HostKind) -> Option<String> {
    match values_of(value, kind)?.as_slice() {
        [Value::Ref(Some(held))] => Some(held.clone()),
        _ => None,
    }
}

/// Mod `id`'s container; `javafml` when the running jar's
/// `neoforge.mods.toml` declares `id` under `modLoader="javafml"`.
pub(crate) fn container(id: &str, javafml: bool) -> Value {
    let class = if id == "minecraft" {
        MINECRAFT
    } else if javafml {
        JAVAFML
    } else {
        UNKNOWN_LOADER
    };
    Value::Host(HostRef::of_values(
        HostKind::NeoForgeModContainer,
        &[text(id), Value::Int(class)],
    ))
}

/// [`container`] for an embedding or a test that hands a jar its container
/// (FML's own handle for mod `id`).
pub fn neoforge_mod_container(id: &str, javafml: bool) -> Value {
    container(id, javafml)
}

/// A container handle's mod id.
pub(crate) fn container_id(value: &Value) -> Option<String> {
    match values_of(value, HostKind::NeoForgeModContainer)?.as_slice() {
        [Value::Ref(Some(id)), Value::Int(_)] => Some(id.clone()),
        _ => None,
    }
}

fn container_class(handle: &HostRef) -> Option<i32> {
    match handle.values()?.as_slice() {
        [_, Value::Int(class)] => Some(*class),
        _ => None,
    }
}

pub(crate) fn container_types(key: &str) -> &'static [&'static str] {
    match container_class(&HostRef::new(HostKind::NeoForgeModContainer, key)) {
        Some(JAVAFML) => &[FML_MOD_CONTAINER, MOD_CONTAINER],
        Some(MINECRAFT) => &[MINECRAFT_MOD_CONTAINER, MOD_CONTAINER],
        _ => &[MOD_CONTAINER],
    }
}

/// `instanceof` of a container. FML builds the `minecraft` container as a
/// `MinecraftModContainer` and a `javafml` mod's as an `FMLModContainer`
/// itself (`new`, never a subclass); each extends `ModContainer` and
/// implements nothing, and FML's classes are not Mixin targets. So their
/// whole type is known and anything else is a proved `false`. Any other
/// container answers from its declared types.
fn container_instance_of(
    handle: &HostRef,
    wanted: &str,
    hierarchy: &dyn ClassHierarchy,
) -> Option<bool> {
    let exact = match container_class(handle) {
        Some(MINECRAFT) => MINECRAFT_MOD_CONTAINER,
        Some(JAVAFML) => FML_MOD_CONTAINER,
        _ => return loader_instance_of(handle, wanted, hierarchy),
    };
    // A legacy Forge run hands out the same handle from `net/minecraftforge`'s
    // `ModList`: its loader's names are not FML 12's to rule out.
    if wanted.starts_with("net/minecraftforge/") {
        return instance_of_class(handle, wanted, hierarchy);
    }
    Some(matches!(wanted, MOD_CONTAINER | "java/lang/Object") || wanted == exact)
}

/// `ModConfig.Type` is a final enum whose only instances are its constants
/// (config type handles): no other object is one.
fn not_a_config_type(wanted: &str) -> Option<bool> {
    (wanted == CONFIG_TYPE).then_some(false)
}

/// `instanceof` of any other loader object: never a `ModConfig.Type`, else
/// its declared types.
fn loader_instance_of(
    handle: &HostRef,
    wanted: &str,
    hierarchy: &dyn ClassHierarchy,
) -> Option<bool> {
    not_a_config_type(wanted).or_else(|| instance_of_class(handle, wanted, hierarchy))
}

/// `instanceof` of a config type: exactly `ModConfig.Type` (final, no
/// constant bodies), whose supertypes are its declared types.
fn config_type_instance_of(_: &HostRef, wanted: &str, _: &dyn ClassHierarchy) -> Option<bool> {
    Some(wanted == "java/lang/Object" || config_type_types("").contains(&wanted))
}

/// Mod `id`'s `IModInfo` (`ModContainer.getModInfo`).
pub(crate) fn mod_info(id: &str) -> Value {
    Value::Host(HostRef::of_values(HostKind::NeoForgeModInfo, &[text(id)]))
}

/// A mod info handle's mod id.
pub(crate) fn mod_info_id(value: &Value) -> Option<String> {
    held_text(value, HostKind::NeoForgeModInfo)
}

pub(crate) fn mod_info_types(_: &str) -> &'static [&'static str] {
    &[
        "net/neoforged/neoforgespi/language/IModInfo",
        "net/neoforged/neoforgespi/language/IConfigurable",
    ]
}

/// The `ModFileInfo` of the mod file whose first mod is `id`.
pub(crate) fn mod_file_info(id: &str) -> Value {
    Value::Host(HostRef::of_values(
        HostKind::NeoForgeModFileInfo,
        &[text(id)],
    ))
}

/// The first mod of a file info handle's file.
pub(crate) fn mod_file_info_id(value: &Value) -> Option<String> {
    held_text(value, HostKind::NeoForgeModFileInfo)
}

pub(crate) fn mod_file_info_types(_: &str) -> &'static [&'static str] {
    &[
        "net/neoforged/fml/loading/moddiscovery/ModFileInfo",
        "net/neoforged/neoforgespi/language/IModFileInfo",
        "net/neoforged/neoforgespi/language/IConfigurable",
    ]
}

/// The `ModConfig.Type` constant `name` (one of [`CONFIG_TYPE_NAMES`]).
pub(crate) fn config_type(name: &str) -> Value {
    Value::Host(HostRef::of_values(
        HostKind::NeoForgeConfigType,
        &[text(name)],
    ))
}

/// `values()` of a loader enum whose constants are handles: `ModConfig.Type`'s
/// four, in declaration order.
pub(crate) fn enum_values(owner: &str) -> Option<Vec<Value>> {
    (owner == CONFIG_TYPE).then(|| CONFIG_TYPES.iter().map(|name| config_type(name)).collect())
}

/// A config type handle's constant name.
pub(crate) fn config_type_name(value: &Value) -> Option<String> {
    held_text(value, HostKind::NeoForgeConfigType)
}

/// A config type's ordinal in 12.0.12, `None` for `COMMON`/`SERVER`.
pub(crate) fn config_type_ordinal(value: &Value) -> Option<i32> {
    let name = config_type_name(value)?;
    CONFIG_TYPES
        .iter()
        .position(|constant| *constant == name)
        .map(|ordinal| ordinal as i32)
}

pub(crate) fn config_type_types(_: &str) -> &'static [&'static str] {
    &[
        CONFIG_TYPE,
        "java/lang/Enum",
        "java/lang/Comparable",
        "java/io/Serializable",
        "java/lang/constant/Constable",
    ]
}

/// A detected `WorldVersion` whose `name()` is `name`.
pub(crate) fn world_version(name: &str) -> Value {
    Value::Host(HostRef::of_values(HostKind::WorldVersion, &[text(name)]))
}

/// A world version handle's name.
pub(crate) fn world_version_name(value: &Value) -> Option<String> {
    held_text(value, HostKind::WorldVersion)
}

pub(crate) fn world_version_types(_: &str) -> &'static [&'static str] {
    &["net/minecraft/WorldVersion"]
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

/// `Object`'s `toString` (a class name and an identity hash) and `hashCode`:
/// counted; identity `equals`.
pub(crate) const OBJECT_METHODS: Methods = Methods {
    to_string: |_, _| Ok(None),
    hash_code: |_, _| Ok(None),
    equals: identity_equals,
    instance_of: loader_instance_of,
    // The loader's objects belong to the run that asked for them.
    crosses_jars: |_, _| false,
    compare_to: None,
};

pub(crate) const CONTAINER_METHODS: Methods = Methods {
    instance_of: container_instance_of,
    ..OBJECT_METHODS
};

/// `Enum`: `toString` the name, `compareTo` the ordinals' difference,
/// identity `equals`, an identity `hashCode` (counted).
pub(crate) const CONFIG_TYPE_METHODS: Methods = Methods {
    instance_of: config_type_instance_of,
    to_string: |handle, _| Ok(config_type_name(&Value::Host(handle.clone()))),
    compare_to: Some(|handle, other, _| {
        Ok(match other {
            Value::Host(other) if other.kind() == HostKind::NeoForgeConfigType => {
                config_type_ordinal(&Value::Host(handle.clone()))
                    .zip(config_type_ordinal(&Value::Host(other.clone())))
                    .map(|(a, b)| a - b)
            }
            _ => None,
        })
    }),
    ..OBJECT_METHODS
};

/// `WorldVersion.Simple`'s record `equals`: every detection in a run parses
/// the one `version.json`, so two detections are equal; a different name is
/// a different record.
pub(crate) const WORLD_VERSION_METHODS: Methods = Methods {
    equals: |handle, other, _| {
        Ok(Some(match other {
            Value::Host(other) => other.kind() == HostKind::WorldVersion && other == handle,
            _ => false,
        }))
    },
    ..OBJECT_METHODS
};

#[cfg(test)]
mod tests {
    use super::*;

    /// Ids and names (jar text) with every separator a layer uses read back
    /// as themselves, and two different ids never make one key: the old
    /// `neoforge-mod-container/<id>/<version>` text split at the first `/`
    /// ("keys never parse jar text", #156).
    #[test]
    fn ids_round_trip_and_never_alias() {
        let alphabet = ["a", "/", ",", "\\", ":", "|", "#", "r", "nfmc:", ""];
        let texts: Vec<String> = alphabet
            .iter()
            .flat_map(|a| alphabet.iter().map(move |b| format!("{a}{b}")))
            .collect();
        let mut seen = std::collections::BTreeMap::new();
        for id in &texts {
            for javafml in [false, true] {
                assert_eq!(container_id(&container(id, javafml)), Some(id.clone()));
            }
            assert_eq!(mod_info_id(&mod_info(id)), Some(id.clone()));
            assert_eq!(mod_file_info_id(&mod_file_info(id)), Some(id.clone()));
            assert_eq!(world_version_name(&world_version(id)), Some(id.clone()));
            let Value::Host(handle) = container(id, true) else {
                panic!("{id:?}")
            };
            if let Some(other) = seen.insert(handle.key().to_owned(), id.clone()) {
                assert_eq!(&other, id);
            }
        }
        for name in CONFIG_TYPE_NAMES {
            assert_eq!(config_type_name(&config_type(name)).as_deref(), Some(*name));
        }
    }
}

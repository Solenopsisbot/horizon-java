//! Registries and registry holders as host handles (#156 P4, second half):
//! `Registry`, a NeoForge/Forge registry holder, and the value a holder
//! registered. Their rows are in the kind table (`host_ref.rs`); the
//! expected values are the real 26.3 classes' on Java 25
//! (`tests/fixture_host_registries.rs`).
//!
//! These were text a jar `String` could spell: `registry/<id>` was both a
//! registry (`BuiltInRegistries.ITEM`) and its key (`Registries.ITEM`), and
//! `holder/<register>/<name>`, `registered/<register>/<name>` named a holder
//! and its value by joining the register's text with the entry's name. A
//! registry key is now a `ResourceKey` handle in `minecraft:root`
//! (`host_ids.rs`), and these three are handles:
//!
//! * `Registry`: key the registry's id. One object per registry key, so
//!   canonical. Its runtime class is an implementation (`MappedRegistry`,
//!   `DefaultedMappedRegistry`, a loader's own), so the class is unknown
//!   ([`ClassRule::Unknown`](crate::host_ref::ClassRule)): `getClass()` is
//!   counted, `instanceof` answers from the types it surely is. A vanilla
//!   built-in registry is a `MappedRegistry` (a `DefaultedRegistry` for the
//!   ten defaulted ones); any other only a `Registry`.
//! * holder: a `DeferredHolder` (NeoForge) or `RegistryObject` (Forge) one
//!   `DeferredRegister.register` call made. Key: [`HostRef::of_values`] of the
//!   loader, the registration's serial and the entry's `ResourceKey` (or
//!   `null` when the host could not name it). Each `register` makes one
//!   holder object and `getEntries` hands back those same objects, so the
//!   serial names one allocation: canonical. `equals` is identity (26.3:
//!   two holders for one id are not equal), `hashCode` an identity hash.
//! * registered: the value a holder's supplier makes, which the host does
//!   not run: an object of no known class, key the registration's serial.
//!   One per registration (`get()` answers the same object): canonical.
//!
//! The facts the host recorded for a holder (its register, entry name and
//! value) live in `StdHost::holder_rows`, by serial; nothing is parsed back
//! out of a key's text.

use crate::host_ref::{HostKind, HostRef, Methods, Nested, instance_of_class};
use crate::{NativeError, Value};

/// The types every `Registry` surely is (Java 25's closure of the 26.3
/// interface).
const REGISTRY_TYPES: &[&str] = &[
    "net/minecraft/core/Registry",
    "net/minecraft/core/IdMap",
    "java/lang/Iterable",
    "com/mojang/serialization/Keyable",
    "net/minecraft/core/HolderLookup$RegistryLookup",
    "net/minecraft/core/HolderLookup",
    "net/minecraft/core/HolderGetter",
    "net/minecraft/core/HolderOwner",
];

/// A vanilla built-in registry: a `MappedRegistry`, so a `WritableRegistry`.
const BUILT_IN_TYPES: &[&str] = &[
    "net/minecraft/core/MappedRegistry",
    "net/minecraft/core/WritableRegistry",
    "net/minecraft/core/Registry",
    "net/minecraft/core/IdMap",
    "java/lang/Iterable",
    "com/mojang/serialization/Keyable",
    "net/minecraft/core/HolderLookup$RegistryLookup",
    "net/minecraft/core/HolderLookup",
    "net/minecraft/core/HolderGetter",
    "net/minecraft/core/HolderOwner",
];

/// One of the ten `BuiltInRegistries` fields typed `DefaultedRegistry`.
const DEFAULTED_TYPES: &[&str] = &[
    "net/minecraft/core/DefaultedRegistry",
    "net/minecraft/core/MappedRegistry",
    "net/minecraft/core/WritableRegistry",
    "net/minecraft/core/Registry",
    "net/minecraft/core/IdMap",
    "java/lang/Iterable",
    "com/mojang/serialization/Keyable",
    "net/minecraft/core/HolderLookup$RegistryLookup",
    "net/minecraft/core/HolderLookup",
    "net/minecraft/core/HolderGetter",
    "net/minecraft/core/HolderOwner",
];

/// The 26.3 `BuiltInRegistries` fields of type `DefaultedRegistry`, by id.
const DEFAULTED: &[&str] = &[
    "minecraft:game_event",
    "minecraft:fluid",
    "minecraft:block",
    "minecraft:entity_type",
    "minecraft:item",
    "minecraft:chunk_status",
    "minecraft:villager_type",
    "minecraft:villager_profession",
    "minecraft:memory_module_type",
    "minecraft:sensor_type",
];

/// A NeoForge `DeferredHolder` (Java 25's closure).
const DEFERRED_HOLDER_TYPES: &[&str] = &[
    "net/neoforged/neoforge/registries/DeferredHolder",
    "net/minecraft/core/Holder$Reference",
    "net/minecraft/core/Holder",
    "java/util/function/Supplier",
];

/// A Forge `RegistryObject` (no Forge jar on hand: only the interface its
/// callers use).
const REGISTRY_OBJECT_TYPES: &[&str] = &[
    "net/minecraftforge/registries/RegistryObject",
    "java/util/function/Supplier",
];

/// Game classes no registry is (a `MappedRegistry` neither extends nor is
/// extended by them; single inheritance does the rest).
const NOT_MAPPED_REGISTRY: &[&str] = &[
    crate::host_ids::IDENTIFIER,
    crate::host_ids::RESOURCE_KEY,
    crate::host_ids::TAG_KEY,
    crate::host_ids::BLOCK_ITEM_ID,
    "net/minecraft/core/Holder$Reference",
    "net/neoforged/neoforge/registries/DeferredHolder",
    "net/minecraftforge/registries/RegistryObject",
];

/// Game classes no `DeferredHolder` is (`Holder.Reference`'s line).
const NOT_DEFERRED_HOLDER: &[&str] = &[
    crate::host_ids::IDENTIFIER,
    crate::host_ids::RESOURCE_KEY,
    crate::host_ids::TAG_KEY,
    crate::host_ids::BLOCK_ITEM_ID,
    "net/minecraft/core/MappedRegistry",
    "net/minecraft/core/DefaultedMappedRegistry",
    "net/minecraftforge/registries/RegistryObject",
];

/// [`instance_of_class`] (#163: `true` from the declared types, `false` only
/// when proved, counted otherwise), and one more proof: a declared *class*
/// (`MappedRegistry`, `DeferredHolder`) rules out a game class that is not
/// in its line. An interface stays unknown: another jar's Mixin may add it.
fn declared_instance_of(
    handle: &HostRef,
    wanted: &str,
    hierarchy: &dyn crate::host_ref::ClassHierarchy,
    class: &str,
    unrelated: &[&str],
) -> Option<bool> {
    instance_of_class(handle, wanted, hierarchy).or_else(|| {
        let declared = match handle.kind() {
            HostKind::Registry => registry_types(handle.key()),
            _ => holder_types(handle.key()),
        };
        (declared.contains(&class) && unrelated.contains(&wanted)).then_some(false)
    })
}

/// `BuiltInRegistries.X`, a jar's built registry: the `Registry` for `id`.
pub(crate) fn registry(id: &str) -> Value {
    Value::Host(HostRef::new(HostKind::Registry, id))
}

/// The id of a `Registry` handle.
pub(crate) fn registry_id(value: &Value) -> Option<&str> {
    match value {
        Value::Host(handle) if handle.kind() == HostKind::Registry => Some(handle.key()),
        _ => None,
    }
}

/// The declared types of a `Registry` with key `id`.
pub(crate) fn registry_types(id: &str) -> &'static [&'static str] {
    if DEFAULTED.contains(&id) {
        DEFAULTED_TYPES
    } else if crate::vanilla_registries::is_built_in(id) {
        BUILT_IN_TYPES
    } else {
        REGISTRY_TYPES
    }
}

/// Which loader's holder class a holder is.
const NEOFORGE: i32 = 0;
const FORGE: i32 = 1;

/// The holder of registration `serial`, with the entry's key when known.
fn holder(forge: bool, serial: usize, key: &Value) -> Value {
    Value::Host(HostRef::of_values(
        HostKind::Holder,
        &[
            Value::Int(if forge { FORGE } else { NEOFORGE }),
            Value::Int(serial as i32),
            key.clone(),
        ],
    ))
}

/// The value registration `serial` registered.
fn registered(serial: usize) -> Value {
    Value::Host(HostRef::new(HostKind::Registered, serial.to_string()))
}

/// A holder handle's parts: whether it is Forge's, its serial, its key.
fn holder_parts(handle: &HostRef) -> Option<(bool, usize, Value)> {
    match handle.values()?.as_slice() {
        [Value::Int(loader), Value::Int(serial), key] => Some((
            *loader == FORGE,
            usize::try_from(*serial).ok()?,
            key.clone(),
        )),
        _ => None,
    }
}

/// The registration serial a holder or registered value names: what a
/// caller keys its own tables by (the NeoForge attachment bridge does).
pub(crate) fn holder_serial(value: &Value) -> Option<usize> {
    match value {
        Value::Host(handle) if handle.kind() == HostKind::Holder => {
            holder_parts(handle).map(|(_, serial, _)| serial)
        }
        _ => None,
    }
}

fn registered_serial(value: &Value) -> Option<usize> {
    match value {
        Value::Host(handle) if handle.kind() == HostKind::Registered => handle.key().parse().ok(),
        _ => None,
    }
}

/// The declared types of a holder, by its key: its loader's class.
pub(crate) fn holder_types(key: &str) -> &'static [&'static str] {
    // The key's first value is the loader (`i0`/`i1`, `value_marker`'s int).
    if key.starts_with(&format!("i{FORGE},")) {
        REGISTRY_OBJECT_TYPES
    } else {
        DEFERRED_HOLDER_TYPES
    }
}

/// A registered value's class is whatever the jar's supplier makes.
pub(crate) fn no_types(_key: &str) -> &'static [&'static str] {
    &[]
}

/// `Object.equals`: identity, which a canonical key decides.
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

/// A `Registry`: `toString` names its lifecycle (`Registry[ResourceKey[…]
/// (Stable)]`), which the host does not hold, and `hashCode` is an identity
/// hash: both counted.
pub(crate) const REGISTRY_METHODS: Methods = Methods {
    to_string: |_, _| Ok(None),
    hash_code: |_, _| Ok(None),
    equals: identity_equals,
    instance_of: |handle, wanted, hierarchy| {
        declared_instance_of(
            handle,
            wanted,
            hierarchy,
            "net/minecraft/core/MappedRegistry",
            NOT_MAPPED_REGISTRY,
        )
    },
    crosses_jars: |_, _| true,
    compare_to: None,
};

/// `DeferredHolder.toString`: `"DeferredHolder{" + key + "}"` (26.3.0.101,
/// `String.format`); a Forge holder's or an unnamed key's is counted.
fn holder_to_string(
    handle: &HostRef,
    nested: &mut dyn Nested,
) -> Result<Option<String>, NativeError> {
    let Some((forge, _, key)) = holder_parts(handle) else {
        return Ok(None);
    };
    if forge || key == Value::Ref(None) {
        return Ok(None);
    }
    Ok(nested
        .text(&key)?
        .map(|key| format!("DeferredHolder{{{key}}}")))
}

pub(crate) const HOLDER_METHODS: Methods = Methods {
    to_string: holder_to_string,
    // `Holder.Reference` keeps `Object.hashCode`: an identity hash.
    hash_code: |_, _| Ok(None),
    equals: identity_equals,
    instance_of: |handle, wanted, hierarchy| {
        declared_instance_of(
            handle,
            wanted,
            hierarchy,
            "net/neoforged/neoforge/registries/DeferredHolder",
            NOT_DEFERRED_HOLDER,
        )
    },
    // A holder is its jar's registration.
    crosses_jars: |_, _| false,
    compare_to: None,
};

/// A registered value: an object of the jar's own class, which the host
/// never made. Every object method is counted but identity.
pub(crate) const REGISTERED_METHODS: Methods = Methods {
    to_string: |_, _| Ok(None),
    hash_code: |_, _| Ok(None),
    equals: identity_equals,
    instance_of: instance_of_class,
    crosses_jars: |_, _| false,
    compare_to: None,
};

/// What `DeferredRegister.register` recorded for one registration.
#[derive(Clone, Debug)]
pub(crate) struct HolderRow {
    /// The register's marker (`deferred-register/…`).
    pub(crate) register: String,
    /// Forge's `RegistryObject` rather than NeoForge's `DeferredHolder`.
    pub(crate) forge: bool,
    /// The registry key and validated id, when the register named them.
    pub(crate) id: Option<(Value, String)>,
}

impl crate::StdHost {
    /// Record a `DeferredRegister.register` call: its holder and its value.
    pub(crate) fn new_holder(
        &mut self,
        register: &str,
        forge: bool,
        id: Option<(Value, Value)>,
    ) -> (Value, Value) {
        let serial = self.holder_rows.len();
        let id = id.and_then(|(registry, id)| {
            crate::host_ids::identifier_key(&id).map(|id| (registry, id.to_owned()))
        });
        let key = id.as_ref().map_or(Value::Ref(None), |(registry, id)| {
            crate::host_ids::resource_key(
                registry,
                &crate::host_ids::identifier_of_text(id),
                Some(id.clone()),
            )
        });
        self.holder_rows.push(HolderRow {
            register: register.to_owned(),
            forge,
            id,
        });
        (holder(forge, serial, &key), registered(serial))
    }

    /// The holders a register made, in registration order (`getEntries`).
    pub(crate) fn register_holders(&self, register: &str) -> Vec<Value> {
        self.holder_rows
            .iter()
            .enumerate()
            .filter(|(_, row)| row.register == register)
            .map(|(serial, row)| {
                let key = row.id.as_ref().map_or(Value::Ref(None), |(registry, id)| {
                    crate::host_ids::resource_key(
                        registry,
                        &crate::host_ids::identifier_of_text(id),
                        Some(id.clone()),
                    )
                });
                holder(row.forge, serial, &key)
            })
            .collect()
    }

    /// The row a holder (or, with `registered`, a registered value) names.
    fn holder_row(&self, value: &Value) -> Option<&HolderRow> {
        let serial = holder_serial(value).or_else(|| registered_serial(value))?;
        self.holder_rows.get(serial)
    }

    /// `holder.get()`/`value()`: the value the registration registered
    /// (the host models registration as done; NeoForge throws for a holder
    /// read before its `RegisterEvent`). `None` for no holder.
    pub(crate) fn holder_value(&self, value: &Value) -> Option<Value> {
        let serial = holder_serial(value)?;
        Some(
            self.holders
                .get(&serial)
                .cloned()
                .unwrap_or_else(|| registered(serial)),
        )
    }

    /// `DeferredHolder.getId()`/`RegistryObject.getId()`/
    /// `IForgeRegistryEntry.getRegistryName()`: the entry's `Identifier`;
    /// counted when the host does not know it.
    pub(crate) fn holder_id(
        &mut self,
        r: &crate::MethodRef,
        receiver: &Value,
    ) -> Result<Value, NativeError> {
        match self.holder_row(receiver).and_then(|row| row.id.clone()) {
            Some((_, id)) => Ok(crate::host_ids::identifier_of_text(&id)),
            None => self.unknown_host_call(r),
        }
    }

    /// `Holder.unwrapKey()`/`DeferredHolder.getKey()`: the entry's
    /// `ResourceKey.create(registryKey, id)`, unnamed when the register's
    /// registry key is one the host cannot name.
    pub(crate) fn holder_key(
        &mut self,
        r: &crate::MethodRef,
        receiver: &Value,
    ) -> Result<Value, NativeError> {
        match self.holder_row(receiver).and_then(|row| row.id.clone()) {
            Some((registry, id)) => Ok(crate::host_ids::resource_key(
                &registry,
                &crate::host_ids::identifier_of_text(&id),
                Some(id),
            )),
            None => self.unknown_host_call(r),
        }
    }
}

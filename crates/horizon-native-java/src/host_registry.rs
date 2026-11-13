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
//! * `Registry`: key one digit for what the host knows of its implementation
//!   (`0` only a `Registry`, `1` a `MappedRegistry`, `2` a
//!   `DefaultedMappedRegistry`) followed by the registry's id. One object per
//!   registry key, so canonical. Its exact class is not known, so the class
//!   is unknown ([`ClassRule::Unknown`](crate::host_ref::ClassRule)):
//!   `getClass()` is counted, `instanceof` answers from the types it surely
//!   is. A vanilla built-in registry is a `MappedRegistry` (a
//!   `DefaultedMappedRegistry` for the ten defaulted ones), and so is one
//!   Fabric's `FabricRegistryBuilder.create`/`createDefaulted` built
//!   (fabric-registry-sync 8.0.4 bytecode); NeoForge's own only a `Registry`.
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

/// A `MappedRegistry`, so a `WritableRegistry`.
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

/// A `DefaultedMappedRegistry`.
const DEFAULTED_TYPES: &[&str] = &[
    "net/minecraft/core/DefaultedMappedRegistry",
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

/// What the host knows of a registry's implementation: the first character
/// of a `Registry` key.
pub(crate) const ONLY_REGISTRY: char = '0';
pub(crate) const MAPPED: char = '1';
pub(crate) const DEFAULTED_MAPPED: char = '2';

/// A registry `id`'s implementation, as far as the host knows it: a vanilla
/// built-in's from 26.3's `BuiltInRegistries`, anything else's unknown.
fn known_implementation(id: &str) -> char {
    if DEFAULTED.contains(&id) {
        DEFAULTED_MAPPED
    } else if crate::vanilla_registries::is_built_in(id) {
        MAPPED
    } else {
        ONLY_REGISTRY
    }
}

/// `BuiltInRegistries.X`, NeoForge's own: the `Registry` for `id`.
pub(crate) fn registry(id: &str) -> Value {
    built_registry(known_implementation(id), id)
}

/// A registry the host built knowing its implementation (`MAPPED`, …), or a
/// vanilla built-in's (its table's).
pub(crate) fn built_registry(implementation: char, id: &str) -> Value {
    let implementation = match known_implementation(id) {
        ONLY_REGISTRY => implementation,
        built_in => built_in,
    };
    Value::Host(HostRef::new(
        HostKind::Registry,
        format!("{implementation}{id}"),
    ))
}

/// The id of a `Registry` handle.
pub(crate) fn registry_id(value: &Value) -> Option<&str> {
    match value {
        Value::Host(handle) if handle.kind() == HostKind::Registry => handle.key().get(1..),
        _ => None,
    }
}

/// The declared types of a `Registry` with this key.
pub(crate) fn registry_types(key: &str) -> &'static [&'static str] {
    match key.chars().next() {
        Some(DEFAULTED_MAPPED) => DEFAULTED_TYPES,
        Some(MAPPED) => BUILT_IN_TYPES,
        _ => REGISTRY_TYPES,
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

/// A `FabricRegistryBuilder` (`fabric_registry.rs`): a final class that
/// keeps `Object`'s `toString`/`hashCode` (an identity hash: counted) and
/// `equals` (identity).
pub(crate) const BUILDER_METHODS: Methods = Methods {
    to_string: |_, _| Ok(None),
    hash_code: |_, _| Ok(None),
    equals: identity_equals,
    instance_of: |handle, wanted, hierarchy| {
        let class = handle.class()?;
        if wanted == class
            || wanted == "java/lang/Object"
            || hierarchy.is_instance(class, wanted) == Some(true)
        {
            return Some(true);
        }
        // It extends `Object` and a Mixin adds no superclass: no other class
        // is a supertype. An interface a loaded Mixin may add is unknown.
        (hierarchy.is_interface(wanted) == Some(false)).then_some(false)
    },
    crosses_jars: |_, _| false,
    compare_to: None,
};

/// The text forms this family's objects had before they were handles. The
/// runtime makes no value spelled so any more, so a text value spelled so is
/// a jar `String` (#177 review B1, M1).
const RETIRED_MARKERS: &[&str] = &[
    "registry/",
    "holder/",
    "registered/",
    "fabric-registry-builder/",
];

/// The game classes only this family's handles (or a jar's own objects) are.
fn family_class(target: &str) -> bool {
    target == crate::fabric_registry::BUILDER
        || [
            DEFAULTED_TYPES,
            DEFERRED_HOLDER_TYPES,
            REGISTRY_OBJECT_TYPES,
        ]
        .iter()
        .any(|types| types.contains(&target) && !target.starts_with("java/"))
        || matches!(
            target,
            "net/neoforged/neoforge/registries/DeferredItem"
                | "net/neoforged/neoforge/registries/DeferredBlock"
        )
}

/// `checkcast` of a String spelled like a retired marker to one of this
/// family's game classes: Java's `ClassCastException`, naming `String`.
pub(crate) fn retired_marker_cast(value: &Value, target: &str) -> Option<String> {
    let Value::Ref(Some(text)) = value else {
        return None;
    };
    (family_class(target)
        && RETIRED_MARKERS
            .iter()
            .any(|prefix| text.starts_with(prefix)))
    .then(|| "java/lang/String".to_owned())
}

/// When NeoForge binds a holder to its entry (`DeferredHolder.bind`): at its
/// registry's `RegisterEvent`, which this runtime does not post
/// (`neoforge_lifecycle.rs`). Before it every holder is unbound (#177 review
/// B2).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Binding {
    /// Mod construction: no `RegisterEvent` has fired.
    #[default]
    Construct,
    /// The registry step: some registries' events have fired, and which is
    /// not modelled, so every answer is counted.
    Registering,
    /// After the registry step: the host models every registration as done.
    Bound,
}

/// `DeferredHolder.value()`'s NPE for an unbound holder (26.3.0.101 @12–31:
/// `"Trying to access unbound value: " + key()`).
fn unbound_value(registry: &str, location: &str) -> NativeError {
    NativeError::UncaughtException {
        class: "java/lang/NullPointerException".into(),
        message: Some(format!(
            "Trying to access unbound value: ResourceKey[{registry} / {location}]"
        )),
        frame: None,
    }
}

/// What a holder call asks.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Ask {
    Value,
    Bound,
    Optional,
}

/// A row's entry key as `(registry, location)`, when that key is unbound
/// during construction: its registry holds no entry for it yet. An entry in
/// `minecraft`'s namespace may be one of vanilla's (bound already), so it is
/// left out (counted), as is a key the host could not name.
fn unbound_key(row: &HolderRow) -> Option<(String, String)> {
    let (registry, id) = row.id.as_ref()?;
    let key = crate::host_ids::resource_key(
        registry,
        &crate::host_ids::identifier_of_text(id),
        Some(id.clone()),
    );
    let (registry, location) = crate::host_ids::resource_key_parts(&key)?;
    (!location.starts_with("minecraft:")).then_some((registry, location))
}

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

    /// NeoForge's registry step began (`register_attachment_types`, the
    /// first `RegisterEvent` this runtime runs).
    pub(crate) fn holders_registering(&mut self) {
        if self.holder_binding == Binding::Construct {
            self.holder_binding = Binding::Registering;
        }
    }

    /// A loading stage after construction began: every `RegisterEvent` has
    /// fired.
    pub(crate) fn holders_bound(&mut self) {
        self.holder_binding = Binding::Bound;
    }

    /// A holder's `get`/`value`/`isBound`/`asOptional` (NeoForge), its
    /// `get`/`orElse`/`orElseGet`/`getOptional`/`isPresent` (Forge), and
    /// `Supplier.get` on a holder (the field type a mod usually gives it):
    /// all one arm, so one object never answers two ways. `None` for a call
    /// that is not one of these.
    pub(crate) fn holder_call(
        &mut self,
        r: &crate::MethodRef,
        receiver: &Value,
    ) -> Option<Result<Value, NativeError>> {
        const DEFERRED: &[&str] = &[
            "net/neoforged/neoforge/registries/DeferredHolder",
            "net/neoforged/neoforge/registries/DeferredItem",
            "net/neoforged/neoforge/registries/DeferredBlock",
        ];
        const HOLDER: &[&str] = &[
            "net/minecraft/core/Holder",
            "net/minecraft/core/Holder$Reference",
        ];
        const FORGE: &str = "net/minecraftforge/registries/RegistryObject";
        let owner = r.owner.as_str();
        let deferred = DEFERRED.contains(&owner);
        let ask = match (r.name.as_str(), r.descriptor.as_str()) {
            ("get", "()Ljava/lang/Object;")
                if deferred
                    || owner == FORGE
                    || (owner == "java/util/function/Supplier"
                        && holder_serial(receiver).is_some()) =>
            {
                Ask::Value
            }
            ("value", "()Ljava/lang/Object;") if deferred || HOLDER.contains(&owner) => Ask::Value,
            ("isBound", "()Z") if deferred || HOLDER.contains(&owner) => Ask::Bound,
            ("asOptional", "()Ljava/util/Optional;") if deferred => Ask::Optional,
            // Forge's (a `DeferredHolder` has neither in 26.3.0.101; kept for
            // older NeoForge callers, as before).
            ("getOptional", "()Ljava/util/Optional;") if deferred || owner == FORGE => {
                Ask::Optional
            }
            ("isPresent", "()Z") if deferred || owner == FORGE => Ask::Bound,
            ("orElse", "(Ljava/lang/Object;)Ljava/lang/Object;")
            | ("orElseGet", "(Ljava/util/function/Supplier;)Ljava/lang/Object;")
                if owner == FORGE =>
            {
                Ask::Value
            }
            _ => return None,
        };
        Some(self.holder_answer(r, receiver, ask))
    }

    fn holder_answer(
        &mut self,
        r: &crate::MethodRef,
        receiver: &Value,
        ask: Ask,
    ) -> Result<Value, NativeError> {
        // Anything but a holder the host made (a vanilla `Holder` it does
        // not model, a String a cast let through): counted.
        let Some((serial, row)) = holder_serial(receiver)
            .and_then(|serial| Some((serial, self.holder_rows.get(serial)?.clone())))
        else {
            return self.unknown_host_call(r);
        };
        let left_unbound = self
            .neoforge_attachments
            .holder_left_unbound(&serial.to_string());
        match self.holder_binding {
            // An attachment holder whose registration failed stays unbound;
            // its registry exists by now.
            Binding::Bound if left_unbound => match (ask, unbound_key(&row)) {
                (Ask::Bound, _) => Ok(Value::Int(0)),
                (Ask::Optional, _) => Ok(crate::host_optional::empty_optional()),
                (Ask::Value, Some((registry, location))) => {
                    Err(unbound_value(&registry, &location))
                }
                (Ask::Value, None) => self.unknown_host_call(r),
            },
            // The host models every registration as done.
            Binding::Bound => Ok(match ask {
                Ask::Bound => Value::Int(1),
                Ask::Value => self.holder_value(receiver).unwrap_or(Value::Ref(None)),
                Ask::Optional => crate::host_optional::optional_wrap(
                    self.holder_value(receiver).unwrap_or(Value::Ref(None)),
                ),
            }),
            Binding::Construct => match (row.forge, unbound_key(&row)) {
                // NeoForge 26.3.0.101: `isBound` is `bind(false)`, then
                // `holder != null && holder.isBound()`; `asOptional` is
                // `isBound() ? Optional.of(value()) : Optional.empty()`.
                (false, Some(_)) if ask == Ask::Bound => Ok(Value::Int(0)),
                (false, Some(_)) if ask == Ask::Optional => {
                    Ok(crate::host_optional::empty_optional())
                }
                // `value()` (`get()` is `value()`) @0 `bind(true)`: the
                // registry is there (a vanilla one), the entry not yet, so
                // @9–31 throws. A registry the host cannot vouch for may
                // throw `bind`'s `IllegalStateException` instead: counted.
                (false, Some((registry, location)))
                    if crate::vanilla_registries::is_built_in(&registry) =>
                {
                    Err(unbound_value(&registry, &location))
                }
                // A Forge `RegistryObject` (no Forge jar to read), or a key
                // the host could not name.
                _ => self.unknown_host_call(r),
            },
            // Some registries' events have fired; which, the runtime does not
            // model.
            Binding::Registering => self.unknown_host_call(r),
        }
    }

    /// `holder.get()`/`value()` once registration is done: the value the
    /// registration registered. `None` for no holder.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Host, MethodRef, StdHost};

    fn method(owner: &str, name: &str, descriptor: &str) -> MethodRef {
        MethodRef {
            owner: owner.into(),
            name: name.into(),
            descriptor: descriptor.into(),
        }
    }

    /// `DeferredRegister.create(Registries.ITEM, "demo").register(name, ..)`.
    fn demo_holder(host: &mut StdHost, name: &str) -> Result<Value, NativeError> {
        let register = host.invoke(
            &method(
                "net/neoforged/neoforge/registries/DeferredRegister",
                "create",
                "(Lnet/minecraft/resources/ResourceKey;Ljava/lang/String;)Lnet/neoforged/neoforge/registries/DeferredRegister;",
            ),
            &[
                crate::host_ids::registry_key_of("minecraft:item"),
                Value::Ref(Some("demo".into())),
            ],
        )?;
        host.invoke_instance(
            &method(
                "net/neoforged/neoforge/registries/DeferredRegister",
                "register",
                "(Ljava/lang/String;Ljava/util/function/Supplier;)Lnet/neoforged/neoforge/registries/DeferredHolder;",
            ),
            &register,
            &[
                Value::Ref(Some(name.into())),
                Value::Ref(Some("@indy:supplier".into())),
            ],
        )
    }

    fn is_bound() -> MethodRef {
        method("net/minecraft/core/Holder", "isBound", "()Z")
    }

    /// M1: `Holder.isBound()` on anything that is no holder the host made (a
    /// registered value, a registry, a vanilla holder's text) is counted,
    /// never a silent `false`.
    #[test]
    fn is_bound_on_what_is_no_holder_is_counted() {
        let mut host = StdHost::new();
        demo_holder(&mut host, "x").unwrap();
        let registered = host.registrations()[0].value.clone();
        for receiver in [
            registered,
            registry("minecraft:item"),
            Value::Ref(Some(
                "holder/deferred-register/registry/minecraft:item/demo/x".into(),
            )),
        ] {
            let before = host.unknown_host_call_count();
            host.invoke_instance(&is_bound(), &receiver, &[]).unwrap();
            assert_eq!(host.unknown_host_call_count(), before + 1, "{receiver:?}");
        }
    }

    /// B2: unbound (exact) during construction, counted while the registry
    /// step runs, bound (the host's model of a finished registration) after.
    #[test]
    fn a_holder_binds_after_the_registry_step() {
        let mut host = StdHost::new();
        let holder = demo_holder(&mut host, "x").unwrap();
        let get = method(
            "net/neoforged/neoforge/registries/DeferredHolder",
            "get",
            "()Ljava/lang/Object;",
        );
        assert_eq!(
            host.invoke_instance(&is_bound(), &holder, &[]).unwrap(),
            Value::Int(0)
        );
        assert!(matches!(
            host.invoke_instance(&get, &holder, &[]),
            Err(NativeError::UncaughtException { class, .. }) if class == "java/lang/NullPointerException"
        ));
        assert_eq!(host.unknown_host_call_count(), 0);
        host.holders_registering();
        host.invoke_instance(&is_bound(), &holder, &[]).unwrap();
        host.invoke_instance(&get, &holder, &[]).unwrap();
        assert_eq!(host.unknown_host_call_count(), 2);
        host.holders_bound();
        assert_eq!(
            host.invoke_instance(&is_bound(), &holder, &[]).unwrap(),
            Value::Int(1)
        );
        let registered = host.registrations()[0].value.clone();
        assert_eq!(
            host.invoke_instance(&get, &holder, &[]).unwrap(),
            registered
        );
        // `Supplier.get` on the holder is the same arm.
        assert_eq!(
            host.invoke_instance(
                &method("java/util/function/Supplier", "get", "()Ljava/lang/Object;"),
                &holder,
                &[]
            )
            .unwrap(),
            registered
        );
        assert_eq!(host.unknown_host_call_count(), 2);
    }

    /// Entry names spelled with every separator a key layer uses either fail
    /// as Java's `Identifier` does or name their own entry: `getId` reads
    /// back the literal name and no two holders share an id or a key
    /// ("keys never parse jar text", #156).
    #[test]
    fn entry_names_with_separators_round_trip_and_never_alias() {
        let alphabet = [
            "a", "|", ",", ":", "/", "\\", "#", "?", ".", "i1", "hhold:", "-", "_",
        ];
        let mut host = StdHost::new();
        let mut ids = std::collections::BTreeSet::new();
        let mut keys = std::collections::BTreeSet::new();
        let get_id = method(
            "net/neoforged/neoforge/registries/DeferredHolder",
            "getId",
            "()Lnet/minecraft/resources/Identifier;",
        );
        for a in alphabet {
            for b in alphabet {
                let name = format!("x{a}{b}");
                let valid = name
                    .chars()
                    .all(|c| matches!(c, 'a'..='z' | '0'..='9' | '/' | '.' | '_' | '-'));
                let Ok(holder) = demo_holder(&mut host, &name) else {
                    assert!(!valid, "{name:?} was refused");
                    continue;
                };
                assert!(valid, "{name:?} was accepted");
                let id = host.invoke_instance(&get_id, &holder, &[]).unwrap();
                assert_eq!(
                    crate::host_ids::identifier_key(&id),
                    Some(format!("demo:{name}").as_str())
                );
                assert!(ids.insert(format!("demo:{name}")), "{name:?}");
                let Value::Host(handle) = &holder else {
                    panic!("{holder:?}")
                };
                assert!(keys.insert(handle.key().to_owned()), "{name:?}");
            }
        }
        assert_eq!(host.unknown_host_call_count(), 0);
    }

    /// A builder's key holds the registry it answers, whatever that
    /// registry's id is spelled with, and two registries never give one
    /// builder key.
    #[test]
    fn builder_keys_round_trip_their_registry() {
        let alphabet = [
            "a", "|", ",", ":", "/", "\\", "#", "?", "r", "i1", "hreg:", "\\,", "",
        ];
        let mut seen = std::collections::BTreeMap::new();
        for a in alphabet {
            for b in alphabet {
                let registry = built_registry(MAPPED, &format!("{a}{b}"));
                let builder = crate::fabric_registry::builder(registry.clone());
                let Value::Host(handle) = &builder else {
                    panic!("{builder:?}")
                };
                assert_eq!(handle.values(), Some(vec![registry.clone()]));
                if let Some(other) = seen.insert(handle.key().to_owned(), registry.clone()) {
                    assert_eq!(other, registry);
                }
            }
        }
    }
}

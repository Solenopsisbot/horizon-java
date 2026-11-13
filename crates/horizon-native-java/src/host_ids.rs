//! Game ids as host handles (#156 P4, first half): `Identifier`,
//! `ResourceKey`, `TagKey` and `BlockItemId`, their keys and their rows in
//! the kind table (`host_ref.rs`), checked against the real 26.3
//! `server-game.jar` on Java 25 (`tests/fixture_host_ids.rs`).
//!
//! These were text a jar `String` could spell (`identifier/a:x`,
//! `tag-key/…|…`, `resource-key/<registry marker>/<id marker>`,
//! `block-item-id/<id>,<id>`), so the string `"identifier/a:x"` was the id
//! `a:x` to every reader that looked at the prefix.
//!
//! Keys (equal keys are equal objects):
//!
//! * `Identifier`: `namespace:path`. Not interned in vanilla: two
//!   `Identifier.parse("a:x")` are equal but not `==`, so the kind is not
//!   canonical and identity on two of them is counted.
//! * `ResourceKey`: `registry|location`, both `namespace:path` (`registry`
//!   is the registry's own id, `minecraft:item` for `Registries.ITEM`).
//!   A key whose registry or id the host cannot name (a registry argument no
//!   model knows) is an *unnamed* key, `?<the two arguments' markers>`: its
//!   location still answers, its `toString` and registry id are counted.
//!   `ResourceKey.create` interns (a weak map keyed by the registry's id and
//!   the location), so the kind is canonical: equal keys were made from equal
//!   arguments, which intern to one object. Different keys prove nothing for
//!   an unnamed key (its registry argument may name a named key's registry),
//!   so `equals` between those is counted.
//! * `TagKey`: the same `registry|location` (or `?…`) shape. `TagKey.create`
//!   interns too (a Guava weak interner): canonical.
//! * `BlockItemId`: [`HostRef::of_values`] of the block and item ids it was
//!   created from. A record, not interned: not canonical.
//!
//! `ResourceKey` keeps `Object.hashCode`, an identity hash, and `TagKey`'s and
//! `BlockItemId`'s record `hashCode` mixes in their keys' identity hashes, so
//! all three `hashCode`s are counted, never answered (Java 25 prints a
//! different number per JVM).
//!
//! A registry key (`Registries.ITEM`) is still the `registry/<id>` text the
//! registry family uses (#156 P4's second half moves `registry/`,
//! `registered/` and `holder/`), so `ResourceKey.registryKey()` and
//! `TagKey.registry()` answer that text, the same value `Registries.ITEM` is.

use crate::host_ref::{HostKind, HostRef, Methods};
use crate::{Value, value_marker};

pub(crate) const IDENTIFIER: &str = "net/minecraft/resources/Identifier";
/// `Identifier`'s name before the 1.21 rename; an older jar's
/// `ResourceLocation` is the same handle.
const RESOURCE_LOCATION: &str = "net/minecraft/resources/ResourceLocation";
pub(crate) const RESOURCE_KEY: &str = "net/minecraft/resources/ResourceKey";
pub(crate) const TAG_KEY: &str = "net/minecraft/tags/TagKey";
pub(crate) const BLOCK_ITEM_ID: &str = "net/minecraft/references/BlockItemId";

/// An `Identifier` for an already validated `namespace` and `path`.
pub(crate) fn identifier(namespace: &str, path: &str) -> Value {
    Value::Host(HostRef::new(
        HostKind::Identifier,
        format!("{namespace}:{path}"),
    ))
}

/// An `Identifier` for `namespace:path` text the host already holds (a bare
/// path is `minecraft`'s, as `Identifier.parse` reads it).
pub(crate) fn identifier_of_text(id: &str) -> Value {
    let (namespace, path) = id.split_once(':').unwrap_or(("minecraft", id));
    identifier(namespace, path)
}

/// The `namespace:path` an `Identifier` handle names.
pub(crate) fn identifier_key(value: &Value) -> Option<&str> {
    match value {
        Value::Host(handle) if handle.kind() == HostKind::Identifier => Some(handle.key()),
        _ => None,
    }
}

/// The `namespace:path` of an id value: an `Identifier` handle, or a
/// `registry/<id>` text (a registry key names its registry's id).
pub(crate) fn named_id(value: &Value) -> Option<String> {
    identifier_key(value).map(str::to_owned).or_else(|| {
        crate::expect_ref(value)?
            .strip_prefix("registry/")
            .map(str::to_owned)
    })
}

/// The id of the registry a registry-key argument names: a `registry/<id>`
/// value (`Registries.ITEM`, still text until #156 P4's second half) or a
/// named `ResourceKey` handle (`ResourceKey.create` reads its argument's
/// `identifier()`, whatever registry that key is in).
pub(crate) fn registry_name(value: &Value) -> Option<String> {
    if let Some((_, location)) = resource_key_parts(value) {
        return Some(location.to_owned());
    }
    crate::expect_ref(value)?
        .strip_prefix("registry/")
        .map(str::to_owned)
}

/// The key of a `ResourceKey`/`TagKey` made from `registry` and `location`:
/// `registry|location` when both name ids, else the unnamed key `?` + the
/// two arguments' markers (the location as an `Identifier` when the host
/// knows its id), which [`unnamed_parts`] reads back.
fn pair_key(registry: &Value, location: &Value, location_id: Option<String>) -> String {
    match (registry_name(registry), location_id) {
        (Some(registry), Some(location)) => format!("{registry}|{location}"),
        (_, location_id) => {
            let location =
                location_id.map_or_else(|| location.clone(), |id| identifier_of_text(&id));
            format!("?{},{}", value_marker(registry), value_marker(&location))
        }
    }
}

/// The registry and location arguments an unnamed key of `kind` holds.
fn unnamed_parts(value: &Value, kind: HostKind) -> Option<(Value, Value)> {
    let Value::Host(handle) = value else {
        return None;
    };
    let key = handle
        .key()
        .strip_prefix('?')
        .filter(|_| handle.kind() == kind)?;
    match crate::split_capture_markers(key).ok()?.as_slice() {
        [registry, location] => Some((
            crate::parse_value_marker(registry).ok()?,
            crate::parse_value_marker(location).ok()?,
        )),
        _ => None,
    }
}

fn pair_kind(value: &Value) -> Option<HostKind> {
    match value {
        Value::Host(handle)
            if matches!(handle.kind(), HostKind::ResourceKey | HostKind::TagKey) =>
        {
            Some(handle.kind())
        }
        _ => None,
    }
}

/// A `ResourceKey`'s `identifier()` or a `TagKey`'s `location()`: the id of
/// a named key, or the location argument an unnamed one was made from.
pub(crate) fn key_location(value: &Value) -> Option<Value> {
    let kind = pair_kind(value)?;
    match named_parts(value, kind) {
        Some((_, location)) => Some(identifier_of_text(location)),
        None => unnamed_parts(value, kind).map(|(_, location)| location),
    }
}

/// The registry key a `ResourceKey`/`TagKey` was made in: `registry/<id>`
/// (`Registries.X`) for a named key, else the registry argument itself.
pub(crate) fn key_registry(value: &Value) -> Option<Value> {
    let kind = pair_kind(value)?;
    match named_parts(value, kind) {
        Some((registry, _)) => Some(Value::Ref(Some(format!("registry/{registry}")))),
        None => unnamed_parts(value, kind).map(|(registry, _)| registry),
    }
}

/// `key.isFor(registryKey)`: the key's registry is that one. Named keys
/// compare registry ids; an unnamed key's own registry argument is the same
/// object as `registryKey` only when provably so (`None`: the caller counts).
pub(crate) fn key_is_for(value: &Value, registry_key: &Value) -> Option<bool> {
    let kind = pair_kind(value)?;
    if let Some((registry, _)) = named_parts(value, kind) {
        return Some(registry_name(registry_key).as_deref() == Some(registry));
    }
    let (own, _) = unnamed_parts(value, kind)?;
    match (&own, registry_key) {
        (Value::Host(own), Value::Host(other)) => own.same_object(other).filter(|same| *same),
        (own, other) => (own == other).then_some(true),
    }
}

/// `ResourceKey.create(registry, location)`; `location_id` is the id the
/// location argument names, when the host knows it.
pub(crate) fn resource_key(
    registry: &Value,
    location: &Value,
    location_id: Option<String>,
) -> Value {
    Value::Host(HostRef::new(
        HostKind::ResourceKey,
        pair_key(registry, location, location_id),
    ))
}

/// A named `ResourceKey` for two ids the host holds.
pub(crate) fn resource_key_named(registry: &str, location: &str) -> Value {
    Value::Host(HostRef::new(
        HostKind::ResourceKey,
        format!("{registry}|{location}"),
    ))
}

/// The `(registry, location)` ids of a named key of `kind`.
fn named_parts(value: &Value, kind: HostKind) -> Option<(&str, &str)> {
    match value {
        Value::Host(handle) if handle.kind() == kind && !handle.key().starts_with('?') => {
            handle.key().split_once('|')
        }
        _ => None,
    }
}

/// The `(registry, location)` ids of a named `ResourceKey`.
pub(crate) fn resource_key_parts(value: &Value) -> Option<(&str, &str)> {
    named_parts(value, HostKind::ResourceKey)
}

/// Whether `value` is a `ResourceKey` handle, named or not.
pub(crate) fn is_resource_key(value: &Value) -> bool {
    matches!(value, Value::Host(handle) if handle.kind() == HostKind::ResourceKey)
}

/// `TagKey.create(registry, location)`.
pub(crate) fn tag_key(registry: &Value, location: &Value, location_id: Option<String>) -> Value {
    Value::Host(HostRef::new(
        HostKind::TagKey,
        pair_key(registry, location, location_id),
    ))
}

/// Whether `value` is a `TagKey` handle, named or not.
pub(crate) fn is_tag_key(value: &Value) -> bool {
    matches!(value, Value::Host(handle) if handle.kind() == HostKind::TagKey)
}

/// `BlockItemId.create(block, item)` for its two id values.
pub(crate) fn block_item_id(block: &Value, item: &Value) -> Value {
    Value::Host(HostRef::of_values(
        HostKind::BlockItemId,
        &[block.clone(), item.clone()],
    ))
}

/// The block and item ids a `BlockItemId` handle was created from.
pub(crate) fn block_item_id_parts(value: &Value) -> Option<(Value, Value)> {
    match value {
        Value::Host(handle) if handle.kind() == HostKind::BlockItemId => {
            match handle.values()?.as_slice() {
                [block, item] => Some((block.clone(), item.clone())),
                _ => None,
            }
        }
        _ => None,
    }
}

/// Whether running `equals`/`hashCode` on `value` has no observable effect
/// (it compares and hashes its own ids only).
pub(crate) fn pure_equality(value: &Value) -> bool {
    matches!(
        value,
        Value::Host(handle) if matches!(
            handle.kind(),
            HostKind::Identifier | HostKind::ResourceKey | HostKind::TagKey
        )
    )
}

/// `instanceof` for a game class the runtime does not hold: its own
/// supertypes, then whatever the hierarchy knows (a Mixin-added interface).
fn instance_of(
    handle: &HostRef,
    wanted: &str,
    supertypes: &[&str],
    hierarchy: &dyn crate::host_ref::ClassHierarchy,
) -> Option<bool> {
    if wanted == handle.class() || wanted == "java/lang/Object" || supertypes.contains(&wanted) {
        return Some(true);
    }
    hierarchy.is_instance(handle.class(), wanted)
}

fn named(handle: &HostRef) -> bool {
    !handle.key().starts_with('?')
}

/// `ResourceKey.equals` is `Object.equals` (identity) and `TagKey.equals`
/// compares its interned registry key by identity and its location by
/// `equals`: for two named keys of `kind` both are key equality. An unnamed
/// key equals only itself: known when the same handle flows to both sides,
/// counted otherwise (its arguments may name what another key names).
fn pair_equals(handle: &HostRef, other: &Value) -> Option<bool> {
    let Value::Host(other) = other else {
        return Some(false);
    };
    if other.kind() != handle.kind() {
        return Some(false);
    }
    if named(handle) && named(other) {
        return Some(handle == other);
    }
    handle.same_object(other).filter(|same| *same)
}

/// `Identifier.toString()` and its `hashCode` (`31 * namespace.hashCode() +
/// path.hashCode()`); `equals` is another `Identifier` with the same parts.
pub(crate) const IDENTIFIER_METHODS: Methods = Methods {
    to_string: |handle, _| Ok(Some(handle.key().to_owned())),
    hash_code: |handle, _| Ok(Some(crate::hash_map_order::identifier_hash(handle.key()))),
    equals: |handle, other, _| {
        Ok(match other {
            Value::Host(other) => Some(other == handle),
            // An object a `new ResourceLocation(..)` built (pre-1.21): its id
            // lives in the host's table, which no `Nested` reaches.
            Value::Ref(Some(marker)) => crate::jar_text::object_marker_class(marker)
                .is_none_or(|class| class != RESOURCE_LOCATION && class != IDENTIFIER)
                .then_some(false),
            _ => Some(false),
        })
    },
    instance_of: |handle, wanted, hierarchy| {
        instance_of(
            handle,
            wanted,
            &[RESOURCE_LOCATION, "java/lang/Comparable"],
            hierarchy,
        )
    },
    crosses_jars: |_, _| true,
    compare_to: Some(identifier_compare),
};

/// `Identifier.compareTo`: the paths by `String.compareTo`, then the
/// namespaces on a tie. Any other value is not an `Identifier` (a JVM throws
/// `ClassCastException`): unknown here.
fn identifier_compare(handle: &HostRef, other: &Value) -> Option<i32> {
    let (namespace, path) = handle.key().split_once(':')?;
    let (other_namespace, other_path) = identifier_key(other)?.split_once(':')?;
    Some(
        match crate::java_identifier::java_compare(path, other_path) {
            0 => crate::java_identifier::java_compare(namespace, other_namespace),
            order => order,
        },
    )
}

fn pair_text(handle: &HostRef, class: &str) -> Option<String> {
    let (registry, location) = handle.key().split_once('|').filter(|_| named(handle))?;
    Some(format!("{class}[{registry} / {location}]"))
}

pub(crate) const RESOURCE_KEY_METHODS: Methods = Methods {
    // `"ResourceKey[" + registryName + " / " + identifier + "]"`.
    to_string: |handle, _| Ok(pair_text(handle, "ResourceKey")),
    // `Object.hashCode`: an identity hash.
    hash_code: |_, _| Ok(None),
    equals: |handle, other, _| Ok(pair_equals(handle, other)),
    instance_of: |handle, wanted, hierarchy| instance_of(handle, wanted, &[], hierarchy),
    crosses_jars: |handle, _| named(handle),
    // Not `Comparable`: a `TreeSet` of keys throws `ClassCastException`.
    compare_to: None,
};

pub(crate) const TAG_KEY_METHODS: Methods = Methods {
    // `"TagKey[" + registry.identifier() + " / " + location + "]"`.
    to_string: |handle, _| Ok(pair_text(handle, "TagKey")),
    // The record's hash mixes in its registry key's identity hash.
    hash_code: |_, _| Ok(None),
    equals: |handle, other, _| Ok(pair_equals(handle, other)),
    instance_of: |handle, wanted, hierarchy| {
        instance_of(handle, wanted, &["java/lang/Record"], hierarchy)
    },
    crosses_jars: |handle, _| named(handle),
    // Not `Comparable`: a `TreeSet` of keys throws `ClassCastException`.
    compare_to: None,
};

/// The `ResourceKey` text of a `BlockItemId` component: `registry` and the
/// id the component was created from.
fn component_text(registry: &str, id: &Value) -> Option<String> {
    Some(format!("ResourceKey[{registry} / {}]", identifier_key(id)?))
}

fn block_item_to_string(handle: &HostRef) -> Option<String> {
    let (block, item) = block_item_id_parts(&Value::Host(handle.clone()))?;
    Some(format!(
        "BlockItemId[block={}, item={}]",
        component_text("minecraft:block", &block)?,
        component_text("minecraft:item", &item)?
    ))
}

/// The record's `equals`: both components' keys by identity, which for two
/// interned keys made from named ids is id equality.
fn block_item_equals(handle: &HostRef, other: &Value) -> Option<bool> {
    if block_item_id_parts(other).is_none() {
        return Some(false);
    }
    let ids = |value: &Value| {
        let (block, item) = block_item_id_parts(value)?;
        Some((
            identifier_key(&block)?.to_owned(),
            identifier_key(&item)?.to_owned(),
        ))
    };
    Some(ids(&Value::Host(handle.clone()))? == ids(other)?)
}

pub(crate) const BLOCK_ITEM_ID_METHODS: Methods = Methods {
    to_string: |handle, _| Ok(block_item_to_string(handle)),
    hash_code: |_, _| Ok(None),
    equals: |handle, other, _| Ok(block_item_equals(handle, other)),
    instance_of: |handle, wanted, hierarchy| {
        instance_of(handle, wanted, &["java/lang/Record"], hierarchy)
    },
    crosses_jars: |handle, crosses| handle.values().is_some_and(|ids| ids.iter().all(crosses)),
    compare_to: None,
};

impl crate::StdHost {
    /// The registry id, namespace and path of the entry a NeoForge/Forge
    /// holder (`holder/<register>/<name>`) or its registered value
    /// (`registered/…`) names: read from the `DeferredRegister.register` row
    /// it came from, whose register is `deferred-register/<registry key>/<namespace>`.
    /// The registry id is `None` when the register's registry argument named
    /// none.
    fn holder_entry(
        &self,
        receiver: &Value,
        prefix: &str,
    ) -> Option<(Option<String>, String, String)> {
        let key = crate::expect_ref(receiver)?.strip_prefix(prefix)?;
        let entry = self.registrations.iter().find(|entry| {
            key.strip_prefix(entry.registry.as_str())
                .and_then(|rest| rest.strip_prefix('/'))
                == Some(entry.id.as_str())
        })?;
        let (register, namespace) = entry
            .registry
            .strip_prefix("deferred-register/")?
            .rsplit_once('/')?;
        let registry = named_id(&Value::Ref(Some(register.to_owned())));
        Some((registry, namespace.to_owned(), entry.id.clone()))
    }

    /// `DeferredHolder.getId()`/`RegistryObject.getId()`: the entry's
    /// `Identifier` (`namespace:name`); counted when the holder is not one a
    /// `DeferredRegister.register` made.
    pub(crate) fn holder_id(
        &mut self,
        r: &crate::MethodRef,
        receiver: &Value,
        prefix: &str,
    ) -> Result<Value, crate::NativeError> {
        match self.holder_entry(receiver, prefix) {
            Some((_, namespace, path)) => Ok(identifier(&namespace, &path)),
            None => self.unknown_host_call(r),
        }
    }

    /// `DeferredHolder.getKey()`/`Holder.unwrapKey()`: the entry's
    /// `ResourceKey` in its register's registry; an unnamed key when the
    /// register named no registry.
    pub(crate) fn holder_key(
        &mut self,
        r: &crate::MethodRef,
        receiver: &Value,
        prefix: &str,
    ) -> Result<Value, crate::NativeError> {
        match self.holder_entry(receiver, prefix) {
            Some((Some(registry), namespace, path)) => Ok(resource_key_named(
                &registry,
                &format!("{namespace}:{path}"),
            )),
            Some((None, namespace, path)) => {
                let register = Value::Ref(crate::expect_ref(receiver).cloned());
                Ok(resource_key(
                    &register,
                    &identifier(&namespace, &path),
                    None,
                ))
            }
            None => self.unknown_host_call(r),
        }
    }
}

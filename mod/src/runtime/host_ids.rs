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
//! * `ResourceKey`: the registry's id and the location (both
//!   `namespace:path`; `minecraft:item` for `Registries.ITEM`), written with
//!   the one nested-key encoder ([`HostRef::of_values`]) so no jar text can
//!   spell another key's parts. A key whose registry the host cannot name (a
//!   registry argument no model knows) is an *unnamed* key holding the two
//!   arguments: its location still answers, its `toString` and registry id
//!   are counted. `ResourceKey.create` interns (a weak map keyed by the
//!   registry's id and the location), so the kind is canonical. Different
//!   keys prove nothing for an unnamed key, so `equals` between those is
//!   counted.
//! * `TagKey`: its registry key's whole identity (that key's registry and
//!   location: `TagKey` compares the registry key by identity, so a key in
//!   another registry that merely shares the id is another tag) and its
//!   location, or the unnamed form. `TagKey.create` interns too: canonical.
//! * `BlockItemId`: [`HostRef::of_values`] of the block and item ids it was
//!   created from. A record, not interned: not canonical.
//!
//! `ResourceKey` keeps `Object.hashCode`, an identity hash, and `TagKey`'s and
//! `BlockItemId`'s record `hashCode` mixes in their keys' identity hashes, so
//! all three `hashCode`s are counted, never answered (Java 25 prints a
//! different number per JVM).
//!
//! A registry key (`Registries.ITEM`) is the named `ResourceKey` in
//! `minecraft:root` ([`registry_key_of`]); the registry itself
//! (`BuiltInRegistries.ITEM`) is a `Registry` handle (`host_registry.rs`).

use crate::runtime::Value;
use crate::runtime::host_ref::{HostKind, HostRef, Methods};

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
/// registry key (`Registries.X`, a key in `minecraft:root`) naming its
/// registry's id.
pub(crate) fn named_id(value: &Value) -> Option<String> {
    identifier_key(value).map(str::to_owned).or_else(|| {
        let (registry, location) = resource_key_parts(value)?;
        (registry == ROOT).then_some(location)
    })
}

/// `Registries.X`/`ResourceKey.createRegistryKey(id)`: the registry key
/// `ResourceKey[minecraft:root / id]`, interned like every key.
pub(crate) fn registry_key_of(id: &str) -> Value {
    resource_key_named(ROOT, id)
}

/// The identity of a registry key argument, what `ResourceKey.equals`
/// (identity, on interned keys) compares: a named `ResourceKey` handle's
/// registry id and location (`Registries.X` is `(minecraft:root, X)`);
/// anything else is unknown.
fn registry_key_identity(value: &Value) -> Option<(String, String)> {
    resource_key_parts(value)
}

/// The registry every registry key is in.
const ROOT: &str = "minecraft:root";

/// The id of the registry a registry-key argument names: what
/// `ResourceKey.create` reads (`registryKey.identifier()`, whatever registry
/// that key is in).
pub(crate) fn registry_name(value: &Value) -> Option<String> {
    registry_key_identity(value).map(|(_, location)| location)
}

/// The two forms of a `ResourceKey`/`TagKey` key, each written with the one
/// nested-key encoder ([`HostRef::of_values`]), so no text a jar passed can
/// spell another key's parts:
///
/// * named: `1`, then the ids as strings. A `ResourceKey`: its registry's id
///   and its location. A `TagKey`: its registry key's identity (that key's
///   registry and location, [`registry_key_identity`]) and its location,
///   since `TagKey` compares its registry key by identity, not by id.
/// * unnamed: `0`, the registry argument and the location (an `Identifier`
///   when the host knows its id): a registry argument the host cannot name.
enum Pair {
    Named(Vec<String>),
    Unnamed(Value, Value),
}

fn named_key(kind: HostKind, ids: &[&str]) -> Value {
    let mut values = vec![Value::Int(1)];
    values.extend(ids.iter().map(|id| Value::Ref(Some((*id).to_owned()))));
    Value::Host(HostRef::of_values(kind, &values))
}

fn unnamed_key(
    kind: HostKind,
    registry: &Value,
    location: &Value,
    location_id: Option<String>,
) -> Value {
    let location = location_id.map_or_else(|| location.clone(), |id| identifier_of_text(&id));
    Value::Host(HostRef::of_values(
        kind,
        &[Value::Int(0), registry.clone(), location],
    ))
}

fn pair(value: &Value) -> Option<(HostKind, Pair)> {
    let Value::Host(handle) = value else {
        return None;
    };
    if !matches!(handle.kind(), HostKind::ResourceKey | HostKind::TagKey) {
        return None;
    }
    let values = handle.values()?;
    let parts = match values.as_slice() {
        [Value::Int(0), registry, location] => Pair::Unnamed(registry.clone(), location.clone()),
        [Value::Int(1), ids @ ..] => Pair::Named(
            ids.iter()
                .map(|id| crate::runtime::expect_ref(id).cloned())
                .collect::<Option<_>>()?,
        ),
        _ => return None,
    };
    Some((handle.kind(), parts))
}

/// `ResourceKey.create(registry, location)`; `location_id` is the id the
/// location argument names, when the host knows it.
pub(crate) fn resource_key(
    registry: &Value,
    location: &Value,
    location_id: Option<String>,
) -> Value {
    match (registry_name(registry), location_id) {
        (Some(registry), Some(location)) => resource_key_named(&registry, &location),
        (_, location_id) => unnamed_key(HostKind::ResourceKey, registry, location, location_id),
    }
}

/// A named `ResourceKey` for two ids the host holds.
pub(crate) fn resource_key_named(registry: &str, location: &str) -> Value {
    named_key(HostKind::ResourceKey, &[registry, location])
}

/// The `(registry, location)` ids of a named `ResourceKey`.
pub(crate) fn resource_key_parts(value: &Value) -> Option<(String, String)> {
    match pair(value)? {
        (HostKind::ResourceKey, Pair::Named(ids)) => match <[String; 2]>::try_from(ids) {
            Ok([registry, location]) => Some((registry, location)),
            Err(_) => None,
        },
        _ => None,
    }
}

/// Whether `value` is a `ResourceKey` handle, named or not.
pub(crate) fn is_resource_key(value: &Value) -> bool {
    matches!(value, Value::Host(handle) if handle.kind() == HostKind::ResourceKey)
}

/// `TagKey.create(registry, location)`: named only when the registry
/// argument's whole identity is known.
pub(crate) fn tag_key(registry: &Value, location: &Value, location_id: Option<String>) -> Value {
    match (registry_key_identity(registry), location_id) {
        (Some((key_registry, key_location)), Some(location)) => {
            named_key(HostKind::TagKey, &[&key_registry, &key_location, &location])
        }
        (_, location_id) => unnamed_key(HostKind::TagKey, registry, location, location_id),
    }
}

/// Whether `value` is a `TagKey` handle, named or not.
pub(crate) fn is_tag_key(value: &Value) -> bool {
    matches!(value, Value::Host(handle) if handle.kind() == HostKind::TagKey)
}

/// A `ResourceKey`'s `identifier()` or a `TagKey`'s `location()`: the id of
/// a named key, or the location argument an unnamed one was made from.
pub(crate) fn key_location(value: &Value) -> Option<Value> {
    match pair(value)?.1 {
        Pair::Named(ids) => ids.last().map(|location| identifier_of_text(location)),
        Pair::Unnamed(_, location) => Some(location),
    }
}

/// The registry key of a registry key's identity: the interned
/// `ResourceKey` (`Registries.X` for one in `minecraft:root`).
fn registry_key_value(registry: &str, location: &str) -> Value {
    resource_key_named(registry, location)
}

/// `ResourceKey.registryKey()` (`createRegistryKey(registryName)`) or
/// `TagKey.registry()`: the registry key the key was made in, else the
/// registry argument itself for an unnamed key.
pub(crate) fn key_registry(value: &Value) -> Option<Value> {
    match pair(value)? {
        (HostKind::ResourceKey, Pair::Named(ids)) => Some(registry_key_value(ROOT, ids.first()?)),
        (_, Pair::Named(ids)) => match ids.as_slice() {
            [registry, location, _] => Some(registry_key_value(registry, location)),
            _ => None,
        },
        (_, Pair::Unnamed(registry, _)) => Some(registry),
    }
}

/// `key.isFor(registryKey)`. `ResourceKey.isFor` is
/// `registryName.equals(registryKey.identifier())`; `TagKey.isFor` is
/// `this.registry == registryKey`, the registry key's whole identity. An
/// unnamed key's own registry argument is that key only when provably so
/// (`None`: the caller counts).
pub(crate) fn key_is_for(value: &Value, registry_key: &Value) -> Option<bool> {
    match pair(value)? {
        (HostKind::ResourceKey, Pair::Named(ids)) => {
            Some(registry_name(registry_key).as_ref() == ids.first())
        }
        (_, Pair::Named(ids)) => {
            let identity = registry_key_identity(registry_key);
            Some(match (ids.as_slice(), identity) {
                ([registry, location, _], Some((other_registry, other_location))) => {
                    *registry == other_registry && *location == other_location
                }
                _ => false,
            })
        }
        (_, Pair::Unnamed(own, _)) => match (&own, registry_key) {
            (Value::Host(own), Value::Host(other)) => own.same_object(other).filter(|same| *same),
            (own, other) => (own == other).then_some(true),
        },
    }
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
/// supertypes, or an interface a loaded Mixin adds (the hierarchy). These
/// classes' superclass chains are fixed (a Mixin adds interfaces, never a
/// superclass), so any other *class* is a proved "no"; another interface is
/// unknown (another jar's Mixin may add it, as #163 settles) and counted.
fn instance_of(
    handle: &HostRef,
    wanted: &str,
    supertypes: &[&str],
    hierarchy: &dyn crate::runtime::host_ref::ClassHierarchy,
) -> Option<bool> {
    // Every kind here has a fixed class.
    let class = handle.class()?;
    if wanted == class || wanted == "java/lang/Object" || supertypes.contains(&wanted) {
        return Some(true);
    }
    if hierarchy.is_instance(class, wanted) == Some(true) {
        return Some(true);
    }
    let a_class = [IDENTIFIER, RESOURCE_KEY, TAG_KEY, BLOCK_ITEM_ID].contains(&wanted)
        || hierarchy.is_interface(wanted) == Some(false);
    a_class.then_some(false)
}

fn named(handle: &HostRef) -> bool {
    matches!(
        pair(&Value::Host(handle.clone())),
        Some((_, Pair::Named(_)))
    )
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
    hash_code: |handle, _| Ok(Some(crate::runtime::hash_map_order::identifier_hash(handle.key()))),
    equals: |handle, other, _| {
        Ok(match other {
            // An object a `new ResourceLocation(..)` built (pre-1.21): its id
            // lives in the host's table, which no `Nested` reaches.
            other if crate::runtime::jar_object::is_object(other) => crate::runtime::jar_object::object_class(other)
                .is_none_or(|class| class != RESOURCE_LOCATION && class != IDENTIFIER)
                .then_some(false),
            Value::Host(other) => Some(other == handle),
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
    // `compareTo(null)` reads `null.path`: a `NullPointerException`.
    compare_to: Some(|handle, other, _| match other {
        Value::Ref(None) => Err(crate::runtime::NativeError::UncaughtException {
            class: "java/lang/NullPointerException".into(),
            message: None,
            frame: None,
        }),
        other => Ok(identifier_compare(handle, other)),
    }),
};

/// `Identifier.compareTo`: the paths by `String.compareTo`, then the
/// namespaces on a tie. Any other value is not an `Identifier` (a JVM throws
/// `ClassCastException`): unknown here.
fn identifier_compare(handle: &HostRef, other: &Value) -> Option<i32> {
    let (namespace, path) = handle.key().split_once(':')?;
    let (other_namespace, other_path) = identifier_key(other)?.split_once(':')?;
    Some(
        match crate::runtime::java_identifier::java_compare(path, other_path) {
            0 => crate::runtime::java_identifier::java_compare(namespace, other_namespace),
            order => order,
        },
    )
}

/// `"ResourceKey[" + registryName + " / " + identifier + "]"` and
/// `"TagKey[" + registry.identifier() + " / " + location + "]"`: the last two
/// ids of a named key.
fn pair_text(handle: &HostRef, class: &str) -> Option<String> {
    match pair(&Value::Host(handle.clone()))?.1 {
        Pair::Named(ids) => match ids.as_slice() {
            [.., registry, location] => Some(format!("{class}[{registry} / {location}]")),
            _ => None,
        },
        Pair::Unnamed(..) => None,
    }
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

impl crate::runtime::StdHost {
    /// `DeferredRegister.create(registryKey, namespace)` (and its
    /// `createDataComponents`): a new register of `class`, named `text` in the
    /// registration rows, remembering what it was made from.
    pub(crate) fn new_deferred_register(
        &mut self,
        class: &str,
        text: &str,
        registry: &Value,
        namespace: Option<&String>,
    ) -> Value {
        let register = crate::runtime::host_deferred_register::new_register(class, text);
        if let Some((key, _)) = crate::runtime::host_deferred_register::parts(&register) {
            let made = namespace.map(|namespace| (registry.clone(), namespace.clone()));
            self.deferred_registers.insert(key.to_owned(), made);
        }
        register
    }

    /// `DeferredRegister.register(name, ..)`: NeoForge builds the entry's id
    /// with `Identifier.fromNamespaceAndPath(namespace, name)` (Forge with
    /// `new ResourceLocation(modid, name)`), which throws for a namespace or
    /// name outside the id alphabet. `Ok(None)` when the register's
    /// arguments are unknown here (nothing is validated or recorded).
    pub(crate) fn deferred_register_id(
        &mut self,
        forge: bool,
        register: &str,
        name: &str,
    ) -> Result<Option<(Value, Value)>, crate::runtime::NativeError> {
        let Some(Some((registry, namespace))) = self.deferred_registers.get(register).cloned()
        else {
            return Ok(None);
        };
        let owner = if forge { RESOURCE_LOCATION } else { IDENTIFIER };
        let id = crate::runtime::java_identifier::checked(owner, &namespace, name)?;
        Ok(Some((registry, id)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Key parts made of text with every separator a layer uses (`|`, `,`,
    /// `:`, `/`, `\`, `#`, `?`, the encoder's tags) read back as themselves,
    /// and two different part lists never make one key: no jar text can
    /// spell another key's registry or location ("keys never parse jar
    /// text", #156).
    #[test]
    fn key_parts_with_separators_round_trip_and_never_alias() {
        let alphabet = [
            "a", "|", ",", ":", "/", "\\", "#", "?", "r", "i1", "hrk:", "\\,", "",
        ];
        let texts: Vec<String> = alphabet
            .iter()
            .flat_map(|a| alphabet.iter().map(move |b| format!("{a}{b}")))
            .collect();
        let mut seen = std::collections::BTreeMap::new();
        for (index, registry) in texts.iter().enumerate() {
            let location = &texts[(index * 7 + 3) % texts.len()];
            let key = resource_key_named(registry, location);
            assert_eq!(
                resource_key_parts(&key),
                Some((registry.clone(), location.clone())),
                "{registry:?} {location:?}"
            );
            let registry_key = registry_key_of(registry);
            let tag = tag_key(&registry_key, &Value::Ref(None), Some(location.clone()));
            assert_eq!(key_location(&tag), Some(identifier_of_text(location)));
            assert_eq!(key_registry(&tag), Some(registry_key));
            let Value::Host(handle) = &key else {
                panic!("{key:?}")
            };
            let parts = (registry.clone(), location.clone());
            if let Some(other) = seen.insert(handle.key().to_owned(), parts.clone()) {
                assert_eq!(other, parts);
            }
        }
    }
}

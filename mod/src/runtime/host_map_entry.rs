//! The host's `Map.Entry` objects as host handles (#156 P14e).
//!
//! They were the text `map-entry/<key>,<value>`: a `String` could spell one, two equal texts
//! were one object, and `setValue` searched every map for a pair equal to the text (so a node
//! whose value a later `put` changed, or whose key was removed, threw
//! `UnsupportedOperationException` without counting, #212). Each JDK entry class keeps its own
//! rules, and Java 25 runs them as follows:
//!
//! * **`Map.entry(k, v)`** and the entries of `Map.of(..)`: a `KeyValueHolder`, new per call,
//!   immutable (`setValue` throws `UnsupportedOperationException("not supported")`), `null`
//!   refused.
//! * **`TreeMap.firstEntry()`** and the other navigation entries: an
//!   `AbstractMap.SimpleImmutableEntry` snapshot, new per call (`setValue` throws a bare
//!   `UnsupportedOperationException`).
//! * **`entrySet()` of a `HashMap`/`LinkedHashMap`/`TreeMap`**: the map's own node
//!   (`HashMap$Node`, `LinkedHashMap$Entry`, `TreeMap$Entry`). Every reading of one node is
//!   the same object; it sees a later `put` of its key; `setValue` writes the map. A node
//!   whose key is removed is detached: it keeps its last value, and `setValue` updates it, not
//!   the map (#212). A key removed and put again is a new node. `TreeMap.deleteEntry` of a node
//!   with two children copies its successor's key and value into it, so that object then holds
//!   the successor's mapping and the successor's own node is the detached one.
//! * Every class answers `equals` as `o instanceof Map.Entry e && eq(key, e.getKey()) &&
//!   eq(value, e.getValue())`, `hashCode` as `Objects.hashCode(key) ^ Objects.hashCode(value)`
//!   and `toString` as `key + "=" + value`.
//!
//! **Keys and identity.** A holder or snapshot is keyed `<class>#entry<n>`, one key per object
//! (canonical). A node is keyed `<class>#node<n>` and is never canonical: the host hands out
//! the same handle (one origin) for every reading of one node, so `==` of two readings is
//! exact, and a new key only when Java makes a new node (a removed key put again). The node's
//! state ([`Entries`]) follows its map through every call on it ([`StdHost::entries_after_call`]):
//! a `HashMap` node by its key, a `TreeMap` node by its tree node (`tree_map.rs` runs
//! `deleteEntry` as the JDK does). Where this model loses a node (a treeified bin, a tree with an
//! unplaced key, a change by a path that is not a call on the map), the entry's contents are
//! counted, and a later reading keeps the key with a fresh origin, so `==` with an earlier one
//! is counted too.
//!
//! **Other maps** (`ConcurrentHashMap`, `Hashtable`, `IdentityHashMap`, a host literal...):
//! their entries are [`HostKind::OpaqueMapEntry`], whose class is not known (`getClass()`
//! counted). Their key and value are as read while the map is unchanged; `setValue` is
//! counted.

use std::collections::{BTreeMap, BTreeSet};

use crate::runtime::host_ref::{HostKind, HostRef, Methods, Nested, instance_of_class};
use crate::runtime::mod_count::ModStamp;
use crate::runtime::{MethodRef, NativeError, StdHost, Value};

pub(crate) const KEY_VALUE_HOLDER: &str = "java/util/KeyValueHolder";
pub(crate) const SIMPLE_IMMUTABLE_ENTRY: &str = "java/util/AbstractMap$SimpleImmutableEntry";
const OPAQUE: &str = "java/util/Map$Entry";

/// What a producer hands out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryMade {
    /// `Map.entry`, `Map.of(..)`'s entries: a new immutable `KeyValueHolder`.
    Holder,
    /// A `TreeMap` navigation entry: a new immutable `SimpleImmutableEntry`.
    Snapshot,
    /// An entry of a map's `entrySet()`: its node.
    Node,
}

/// `ClassRule::FromKey`: the class before `#`.
pub(crate) fn entry_class(key: &str) -> &str {
    key.rsplit_once('#').map_or(key, |(class, _)| class)
}

/// One key per object for holders and snapshots; a node's identity is its origin.
pub(crate) fn entry_canonical(key: &str) -> bool {
    key.rsplit_once('#')
        .is_some_and(|(_, serial)| serial.starts_with("entry"))
}

pub(crate) fn opaque_types(_: &str) -> &'static [&'static str] {
    &["java/util/Map$Entry"]
}

/// Whether `value` is one of the host's entries.
pub(crate) fn is_entry(value: &Value) -> bool {
    matches!(value, Value::Host(handle)
        if matches!(handle.kind(), HostKind::MapEntry | HostKind::OpaqueMapEntry))
}

/// Whether a text spells an entry's key (`java/util/HashMap$Node#node3`): a `String`, since
/// every entry is a handle (`value_class::is_plain_string`).
pub(crate) fn spells_entry_key(text: &str) -> bool {
    text.rsplit_once('#').is_some_and(|(class, serial)| {
        !class.is_empty()
            && serial
                .strip_prefix("entry")
                .or_else(|| serial.strip_prefix("node"))
                .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
    })
}

/// Whether `value` is a `KeyValueHolder` (`Map.entry`).
pub(crate) fn is_key_value_holder(value: &Value) -> bool {
    matches!(value, Value::Host(handle)
        if handle.kind() == HostKind::MapEntry && entry_class(handle.key()) == KEY_VALUE_HOLDER)
}

/// An entry whose contents no host keeps (a host without an entry table).
pub(crate) fn unknown_entry() -> Value {
    Value::Host(HostRef::new(
        HostKind::OpaqueMapEntry,
        "java/util/Map$Entry#node0",
    ))
}

/// The node class of a map class whose nodes this model follows.
fn node_class(map_class: &str) -> Option<&'static str> {
    Some(match map_class {
        "java/util/HashMap" => "java/util/HashMap$Node",
        "java/util/LinkedHashMap" => "java/util/LinkedHashMap$Entry",
        "java/util/TreeMap" => "java/util/TreeMap$Entry",
        _ => return None,
    })
}

fn class_of_marker(marker: &str) -> &str {
    marker.rsplit_once('#').map_or("", |(class, _)| class)
}

/// What the host knows of one entry.
#[derive(Clone, Debug, PartialEq)]
enum Life {
    /// A holder or snapshot: its key and value never change.
    Fixed,
    /// A node `map` holds: its value is the map's. `node` is its `TreeMap` tree node. `stamp`
    /// is the map's `modCount` when the host last followed it.
    Live {
        map: String,
        stamp: ModStamp,
        node: Option<usize>,
    },
    /// A node removed from its map: the key and value it kept.
    Detached,
    /// A node this model lost track of.
    Lost,
    /// An entry of a map whose entry class is not modelled, as read at `stamp`.
    Opaque { map: String, stamp: ModStamp },
}

#[derive(Clone, Debug, PartialEq)]
struct EntryState {
    key: Value,
    value: Value,
    life: Life,
    /// The handle every reading of a live node answers (one origin).
    handle: Value,
}

/// The entries the host made, by handle key.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Entries {
    states: BTreeMap<String, EntryState>,
    /// A live node's entry key, by its map and the node (`k<key marker>` for a hash node,
    /// `n<index>` for a tree node).
    nodes: BTreeMap<(String, String), String>,
    /// Maps whose nodes this model lost: their entries' identity is counted.
    lost_maps: BTreeSet<String>,
    /// The key and value of each live node of a map, taken before a call on it.
    before: BTreeMap<String, Vec<(String, Value, Value)>>,
    next: u64,
}

impl Entries {
    /// Every key and value an entry holds (heap reachability).
    pub(crate) fn held(&self, key: &str) -> impl Iterator<Item = &Value> {
        self.states
            .get(key)
            .into_iter()
            .flat_map(|state| [&state.key, &state.value])
    }

    /// The entry keys, for the heap's collectable set.
    pub(crate) fn keys(&self) -> impl Iterator<Item = &String> {
        self.states.keys()
    }

    /// The map a live node follows (kept alive with it).
    pub(crate) fn map_of(&self, key: &str) -> Option<&str> {
        match &self.states.get(key)?.life {
            Life::Live { map, .. } | Life::Opaque { map, .. } => Some(map),
            _ => None,
        }
    }

    /// Drop the entries the heap collected.
    pub(crate) fn retain(&mut self, keep: impl Fn(&str) -> bool) {
        self.states.retain(|key, _| keep(key));
        let states = &self.states;
        self.nodes.retain(|_, key| states.contains_key(key));
    }
}

impl StdHost {
    /// The value `map` holds for `key` (compared as stored: the node's own key).
    fn entry_map_value(&self, map: &str, key: &Value) -> Option<Value> {
        self.mutable_collections
            .get(map)?
            .chunks(2)
            .find(|pair| pair.first() == Some(key))
            .and_then(|pair| pair.get(1).cloned())
    }

    /// The tree node of `key` in `map`'s tree, and whether the tree is followed exactly.
    fn tree_node(&self, map: &str, key: &Value) -> Option<usize> {
        let tree = self.red_black_trees.get(map)?;
        (!tree.tainted).then(|| tree.index_of(key)).flatten()
    }

    /// Whether this model follows `map`'s nodes: not a treeified hash table, not a tree with an
    /// unplaced key.
    fn nodes_followed(&self, map: &str) -> bool {
        !self.map_entries.lost_maps.contains(map)
            && !self
                .hash_tables
                .get(map)
                .is_some_and(|table| table.treeified)
            && !self
                .red_black_trees
                .get(map)
                .is_some_and(|tree| tree.tainted)
    }

    fn mint_key(&mut self, class: &str, node: bool) -> String {
        self.map_entries.next += 1;
        let serial = self.map_entries.next;
        format!("{class}#{}{serial}", if node { "node" } else { "entry" })
    }

    /// The entry a producer hands out: a new holder or snapshot, or `map`'s node of `key`.
    pub(crate) fn mint_map_entry(
        &mut self,
        map: Option<&Value>,
        key: &Value,
        value: &Value,
        made: EntryMade,
    ) -> Value {
        let fixed = |host: &mut Self, class: &str| {
            let entry_key = host.mint_key(class, false);
            let handle = Value::Host(HostRef::new(HostKind::MapEntry, entry_key.clone()));
            host.map_entries.states.insert(
                entry_key,
                EntryState {
                    key: key.clone(),
                    value: value.clone(),
                    life: Life::Fixed,
                    handle: handle.clone(),
                },
            );
            handle
        };
        match made {
            EntryMade::Holder => return fixed(self, KEY_VALUE_HOLDER),
            EntryMade::Snapshot => return fixed(self, SIMPLE_IMMUTABLE_ENTRY),
            EntryMade::Node => {}
        }
        let map = map.and_then(crate::runtime::expect_ref).cloned().unwrap_or_default();
        let stamp = self.mod_stamp_of(&map);
        let Some(class) = node_class(class_of_marker(&map)) else {
            // A map whose entry class is not modelled.
            let entry_key = self.mint_key(OPAQUE, true);
            let handle = Value::Host(HostRef::new(HostKind::OpaqueMapEntry, entry_key.clone()));
            self.map_entries.states.insert(
                entry_key,
                EntryState {
                    key: key.clone(),
                    value: value.clone(),
                    life: Life::Opaque { map, stamp },
                    handle: handle.clone(),
                },
            );
            return handle;
        };
        let tree = class == "java/util/TreeMap$Entry";
        let node = if tree {
            self.tree_node(&map, key)
        } else {
            None
        };
        let node_name = match node {
            Some(index) => format!("n{index}"),
            None => format!("k{}", crate::runtime::value_marker(key)),
        };
        let followed = self.nodes_followed(&map) && (!tree || node.is_some());
        if let Some(entry_key) = self
            .map_entries
            .nodes
            .get(&(map.clone(), node_name.clone()))
            && let Some(state) = self.map_entries.states.get(entry_key)
        {
            // The same node: the same handle while followed; otherwise the same key with a
            // fresh origin, so `==` with an earlier reading is counted.
            if followed && state.life != Life::Lost {
                return state.handle.clone();
            }
            return Value::Host(HostRef::new(HostKind::MapEntry, entry_key.clone()));
        }
        let entry_key = self.mint_key(class, true);
        let handle = Value::Host(HostRef::new(HostKind::MapEntry, entry_key.clone()));
        let life = if followed {
            Life::Live {
                map: map.clone(),
                stamp,
                node,
            }
        } else {
            Life::Lost
        };
        self.map_entries.states.insert(
            entry_key.clone(),
            EntryState {
                key: key.clone(),
                value: value.clone(),
                life,
                handle: handle.clone(),
            },
        );
        self.map_entries.nodes.insert((map, node_name), entry_key);
        handle
    }

    fn mod_stamp_of(&self, map: &str) -> ModStamp {
        crate::runtime::Host::mod_stamp(self, map)
    }

    /// The key and value `entry` holds now; `None` for any other value, or when unknown.
    pub(crate) fn map_entry_now(&self, entry: &Value) -> Option<(Value, Value)> {
        let Value::Host(handle) = entry else {
            return None;
        };
        let state = self.map_entries.states.get(handle.key())?;
        match &state.life {
            Life::Fixed | Life::Detached => Some((state.key.clone(), state.value.clone())),
            Life::Live { map, stamp, node } => {
                if self.mod_stamp_of(map) != *stamp {
                    return None;
                }
                let key = match node {
                    Some(index) => self.red_black_trees.get(map)?.key_at(*index)?.clone(),
                    None => state.key.clone(),
                };
                let value = self.entry_map_value(map, &key)?;
                Some((key, value))
            }
            Life::Lost => None,
            // A map the host keeps no row for (an immutable literal) cannot have changed;
            // a kept one must still hold what was read, with no structural change since.
            Life::Opaque { map, stamp } => (!self.mutable_collections.contains_key(map)
                || self.mod_stamp_of(map) == *stamp
                    && self.entry_map_value(map, &state.key).as_ref() == Some(&state.value))
            .then(|| (state.key.clone(), state.value.clone())),
        }
    }

    /// `entry.setValue(value)`: an immutable entry throws, a live node writes its map, a
    /// detached node keeps the value; an unknown one is counted.
    pub(crate) fn map_entry_set_value(
        &mut self,
        r: &MethodRef,
        entry: &Value,
        value: Value,
    ) -> Result<Value, NativeError> {
        let Value::Host(handle) = entry else {
            return self.unknown_host_call(r);
        };
        let Some(state) = self.map_entries.states.get(handle.key()).cloned() else {
            return self.unknown_host_call(r);
        };
        let unsupported = |message: Option<&str>| NativeError::UncaughtException {
            class: "java/lang/UnsupportedOperationException".into(),
            message: message.map(str::to_owned),
            frame: None,
        };
        match &state.life {
            // `KeyValueHolder.setValue` and `SimpleImmutableEntry.setValue`.
            Life::Fixed if entry_class(handle.key()) == KEY_VALUE_HOLDER => {
                Err(unsupported(Some("not supported")))
            }
            Life::Fixed => Err(unsupported(None)),
            Life::Detached => {
                let states = &mut self.map_entries.states;
                let state = states.get_mut(handle.key()).expect("found above");
                Ok(std::mem::replace(&mut state.value, value))
            }
            Life::Live { map, .. } => {
                let Some((key, _)) = self.map_entry_now(entry) else {
                    return self.unknown_host_call(r);
                };
                let map = map.clone();
                let Some(row) = self.mutable_collections.get_mut(&map) else {
                    return self.unknown_host_call(r);
                };
                let Some(slot) = row
                    .chunks_mut(2)
                    .find(|pair| pair.first() == Some(&key))
                    .and_then(|pair| pair.get_mut(1))
                else {
                    return self.unknown_host_call(r);
                };
                Ok(std::mem::replace(slot, value))
            }
            Life::Lost | Life::Opaque { .. } => self.unknown_host_call(r),
        }
    }

    /// Before a call on `map`: the key and value of each of its live nodes (what a node the
    /// call removes keeps).
    pub(crate) fn entries_before_call(&mut self, map: &str) {
        let live: Vec<String> = self
            .map_entries
            .states
            .iter()
            .filter(|(_, state)| matches!(&state.life, Life::Live { map: m, .. } if m == map))
            .map(|(key, _)| key.clone())
            .collect();
        if live.is_empty() {
            return;
        }
        let mut taken = Vec::new();
        for key in live {
            let handle = self.map_entries.states[&key].handle.clone();
            if let Some((entry_key, value)) = self.map_entry_now(&handle) {
                taken.push((key, entry_key, value));
            }
        }
        self.map_entries.before.insert(map.to_owned(), taken);
    }

    /// After a call on `map`: follow its live nodes. Without a structural change they stay as
    /// they are. After one, a hash node whose key is gone is detached with what it held before
    /// the call, and the others stay; a tree node follows its tree node (a freed one is
    /// detached, a kept one holds whatever key `deleteEntry` left in it). A treeified table or
    /// a tree with an unplaced key loses them all.
    pub(crate) fn entries_after_call(&mut self, map: &str) {
        let Some(before) = self.map_entries.before.remove(map) else {
            return;
        };
        let now = self.mod_stamp_of(map);
        let followed = self.nodes_followed(map);
        if !followed {
            self.map_entries.lost_maps.insert(map.to_owned());
        }
        let live: Vec<String> = self
            .map_entries
            .states
            .iter()
            .filter(|(_, state)| matches!(&state.life, Life::Live { map: m, .. } if m == map))
            .map(|(key, _)| key.clone())
            .collect();
        for entry_key in live {
            let Life::Live { stamp, node, .. } = self.map_entries.states[&entry_key].life.clone()
            else {
                continue;
            };
            // What the node held before the call; a node the host could not read then (its map
            // changed by another path) is lost.
            let held = before.iter().find(|(key, _, _)| *key == entry_key);
            let life = match held {
                _ if !followed => Life::Lost,
                None => Life::Lost,
                Some(_) if stamp == now => Life::Live {
                    map: map.to_owned(),
                    stamp,
                    node,
                },
                Some((_, key, _)) => match node {
                    // A tree node: still in the tree (holding whatever key `deleteEntry` left
                    // in it), or freed.
                    Some(index) => {
                        let kept = self
                            .red_black_trees
                            .get(map)
                            .and_then(|tree| tree.key_at(index))
                            .is_some();
                        if kept {
                            Life::Live {
                                map: map.to_owned(),
                                stamp: now,
                                node,
                            }
                        } else {
                            Life::Detached
                        }
                    }
                    None if self.entry_map_value(map, key).is_some() => Life::Live {
                        map: map.to_owned(),
                        stamp: now,
                        node,
                    },
                    None => Life::Detached,
                },
            };
            let state = self
                .map_entries
                .states
                .get_mut(&entry_key)
                .expect("listed above");
            if let (Life::Detached, Some((_, key, value))) = (&life, held) {
                state.key = key.clone();
                state.value = value.clone();
            }
            state.life = life;
        }
        // A detached node is never handed out again: a later reading of its key is a new node.
        let states = &self.map_entries.states;
        self.map_entries.nodes.retain(|(m, _), entry_key| {
            m != map
                || states
                    .get(entry_key)
                    .is_some_and(|state| state.life != Life::Detached)
        });
    }
}

/// The entry's key and value now, as the context can read them.
fn parts(handle: &HostRef, nested: &mut dyn Nested) -> Option<(Value, Value)> {
    nested.map_entry_parts(&Value::Host(handle.clone()))
}

fn entry_text(handle: &HostRef, nested: &mut dyn Nested) -> Result<Option<String>, NativeError> {
    let Some((key, value)) = parts(handle, nested) else {
        return Ok(None);
    };
    let (Some(key), Some(value)) = (nested.text(&key)?, nested.text(&value)?) else {
        return Ok(None);
    };
    Ok(Some(format!("{key}={value}")))
}

fn entry_hash(handle: &HostRef, nested: &mut dyn Nested) -> Result<Option<i32>, NativeError> {
    let Some((key, value)) = parts(handle, nested) else {
        return Ok(None);
    };
    let (Some(key), Some(value)) = (nested.hash(&key)?, nested.hash(&value)?) else {
        return Ok(None);
    };
    Ok(Some(key ^ value))
}

fn entry_equals(
    handle: &HostRef,
    other: &Value,
    nested: &mut dyn Nested,
) -> Result<Option<bool>, NativeError> {
    match other {
        Value::Host(theirs) if is_entry(other) => {
            if handle.same_object(theirs) == Some(true) {
                return Ok(Some(true));
            }
            let (Some((key, value)), Some((other_key, other_value))) =
                (parts(handle, nested), nested.map_entry_parts(other))
            else {
                return Ok(None);
            };
            Ok(match nested.equals(&key, &other_key)? {
                Some(true) => nested.equals(&value, &other_value)?,
                answer => answer,
            })
        }
        // A jar's own `Map.Entry` answers through its accessors (`jdk_skeletons`'s
        // `host_entry_equals` runs them); here it is unknown.
        other if crate::runtime::jar_object::object_key(other).is_some() => Ok(None),
        // `null`, a `String`, a box, another host value: no `Map.Entry`.
        _ => Ok(Some(false)),
    }
}

pub(crate) const ENTRY_METHODS: Methods = Methods {
    to_string: entry_text,
    hash_code: entry_hash,
    equals: entry_equals,
    instance_of: instance_of_class,
    // An entry lives in this jar's host.
    crosses_jars: |_, _| false,
    // `Map.Entry` is not `Comparable`.
    compare_to: None,
};

#[cfg(test)]
#[path = "host_map_entry_tests.rs"]
mod tests;

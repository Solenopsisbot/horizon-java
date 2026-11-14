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
//! **Other maps** follow their own classes (Java 25): `Hashtable`'s and `WeakHashMap`'s
//! nodes are followed like `HashMap`'s (`Hashtable` refuses `setValue(null)`); `EnumMap` and
//! `IdentityHashMap` make a new entry per `next()` that reads and writes the map by its key;
//! `ConcurrentHashMap` makes a new `MapEntry` per `next()` that keeps its own value and
//! `put`s on `setValue`; `ConcurrentSkipListMap` hands out immutable snapshots. The entries of
//! any other map (a host literal, `Collections.unmodifiableMap`, a library's) are
//! [`HostKind::OpaqueMapEntry`], whose class is not known (`getClass()` counted): their key and
//! value are as read while the map is unchanged, and `setValue` is counted.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::host_ref::{HostKind, HostRef, Methods, Nested, instance_of_class};
use crate::mod_count::ModStamp;
use crate::{MethodRef, NativeError, StdHost, Value};

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
/// How a map's `entrySet()` hands out its entries (Java 25).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    /// The map's own node: every reading is one object, live, written through. `nulls`:
    /// `setValue(null)` is allowed (`Hashtable` throws).
    Node { nulls: bool },
    /// A new entry per `next()` that reads and writes the map by its key (`EnumMap`,
    /// `IdentityHashMap`).
    Fresh,
    /// `ConcurrentHashMap.MapEntry`: a new entry per `next()` holding its own value;
    /// `setValue(v)` sets it and `put`s it (`null` refused).
    Concurrent,
    /// An immutable snapshot per `next()` (`ConcurrentSkipListMap`).
    Snapshot,
}

/// The entry class and mode of a map class whose entries this model knows.
fn entry_mode(map_class: &str) -> Option<(&'static str, Mode)> {
    Some(match map_class {
        "java/util/HashMap" => ("java/util/HashMap$Node", Mode::Node { nulls: true }),
        "java/util/LinkedHashMap" => ("java/util/LinkedHashMap$Entry", Mode::Node { nulls: true }),
        "java/util/TreeMap" => ("java/util/TreeMap$Entry", Mode::Node { nulls: true }),
        "java/util/Hashtable" => ("java/util/Hashtable$Entry", Mode::Node { nulls: false }),
        "java/util/WeakHashMap" => ("java/util/WeakHashMap$Entry", Mode::Node { nulls: true }),
        "java/util/EnumMap" => ("java/util/EnumMap$EntryIterator$Entry", Mode::Fresh),
        "java/util/IdentityHashMap" => {
            ("java/util/IdentityHashMap$EntryIterator$Entry", Mode::Fresh)
        }
        "java/util/concurrent/ConcurrentHashMap" => (
            "java/util/concurrent/ConcurrentHashMap$MapEntry",
            Mode::Concurrent,
        ),
        "java/util/concurrent/ConcurrentSkipListMap" => (SIMPLE_IMMUTABLE_ENTRY, Mode::Snapshot),
        _ => return None,
    })
}

/// What two readings of one mapping of a map this model does not follow are (Java 25 and the
/// libraries' own classes).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OpaqueReading {
    /// A new entry per `next()`: `Collections.unmodifiableMap`'s `UnmodifiableEntry` and
    /// `checkedMap`'s `CheckedEntry` wrap the map's entry afresh.
    New,
    /// The map's stored entry, one object while the map has no structural change:
    /// `Collections.singletonMap`'s cached `SimpleImmutableEntry`, a host literal (Guava's
    /// `ImmutableMap.of`, whose `ImmutableMapEntry` array `entrySet()` walks), Gson's
    /// `LinkedTreeMap` node.
    Stored,
    /// Not known: one key per mapping, a fresh origin per reading (`==` counted).
    Unknown,
}

fn opaque_reading(map: &str, view: bool) -> OpaqueReading {
    let class = class_of_marker(map);
    if view || class.starts_with("java/util/Collections$Checked") {
        OpaqueReading::New
    } else if class == "java/util/Collections$SingletonMap"
        || class == crate::gson_tree::TREE_MAP
        // A host literal (`map/k,v...`): an immutable map the host keeps no row for.
        || (!map.is_empty() && !map.contains('#'))
    {
        OpaqueReading::Stored
    } else {
        OpaqueReading::Unknown
    }
}

fn class_of_marker(marker: &str) -> &str {
    marker.rsplit_once('#').map_or("", |(class, _)| class)
}

/// What the host knows of one entry.
#[derive(Clone, Debug, PartialEq)]
enum Life {
    /// A holder or snapshot: its key and value never change.
    Fixed,
    /// A node `map` holds (or an entry reading `map` by its key, `fresh`): its value is the
    /// map's, read while the map's `modCount` is where the host last followed it
    /// ([`Entries::followed`]). `node` is its `TreeMap` tree node; `slot` the pair index its key
    /// last sat at in the map's row (a hint: a read checks it before scanning). `nulls`:
    /// `setValue(null)` is allowed.
    Live {
        map: String,
        node: Option<usize>,
        slot: usize,
        fresh: bool,
        nulls: bool,
    },
    /// A `ConcurrentHashMap.MapEntry` of `map`: it keeps its own value.
    Concurrent { map: String },
    /// A node removed from its map: the key and value it kept.
    Detached,
    /// A node this model lost track of.
    Lost,
    /// An entry of a map whose entry class is not modelled, as read at `stamp`. `view`: `map` is
    /// the map an unmodifiable view reads, whose entries the view's entries read live.
    Opaque {
        map: String,
        stamp: ModStamp,
        view: bool,
    },
}

#[derive(Clone, Debug, PartialEq)]
struct EntryState {
    key: Value,
    value: Value,
    life: Life,
    /// The handle every reading of a live node answers (one origin).
    handle: Value,
}

/// What a call on a map with live nodes started from.
#[derive(Clone, Debug, PartialEq)]
struct Before {
    stamp: ModStamp,
    /// The live nodes the call may remove, with the key and value each held (what a removed node
    /// keeps); `None` for a call that only reads, inserts or replaces values (`put`, `get`...),
    /// after which every node is still there.
    taken: Option<Taken>,
}

/// The nodes a removing call may remove.
#[derive(Clone, Debug, PartialEq)]
enum Taken {
    /// Any of them (`clear`, `removeIf`, a `TreeMap`'s `deleteEntry`...): every live node.
    All(HashMap<String, (Value, Value)>),
    /// Only the node of the one key `remove(key)` names, if it has one (#221 review B3: a
    /// `remove` after an `entrySet()` walk looks at that node, never every node the walk made).
    Key(Option<(String, Value, Value)>),
}

/// The entries the host made, by handle key.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Entries {
    states: HashMap<String, EntryState>,
    /// A live node's entry key, by its map and then the node (`k<key marker>` for a hash node,
    /// `n<index>` for a tree node).
    nodes: BTreeMap<String, HashMap<String, String>>,
    /// Lookup memos, rebuilt when the map's row (its collection generation) or tree (its
    /// `modCount` and size) changes: each key's pair index in the row, each key's tree node
    /// (#221 review B3: a view of n entries is minted in O(n), not O(n²)).
    slot_memo: BTreeMap<String, (u64, HashMap<String, usize>)>,
    tree_memo: BTreeMap<String, (u64, usize, HashMap<String, usize>)>,
    /// The keys of the last `entrySet()` view built per map, position by position (its handles
    /// are the view's own contents): a rebuild reuses each handle whose key still sits at its
    /// position. Dropped whenever one of the map's entries stops being live.
    view_memo: BTreeMap<String, Vec<Value>>,
    /// The live entries of each map, by entry key (#221 review B3: a call on a map looks at its
    /// own, never every entry).
    by_map: BTreeMap<String, BTreeSet<String>>,
    /// Each map's `modCount` at which its live entries were last followed: they read the map
    /// while it is still there.
    followed: BTreeMap<String, ModStamp>,
    /// Maps whose nodes this model lost: their entries' identity is counted.
    lost_maps: BTreeSet<String>,
    before: BTreeMap<String, Before>,
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
            Life::Live { map, .. } | Life::Opaque { map, .. } | Life::Concurrent { map } => {
                Some(map)
            }
            _ => None,
        }
    }

    /// Drop the entries the heap collected.
    pub(crate) fn retain(&mut self, keep: impl Fn(&str) -> bool) {
        self.states.retain(|key, _| keep(key));
        let states = &self.states;
        for nodes in self.nodes.values_mut() {
            nodes.retain(|_, key| states.contains_key(key));
        }
        self.nodes.retain(|_, nodes| !nodes.is_empty());
        // The memos are caches: rebuilt on the next use.
        self.slot_memo.clear();
        self.tree_memo.clear();
        self.view_memo.clear();
        for live in self.by_map.values_mut() {
            live.retain(|key| states.contains_key(key));
        }
        self.by_map.retain(|_, live| !live.is_empty());
    }

    /// Set an entry's life, keeping [`Self::by_map`] to the live ones.
    fn set_life(&mut self, entry_key: &str, life: Life) {
        let Some(state) = self.states.get_mut(entry_key) else {
            return;
        };
        if let Life::Live { map, .. } = &state.life {
            if let Some(live) = self.by_map.get_mut(map) {
                live.remove(entry_key);
            }
            if !matches!(life, Life::Live { .. }) {
                self.view_memo.remove(map);
            }
        }
        if let Life::Live { map, .. } = &life {
            self.by_map
                .entry(map.clone())
                .or_default()
                .insert(entry_key.to_owned());
        }
        state.life = life;
    }

    fn live_of(&self, map: &str) -> Vec<String> {
        self.by_map
            .get(map)
            .map(|live| live.iter().cloned().collect())
            .unwrap_or_default()
    }
}

/// The members of a map that can remove a key: before one, each live node's key and value are
/// taken (what a removed node keeps). Every other member only reads, inserts or replaces values
/// (`get` moves an access-ordered node, `putFirst` moves one: neither replaces it), so every
/// node it leaves is still there (#221 review B3: `get` no longer walks the live nodes). A key a
/// member not listed here did remove reads as absent and is counted, never answered.
fn may_remove(name: &str) -> bool {
    matches!(
        name,
        "remove"
            | "clear"
            | "compute"
            | "computeIfPresent"
            | "merge"
            | "retainAll"
            | "removeAll"
            | "removeIf"
            | "pollFirstEntry"
            | "pollLastEntry"
            | "pollFirst"
            | "pollLast"
            | "removeFirst"
            | "removeLast"
    )
}

impl StdHost {
    /// The value `map` holds for `key` (compared as stored: the node's own key), checking the
    /// pair at `slot` first; with the pair index it was found at.
    fn entry_map_value(&self, map: &str, key: &Value, slot: usize) -> Option<(usize, Value)> {
        let row = self.mutable_collections.get(map)?;
        if row.get(slot * 2) == Some(key) {
            return row.get(slot * 2 + 1).map(|value| (slot, value.clone()));
        }
        row.chunks(2)
            .position(|pair| pair.first() == Some(key))
            .and_then(|at| row.get(at * 2 + 1).map(|value| (at, value.clone())))
    }

    /// The tree node of `key` in `map`'s tree, when the tree is followed exactly.
    fn tree_node(&mut self, map: &str, key: &Value) -> Option<usize> {
        let tree = self.red_black_trees.get(map)?;
        if tree.tainted {
            return None;
        }
        let stamp = (tree.mod_count(), tree.slots());
        let fresh = self
            .map_entries
            .tree_memo
            .get(map)
            .is_some_and(|(count, len, _)| (*count, *len) == stamp);
        if !fresh {
            let index = tree.index_by_key();
            self.map_entries
                .tree_memo
                .insert(map.to_owned(), (stamp.0, stamp.1, index));
        }
        let (_, _, index) = self.map_entries.tree_memo.get(map)?;
        let at = *index.get(&crate::value_marker(key))?;
        // The memo names a node by its key's text; the node holds that very key.
        let tree = self.red_black_trees.get(map)?;
        (tree.key_at(at) == Some(key)).then_some(at)
    }

    /// The pair index of `key` in `map`'s row (0 when absent: a hint only).
    fn slot_of(&mut self, map: &str, key: &Value) -> usize {
        let generation = self.mutable_collections.generation(map);
        let fresh = self
            .map_entries
            .slot_memo
            .get(map)
            .is_some_and(|(at, _)| *at == generation);
        if !fresh {
            let index = self
                .mutable_collections
                .get(map)
                .map(|row| {
                    row.chunks(2)
                        .enumerate()
                        .filter_map(|(at, pair)| Some((crate::value_marker(pair.first()?), at)))
                        .collect()
                })
                .unwrap_or_default();
            self.map_entries
                .slot_memo
                .insert(map.to_owned(), (generation, index));
        }
        self.map_entries
            .slot_memo
            .get(map)
            .and_then(|(_, index)| index.get(&crate::value_marker(key)).copied())
            .unwrap_or(0)
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

    /// Whether `map`'s live entries read it now: no change since they were last followed.
    fn map_followed_now(&self, map: &str) -> bool {
        self.map_entries.followed.get(map) == Some(&self.mod_stamp_of(map))
    }

    /// Lose `map`'s live entries (changed by a path this model did not follow).
    fn lose_live_of(&mut self, map: &str) {
        for entry_key in self.map_entries.live_of(map) {
            self.map_entries.set_life(&entry_key, Life::Lost);
        }
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
        let map = map.and_then(crate::expect_ref).cloned().unwrap_or_default();
        let state = |life: Life, handle: &Value| EntryState {
            key: key.clone(),
            value: value.clone(),
            life,
            handle: handle.clone(),
        };
        let Some((class, mode)) = entry_mode(class_of_marker(&map)) else {
            // A map whose entry class is not modelled. An unmodifiable view's entries read
            // the map it wraps (`UnmodifiableEntry` delegates to that map's entry).
            let source = crate::Host::unmodifiable_view_source(self, &map);
            let view = source.is_some();
            let reading = opaque_reading(&map, view);
            let watched = source.unwrap_or_else(|| map.clone());
            let stamp = self.mod_stamp_of(&watched);
            let life = Life::Opaque {
                map: watched,
                stamp,
                view,
            };
            // One key per mapping (#221 review B4): a stored entry is one object across
            // readings, so a reading never makes a new key for it.
            let name = format!("o{}", crate::value_marker(key));
            let known = (reading != OpaqueReading::New)
                .then(|| self.map_entries.nodes.get(&map)?.get(&name).cloned())
                .flatten()
                .filter(|entry_key| self.map_entries.states.contains_key(entry_key));
            if let Some(entry_key) = known {
                let known = self
                    .map_entries
                    .states
                    .get_mut(&entry_key)
                    .expect("checked");
                let same = reading == OpaqueReading::Stored && known.life == life;
                known.value = value.clone();
                known.life = life;
                // The stored entry while the map has had no structural change; otherwise (or
                // for a class whose readings are not known) the same key with a fresh origin,
                // so `==` with an earlier reading is counted.
                if same {
                    return known.handle.clone();
                }
                return Value::Host(HostRef::new(HostKind::OpaqueMapEntry, entry_key));
            }
            let entry_key = self.mint_key(OPAQUE, true);
            let handle = Value::Host(HostRef::new(HostKind::OpaqueMapEntry, entry_key.clone()));
            self.map_entries
                .states
                .insert(entry_key.clone(), state(life, &handle));
            if reading != OpaqueReading::New {
                self.map_entries
                    .nodes
                    .entry(map)
                    .or_default()
                    .insert(name, entry_key);
            }
            return handle;
        };
        let nulls = match mode {
            Mode::Snapshot => return fixed(self, class),
            Mode::Concurrent => {
                let entry_key = self.mint_key(class, false);
                let handle = Value::Host(HostRef::new(HostKind::MapEntry, entry_key.clone()));
                self.map_entries
                    .states
                    .insert(entry_key, state(Life::Concurrent { map }, &handle));
                return handle;
            }
            Mode::Fresh => None,
            Mode::Node { nulls } => Some(nulls),
        };
        // Live entries read the map while it is where they were last followed; a change since
        // by a path this model did not follow loses the ones it holds.
        let now = self.mod_stamp_of(&map);
        if self.map_entries.followed.get(&map) != Some(&now) {
            self.lose_live_of(&map);
            self.map_entries.followed.insert(map.clone(), now);
        }
        let Some(nulls) = nulls else {
            let slot = self.slot_of(&map, key);
            // `EnumMap`/`IdentityHashMap`: a new entry per reading, reading the map by key.
            let entry_key = self.mint_key(class, false);
            let handle = Value::Host(HostRef::new(HostKind::MapEntry, entry_key.clone()));
            self.map_entries
                .states
                .insert(entry_key.clone(), state(Life::Fixed, &handle));
            let life = Life::Live {
                map,
                node: None,
                slot,
                fresh: true,
                nulls: true,
            };
            self.map_entries.set_life(&entry_key, life);
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
            None => format!("k{}", crate::value_marker(key)),
        };
        let followed = self.nodes_followed(&map) && (!tree || node.is_some());
        if let Some(entry_key) = self
            .map_entries
            .nodes
            .get(&map)
            .and_then(|nodes| nodes.get(&node_name))
            && let Some(known) = self.map_entries.states.get(entry_key)
        {
            // The same node: the same handle while followed; otherwise the same key with a
            // fresh origin, so `==` with an earlier reading is counted.
            if followed && known.life != Life::Lost {
                return known.handle.clone();
            }
            return Value::Host(HostRef::new(HostKind::MapEntry, entry_key.clone()));
        }
        let entry_key = self.mint_key(class, true);
        let handle = Value::Host(HostRef::new(HostKind::MapEntry, entry_key.clone()));
        self.map_entries
            .states
            .insert(entry_key.clone(), state(Life::Lost, &handle));
        if followed {
            let slot = self.slot_of(&map, key);
            let life = Life::Live {
                map: map.clone(),
                node,
                slot,
                fresh: false,
                nulls,
            };
            self.map_entries.set_life(&entry_key, life);
        }
        self.map_entries
            .nodes
            .entry(map)
            .or_default()
            .insert(node_name, entry_key);
        handle
    }

    /// A value written through an entry changes that entry's hash: the map's `entrySet()` view
    /// lays its table out afresh (Java's `EntrySet` looks keys up in the map, never by hash).
    fn forget_entry_view_layout(&mut self, map: &str) {
        if let Some(view) = self
            .entry_views
            .get(map)
            .and_then(crate::expect_ref)
            .cloned()
        {
            self.hash_tables.remove(&view);
        }
    }

    /// [`Self::mint_map_entry`] for every pair of `pairs` (`[k, v, k, v...]`), as a map's
    /// `entrySet()` view is (re)built: a followed hash map's node table is read once and each
    /// known node answered by one lookup; a new one, or any other map, goes the long way.
    pub(crate) fn mint_map_entries(
        &mut self,
        map: &Value,
        pairs: &[Value],
        made: EntryMade,
    ) -> Vec<Value> {
        let marker = crate::expect_ref(map).cloned().unwrap_or_default();
        let hash_nodes = made == EntryMade::Node
            && matches!(
                entry_mode(class_of_marker(&marker)),
                Some((class, Mode::Node { .. })) if class != "java/util/TreeMap$Entry"
            )
            && self.nodes_followed(&marker)
            && self.map_followed_now(&marker);
        if !hash_nodes {
            self.map_entries.view_memo.remove(&marker);
            return pairs
                .chunks(2)
                .filter_map(|pair| match pair {
                    [key, value] => Some(self.mint_map_entry(Some(map), key, value, made)),
                    _ => None,
                })
                .collect();
        }
        // The previous view's handles, where its key still sits at the same position.
        let memo = self
            .map_entries
            .view_memo
            .remove(&marker)
            .unwrap_or_default();
        let previous = self
            .entry_views
            .get(&marker)
            .and_then(crate::expect_ref)
            .and_then(|view| self.mutable_collections.get(view));
        let mut reused: Vec<Option<Value>> = Vec::with_capacity(pairs.len() / 2);
        for (at, pair) in pairs.chunks(2).enumerate() {
            reused.push(
                (memo.get(at) == pair.first())
                    .then(|| previous.and_then(|view| view.get(at)).cloned())
                    .flatten(),
            );
        }
        let mut handles = Vec::with_capacity(reused.len());
        for (pair, reused) in pairs.chunks(2).zip(reused) {
            let [key, value] = pair else {
                continue;
            };
            let known = reused.or_else(|| {
                let entry_key = self
                    .map_entries
                    .nodes
                    .get(&marker)?
                    .get(&format!("k{}", crate::value_marker(key)))?;
                let state = self.map_entries.states.get(entry_key)?;
                matches!(state.life, Life::Live { .. }).then(|| state.handle.clone())
            });
            handles.push(match known {
                Some(handle) => handle,
                None => self.mint_map_entry(Some(map), key, value, made),
            });
        }
        // The keys, kept in place where the map only grew.
        let mut keys = memo;
        let count = pairs.len() / 2;
        let same_prefix = keys.len() <= count
            && keys
                .iter()
                .zip(pairs.chunks(2))
                .all(|(old, pair)| pair.first() == Some(old));
        if !same_prefix {
            keys.clear();
        }
        keys.extend(
            pairs
                .chunks(2)
                .skip(keys.len())
                .filter_map(|pair| pair.first().cloned()),
        );
        self.map_entries.view_memo.insert(marker, keys);
        handles
    }

    fn mod_stamp_of(&self, map: &str) -> ModStamp {
        crate::Host::mod_stamp(self, map)
    }

    /// The key and value `entry` holds now; `None` for any other value, or when unknown.
    pub(crate) fn map_entry_now(&self, entry: &Value) -> Option<(Value, Value)> {
        let Value::Host(handle) = entry else {
            return None;
        };
        let state = self.map_entries.states.get(handle.key())?;
        match &state.life {
            Life::Fixed | Life::Detached => Some((state.key.clone(), state.value.clone())),
            Life::Live {
                map, node, slot, ..
            } => {
                if !self.map_followed_now(map) {
                    return None;
                }
                let key = match node {
                    Some(index) => self.red_black_trees.get(map)?.key_at(*index)?.clone(),
                    None => state.key.clone(),
                };
                let (_, value) = self.entry_map_value(map, &key, *slot)?;
                Some((key, value))
            }
            // Its own value (`ConcurrentHashMap.MapEntry.val`).
            Life::Concurrent { .. } => Some((state.key.clone(), state.value.clone())),
            Life::Lost => None,
            // An unmodifiable view's entry reads the wrapped map's node: live, while that map
            // has had no structural change (a removed and re-put key would be another node).
            Life::Opaque {
                map,
                stamp,
                view: true,
            } => {
                if self.mod_stamp_of(map) != *stamp {
                    return None;
                }
                let (_, value) = self.entry_map_value(map, &state.key, 0)?;
                Some((state.key.clone(), value))
            }
            // A map the host keeps no row for (an immutable literal) cannot have changed;
            // a kept one must still hold what was read, with no structural change since.
            Life::Opaque { map, stamp, .. } => (!self.mutable_collections.contains_key(map)
                || self.mod_stamp_of(map) == *stamp
                    && self
                        .entry_map_value(map, &state.key, 0)
                        .is_some_and(|(_, value)| value == state.value))
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
            // `Hashtable.Entry.setValue` and `ConcurrentHashMap.MapEntry.setValue` refuse null.
            Life::Live { nulls: false, .. } | Life::Concurrent { .. }
                if value == Value::Ref(None) =>
            {
                Err(NativeError::UncaughtException {
                    class: "java/lang/NullPointerException".into(),
                    message: None,
                    frame: None,
                })
            }
            // `val = value; map.put(key, value)`: a `put` of a key the map still holds writes
            // its value; one it no longer holds would insert it again (counted).
            Life::Concurrent { map } => {
                let map = map.clone();
                let Some(slot) = self.mutable_collections.get_mut(&map).and_then(|row| {
                    row.chunks_mut(2)
                        .find(|pair| pair.first() == Some(&state.key))
                        .and_then(|pair| pair.get_mut(1))
                }) else {
                    return self.unknown_host_call(r);
                };
                *slot = value.clone();
                self.forget_entry_view_layout(&map);
                let states = &mut self.map_entries.states;
                let own = states.get_mut(handle.key()).expect("found above");
                Ok(std::mem::replace(&mut own.value, value))
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
                let old = std::mem::replace(slot, value);
                self.forget_entry_view_layout(&map);
                Ok(old)
            }
            Life::Lost | Life::Opaque { .. } => self.unknown_host_call(r),
        }
    }

    /// Before the call `name(args)` on `map`: where its live entries stand. A call that may
    /// remove a key also takes the key and value of each live node it may remove (what a removed
    /// node keeps).
    pub(crate) fn entries_before_call(&mut self, map: &str, name: &str, args: &[Value]) {
        if self
            .map_entries
            .by_map
            .get(map)
            .is_none_or(|live| live.is_empty())
        {
            return;
        }
        let stamp = self.mod_stamp_of(map);
        if self.map_entries.followed.get(map) != Some(&stamp) {
            // Changed by a path this model did not follow.
            self.lose_live_of(map);
            return;
        }
        // `remove(key)`/`remove(key, value)` on a hash map runs no callback and can remove only
        // the node of `key`, found by the node index (a tree's `deleteEntry` may free its
        // successor's node instead, so a tree takes every node).
        let one_key = name == "remove"
            && matches!(args.len(), 1 | 2)
            && !self.red_black_trees.contains_key(map)
            && entry_mode(class_of_marker(map)).is_some_and(|(class, mode)| {
                class != "java/util/TreeMap$Entry" && matches!(mode, Mode::Node { .. })
            });
        let taken = may_remove(name).then(|| {
            if one_key {
                let node = args.first().and_then(|key| {
                    let entry_key = self
                        .map_entries
                        .nodes
                        .get(map)?
                        .get(&format!("k{}", crate::value_marker(key)))?
                        .clone();
                    let state = self.map_entries.states.get(&entry_key)?;
                    if !matches!(state.life, Life::Live { .. }) {
                        return None;
                    }
                    let (key, value) = self.map_entry_now(&state.handle.clone())?;
                    Some((entry_key, key, value))
                });
                return Taken::Key(node);
            }
            Taken::All(
                self.map_entries
                    .live_of(map)
                    .into_iter()
                    .filter_map(|entry_key| {
                        let handle = self.map_entries.states.get(&entry_key)?.handle.clone();
                        let (key, value) = self.map_entry_now(&handle)?;
                        Some((entry_key, (key, value)))
                    })
                    .collect(),
            )
        });
        self.map_entries
            .before
            .insert(map.to_owned(), Before { stamp, taken });
    }

    /// After a call on `map`: follow its live nodes. Without a structural change they stay as
    /// they are; after one that only inserts, too (one step for the map, #221 review B3). After
    /// one that may remove, a hash node whose key is gone is detached with what it held before
    /// the call, and the others stay; a tree node follows its tree node (a freed one is
    /// detached, a kept one holds whatever key `deleteEntry` left in it). A treeified table or a
    /// tree with an unplaced key loses them all.
    pub(crate) fn entries_after_call(&mut self, map: &str) {
        let Some(before) = self.map_entries.before.remove(map) else {
            return;
        };
        let now = self.mod_stamp_of(map);
        if now == before.stamp {
            return;
        }
        if !self.nodes_followed(map) {
            self.map_entries.lost_maps.insert(map.to_owned());
            self.lose_live_of(map);
            return;
        }
        let taken = match before.taken {
            None => {
                self.map_entries.followed.insert(map.to_owned(), now);
                return;
            }
            Some(Taken::Key(node)) => {
                // Every other node is still there; only this one may be gone.
                if let Some((entry_key, key, value)) = node
                    && self.entry_map_value(map, &key, 0).is_none()
                {
                    if let Some(state) = self.map_entries.states.get_mut(&entry_key) {
                        state.key = key.clone();
                        state.value = value;
                    }
                    self.map_entries.set_life(&entry_key, Life::Detached);
                    // Never handed out again: a later reading of its key is a new node.
                    if let Some(nodes) = self.map_entries.nodes.get_mut(map) {
                        nodes.remove(&format!("k{}", crate::value_marker(&key)));
                    }
                }
                self.map_entries.followed.insert(map.to_owned(), now);
                return;
            }
            Some(Taken::All(taken)) => taken,
        };
        // Where each key sits now, once: a node's check is then one lookup.
        let slots: std::collections::HashMap<String, usize> = self
            .mutable_collections
            .get(map)
            .map(|row| {
                row.chunks(2)
                    .enumerate()
                    .filter_map(|(at, pair)| Some((crate::value_marker(pair.first()?), at)))
                    .collect()
            })
            .unwrap_or_default();
        for entry_key in self.map_entries.live_of(map) {
            let Some(Life::Live {
                node, fresh, nulls, ..
            }) = self
                .map_entries
                .states
                .get(&entry_key)
                .map(|state| state.life.clone())
            else {
                continue;
            };
            // What the node held before the call; one the host could not read then is lost.
            let Some((key, value)) = taken.get(&entry_key) else {
                self.map_entries.set_life(&entry_key, Life::Lost);
                continue;
            };
            let live = |slot: usize| Life::Live {
                map: map.to_owned(),
                node,
                slot,
                fresh,
                nulls,
            };
            let life = match node {
                // A tree node: still in the tree (holding whatever key `deleteEntry` left in
                // it), or freed.
                Some(index) => match self
                    .red_black_trees
                    .get(map)
                    .and_then(|tree| tree.key_at(index))
                {
                    Some(now_key) => live(
                        slots
                            .get(&crate::value_marker(now_key))
                            .copied()
                            .unwrap_or(0),
                    ),
                    None => Life::Detached,
                },
                None => match slots.get(&crate::value_marker(key)) {
                    Some(slot) => live(*slot),
                    // An `EnumMap`/`IdentityHashMap` entry reads its map's slot: once its key
                    // is gone that answer is not this model's.
                    None if fresh => Life::Lost,
                    None => Life::Detached,
                },
            };
            if life == Life::Detached
                && let Some(state) = self.map_entries.states.get_mut(&entry_key)
            {
                state.key = key.clone();
                state.value = value.clone();
            }
            self.map_entries.set_life(&entry_key, life);
        }
        self.map_entries.followed.insert(map.to_owned(), now);
        // A detached node is never handed out again: a later reading of its key is a new node.
        let states = &self.map_entries.states;
        if let Some(nodes) = self.map_entries.nodes.get_mut(map) {
            nodes.retain(|_, entry_key| {
                states
                    .get(entry_key)
                    .is_some_and(|state| state.life != Life::Detached)
            });
        }
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

/// `IdentityHashMap`'s entries compare and hash by identity (Java 25's
/// `IdentityHashMap.EntryIterator.Entry`), every other entry class by `equals`/`hashCode`.
const IDENTITY_ENTRY: &str = "java/util/IdentityHashMap$EntryIterator$Entry";

/// Whether `value` is an `IdentityHashMap` entry.
pub(crate) fn is_identity_entry(value: &Value) -> bool {
    matches!(value, Value::Host(handle) if identity_entry(handle))
}

fn identity_entry(handle: &HostRef) -> bool {
    handle.kind() == HostKind::MapEntry && entry_class(handle.key()) == IDENTITY_ENTRY
}

/// `a == b` where this model can tell: two handles by `same_object`, `null`s, a handle against
/// any text or box (two objects). Two texts or boxes are `None`: their identity is not kept.
pub(crate) fn same_reference(a: &Value, b: &Value) -> Option<bool> {
    match (a, b) {
        (Value::Ref(None), Value::Ref(None)) => Some(true),
        (Value::Ref(None), _) | (_, Value::Ref(None)) => Some(false),
        (Value::Host(left), Value::Host(right)) => left.same_object(right),
        (Value::Host(_), _) | (_, Value::Host(_)) => Some(false),
        _ => None,
    }
}

fn entry_hash(handle: &HostRef, nested: &mut dyn Nested) -> Result<Option<i32>, NativeError> {
    // `System.identityHashCode(key) ^ System.identityHashCode(value)`: counted.
    if identity_entry(handle) {
        return Ok(None);
    }
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
            // `e.getKey() == key && e.getValue() == value`, counted where identity is unknown.
            if identity_entry(handle) {
                return Ok(match same_reference(&key, &other_key) {
                    Some(true) => same_reference(&value, &other_value),
                    answer => answer,
                });
            }
            Ok(match nested.equals(&key, &other_key)? {
                Some(true) => nested.equals(&value, &other_value)?,
                answer => answer,
            })
        }
        // A jar's own `Map.Entry` answers through its accessors (`jdk_skeletons`'s
        // `host_entry_equals` runs them); here it is unknown.
        other if crate::jar_object::object_key(other).is_some() => Ok(None),
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

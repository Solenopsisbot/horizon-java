//! The bucket table of a `java.util.HashMap` (and the `HashSet`,
//! `LinkedHashMap` and `LinkedHashSet` built on it), as JDK 25 lays it out.
//!
//! The host keeps a map's entries in a plain list. A `HashMap` iterates its
//! table bin by bin, so its order depends on every key's hash, the table's
//! capacity and the history of puts and resizes: `HashSet` of
//! `"pear","b","apple","a"` iterates `apple,a,pear,b` on a JVM. This module
//! keeps the table the JDK would have, so the host's list can be put in that
//! order, and so a lookup knows which entries the JDK compares a key with
//! (only those in the key's bin with an equal hash).
//!
//! What it models, from `HashMap`'s source:
//!
//! - **`hash(key)`** is `h ^ (h >>> 16)`; a key's bin is that masked by
//!   `capacity - 1`.
//! - **Sizing.** A table is allocated by the first insertion: at the
//!   capacity a constructor stored in `threshold` (`tableSizeFor`), or 16.
//!   `resize()` doubles it and its threshold (`capacity * loadFactor`,
//!   computed in `float` while the capacity is below 16), and splits every
//!   bin into its low and high half keeping order.
//! - **`putVal`** (`put`, `putIfAbsent`, `HashSet.add`, `putAll`) appends to
//!   the bin's tail, turns a bin that already held 8 nodes into a tree (or
//!   resizes a table smaller than 64 instead), then resizes when the size
//!   passes the threshold.
//! - **`computeIfAbsent`/`compute`/`merge`** resize *before* inserting when
//!   the size already passed the threshold, insert at the bin's **head**, and
//!   treeify a bin that already held 7 nodes.
//! - **Removal** unlinks a node; the table never shrinks, `clear` keeps it.
//!
//! A tree bin orders and searches its nodes by `compareTo` and identity
//! hashes, which this does not model: once a bin would become a tree the
//! table is marked [`HashTable::treeified`] and its order is unknown. So is
//! the order of a table holding a key whose hash no run can reproduce (an
//! identity hash): such a key is kept in [`HashTable::unplaced`], counted in
//! the size but in no bin.
//!
//! Each node keeps the key's [`Value`] itself, not only its item marker: a
//! handle's origin says which object it is, so `p.key == key` (the same
//! handle added twice) and a stored handle's own `equals` answer exactly. A
//! node is still found by its marker (the host's list holds one entry per
//! marker), and its stored hash is the one it was inserted with, so a key
//! whose hash changed after it was put (a chat component appended to) is not
//! in its new bin, as in the JDK.

use std::collections::{BTreeMap, HashMap};

use crate::runtime::{Value, collection_item_marker};

/// `HashMap.DEFAULT_INITIAL_CAPACITY`.
const DEFAULT_CAPACITY: usize = 16;
/// `HashMap.MAXIMUM_CAPACITY`.
const MAXIMUM_CAPACITY: usize = 1 << 30;
/// `HashMap.TREEIFY_THRESHOLD`.
const TREEIFY_THRESHOLD: usize = 8;
/// `HashMap.MIN_TREEIFY_CAPACITY`.
const MIN_TREEIFY_CAPACITY: usize = 64;

/// `HashMap.hash(key)` for a key whose `hashCode()` is `hash`. It is its
/// own inverse: `spread(spread(h)) == h`.
pub(crate) fn spread(hash: i32) -> i32 {
    hash ^ ((hash as u32) >> 16) as i32
}

/// `HashMap.tableSizeFor(cap)`: the power of two at or above `cap`.
pub(crate) fn table_size_for(cap: i64) -> i32 {
    if cap <= 1 {
        return 1;
    }
    let size = (cap as u64).next_power_of_two();
    size.min(MAXIMUM_CAPACITY as u64) as i32
}

/// `(int) Math.ceil(n / (double) 0.75f)`: `HashMap.calculateHashMapCapacity`.
pub(crate) fn capacity_for(mappings: usize) -> i64 {
    (mappings as f64 / f64::from(0.75_f32)).ceil() as i64
}

/// A `HashMap.Node`: the key, its item marker (how the host's list names
/// it), and the spread hash it was inserted with.
#[derive(Clone, Debug, PartialEq)]
struct Node {
    marker: String,
    key: Value,
    hash: i32,
}

/// One table's state. Nodes are named by the host's item markers and keep
/// their key's value and the spread hash it was inserted with, as a
/// `HashMap.Node` does.
#[derive(Clone, Debug, PartialEq)]
pub struct HashTable {
    /// The JDK's `table`, bin by bin; empty until the first insertion.
    bins: Vec<Vec<Node>>,
    /// The JDK's `threshold` (the initial capacity before allocation).
    threshold: i64,
    load_factor: f32,
    /// Keys whose hash is an identity hash, by marker: in the size, in no
    /// bin.
    pub(crate) unplaced: BTreeMap<String, Value>,
    /// A bin became a tree, whose order this does not model.
    pub(crate) treeified: bool,
    /// Iterates in insertion order (`LinkedHashMap`/`LinkedHashSet`), which
    /// the host's list already is.
    pub(crate) linked: bool,
    /// A `LinkedHashMap` in access order, whose reordering the host does not
    /// model.
    pub(crate) access_order: bool,
    /// Filled from a source whose order was unknown.
    pub(crate) tainted: bool,
    /// Every node by marker, with its spread hash (`None`: unplaced), so
    /// `contains`/`stored_hash`/`remove`/`refresh` reach a node in its bin
    /// without walking the table (#203).
    index: HashMap<String, Option<i32>>,
    /// The host collection generation (`collection_store.rs`) at which the
    /// table last mirrored the host's list exactly (the same keys and, when
    /// [`Self::ordered`], in this table's order): while the generation is
    /// still that, the reconciliation (`Runtime::resync_hash_table`) has
    /// nothing to do. Every structural change clears it; the paths that
    /// change the list and the table together set it again.
    pub(crate) synced: Option<u64>,
    /// Moves on every structural change (insert, remove, resize, clear), as
    /// a `HashMap`'s `modCount` does for a function run inside `compute`.
    pub(crate) version: u64,
    /// For an insertion-ordered table (`linked`) mirrored by its list: each
    /// key's entry index in the list, `None` until rebuilt after a removal
    /// shifted them (#203).
    pub(crate) positions: Option<HashMap<String, usize>>,
}

impl HashTable {
    /// `new HashMap<>(initialCapacity, loadFactor)`; `None` is `new
    /// HashMap<>()`.
    pub(crate) fn new(initial_capacity: Option<i32>, load_factor: f32) -> Self {
        Self {
            bins: Vec::new(),
            threshold: initial_capacity.map_or(0, |cap| i64::from(table_size_for(cap.into()))),
            load_factor,
            unplaced: BTreeMap::new(),
            treeified: false,
            linked: false,
            access_order: false,
            tainted: false,
            index: HashMap::new(),
            synced: None,
            version: 0,
            positions: None,
        }
    }

    /// Whether the host's list is kept in this table's iteration order
    /// (`Runtime::resync_hash_table` sorts it so): not an insertion-ordered
    /// table, and an order that is known.
    pub(crate) fn ordered(&self) -> bool {
        !self.linked && !self.order_unknown()
    }

    /// The position of the node `marker` in iteration order: the nodes of
    /// the bins before its own, then its place in its bin. `None` for an
    /// unplaced key and a marker not in the table.
    pub(crate) fn rank_of(&self, marker: &str) -> Option<usize> {
        let spread = (*self.index.get(marker)?)?;
        let bin = spread as u32 as usize & (self.bins.len() - 1);
        let within = self.bins[bin]
            .iter()
            .position(|node| node.marker == marker)?;
        Some(self.bins[..bin].iter().map(Vec::len).sum::<usize>() + within)
    }

    /// Whether the table's iteration order is unknown. One entry (or none)
    /// has only one order, wherever it sits.
    pub(crate) fn order_unknown(&self) -> bool {
        if self.len() <= 1 {
            return false;
        }
        !self.unplaced.is_empty() || self.treeified || self.access_order || self.tainted
    }

    /// Whether a lookup's `equals` calls are unknown: an unplaced key may sit
    /// in any bin, and a tree bin searches by `compareTo`.
    pub(crate) fn lookups_unknown(&self) -> bool {
        !self.unplaced.is_empty() || self.treeified
    }

    pub(crate) fn len(&self) -> usize {
        self.index.len()
    }

    /// The number of bins (a resize changes it).
    pub(crate) fn capacity(&self) -> usize {
        self.bins.len()
    }

    /// Whether the table has been allocated (`table != null`).
    pub(crate) fn allocated(&self) -> bool {
        !self.bins.is_empty()
    }

    /// Whether a node is named `marker`.
    pub(crate) fn contains(&self, marker: &str) -> bool {
        self.index.contains_key(marker)
    }

    /// The keys a lookup with `hash` compares, in bin order: the bin's nodes
    /// whose stored hash equals `spread(hash)`, as they were put.
    pub(crate) fn candidates(&self, hash: i32) -> Vec<Value> {
        if self.bins.is_empty() {
            return Vec::new();
        }
        let spread = spread(hash);
        let bin = spread as u32 as usize & (self.bins.len() - 1);
        self.bins[bin]
            .iter()
            .filter(|node| node.hash == spread)
            .map(|node| node.key.clone())
            .collect()
    }

    /// The stored (spread) hash of the node `marker`, when it is in a bin.
    pub(crate) fn stored_hash(&self, marker: &str) -> Option<i32> {
        *self.index.get(marker)?
    }

    /// The node `marker` in its bin, through the index.
    fn node_mut(&mut self, marker: &str) -> Option<&mut Node> {
        let spread = (*self.index.get(marker)?)?;
        let bin = spread as u32 as usize & (self.bins.len() - 1);
        self.bins[bin].iter_mut().find(|node| node.marker == marker)
    }

    /// The nodes' markers in iteration order (bin by bin), unplaced keys last.
    pub(crate) fn order(&self) -> Vec<String> {
        self.bins
            .iter()
            .flatten()
            .map(|node| node.marker.clone())
            .chain(self.unplaced.keys().cloned())
            .collect()
    }

    /// The stored keys, in the order of [`Self::order`].
    pub(crate) fn keys(&self) -> Vec<Value> {
        self.bins
            .iter()
            .flatten()
            .map(|node| node.key.clone())
            .chain(self.unplaced.values().cloned())
            .collect()
    }

    /// Point the node `marker` at `key` itself: the host's list holds the
    /// object a node stands for (a path that rebuilt the list, or a literal
    /// read again, may hand back a fresh value for it).
    pub(crate) fn refresh_named(&mut self, marker: &str, key: &Value) {
        if let Some(stored) = self.unplaced.get_mut(marker) {
            *stored = key.clone();
            return;
        }
        if let Some(node) = self.node_mut(marker) {
            node.key = key.clone();
        }
    }

    /// `HashMap.resize()`.
    fn resize(&mut self) {
        self.version += 1;
        self.synced = None;
        let old_cap = self.bins.len();
        let old_thr = self.threshold;
        let mut new_thr = 0_i64;
        let new_cap;
        if old_cap > 0 {
            if old_cap >= MAXIMUM_CAPACITY {
                self.threshold = i64::from(i32::MAX);
                return;
            }
            new_cap = old_cap << 1;
            if new_cap < MAXIMUM_CAPACITY && old_cap >= DEFAULT_CAPACITY {
                new_thr = old_thr << 1;
            }
        } else if old_thr > 0 {
            new_cap = old_thr as usize;
        } else {
            new_cap = DEFAULT_CAPACITY;
            new_thr = 12;
        }
        if new_thr == 0 {
            let ft = new_cap as f32 * self.load_factor;
            new_thr = if new_cap < MAXIMUM_CAPACITY && ft < MAXIMUM_CAPACITY as f32 {
                ft as i64
            } else {
                i64::from(i32::MAX)
            };
        }
        self.threshold = new_thr;
        let mut bins = vec![Vec::new(); new_cap];
        for (index, bin) in std::mem::take(&mut self.bins).into_iter().enumerate() {
            // Low and high halves, each keeping the bin's order.
            for node in bin {
                let high = node.hash as u32 as usize & old_cap != 0;
                bins[index + if high { old_cap } else { 0 }].push(node);
            }
        }
        self.bins = bins;
    }

    /// `treeifyBin`: a table below 64 bins resizes instead.
    fn treeify_bin(&mut self) {
        if self.bins.len() < MIN_TREEIFY_CAPACITY {
            self.resize();
        } else {
            self.treeified = true;
        }
    }

    /// `putMapEntries`' pre-sizing for `mappings` more entries, before they
    /// are put one by one.
    pub(crate) fn presize(&mut self, mappings: usize) {
        if mappings == 0 {
            return;
        }
        if self.bins.is_empty() {
            let wanted = (mappings as f64 / f64::from(self.load_factor)).ceil();
            let wanted = if wanted < MAXIMUM_CAPACITY as f64 {
                wanted as i64
            } else {
                MAXIMUM_CAPACITY as i64
            };
            if wanted > self.threshold {
                self.threshold = table_size_for(wanted).into();
            }
        } else {
            while mappings as i64 > self.threshold && self.bins.len() < MAXIMUM_CAPACITY {
                self.resize();
            }
        }
    }

    /// The start of `computeIfAbsent`/`compute`/`merge`: they resize before
    /// they search, whether or not the key is there, when the size already
    /// passed the threshold or no table is allocated.
    pub(crate) fn begin_compute(&mut self) {
        if self.len() as i64 > self.threshold || self.bins.is_empty() {
            self.resize();
        }
    }

    /// `putVal`'s insertion of `key`, whose `hashCode()` is `hash` (`None`:
    /// an identity hash): allocate, append to the bin's tail, treeify a bin
    /// that already held 8 nodes, resize past the threshold.
    pub(crate) fn insert(&mut self, key: &Value, hash: Option<i32>) {
        let marker = collection_item_marker(key);
        let size = self.len() as i64;
        self.version += 1;
        self.synced = None;
        self.index.insert(marker.clone(), hash.map(spread));
        if self.bins.is_empty() {
            self.resize();
        }
        if let Some(hash) = hash {
            let spread = spread(hash);
            let bin = spread as u32 as usize & (self.bins.len() - 1);
            let held = self.bins[bin].len();
            self.bins[bin].push(Node {
                marker,
                key: key.clone(),
                hash: spread,
            });
            if held >= TREEIFY_THRESHOLD {
                self.treeify_bin();
            }
        } else {
            self.unplaced.insert(marker, key.clone());
        }
        if size + 1 > self.threshold {
            self.resize();
        }
    }

    /// `computeIfAbsent`/`compute`/`merge`'s insertion (after
    /// [`Self::begin_compute`]): the bin's head, treeifying a bin that
    /// already held 7 nodes, and no resize afterwards.
    pub(crate) fn insert_at_head(&mut self, key: &Value, hash: Option<i32>) {
        let marker = collection_item_marker(key);
        self.version += 1;
        self.synced = None;
        self.index.insert(marker.clone(), hash.map(spread));
        if self.bins.is_empty() {
            self.resize();
        }
        if let Some(hash) = hash {
            let spread = spread(hash);
            let bin = spread as u32 as usize & (self.bins.len() - 1);
            let held = self.bins[bin].len();
            self.bins[bin].insert(
                0,
                Node {
                    marker,
                    key: key.clone(),
                    hash: spread,
                },
            );
            if held >= TREEIFY_THRESHOLD - 1 {
                self.treeify_bin();
            }
        } else {
            self.unplaced.insert(marker, key.clone());
        }
    }

    /// Unlink the node `marker`.
    pub(crate) fn remove(&mut self, marker: &str) {
        let Some(spread) = self.index.remove(marker) else {
            return;
        };
        self.version += 1;
        self.synced = None;
        let Some(spread) = spread else {
            self.unplaced.remove(marker);
            return;
        };
        let bin = spread as u32 as usize & (self.bins.len() - 1);
        if let Some(index) = self.bins[bin].iter().position(|n| n.marker == marker) {
            self.bins[bin].remove(index);
        }
    }

    /// `clear()`: every node goes, the table stays.
    pub(crate) fn clear(&mut self) {
        for bin in &mut self.bins {
            bin.clear();
        }
        self.unplaced.clear();
        self.index.clear();
        self.version += 1;
        self.synced = None;
    }
}

#[cfg(test)]
#[path = "hash_perf_tests.rs"]
mod perf_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::hash_map_order::java_string_hash;

    fn table_of(keys: &[&str]) -> HashTable {
        let mut table = HashTable::new(None, 0.75);
        for key in keys {
            table.insert(
                &Value::Ref(Some((*key).into())),
                Some(java_string_hash(key)),
            );
        }
        table
    }

    /// OpenJDK 25.0.5: `HashSet` of `"pear","b","apple","a"` iterates
    /// `apple, a, pear, b`.
    #[test]
    fn a_hash_set_of_strings_iterates_in_bucket_order() {
        let table = table_of(&["pear", "b", "apple", "a"]);
        let order = ["apple", "a", "pear", "b"].map(|key| Value::Ref(Some(key.into())));
        assert_eq!(table.keys(), order);
    }

    /// `tableSizeFor` and the threshold `new HashMap<>(n)` starts with.
    #[test]
    fn initial_capacities_round_up_to_powers_of_two() {
        assert_eq!(table_size_for(0), 1);
        assert_eq!(table_size_for(1), 1);
        assert_eq!(table_size_for(13), 16);
        assert_eq!(table_size_for(16), 16);
        assert_eq!(table_size_for(17), 32);
        // `new HashMap<>(0)`: the first put allocates one bin with threshold
        // `(int) (1 * 0.75f) = 0`, so it resizes to two at once.
        let mut table = HashTable::new(Some(0), 0.75);
        table.insert(&Value::Int(1), Some(1));
        assert_eq!(table.bins.len(), 2);
        assert_eq!(table.threshold, 1);
    }
}

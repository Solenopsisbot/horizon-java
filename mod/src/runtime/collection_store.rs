//! The host's mutable collections by marker, each with a generation (#203).
//!
//! A hash collection's bucket table (`hash_table.rs`) mirrors the host's list
//! of its entries. Before #203 every member call reconciled the two
//! (`Runtime::resync_hash_table`), an O(n) pass (O(n²) with the table's
//! linear lookups) even when nothing had touched the list. The store gives
//! every collection a generation that moves on **every** mutable access:
//! `insert`, `get_mut`, `entry`, `remove`, `retain`. There is no other way to
//! change a list (no `DerefMut`), so a table synced at generation `g` still
//! mirrors its list while the generation is `g`, and the reconciliation can
//! be skipped. Reads go through `Deref` to the map and move nothing. A
//! mutable access that changes nothing only costs the next call a
//! reconciliation.

use std::collections::{BTreeMap, HashMap, btree_map};

use crate::runtime::Value;

#[derive(Clone, Debug, Default)]
pub struct CollectionStore {
    map: BTreeMap<String, Vec<Value>>,
    generations: HashMap<String, u64>,
    next: u64,
}

impl std::ops::Deref for CollectionStore {
    type Target = BTreeMap<String, Vec<Value>>;

    fn deref(&self) -> &Self::Target {
        &self.map
    }
}

impl CollectionStore {
    fn bump(&mut self, marker: &str) -> u64 {
        self.next += 1;
        let next = self.next;
        match self.generations.get_mut(marker) {
            Some(generation) => *generation = next,
            None => {
                self.generations.insert(marker.to_owned(), next);
            }
        }
        next
    }

    /// The generation of `marker`'s list: it changes whenever the list may
    /// have (`0` for a marker never written).
    pub(crate) fn generation(&self, marker: &str) -> u64 {
        self.generations.get(marker).copied().unwrap_or(0)
    }

    pub(crate) fn insert(&mut self, marker: String, values: Vec<Value>) -> Option<Vec<Value>> {
        self.bump(&marker);
        self.map.insert(marker, values)
    }

    pub(crate) fn remove(&mut self, marker: &str) -> Option<Vec<Value>> {
        self.bump(marker);
        self.map.remove(marker)
    }

    pub(crate) fn get_mut(&mut self, marker: &str) -> Option<&mut Vec<Value>> {
        if self.map.contains_key(marker) {
            self.bump(marker);
        }
        self.map.get_mut(marker)
    }

    pub(crate) fn entry(&mut self, marker: String) -> btree_map::Entry<'_, String, Vec<Value>> {
        self.bump(&marker);
        self.map.entry(marker)
    }

    /// Keep the collections `keep` accepts; every generation moves.
    pub(crate) fn retain(&mut self, mut keep: impl FnMut(&String, &mut Vec<Value>) -> bool) {
        self.next += 1;
        let next = self.next;
        self.map.retain(|marker, values| keep(marker, values));
        self.generations
            .retain(|marker, _| self.map.contains_key(marker));
        self.generations.values_mut().for_each(|g| *g = next);
    }
}

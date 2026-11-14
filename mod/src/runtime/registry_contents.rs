//! What a registry marker iterates over.
//!
//! The host models a game registry as a `Registry` handle keyed by its id
//! (`host_registry.rs`), and records the entries a jar registers into it. A real `MappedRegistry.iterator()` yields
//! every value registered in that registry; the only entries this runtime has
//! values for are the ones the jar registered, so that is what the iterator
//! yields, in registration order.
//!
//! The game's own entries (vanilla's ~140 entity types for
//! `minecraft:entity_type`) are not materialised, because the host has no
//! `EntityType` objects to hand out and inventing them would be worse than an
//! empty iteration. So walking a registry the jar did not build itself
//! (`StdHost::jar_registries`: every vanilla one, and any other jar's) is
//! counted, never answered as complete (#182 P3: Java 25 answers `true` for
//! `BuiltInRegistries.ITEM.iterator().hasNext()`), and still walks the
//! entries the jar registered, so the entrypoint goes on. A registry the jar
//! built and registered holds only what the jar registers into it: its walk
//! is exact.
//! Before this, the call answered null, and `Iterator.hasNext` on it failed the
//! whole entrypoint (`entity-view-distance`'s `main`), which is what
//! `docs/in-game-survey.md` records.

use crate::runtime::StdHost;

/// The values registered into registry `id`, in registration order.
pub(crate) fn values(host: &StdHost, id: &str) -> Vec<crate::runtime::Value> {
    host.registrations
        .iter()
        .filter(|entry| entry.registry == id)
        .map(|entry| entry.value.clone())
        .collect()
}

/// Whether `owner` names one of the registry interfaces a registry marker is
/// called through.
pub(crate) fn is_registry_owner(owner: &str) -> bool {
    matches!(
        owner,
        "net/minecraft/core/Registry"
            | "net/minecraft/core/DefaultedRegistry"
            | "net/minecraft/core/WritableRegistry"
            | "net/minecraft/core/MappedRegistry"
            | "net/minecraft/core/DefaultedMappedRegistry"
    )
}

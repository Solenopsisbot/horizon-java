//! The event objects a jar's listeners receive, as host handles (#156 P8b): the object a
//! NeoForge/Forge game event listener gets (`game-event/<event>` before) and the loading-stage
//! events FML posts to a mod's bus (`neoforge-lifecycle|<class>|<container>` before). A String
//! spelled like either was that event to the façade.
//!
//! **Which class.** Both know their exact class. bus-8 runs a listener only for the class it
//! named (abstract event classes cannot hold listeners, `neoforge_bus.rs`), and Forge's per-event
//! `BUS` is that event's own, so a game event's class is the class its listener registered on,
//! one `events::bus_event` knows. Its key is that class, validated against that table, never other
//! jar text. A loading stage's event is the stage's FML class (`LoadingStage::event`), built for one
//! container; its key holds both parts through the escaping encoder.
//!
//! **Its supertypes.** NeoForge's generated event table (`neoforge_event_classes.rs`, from the real
//! jars) gives the superclass chain up to bus-8's `Event` and whether the class is an
//! `ICancellableEvent`/`IModBusEvent`. That chain is complete, so a class outside it is a proved
//! "no" (a Mixin adds no superclass). FML's lifecycle classes sit outside the Mixin layer and
//! implement nothing else (`javap`, loader-12.0.12), so their row is exhaustive. A NeoForge game
//! event class is a Mixin target: `true` for an interface a loaded Mixin adds, any other interface
//! counted. A Forge event's chain is not in the table: only its class (and `Object`) is known.
//!
//! **Identity.** A firing's event is a fresh object (every tick a new `ServerTickEvent.Post`), so
//! two game-event handles are not provably one: counted unless the same handle flows. A stage's
//! event for one container is posted once, one object: canonical.
//!
//! Neither class overrides `toString`/`hashCode` (`Object`'s, an identity hash): counted. The façade
//! answered the marker's text before, a silent wrong answer. Not `Comparable`.

use crate::events::GameEvent;
use crate::host_ref::{ClassHierarchy, HostKind, HostRef, Methods, Nested, exact_class_rules_out};
use crate::neoforge_lifecycle::LoadingStage;
use crate::{NativeError, Value};

const OBJECT: &str = "java/lang/Object";
const EVENT: &str = "net/neoforged/bus/api/Event";
const CANCELLABLE: &str = "net/neoforged/bus/api/ICancellableEvent";
const MOD_BUS: &str = "net/neoforged/fml/event/IModBusEvent";

/// The event a NeoForge/Forge listener on `class` receives; `class` is one `events::bus_event`
/// names (the listener's registration was checked against it).
pub(crate) fn game_event(class: &str) -> Value {
    debug_assert!(crate::events::bus_event(class).is_some(), "{class}");
    Value::Host(HostRef::new(HostKind::GameEvent, class))
}

/// The game event a game-event handle stands for.
pub(crate) fn fired_event(value: &Value) -> Option<GameEvent> {
    match value {
        Value::Host(handle) if handle.kind() == HostKind::GameEvent => {
            crate::events::bus_event(handle.key()).map(|(event, _)| event)
        }
        _ => None,
    }
}

/// The event loading stage `class` posts, built for `container` (none for the one shared
/// `RegisterPayloadHandlersEvent`).
pub(crate) fn lifecycle_event(class: &str, container: Option<&str>) -> Value {
    Value::Host(HostRef::of_values(
        HostKind::LifecycleEvent,
        &[
            Value::Ref(Some(class.to_owned())),
            Value::Ref(container.map(str::to_owned)),
        ],
    ))
}

/// `(class, container)` of a loading-stage event handle.
pub(crate) fn lifecycle_parts(value: &Value) -> Option<(String, Option<String>)> {
    match value {
        Value::Host(handle) if handle.kind() == HostKind::LifecycleEvent => {
            match handle.values()?.as_slice() {
                [Value::Ref(Some(class)), Value::Ref(container)] => {
                    Some((class.clone(), container.clone()))
                }
                _ => None,
            }
        }
        _ => None,
    }
}

/// [`crate::host_ref::ClassRule::FromKey`] of a game event: its key is its class.
pub(crate) fn game_event_class(key: &str) -> &str {
    key
}

/// [`crate::host_ref::ClassRule::FromKey`] of a loading-stage event: the stage class its key
/// holds (every one is built from a [`LoadingStage`]'s class).
pub(crate) fn lifecycle_class(key: &str) -> &str {
    let class = lifecycle_parts(&Value::Host(HostRef::new(HostKind::LifecycleEvent, key)))
        .map(|(class, _)| class);
    LoadingStage::ALL
        .iter()
        .flat_map(|stage| [stage.event(true), stage.event(false)])
        .find(|stage_class| class.as_deref() == Some(*stage_class))
        .unwrap_or(EVENT)
}

/// `instanceof` for an event of the exact class `class`: its table chain and flagged interfaces
/// are `true`; with `exhaustive` (a class outside the Mixin layer) anything else is `false`;
/// otherwise a loaded Mixin's interface is `true`, a class outside a complete chain `false`, and any
/// other interface counted.
fn event_instance_of(
    class: &str,
    wanted: &str,
    hierarchy: &dyn ClassHierarchy,
    exhaustive: bool,
) -> Option<bool> {
    if wanted == OBJECT || wanted == class {
        return Some(true);
    }
    let mut chain = Vec::new();
    let mut at = class.to_owned();
    let mut complete = false;
    while chain.len() < 32 {
        let Some(parent) = crate::neoforge_bus::table_superclass(&at) else {
            break;
        };
        complete = parent == EVENT;
        chain.push(parent.clone());
        if complete {
            break;
        }
        at = parent;
    }
    let (cancellable, mod_bus) =
        crate::neoforge_bus::table_interfaces(class).unwrap_or((false, false));
    if chain.iter().any(|parent| parent == wanted)
        || (cancellable && wanted == CANCELLABLE)
        || (mod_bus && wanted == MOD_BUS)
        || (!exhaustive && hierarchy.is_instance(class, wanted) == Some(true))
    {
        return Some(true);
    }
    // A class (every class in NeoForge's event table is one) outside a complete chain is a
    // proved "no": a Mixin adds no superclass.
    let is_class = wanted == EVENT
        || crate::neoforge_bus::table_superclass(wanted).is_some()
        || hierarchy.is_interface(wanted) == Some(false);
    if complete && (exhaustive || is_class) {
        return Some(false);
    }
    exact_class_rules_out(class, wanted).then_some(false)
}

/// `class instanceof wanted` for an event class name (`getClass()` of an event handle handed to
/// `Class.isAssignableFrom`, a class walk): `None` when `class` is neither a NeoForge event class
/// nor one the game fires (a Forge event, whose chain is unknown: only itself is known).
pub(crate) fn event_class_instance_of(
    class: &str,
    wanted: &str,
    hierarchy: &dyn ClassHierarchy,
) -> Option<Option<bool>> {
    if crate::neoforge_bus::table_superclass(class).is_none()
        && crate::events::bus_event(class).is_none()
    {
        return None;
    }
    Some(event_instance_of(
        class,
        wanted,
        hierarchy,
        class.starts_with("net/neoforged/fml/"),
    ))
}

/// `Object.equals`: identity, where it is known.
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

fn unknown_text(_: &HostRef, _: &mut dyn Nested) -> Result<Option<String>, NativeError> {
    Ok(None)
}

fn unknown_hash(_: &HostRef, _: &mut dyn Nested) -> Result<Option<i32>, NativeError> {
    Ok(None)
}

/// An event never means the same in another jar's runtime.
fn stays_in_its_jar(_: &HostRef, _: &dyn Fn(&Value) -> bool) -> bool {
    false
}

pub(crate) const GAME_EVENT: Methods = Methods {
    to_string: unknown_text,
    hash_code: unknown_hash,
    equals: identity_equals,
    instance_of: |handle, wanted, hierarchy| {
        event_instance_of(handle.key(), wanted, hierarchy, false)
    },
    crosses_jars: stays_in_its_jar,
    compare_to: None,
};

pub(crate) const LIFECYCLE_EVENT: Methods = Methods {
    instance_of: |handle, wanted, hierarchy| {
        let class = lifecycle_class(handle.key());
        event_instance_of(
            class,
            wanted,
            hierarchy,
            class.starts_with("net/neoforged/fml/"),
        )
    },
    ..GAME_EVENT
};

#[cfg(test)]
#[path = "host_game_event_tests.rs"]
mod tests;

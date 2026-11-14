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
//! **Identity.** A firing's event is a fresh object (every tick a new `ServerTickEvent.Post`), and
//! every listener of that firing gets that one object: the key holds a serial minted once per
//! firing and class (`EventFiring::event_object`), so one key is one object (canonical), two
//! firings' events are two keys, and `equals`, which is `Object`'s, is exact either way. A stage's
//! event for one container is posted once, one object: canonical.
//!
//! **Class names.** `getClass()` of an event is its exact class, so the reflective `Class` surface
//! reaches event class names: [`EVENT_CLASS_SIDE`]/[`LIFECYCLE_CLASS_SIDE`] (`host_ref::ClassSide`)
//! answer for them from the table (supertypes, the direct superclass, "a class, not an interface"),
//! and every other `Class` call on one is counted (`Runtime::invoke_owned_class`).
//!
//! Neither class overrides `toString`/`hashCode` (`Object`'s, an identity hash): counted. The façade
//! answered the marker's text before, a silent wrong answer. Not `Comparable`.

use crate::events::GameEvent;
use std::collections::HashSet;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Mutex, OnceLock};

use crate::host_ref::{
    ClassHierarchy, ClassSide, HostKind, HostRef, Methods, Nested, exact_class_rules_out,
};
use crate::neoforge_lifecycle::LoadingStage;
use crate::{NativeError, Value};

const OBJECT: &str = "java/lang/Object";
const EVENT: &str = "net/neoforged/bus/api/Event";
const CANCELLABLE: &str = "net/neoforged/bus/api/ICancellableEvent";
const MOD_BUS: &str = "net/neoforged/fml/event/IModBusEvent";

/// A new event object of `class` for one firing (`EventFiring::event_object` hands the same one
/// to every listener of that firing). `class` is one `events::bus_event` names (the listener's
/// registration was checked against it). Its key holds the class and a serial no other event
/// gets, so one key is one object.
pub(crate) fn game_event(class: &str) -> Value {
    static NEXT_SERIAL: AtomicI64 = AtomicI64::new(0);
    debug_assert!(crate::events::bus_event(class).is_some(), "{class}");
    let serial = NEXT_SERIAL.fetch_add(1, Ordering::Relaxed);
    Value::Host(HostRef::of_values(
        HostKind::GameEvent,
        &[Value::Ref(Some(class.to_owned())), Value::Long(serial)],
    ))
}

/// The class a game-event handle's key holds.
fn event_class_of(handle: &HostRef) -> Option<String> {
    match handle.values()?.as_slice() {
        [Value::Ref(Some(class)), Value::Long(_)] => Some(class.clone()),
        _ => None,
    }
}

/// The class of a game-event handle, `None` for any other value.
pub(crate) fn game_event_class_of(value: &Value) -> Option<String> {
    match value {
        Value::Host(handle) if handle.kind() == HostKind::GameEvent => event_class_of(handle),
        _ => None,
    }
}

/// The game event a game-event handle stands for.
pub(crate) fn fired_event(value: &Value) -> Option<GameEvent> {
    crate::events::bus_event(&game_event_class_of(value)?).map(|(event, _)| event)
}

/// The event loading stage `class` posts, built for mod `container`'s container (its mod id;
/// none for the one shared `RegisterPayloadHandlersEvent`).
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

/// [`crate::host_ref::ClassRule::FromKey`] of a game event: the class its key holds, one of the
/// classes `events::bus_event` names (interned once each: a bounded set).
pub(crate) fn game_event_class(key: &str) -> &str {
    static CLASSES: OnceLock<Mutex<HashSet<&'static str>>> = OnceLock::new();
    let Some(class) = event_class_of(&HostRef::new(HostKind::GameEvent, key)) else {
        return EVENT;
    };
    let mut classes = CLASSES
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(known) = classes.get(class.as_str()) {
        return known;
    }
    let interned: &'static str = Box::leak(class.into_boxed_str());
    classes.insert(interned);
    interned
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

/// Whether `class` is a NeoForge event class (its generated table) or one the game fires (a Forge
/// event, whose chain is unknown), but no loading stage's.
fn game_family(class: &str) -> bool {
    (crate::neoforge_bus::table_superclass(class).is_some()
        || crate::events::bus_event(class).is_some())
        && !stage_class(class)
}

/// Whether `class` is a loading stage's event class.
fn stage_class(class: &str) -> bool {
    LoadingStage::ALL
        .iter()
        .any(|stage| stage.event(true) == class || stage.event(false) == class)
}

/// The class name of an event family: what `getClass()` of an event handle returns, and its
/// supertypes in the table. Its direct superclass is the table's; every class in the table is a
/// class (it extends `Event`).
fn class_side_instance_of(
    class: &str,
    wanted: &str,
    hierarchy: &dyn ClassHierarchy,
) -> Option<bool> {
    event_instance_of(
        class,
        wanted,
        hierarchy,
        class.starts_with("net/neoforged/fml/"),
    )
}

fn table_superclass(class: &str) -> Option<String> {
    crate::neoforge_bus::table_superclass(class)
}

fn table_class(class: &str) -> Option<bool> {
    crate::neoforge_bus::table_superclass(class).map(|_| false)
}

/// The game events' class names: the event table's classes and the classes the game fires, but
/// the loading stages'.
fn game_family_classes() -> Vec<String> {
    let mut classes: Vec<String> = crate::neoforge_event_classes::EVENT_CLASSES
        .iter()
        .map(|(class, _, _)| (*class).to_owned())
        .chain(crate::events::bus_event_classes())
        .filter(|class| !stage_class(class))
        .collect();
    classes.sort();
    classes.dedup();
    classes
}

fn stage_classes() -> Vec<String> {
    let mut classes: Vec<String> = LoadingStage::ALL
        .iter()
        .flat_map(|stage| [stage.event(true), stage.event(false)])
        .map(str::to_owned)
        .collect();
    classes.sort();
    classes.dedup();
    classes
}

pub(crate) const EVENT_CLASS_SIDE: ClassSide = ClassSide {
    owns: game_family,
    instance_of: class_side_instance_of,
    superclass: table_superclass,
    is_interface: table_class,
    classes: game_family_classes,
};

pub(crate) const LIFECYCLE_CLASS_SIDE: ClassSide = ClassSide {
    owns: stage_class,
    classes: stage_classes,
    ..EVENT_CLASS_SIDE
};

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
        let class = game_event_class(handle.key());
        event_instance_of(
            class,
            wanted,
            hierarchy,
            class.starts_with("net/neoforged/fml/"),
        )
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

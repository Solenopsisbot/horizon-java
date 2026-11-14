//! Game objects (#156 P16): the one place that reads one.
//!
//! The game hands the runtime what it knows about its objects as snapshots
//! (`events::EventFacade`, `EventContext`: a player's UUID and name, a level's
//! dimension id, an entity's id and type) and the runtime has spelled each one
//! as marker text: `game/player/<uuid>`, `game/level/<dimension>`,
//! `block-pos/x,y,z`, `client/minecraft`... A `String` spelled the same way was
//! then that object, and the families' identity rules were the text's (two
//! absent players, `game/unknown/player`, were one object). P16 moves them to
//! handles, in steps (`hj-notes/p16-plan.md`):
//!
//! * **P16a (this layer):** the six kinds below, and one accessor every reader
//!   of a game object goes through: [`text`] answers a game handle's key, and
//!   (until P16 ends) the marker text every producer still answers. A handle's
//!   key is the marker it replaces, so a reader parses either alike. No
//!   producer changes, so no answer changes.
//! * **P16b–e:** the producers (server objects, value types, client objects,
//!   the game-hook tier) answer handles, each family with its vanilla 26.3
//!   identity, `equals`, `hashCode` and `toString` rules.
//! * **P17:** text is never a game object.
//!
//! The kinds, by vanilla's own identity rules (decompiled 26.3 server
//! sources), are below. The client families' rows are provisional until the
//! client classes are read (P16d).
//!
//! | kind | identity | families |
//! |---|---|---|
//! | [`HostKind::GameEntity`] | one per entity (`Entity.equals`/`hashCode` are by id) | `game/player/`, `game/entity/`, `game/original-player/`, `client/player` |
//! | [`HostKind::GameLevel`] | one per dimension | `game/level/`, `client/level` |
//! | [`HostKind::GameSingleton`] | one per name | `game/server`, `game/player-list`, `game/commands`, `game/rules/main`, `game/registries/…`, `game/saved-data/storage`, `game/world-border/<dim>`, the client's services (`client/minecraft`, `client/gui`, `client/option/<name>`…) |
//! | [`HostKind::GameRegistryObject`] | one per registry entry | `game/item/`, `game/block/`, `game/block-state/`, `game/registry/`, `game-rule/`, `client/item/`, `client/block/`, `client/mob-effect/`, `client/effect-holder/` |
//! | [`HostKind::GameValue`] | a value: `equals` by its fields, `==` per producer | `block-pos/`, `level-resource/`, `key-category/`, `client/style/`, `client/text-color/`, `client/vec3/`, `client/awt-color/` |
//! | [`HostKind::GameTransient`] | its own object each time | `game/damage/`, `game/stack/`, `game/block-hit/`, `game/connection/`, `game/packet-sender/`, `game/unknown/…`, `game/<what>/0`, `client/stack/`, `client/effect/`, `made/`, the client's per-frame objects |
//!
//! Every kind's class is `ClassRule::Unknown` with the types the family surely
//! is, as the markers' answers are today: `getClass()` is counted, and
//! `instanceof` follows the declared types. P16b–e give each family its exact
//! class where the snapshot names it.

use crate::host_ref::{HostKind, HostRef, Methods, instance_of_class};
use crate::{NativeError, Value};

/// The client's service objects: one each, by name (`client/minecraft`).
const CLIENT_SINGLETONS: &[&str] = &[
    "minecraft",
    "camera",
    "chat",
    "connection",
    "debug-overlay",
    "font",
    "food-data",
    "game-mode",
    "game-renderer",
    "gui",
    "hud",
    "integrated-server",
    "inventory",
    "options",
    "packet-sender",
    "texture-manager",
    "tooltip-flag",
    "user",
    "window",
];

/// Which kind a game marker's text names, `None` for text that names no game
/// object (it is no game family's).
pub(crate) fn kind_of(marker: &str) -> Option<HostKind> {
    if let Some(rest) = marker.strip_prefix("game/") {
        return Some(server_kind(rest));
    }
    if let Some(rest) = marker.strip_prefix("client/") {
        return Some(client_kind(rest));
    }
    let (family, _) = marker.split_once('/')?;
    Some(match family {
        "block-pos" | "level-resource" | "key-category" => HostKind::GameValue,
        "game-rule" => HostKind::GameRegistryObject,
        "made" => HostKind::GameTransient,
        _ => return None,
    })
}

/// The kind of a `game/<rest>` marker.
fn server_kind(rest: &str) -> HostKind {
    let (family, _) = rest.split_once('/').unwrap_or((rest, ""));
    match family {
        "player" | "entity" | "original-player" => HostKind::GameEntity,
        "level" => HostKind::GameLevel,
        "server" | "player-list" | "commands" | "rules" | "registries" | "saved-data"
        | "world-border" => HostKind::GameSingleton,
        "item" | "block" | "block-state" | "registry" => HostKind::GameRegistryObject,
        // `damage`, `stack`, `block-hit`, `connection`, `packet-sender`,
        // `unknown`, `<what>/0` placeholders, and any family a model adds
        // before it gets a row: their own objects (never one shared object).
        _ => HostKind::GameTransient,
    }
}

/// The kind of a `client/<rest>` marker.
fn client_kind(rest: &str) -> HostKind {
    let (family, key) = rest.split_once('/').unwrap_or((rest, ""));
    match family {
        "player" if key.is_empty() => HostKind::GameEntity,
        "level" if key.is_empty() => HostKind::GameLevel,
        // `OptionInstance`s and vanilla `KeyMapping`s: one per name.
        "option" | "vanilla-key" => HostKind::GameSingleton,
        name if key.is_empty() && CLIENT_SINGLETONS.contains(&name) => HostKind::GameSingleton,
        "item" | "block" | "mob-effect" | "effect-holder" => HostKind::GameRegistryObject,
        "style" | "text-color" | "vec3" | "awt-color" => HostKind::GameValue,
        // `stack`, `effect`, and the per-frame `gui-graphics`, `pose`,
        // `delta-tracker`, `tooltip-context`.
        _ => HostKind::GameTransient,
    }
}

/// Whether `kind` is one of the game kinds.
pub(crate) fn is_game_kind(kind: HostKind) -> bool {
    matches!(
        kind,
        HostKind::GameEntity
            | HostKind::GameLevel
            | HostKind::GameSingleton
            | HostKind::GameRegistryObject
            | HostKind::GameValue
            | HostKind::GameTransient
    )
}

/// The handle of the game object `marker` names (its kind by [`kind_of`]).
/// `None` for text that names no game object.
pub(crate) fn value(marker: &str) -> Option<Value> {
    Some(Value::Host(HostRef::new(kind_of(marker)?, marker)))
}

/// The marker a game object's value is read as: a game handle's key (the
/// marker it replaces), or, until #156 P16 ends, what `expect_ref` reads (the
/// text every producer still answers, a jar object's key). A reader of a game
/// object takes its text here, so it parses a handle and the old text alike.
pub(crate) fn text(value: &Value) -> Option<&String> {
    match value {
        Value::Host(handle) if is_game_kind(handle.kind()) => Some(handle.key_text()),
        other => crate::expect_ref(other),
    }
}

/// A server-side value of the game (#156 P16b): the handle of a `game/…`
/// marker. Text of any other family stays the text it was: its producers
/// move with P16c (values and registry objects) and P16d (the client).
pub(crate) fn server_value(marker: String) -> Value {
    if marker.starts_with("game/")
        && let Some(value) = value(&marker)
    {
        return value;
    }
    Value::Ref(Some(marker))
}

/// A player's key: `game/player/<uuid>/<session>.<life>`.
///
/// One `ServerPlayer` is one object from login to respawn: `PlayerList.respawn`
/// builds a new one and gives it the old id (`setId(serverPlayer.getId())`),
/// a relog builds a new one with a new id, and a dimension change keeps the
/// same object (`ServerPlayer.teleport` returns `this`). The game's bridge
/// counts the sessions (logins) and lives (respawns, the clone moment) of each
/// player (`events::FacadePlayer`), so one key is one object, and `Entity`'s
/// `equals` (by id) is "same UUID and same session".
fn player_key(uuid: u128, session: u32, life: u32) -> String {
    format!("game/player/{uuid:032x}/{session}.{life}")
}

/// The player `uuid` as the firing's façade knows them (session and life;
/// `0.0` for a player the façade does not list).
pub(crate) fn player_value(facade: &crate::events::EventFacade, uuid: u128) -> Value {
    let (session, life) = facade
        .players
        .iter()
        .find(|player| player.uuid == uuid)
        .map_or((0, 0), |player| (player.session, player.life));
    server_value(player_key(uuid, session, life))
}

/// The entity with id `id`: the player of that id when the façade lists one
/// (an attacked player is that player, not a second object), else
/// `game/entity/<id>`.
pub(crate) fn entity_value(facade: &crate::events::EventFacade, id: i32) -> Value {
    match facade
        .players
        .iter()
        .find(|player| player.entity_id == Some(id))
    {
        Some(player) => player_value(facade, player.uuid),
        None => server_value(format!("game/entity/{id}")),
    }
}

/// The UUID of a player marker (`game/player/<uuid>[/<session>.<life>]`).
pub(crate) fn player_uuid(marker: &str) -> Option<u128> {
    player_parts(marker).map(|(uuid, _)| uuid)
}

/// A player marker's UUID and session (`None` for the session of the old
/// key without one).
fn player_parts(marker: &str) -> Option<(u128, Option<u32>)> {
    let rest = marker.strip_prefix("game/player/")?;
    let (hex, life) = rest
        .split_once('/')
        .map_or((rest, None), |(hex, life)| (hex, Some(life)));
    let uuid = (hex.len() == 32).then(|| u128::from_str_radix(hex, 16).ok())??;
    let session = match life {
        Some(life) => Some(life.split_once('.')?.0.parse().ok()?),
        None => None,
    };
    Some((uuid, session))
}

/// The id of a `game/entity/<id>` marker.
fn entity_id(marker: &str) -> Option<i32> {
    marker.strip_prefix("game/entity/")?.parse().ok()
}

/// `Entity.equals` (vanilla 26.3: `obj instanceof Entity e && e.getId() ==
/// getId()`) between two entity keys: one key is one object; two players are
/// equal when their UUID and session match (a respawn keeps the id, a relog
/// takes a new one); two entities by id. A player against an entity the
/// façade did not resolve, or a clone's original against a player, is not
/// known here: `None`, counted by the caller.
fn entity_equals(a: &str, b: &str) -> Option<bool> {
    if a == b {
        return Some(true);
    }
    if let (Some((u1, s1)), Some((u2, s2))) = (player_parts(a), player_parts(b)) {
        return match (s1, s2) {
            _ if u1 != u2 => Some(false),
            (Some(s1), Some(s2)) => Some(s1 == s2),
            _ => None,
        };
    }
    match (entity_id(a), entity_id(b)) {
        (Some(x), Some(y)) => Some(x == y),
        _ => None,
    }
}

/// The types each family surely is (its `ClassRule::Unknown` declared
/// types): what a reader of the marker has always taken it for. The exact
/// class (a `ServerPlayer` or a mod's subclass, `ServerLevel`, a dedicated or
/// integrated server) comes with each family's producer in P16b–e.
pub(crate) fn declared_types(key: &str) -> &'static [&'static str] {
    let family = key
        .split_once('/')
        .map(|(head, rest)| match head {
            "game" | "client" => rest.split_once('/').map_or(rest, |(family, _)| family),
            other => other,
        })
        .unwrap_or(key);
    match (key.starts_with("client/"), family) {
        (false, "player") => &["net/minecraft/server/level/ServerPlayer"],
        (false, "original-player") => &["net/minecraft/world/entity/player/Player"],
        (false, "entity") => &["net/minecraft/world/entity/Entity"],
        (false, "level") => &["net/minecraft/server/level/ServerLevel"],
        (false, "server") => &["net/minecraft/server/MinecraftServer"],
        (false, "player-list") => &["net/minecraft/server/players/PlayerList"],
        (false, "commands") => &["net/minecraft/commands/Commands"],
        (false, "world-border") => &["net/minecraft/world/level/border/WorldBorder"],
        (false, "item") => &["net/minecraft/world/item/Item"],
        (false, "block") => &["net/minecraft/world/level/block/Block"],
        (false, "block-state") => &["net/minecraft/world/level/block/state/BlockState"],
        (false, "stack") => &["net/minecraft/world/item/ItemStack"],
        (false, "damage") => &["net/minecraft/world/damagesource/DamageSource"],
        (_, "block-pos") => &["net/minecraft/core/BlockPos"],
        (_, "level-resource") => &["net/minecraft/world/level/storage/LevelResource"],
        (_, "game-rule") => &["net/minecraft/world/level/gamerules/GameRule"],
        (true, "minecraft") => &["net/minecraft/client/Minecraft"],
        (true, "level") => &["net/minecraft/client/multiplayer/ClientLevel"],
        (true, "player") => &["net/minecraft/client/player/LocalPlayer"],
        (true, "stack") => &["net/minecraft/world/item/ItemStack"],
        (true, "item") => &["net/minecraft/world/item/Item"],
        (true, "block") => &["net/minecraft/world/level/block/Block"],
        // A family whose class P16b–e reads from its snapshot: only `Object`
        // is sure until then.
        _ => &[],
    }
}

/// `equals` of an object with one identity per key: the same object exactly
/// when [`HostRef::same_object`] says so; any other value (a `String` spelled
/// like the marker included) is another object.
fn identity_equals(
    handle: &HostRef,
    other: &Value,
    _: &mut dyn crate::host_ref::Nested,
) -> Result<Option<bool>, NativeError> {
    Ok(match other {
        Value::Host(other) => handle.same_object(other),
        _ => Some(false),
    })
}

/// Entities, levels, singletons, registry entries and transient objects:
/// identity `equals` (an entity's is by id, which its key carries until P16b
/// adds incarnations); `hashCode` and `toString` counted until each family's
/// producer answers vanilla's (P16b–e). A game object is the game's, the same
/// object to every jar's runtime.
pub(crate) const IDENTITY_METHODS: Methods = Methods {
    to_string: |_, _| Ok(None),
    hash_code: |_, _| Ok(None),
    equals: identity_equals,
    instance_of: instance_of_class,
    crosses_jars: |_, _| true,
    compare_to: None,
};

/// An entity (`Entity.equals`/`hashCode` are by `getId()`, vanilla 26.3):
/// `equals` by [`entity_equals`]; `hashCode` is the id where the key carries
/// it (`game/entity/<id>`), counted for a player (their id is the façade's,
/// not the key's). `toString` needs the position: counted.
pub(crate) const ENTITY_METHODS: Methods = Methods {
    hash_code: |handle, _| Ok(entity_id(handle.key())),
    equals: |handle, other, _| {
        Ok(match other {
            Value::Host(other) if other.kind() == HostKind::GameEntity => {
                entity_equals(handle.key(), other.key())
            }
            _ => Some(false),
        })
    },
    ..IDENTITY_METHODS
};

/// A value (`BlockPos`, `LevelResource`, `Style`...): `equals` by its fields,
/// which its key spells exactly, so two handles of one key are equal (and two
/// keys two values); `==` is per producer (the kind is never canonical).
pub(crate) const VALUE_METHODS: Methods = Methods {
    equals: |handle, other, _| {
        Ok(Some(matches!(
            other,
            Value::Host(other) if other.kind() == handle.kind() && other.key() == handle.key()
        )))
    },
    ..IDENTITY_METHODS
};

#[cfg(test)]
mod tests {
    use super::*;

    /// Every family names its kind; text of no game family names none.
    #[test]
    fn markers_name_their_kinds() {
        for (marker, kind) in [
            (
                "game/player/0000000000000000000000000000002a",
                HostKind::GameEntity,
            ),
            ("game/entity/7", HostKind::GameEntity),
            ("game/original-player/00", HostKind::GameEntity),
            ("client/player", HostKind::GameEntity),
            ("game/level/minecraft:overworld", HostKind::GameLevel),
            ("client/level", HostKind::GameLevel),
            ("game/server", HostKind::GameSingleton),
            ("game/rules/main", HostKind::GameSingleton),
            ("game/registries/layered", HostKind::GameSingleton),
            (
                "game/world-border/minecraft:overworld",
                HostKind::GameSingleton,
            ),
            ("client/minecraft", HostKind::GameSingleton),
            ("client/option/fov", HostKind::GameSingleton),
            ("game/item/minecraft:stone", HostKind::GameRegistryObject),
            (
                "game/block-state/minecraft:stone",
                HostKind::GameRegistryObject,
            ),
            ("game-rule/keep_inventory", HostKind::GameRegistryObject),
            (
                "client/mob-effect/minecraft:speed",
                HostKind::GameRegistryObject,
            ),
            ("block-pos/1,-2,3", HostKind::GameValue),
            ("level-resource/data", HostKind::GameValue),
            ("key-category/minecraft:misc", HostKind::GameValue),
            ("client/style/§4", HostKind::GameValue),
            ("game/damage/minecraft:generic", HostKind::GameTransient),
            ("game/unknown/player", HostKind::GameTransient),
            ("game/hand/0", HostKind::GameTransient),
            ("client/stack/main", HostKind::GameTransient),
            ("client/gui-graphics", HostKind::GameTransient),
            ("made/3", HostKind::GameTransient),
        ] {
            assert_eq!(kind_of(marker), Some(kind), "{marker}");
        }
        for text in [
            "hello",
            "list/3",
            "enum/a/B",
            "minecraft:stone",
            "demo/Main",
        ] {
            assert_eq!(kind_of(text), None, "{text}");
        }
    }

    /// A handle reads as the marker it replaces; text reads as it always did.
    #[test]
    fn text_reads_a_handle_and_the_old_marker_alike() {
        let handle = value("game/player/2a").unwrap();
        assert_eq!(text(&handle).map(String::as_str), Some("game/player/2a"));
        let marker = Value::Ref(Some("game/player/2a".into()));
        assert_eq!(text(&marker).map(String::as_str), Some("game/player/2a"));
        assert_eq!(text(&Value::Int(1)), None);
        assert_eq!(value("hello"), None);
    }

    /// One key, one object for the canonical kinds; values equal by key but
    /// two objects; transient objects never one by key.
    #[test]
    fn identity_follows_each_kind() {
        let same = |a: &Value, b: &Value| match (a, b) {
            (Value::Host(a), Value::Host(b)) => a.same_object(b),
            _ => unreachable!(),
        };
        let level = || value("game/level/minecraft:overworld").unwrap();
        assert_eq!(same(&level(), &level()), Some(true));
        let pos = || value("block-pos/1,2,3").unwrap();
        assert_ne!(same(&pos(), &pos()), Some(true));
        let damage = || value("game/damage/minecraft:generic").unwrap();
        assert_ne!(same(&damage(), &damage()), Some(true));
        assert_eq!(
            declared_types("game/player/2a"),
            &["net/minecraft/server/level/ServerPlayer"]
        );
        assert_eq!(
            declared_types("block-pos/1,2,3"),
            &["net/minecraft/core/BlockPos"]
        );
        assert_eq!(
            declared_types("client/level"),
            &["net/minecraft/client/multiplayer/ClientLevel"]
        );
    }
}

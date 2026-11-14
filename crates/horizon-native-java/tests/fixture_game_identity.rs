//! Game objects keep the server's identity across firings (#156 P16b): a
//! source-built Fabric jar keeps the player, a damage source and an entity it
//! was handed in statics and compares them with the next firing's, the way
//! real mods keep a `ServerPlayer` in a map or a field.
//!
//! Every expected value comes from vanilla 26.3's real classes on Java 25:
//! `ServerPlayer`, `ItemEntity` and `DamageSource` objects made without their
//! constructors (`Unsafe.allocateInstance`, `setId`) and asked the same
//! questions (`==`, `equals`, `hashCode`, `HashSet`, `IdentityHashMap`,
//! `HashMap`, `List.contains`, `Set.of`, `instanceof`, a cast, a pattern
//! `switch`). `PlayerList.respawn` builds a new `ServerPlayer` and gives it
//! the old one's id, so a respawned player is `equals` but not `==`; a relog
//! is a new id. Where the runtime cannot know the answer it counts the call
//! (two damage sources from two firings may be one object or two), and the
//! test pins the count.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::{
    ExecutionBudget, Host, NativeJar, StdHost, Value,
    events::{
        EventContext, EventFacade, EventFiring, FacadeEntity, FacadePlayer, GameEvent, GameListener,
    },
};
use javac_fixture::Fixture;
use std::sync::Arc;

const ALEX: u128 = 0x0123_4567_89ab_cdef_fedc_ba98_7654_3210;

const STUBS: &[(&str, &str)] = &[
    (
        "net/fabricmc/fabric/api/event/Event.java",
        "package net.fabricmc.fabric.api.event;\n\
         public abstract class Event<T> { public void register(T listener) {} }",
    ),
    (
        "net/fabricmc/fabric/api/event/lifecycle/v1/ServerTickEvents.java",
        "package net.fabricmc.fabric.api.event.lifecycle.v1;\n\
         import net.fabricmc.fabric.api.event.Event;\n\
         import net.minecraft.server.MinecraftServer;\n\
         public final class ServerTickEvents {\n\
           public static final Event<EndTick> END_SERVER_TICK = null;\n\
           public interface EndTick { void onEndTick(MinecraftServer server); } }",
    ),
    (
        "net/fabricmc/fabric/api/entity/event/v1/ServerPlayerEvents.java",
        "package net.fabricmc.fabric.api.entity.event.v1;\n\
         import net.fabricmc.fabric.api.event.Event;\n\
         import net.minecraft.server.level.ServerPlayer;\n\
         public final class ServerPlayerEvents {\n\
           public static final Event<AfterRespawn> AFTER_RESPAWN = null;\n\
           public interface AfterRespawn { void afterRespawn(ServerPlayer oldPlayer, ServerPlayer newPlayer, boolean alive); } }",
    ),
    (
        "net/fabricmc/fabric/api/entity/event/v1/ServerLivingEntityEvents.java",
        "package net.fabricmc.fabric.api.entity.event.v1;\n\
         import net.fabricmc.fabric.api.event.Event;\n\
         public final class ServerLivingEntityEvents {\n\
           public static final Event<AllowDamage> ALLOW_DAMAGE = null;\n\
           public interface AllowDamage { boolean allowDamage(net.minecraft.world.entity.LivingEntity entity,\n\
             net.minecraft.world.damagesource.DamageSource source, float amount); } }",
    ),
    (
        "net/minecraft/server/MinecraftServer.java",
        "package net.minecraft.server;\n\
         public class MinecraftServer {\n\
           public net.minecraft.server.level.ServerLevel overworld() { return null; }\n\
           public net.minecraft.server.players.PlayerList getPlayerList() { return null; } }",
    ),
    (
        "net/minecraft/server/players/PlayerList.java",
        "package net.minecraft.server.players;\n\
         public class PlayerList { public java.util.List<net.minecraft.server.level.ServerPlayer> getPlayers() { return null; } }",
    ),
    (
        "net/minecraft/world/entity/Entity.java",
        "package net.minecraft.world.entity;\npublic abstract class Entity {}",
    ),
    (
        "net/minecraft/world/entity/LivingEntity.java",
        "package net.minecraft.world.entity;\npublic abstract class LivingEntity extends Entity {}",
    ),
    (
        "net/minecraft/world/entity/player/Player.java",
        "package net.minecraft.world.entity.player;\n\
         public abstract class Player extends net.minecraft.world.entity.LivingEntity {}",
    ),
    (
        "net/minecraft/server/level/ServerPlayer.java",
        "package net.minecraft.server.level;\n\
         public class ServerPlayer extends net.minecraft.world.entity.player.Player {}",
    ),
    (
        "net/minecraft/server/level/ServerLevel.java",
        "package net.minecraft.server.level;\npublic class ServerLevel {}",
    ),
    (
        "net/minecraft/world/damagesource/DamageSource.java",
        "package net.minecraft.world.damagesource;\npublic class DamageSource {}",
    ),
];

const MOD: &str = r#"
package demo;
import java.util.*;
import net.fabricmc.fabric.api.entity.event.v1.ServerLivingEntityEvents;
import net.fabricmc.fabric.api.entity.event.v1.ServerPlayerEvents;
import net.fabricmc.fabric.api.event.lifecycle.v1.ServerTickEvents;
import net.minecraft.server.level.ServerPlayer;
public final class Probe {
  static ServerPlayer last;
  static ServerPlayer firstLife;
  static Object firstDamage;
  static Object firstEntity;
  static Object lastNew;
  public static int sameEq, sameEquals, hash, hashSet, identityMap, hashMapSize, hashMapGet;
  public static int listContains, setOf, levelText;
  public static int oldEq, oldEquals, oldIsFirstLife, absentEq;
  public static int damageSelf, damageEq, damageEquals, entityEq, entityEquals, entityHash;
  public static int forgedInstance, forgedCast, forgedEquals, equalsForged, forgedEq, forgedSet;
  public static int forgedContains, forgedGet, forgedIdentity, forgedSetOf, forgedSwitch;
  public void onInitialize() {
    ServerTickEvents.END_SERVER_TICK.register(server -> {
      ServerPlayer p = server.getPlayerList().getPlayers().get(0);
      levelText = ("" + server.overworld()).length();
      if (firstLife == null) firstLife = p;
      if (last != null) {
        sameEq = last == p ? 1 : 0;
        sameEquals = last.equals(p) ? 1 : 0;
        hashSet = new HashSet<Object>(List.of(last, p)).size();
        IdentityHashMap<Object, Integer> im = new IdentityHashMap<>();
        im.put(last, 1);
        im.put(p, 2);
        identityMap = im.size();
        HashMap<Object, Integer> hm = new HashMap<>();
        hm.put(last, 1);
        hm.put(p, 2);
        hashMapSize = hm.size();
        hashMapGet = hm.get(last);
        listContains = List.of(last).contains(p) ? 1 : 0;
        try { setOf = Set.of(last, p).size(); } catch (IllegalArgumentException e) { setOf = -1; }
      }
      hash = p.hashCode();
      last = p;
      forgery(p);
    });
    ServerPlayerEvents.AFTER_RESPAWN.register((oldPlayer, newPlayer, alive) -> {
      oldEq = oldPlayer == newPlayer ? 1 : 0;
      oldEquals = oldPlayer.equals(newPlayer) ? 1 : 0;
      oldIsFirstLife = oldPlayer == firstLife ? 1 : 0;
      absentEq = lastNew == newPlayer ? 1 : 0;
      lastNew = newPlayer;
    });
    ServerLivingEntityEvents.ALLOW_DAMAGE.register((entity, source, amount) -> {
      if (firstDamage == null) {
        firstDamage = source;
        firstEntity = entity;
        damageSelf = source == source && source.equals(source) ? 1 : 0;
      } else {
        damageEq = firstDamage == source ? 1 : 0;
        damageEquals = firstDamage.equals(source) ? 1 : 0;
        entityEq = firstEntity == entity ? 1 : 0;
        entityEquals = firstEntity.equals(entity) ? 1 : 0;
      }
      entityHash = entity.hashCode();
      return true;
    });
  }
  // Text spelled like the player's key is a String: a mod's own text never
  // becomes the game object it looks like.
  static void forgery(ServerPlayer p) {
    String head = "game";
    Object forged = head + "/player/0123456789abcdeffedcba9876543210/1.0/7";
    forgedInstance = forged instanceof ServerPlayer ? 1 : 0;
    try { ServerPlayer cast = (ServerPlayer) forged; forgedCast = 1; } catch (ClassCastException e) { forgedCast = -1; }
    forgedEquals = forged.equals(p) ? 1 : 0;
    equalsForged = p.equals(forged) ? 1 : 0;
    forgedEq = forged == (Object) p ? 1 : 0;
    forgedSet = new HashSet<Object>(List.of(p, forged)).size();
    forgedContains = List.of(p).contains(forged) ? 1 : 0;
    HashMap<Object, Integer> hm = new HashMap<>();
    hm.put(p, 1);
    forgedGet = hm.get(forged) == null ? 0 : 1;
    IdentityHashMap<Object, Integer> im = new IdentityHashMap<>();
    im.put(p, 1);
    forgedIdentity = im.get(forged) == null ? 0 : 1;
    try { forgedSetOf = Set.of(p, forged).size(); } catch (IllegalArgumentException e) { forgedSetOf = -1; }
    forgedSwitch = switch (forged) { case ServerPlayer sp -> 1; default -> 0; };
  }
}
"#;

/// Alex as the façade lists them: entity id `id`, `session` logins and
/// `life` respawns into this one.
fn facade(id: i32, session: u32, life: u32) -> Arc<EventFacade> {
    Arc::new(EventFacade {
        players: vec![FacadePlayer {
            uuid: ALEX,
            name: "Alex".into(),
            level: Some("minecraft:overworld".into()),
            entity_id: Some(id),
            session,
            life,
        }],
        ..EventFacade::default()
    })
}

struct Run<'a> {
    jar: &'a NativeJar,
    host: StdHost,
    listeners: Vec<GameListener>,
}

impl Run<'_> {
    fn fire(&mut self, event: GameEvent, facade: Arc<EventFacade>, context: EventContext) {
        let mut firing = EventFiring::new(event, facade, context);
        for listener in self.listeners.iter().filter(|l| l.event == event) {
            self.jar
                .run_game_listener(
                    listener,
                    &mut firing,
                    &mut self.host,
                    &ExecutionBudget::UNBOUNDED,
                )
                .unwrap_or_else(|error| panic!("{event:?} refused: {error}"));
        }
    }

    /// The static int `field` (0 when the jar never wrote it).
    fn int(&self, field: &str) -> i32 {
        match self
            .host
            .persistent_class_state()
            .0
            .remove(&format!("demo/Probe#{field}I"))
        {
            Some(Value::Int(value)) => value,
            None => 0,
            other => panic!("{field}: {other:?}"),
        }
    }

    /// How many unknown calls the run counted whose name starts with `call`.
    fn counted(&self, call: &str) -> usize {
        self.host
            .unknown_host_calls()
            .iter()
            .filter(|(name, _)| name.starts_with(call))
            .map(|(_, count)| count)
            .sum()
    }

    fn ints(&self, fields: &[&str]) -> Vec<i32> {
        fields.iter().map(|field| self.int(field)).collect()
    }
}

/// The checks the tick listener makes between the last tick's player and
/// this one's, in order.
const PAIR: &[&str] = &[
    "sameEq",
    "sameEquals",
    "hash",
    "hashSet",
    "identityMap",
    "hashMapSize",
    "hashMapGet",
    "listContains",
    "setOf",
];

#[test]
fn players_entities_and_damage_keep_the_servers_identity() {
    let mut sources = STUBS.to_vec();
    sources.push(("demo/Probe.java", MOD));
    let fixture = Fixture::build("game-identity", &sources);
    let jar = &fixture.jar;
    let entry = jar.discover_entrypoint("main").unwrap();
    let mut host = StdHost::new();
    jar.execute(&entry.class, entry.method.as_deref(), "main", &mut host)
        .unwrap();
    let listeners = host.game_listeners(jar, true).listeners;
    let mut run = Run {
        jar,
        host,
        listeners,
    };
    let tick = |run: &mut Run, facade| {
        run.fire(GameEvent::ServerTickEnd, facade, EventContext::default());
    };

    // One player object across two ticks (Java: `same_firings_eq`,
    // `same_firings_equals`; `Set.of(p, p)` throws `IAE`).
    // `ServerLevel.toString` is `ServerLevel[<world folder name>]`, a name
    // the façade does not give: counted, the tick's only counted call.
    let printed = run.counted("java/lang/Object#toString");
    tick(&mut run, facade(7, 1, 0));
    assert_eq!(run.counted("java/lang/Object#toString") - printed, 1);
    tick(&mut run, facade(7, 1, 0));
    assert_eq!(run.ints(PAIR), [1, 1, 7, 1, 1, 1, 2, 1, -1]);

    // A respawn: a new `ServerPlayer` with the old id. `equals`, not `==`;
    // `hashCode` is the id; `HashSet`/`HashMap` see one key, an
    // `IdentityHashMap` two (Java: `respawn_*`).
    let printed = run.counted("java/lang/Object#toString");
    tick(&mut run, facade(7, 1, 1));
    assert_eq!(run.ints(PAIR), [0, 1, 7, 1, 2, 1, 2, 1, -1]);
    // `AFTER_RESPAWN(oldPlayer, newPlayer)`: the old one is the first
    // life's object itself.
    run.fire(
        GameEvent::PlayerRespawn,
        facade(7, 1, 1),
        EventContext {
            player: Some(ALEX),
            ..EventContext::default()
        },
    );
    assert_eq!(
        run.ints(&["oldEq", "oldEquals", "oldIsFirstLife"]),
        [0, 1, 1]
    );

    // A relog: a new player with a new id, not `equals` (Java: `relog_equals`).
    tick(&mut run, facade(9, 2, 0));
    assert_eq!(run.ints(PAIR), [0, 0, 9, 2, 2, 2, 1, 0, 2]);
    // `Set.of`'s duplicate message prints the player, and `Entity.toString`
    // needs its position: counted, once for the respawn's `Set.of`, beside
    // the two ticks' level prints.
    assert_eq!(run.counted("java/lang/Object#toString") - printed, 3);

    // Forgery, every tick: text spelled like a player key stays text
    // (Java: `forged_*`): no `equals`, `==`, collection or map takes it for
    // the player.
    assert_eq!(
        run.ints(&[
            "forgedEquals",
            "equalsForged",
            "forgedEq",
            "forgedSet",
            "forgedContains",
            "forgedGet",
            "forgedIdentity",
            "forgedSetOf",
        ]),
        [0, 0, 0, 2, 0, 0, 0, 2]
    );
    // Its class is not known yet: `game/` text still reads as a marker while
    // `game_hooks` makes it (#156 P16e; P17 drops the head). `instanceof
    // ServerPlayer` and the pattern `switch` answer no, as Java does, but are
    // counted; the cast passes, counted, where Java throws (`forged_cast`).
    assert_eq!(run.ints(&["forgedInstance", "forgedSwitch"]), [0, 0]);
    assert!(run.counted("net/minecraft/server/level/ServerPlayer#<instanceof>") > 0);

    // Two firings that hurt entity 42 with `minecraft:generic`: the entity is
    // one object (`==`, `equals`, `hashCode` 42: Java's `entity_*` give
    // `equals` and the hash for any two objects of one id). The two damage
    // sources may be one object (one hurt's `ALLOW_DAMAGE` and
    // `AFTER_DAMAGE`) or two (Java: `damage_eq`, `damage_equals` false for
    // two): `==` and `equals` are counted, never answered as one object
    // silently. A counted `==` keeps the old value answer
    // (`reference_identity.rs`), so only the counts are pinned.
    let hurt = || EventContext {
        entity: Some(FacadeEntity {
            id: 42,
            kind: "minecraft:zombie".into(),
            level: Some("minecraft:overworld".into()),
        }),
        damage: Some("minecraft:generic".into()),
        amount: 1.0,
        ..EventContext::default()
    };
    run.fire(GameEvent::LivingAllowDamage, facade(9, 2, 0), hurt());
    assert_eq!(run.ints(&["damageSelf", "entityHash"]), [1, 42]);
    let compared = run.counted("java/lang/Object#if_acmp");
    let equals = run.counted("java/lang/Object#equals");
    run.fire(GameEvent::LivingAllowDamage, facade(9, 2, 0), hurt());
    assert_eq!(
        run.ints(&["entityEq", "entityEquals", "entityHash"]),
        [1, 1, 42]
    );
    assert_eq!(run.counted("java/lang/Object#if_acmp") - compared, 1);
    assert_eq!(run.counted("java/lang/Object#equals") - equals, 1);

    // Two firings with no player name none: two absent players are two
    // `game/unknown/player` placeholders, never silently one object: the
    // `==` between them is counted.
    run.fire(
        GameEvent::PlayerRespawn,
        facade(9, 2, 0),
        EventContext::default(),
    );
    let compared = run.counted("java/lang/Object#if_acmp");
    run.fire(
        GameEvent::PlayerRespawn,
        facade(9, 2, 0),
        EventContext::default(),
    );
    assert_eq!(run.counted("java/lang/Object#if_acmp") - compared, 1);
}

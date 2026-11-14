//! The server's values keep Java's text, equality and order as host handles
//! (#156 P16c): a Fabric mod breaks blocks and compares the `BlockPos`es it
//! was handed, reads `LevelResource` and `GameRule` constants, and meets text
//! spelled like their old markers.
//!
//! `java25.txt` is what `Oracle.java` printed on OpenJDK 25.0.5+8 against the
//! real 26.3 server classes (`Bootstrap.bootStrap()`, two runs alike; kept on
//! elysium under `~/ava/results/p16c/`). `fixtures/server_values/demo/Probe.java`
//! asks the same questions under the same names; every line it prints must
//! be Java's. The game stubs only let `javac` compile it and are never packed.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::{
    ExecutionBudget, Host, StdHost, Value,
    events::{EventContext, EventFacade, EventFiring, GameEvent},
};
use std::{collections::BTreeMap, sync::Arc};

const ORACLE: &str = include_str!("fixtures/server_values/java25.txt");
const PROBE: &str = include_str!("fixtures/server_values/demo/Probe.java");

const STUBS: &[(&str, &str)] = &[
    (
        "net/fabricmc/fabric/api/event/Event.java",
        "package net.fabricmc.fabric.api.event; public abstract class Event<T> { public void register(T l) {} }",
    ),
    (
        "net/fabricmc/fabric/api/event/lifecycle/v1/ServerTickEvents.java",
        "package net.fabricmc.fabric.api.event.lifecycle.v1; public final class ServerTickEvents { \
         public static final net.fabricmc.fabric.api.event.Event<EndTick> END_SERVER_TICK = null; \
         public interface EndTick { void onEndTick(net.minecraft.server.MinecraftServer server); } }",
    ),
    (
        "net/fabricmc/fabric/api/event/player/PlayerBlockBreakEvents.java",
        "package net.fabricmc.fabric.api.event.player; public final class PlayerBlockBreakEvents { \
         public static final net.fabricmc.fabric.api.event.Event<After> AFTER = null; \
         public interface After { void afterBlockBreak(net.minecraft.world.level.Level level, \
         net.minecraft.world.entity.player.Player player, net.minecraft.core.BlockPos pos, \
         net.minecraft.world.level.block.state.BlockState state, \
         net.minecraft.world.level.block.entity.BlockEntity entity); } }",
    ),
    (
        "net/minecraft/server/MinecraftServer.java",
        "package net.minecraft.server; public class MinecraftServer {}",
    ),
    (
        "net/minecraft/world/level/Level.java",
        "package net.minecraft.world.level; public class Level {}",
    ),
    (
        "net/minecraft/world/entity/player/Player.java",
        "package net.minecraft.world.entity.player; public class Player {}",
    ),
    (
        "net/minecraft/core/Vec3i.java",
        "package net.minecraft.core; public class Vec3i implements Comparable<Vec3i> { \
         public int getX() { return 0; } public int getY() { return 0; } public int getZ() { return 0; } \
         public int compareTo(Vec3i o) { return 0; } }",
    ),
    (
        "net/minecraft/core/BlockPos.java",
        "package net.minecraft.core; public class BlockPos extends Vec3i { public long asLong() { return 0; } }",
    ),
    (
        "net/minecraft/world/level/block/Block.java",
        "package net.minecraft.world.level.block; public class Block {}",
    ),
    (
        "net/minecraft/world/level/block/state/BlockState.java",
        "package net.minecraft.world.level.block.state; public class BlockState { \
         public net.minecraft.world.level.block.Block getBlock() { return null; } }",
    ),
    (
        "net/minecraft/world/level/block/entity/BlockEntity.java",
        "package net.minecraft.world.level.block.entity; public class BlockEntity {}",
    ),
    (
        "net/minecraft/world/level/gamerules/GameRule.java",
        "package net.minecraft.world.level.gamerules; public final class GameRule<T> {}",
    ),
    (
        "net/minecraft/world/level/gamerules/GameRules.java",
        "package net.minecraft.world.level.gamerules; public class GameRules { \
         public static final GameRule<Boolean> KEEP_INVENTORY = null; }",
    ),
    (
        "net/minecraft/world/level/storage/LevelResource.java",
        "package net.minecraft.world.level.storage; public final class LevelResource { \
         public static final LevelResource DATA = null, ROOT = null, PLAYER_DATA_DIR = null; \
         public LevelResource(String id) {} }",
    ),
];

/// The positions broken, in order: the oracle's five (zero, negative and
/// extreme coordinates), the ones its `compareTo` and `TreeSet` read, and
/// `1,2,3` again, a second object of the first's coordinates.
const BROKEN: &[[i32; 3]] = &[
    [1, 2, 3],
    [0, 0, 0],
    [-1, -2, -3],
    [30_000_000, 319, -30_000_000],
    [-33_554_432, -2048, 33_554_431],
    [1, 2, 4],
    [5, 1, 0],
    [0, 2, 0],
    [2, 2, 2],
    [1, 2, 2],
    [3, 1, 0],
    [0, 1, 5],
    [1, 2, 3],
];

fn statics(host: &StdHost, field: &str) -> String {
    match host
        .persistent_class_state()
        .0
        .remove(&format!("demo/Probe#{field}Ljava/lang/String;"))
    {
        Some(Value::Ref(Some(text))) => text,
        other => panic!("{field}: {other:?}"),
    }
}

#[test]
fn server_values_answer_as_java() {
    let mut sources = STUBS.to_vec();
    sources.push(("demo/Probe.java", PROBE));
    let fixture = javac_fixture::Fixture::build("server-values", &sources);
    let jar = &fixture.jar;
    let entry = jar.discover_entrypoint("main").unwrap();
    let mut host = StdHost::new();
    jar.execute(&entry.class, entry.method.as_deref(), "main", &mut host)
        .unwrap();
    let listeners = host.game_listeners(jar, true).listeners;
    let mut fire = |host: &mut StdHost, event: GameEvent, context: EventContext| {
        let mut firing = EventFiring::new(event, Arc::new(EventFacade::default()), context);
        for listener in listeners.iter().filter(|l| l.event == event) {
            jar.run_game_listener(listener, &mut firing, host, &ExecutionBudget::UNBOUNDED)
                .unwrap_or_else(|error| panic!("{event:?} refused: {error}"));
        }
    };
    for pos in BROKEN {
        let context = EventContext {
            pos: Some(*pos),
            block: Some("minecraft:stone".into()),
            ..EventContext::default()
        };
        fire(&mut host, GameEvent::BlockBreakAfter, context);
    }
    fire(&mut host, GameEvent::ServerTickEnd, EventContext::default());
    assert_eq!(host.unknown_host_calls(), Vec::<(String, usize)>::new());

    let java: BTreeMap<&str, &str> = ORACLE.lines().filter_map(|l| l.split_once('=')).collect();
    let log = statics(&host, "log");
    let mut seen = 0;
    for line in log.lines() {
        let (key, value) = line.split_once('=').unwrap();
        assert_eq!(Some(&value), java.get(key), "{key}");
        seen += 1;
    }
    // Every oracle line but eight: `Item`'s and `Registry`'s text and the
    // forged item (no `Item` producer in a firing; `server_values`' unit
    // test), the two block states' text (their key names no properties:
    // counted), the cast (#167, #236), and the two counted below.
    assert_eq!(seen, java.len() - 8, "{log}");

    // What the model cannot know is counted, never answered as Java's
    // opposite silently: two firings' positions may be one object or two
    // (Java: `pos_eq=false`), and a jar's own `LevelResource` is no handle
    // (Java: `res_equals_new=true`).
    fire(&mut host, GameEvent::ServerTickEnd, EventContext::default());
    let counted: Vec<String> = host
        .unknown_host_calls()
        .into_iter()
        .map(|(c, _)| c)
        .collect();
    assert_eq!(
        counted,
        [
            "java/lang/Object#equals(Ljava/lang/Object;)Z",
            "java/lang/Object#if_acmp(Ljava/lang/Object;Ljava/lang/Object;)Z",
        ]
    );
}

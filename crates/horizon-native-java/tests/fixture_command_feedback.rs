//! `CommandSourceStack.sendSuccess(Supplier, boolean)` as vanilla 26.3 runs it (#193 review N4):
//! the supplier runs inside the call, and only when the feedback goes somewhere
//! (`acceptsSuccess() && !silent`, or a broadcast to operators). A component it answers is sent
//! as it is then: changed afterwards, Java has already sent it.
//!
//! Java 25 on the real server (the reviewer's `rv193` run, `~/ava/results/rv193`): `late` sends
//! `a`, `latestate` sends `n=0`. A player's source accepts success when `sendCommandFeedback` is on.

#[path = "support/scratch.rs"]
mod scratch;

use horizon_native_java::{
    ExecutionBudget, GameRuleValue, NativeJar, ServerFacts, StdHost,
    brigadier::{CommandInvocation, CommandOutcome, CommandSource},
    events::{EventContext, EventFacade, EventFiring, FacadePlayer, GameEvent},
};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
    time::Duration,
};
use zip::write::SimpleFileOptions;

const STUBS: &[(&str, &str)] = &[
    (
        "net/fabricmc/fabric/api/event/Event.java",
        "package net.fabricmc.fabric.api.event;\n\
         public abstract class Event<T> { public void register(T listener) {} }",
    ),
    (
        "net/fabricmc/fabric/api/command/v2/CommandRegistrationCallback.java",
        "package net.fabricmc.fabric.api.command.v2;\n\
         public interface CommandRegistrationCallback {\n\
           net.fabricmc.fabric.api.event.Event<CommandRegistrationCallback> EVENT = null;\n\
           void register(com.mojang.brigadier.CommandDispatcher<net.minecraft.commands.CommandSourceStack> d,\n\
             net.minecraft.commands.CommandBuildContext c, net.minecraft.commands.Commands.CommandSelection s); }",
    ),
    (
        "com/mojang/brigadier/CommandDispatcher.java",
        "package com.mojang.brigadier;\n\
         public class CommandDispatcher<S> {\n\
           public com.mojang.brigadier.tree.LiteralCommandNode<S> register(\n\
             com.mojang.brigadier.builder.LiteralArgumentBuilder<S> b) { return null; } }",
    ),
    (
        "com/mojang/brigadier/Command.java",
        "package com.mojang.brigadier;\n\
         public interface Command<S> { int run(com.mojang.brigadier.context.CommandContext<S> c)\n\
           throws com.mojang.brigadier.exceptions.CommandSyntaxException; }",
    ),
    (
        "com/mojang/brigadier/exceptions/CommandSyntaxException.java",
        "package com.mojang.brigadier.exceptions;\n\
         public class CommandSyntaxException extends Exception {}",
    ),
    (
        "com/mojang/brigadier/context/CommandContext.java",
        "package com.mojang.brigadier.context;\n\
         public class CommandContext<S> { public S getSource() { return null; } }",
    ),
    (
        "com/mojang/brigadier/tree/LiteralCommandNode.java",
        "package com.mojang.brigadier.tree;\n\
         public class LiteralCommandNode<S> {}",
    ),
    (
        "com/mojang/brigadier/builder/ArgumentBuilder.java",
        "package com.mojang.brigadier.builder;\n\
         public abstract class ArgumentBuilder<S, T extends ArgumentBuilder<S, T>> {\n\
           public T then(ArgumentBuilder<S, ?> a) { return null; }\n\
           public T executes(com.mojang.brigadier.Command<S> c) { return null; }\n\
           public T requires(java.util.function.Predicate<S> p) { return null; } }",
    ),
    (
        "com/mojang/brigadier/builder/LiteralArgumentBuilder.java",
        "package com.mojang.brigadier.builder;\n\
         public class LiteralArgumentBuilder<S> extends ArgumentBuilder<S, LiteralArgumentBuilder<S>> {}",
    ),
    (
        "com/mojang/brigadier/builder/RequiredArgumentBuilder.java",
        "package com.mojang.brigadier.builder;\n\
         public class RequiredArgumentBuilder<S, T> extends ArgumentBuilder<S, RequiredArgumentBuilder<S, T>> {}",
    ),
    (
        "com/mojang/brigadier/arguments/ArgumentType.java",
        "package com.mojang.brigadier.arguments;\n\
         public interface ArgumentType<T> {}",
    ),
    (
        "com/mojang/brigadier/arguments/IntegerArgumentType.java",
        "package com.mojang.brigadier.arguments;\n\
         public class IntegerArgumentType implements ArgumentType<Integer> {\n\
           public static IntegerArgumentType integer(int min, int max) { return null; }\n\
           public static int getInteger(com.mojang.brigadier.context.CommandContext<?> c, String n) { return 0; } }",
    ),
    (
        "com/mojang/brigadier/arguments/StringArgumentType.java",
        "package com.mojang.brigadier.arguments;\n\
         public class StringArgumentType implements ArgumentType<String> {\n\
           public static StringArgumentType word() { return null; }\n\
           public static String getString(com.mojang.brigadier.context.CommandContext<?> c, String n) { return null; } }",
    ),
    (
        "net/minecraft/commands/CommandBuildContext.java",
        "package net.minecraft.commands;\npublic interface CommandBuildContext {}",
    ),
    (
        "net/minecraft/commands/Commands.java",
        "package net.minecraft.commands;\n\
         import com.mojang.brigadier.builder.*;\n\
         import net.minecraft.server.permissions.*;\n\
         public class Commands {\n\
           public static final PermissionCheck LEVEL_GAMEMASTERS = null;\n\
           public enum CommandSelection { ALL, DEDICATED, INTEGRATED }\n\
           public static LiteralArgumentBuilder<CommandSourceStack> literal(String s) { return null; }\n\
           public static <T> RequiredArgumentBuilder<CommandSourceStack, T> argument(String s,\n\
             com.mojang.brigadier.arguments.ArgumentType<T> t) { return null; }\n\
           public static <T> PermissionProviderCheck<T> hasPermission(PermissionCheck c) { return null; } }",
    ),
    (
        "net/minecraft/commands/CommandSourceStack.java",
        "package net.minecraft.commands;\n\
         public class CommandSourceStack {\n\
           public void sendSuccess(java.util.function.Supplier<net.minecraft.network.chat.Component> s, boolean b) {}\n\
           public void sendSystemMessage(net.minecraft.network.chat.Component c) {}\n\
           public net.minecraft.server.permissions.PermissionSet permissions() { return null; }\n\
           public net.minecraft.server.level.ServerPlayer getPlayerOrException()\n\
             throws com.mojang.brigadier.exceptions.CommandSyntaxException { return null; }\n\
           public Object registryAccess() { return null; } }",
    ),
    (
        "net/minecraft/server/permissions/PermissionCheck.java",
        "package net.minecraft.server.permissions;\npublic interface PermissionCheck {}",
    ),
    (
        "net/minecraft/server/permissions/PermissionProviderCheck.java",
        "package net.minecraft.server.permissions;\n\
         public final class PermissionProviderCheck<T> implements java.util.function.Predicate<T> {\n\
           public boolean test(T t) { return false; } }",
    ),
    (
        "net/minecraft/server/permissions/Permission.java",
        "package net.minecraft.server.permissions;\npublic interface Permission {}",
    ),
    (
        "net/minecraft/server/permissions/Permissions.java",
        "package net.minecraft.server.permissions;\n\
         public class Permissions { public static final Permission COMMANDS_MODERATOR = null; }",
    ),
    (
        "net/minecraft/server/permissions/PermissionSet.java",
        "package net.minecraft.server.permissions;\n\
         public interface PermissionSet { boolean hasPermission(Permission p); }",
    ),
    (
        "net/minecraft/network/chat/Component.java",
        "package net.minecraft.network.chat;\n\
         public interface Component { static MutableComponent literal(String s) { return null; }\n\
           static MutableComponent translatable(String k, Object... args) { return null; } }",
    ),
    (
        "net/minecraft/network/chat/MutableComponent.java",
        "package net.minecraft.network.chat;\n\
         public class MutableComponent implements Component {\n\
           public MutableComponent withStyle(net.minecraft.ChatFormatting f) { return this; }\n\
           public MutableComponent append(Component c) { return this; }\n\
           public MutableComponent append(String s) { return this; } }",
    ),
    (
        "net/minecraft/ChatFormatting.java",
        "package net.minecraft;\npublic enum ChatFormatting { AQUA, GOLD, BOLD }",
    ),
    (
        "net/minecraft/world/entity/Entity.java",
        "package net.minecraft.world.entity;\n\
         public class Entity { public int getId() { return 0; } }",
    ),
    (
        "net/minecraft/server/level/ServerPlayer.java",
        "package net.minecraft.server.level;\n\
         public class ServerPlayer extends net.minecraft.world.entity.Entity {\n\
           public String getScoreboardName() { return null; } }",
    ),
    (
        "net/minecraft/commands/arguments/EntityArgument.java",
        "package net.minecraft.commands.arguments;\n\
         import com.mojang.brigadier.context.CommandContext;\n\
         import com.mojang.brigadier.exceptions.CommandSyntaxException;\n\
         import net.minecraft.commands.CommandSourceStack;\n\
         import net.minecraft.server.level.ServerPlayer;\n\
         import net.minecraft.world.entity.Entity;\n\
         import java.util.Collection;\n\
         public class EntityArgument implements com.mojang.brigadier.arguments.ArgumentType<Object> {\n\
           public static EntityArgument entity() { return null; }\n\
           public static EntityArgument entities() { return null; }\n\
           public static EntityArgument player() { return null; }\n\
           public static EntityArgument players() { return null; }\n\
           public static Entity getEntity(CommandContext<CommandSourceStack> c, String n) throws CommandSyntaxException { return null; }\n\
           public static Collection<? extends Entity> getEntities(CommandContext<CommandSourceStack> c, String n) throws CommandSyntaxException { return null; }\n\
           public static Collection<? extends Entity> getOptionalEntities(CommandContext<CommandSourceStack> c, String n) throws CommandSyntaxException { return null; }\n\
           public static ServerPlayer getPlayer(CommandContext<CommandSourceStack> c, String n) throws CommandSyntaxException { return null; }\n\
           public static Collection<ServerPlayer> getPlayers(CommandContext<CommandSourceStack> c, String n) throws CommandSyntaxException { return null; }\n\
           public static Collection<ServerPlayer> getOptionalPlayers(CommandContext<CommandSourceStack> c, String n) throws CommandSyntaxException { return null; } }",
    ),
];

/// `/greet <count> <name>` (operators, a level check), `/whoami` (a `requires`
/// lambda at moderator level) and `/boom`, whose body reaches past the façade.
const MOD: &str = r#"
package demo;
import net.fabricmc.fabric.api.command.v2.CommandRegistrationCallback;
import net.minecraft.commands.Commands;
import net.minecraft.network.chat.Component;
import net.minecraft.network.chat.MutableComponent;
public final class CmdMod {
  public void onInitialize() {
    CommandRegistrationCallback.EVENT.register((dispatcher, context, selection) -> {
      dispatcher.register(Commands.literal("late").executes(ctx -> {
        MutableComponent c = Component.literal("a");
        ctx.getSource().sendSuccess(() -> c, false);
        c.append("b");
        return 1; }));
      dispatcher.register(Commands.literal("latestate").executes(ctx -> {
        int[] n = {0};
        ctx.getSource().sendSuccess(() -> Component.literal("n=" + n[0]), false);
        n[0] = 5;
        return 1; }));
      dispatcher.register(Commands.literal("calls").executes(ctx -> {
        int[] n = {0};
        ctx.getSource().sendSuccess(() -> { n[0]++; return Component.literal("x"); }, false);
        ctx.getSource().sendSystemMessage(Component.literal("n=" + n[0]));
        return 1; }));
    });
  }
}
"#;

fn scratch(name: &str) -> scratch::Scratch {
    let root = scratch::Scratch::new(&format!("brigadier-{name}"));
    fs::create_dir_all(&root).unwrap();
    root
}

/// The mod compiled against the stubs, its own classes packed with a
/// `fabric.mod.json`. `None` without a Java toolchain.
fn build_jar(root: &Path) -> Option<PathBuf> {
    let src = root.join("src");
    let classes = root.join("classes");
    let mut paths = Vec::new();
    for (path, text) in STUBS.iter().chain([("demo/CmdMod.java", MOD)].iter()) {
        let file = src.join(path);
        fs::create_dir_all(file.parent()?).ok()?;
        fs::write(&file, text).ok()?;
        paths.push(file);
    }
    fs::create_dir_all(&classes).ok()?;
    let javac = std::env::var_os("JAVA_HOME")
        .map(|home| PathBuf::from(home).join("bin/javac"))
        .unwrap_or_else(|| "javac".into());
    let status = Command::new(javac)
        .args(["--release", "17", "-d"])
        .arg(&classes)
        .args(&paths)
        .status()
        .ok()?;
    assert!(status.success(), "javac failed");
    let jar_path = root.join("mod.jar");
    let mut zip = zip::ZipWriter::new(fs::File::create(&jar_path).ok()?);
    zip.start_file("fabric.mod.json", SimpleFileOptions::default())
        .ok()?;
    zip.write_all(br#"{"id":"cmd_mod","version":"1.0","entrypoints":{"main":["demo.CmdMod"]}}"#)
        .ok()?;
    for entry in fs::read_dir(classes.join("demo")).ok()? {
        let path = entry.ok()?.path();
        let name = format!("demo/{}", path.file_name()?.to_string_lossy());
        zip.start_file(name, SimpleFileOptions::default()).ok()?;
        zip.write_all(&fs::read(&path).ok()?).ok()?;
    }
    zip.finish().ok()?;
    Some(jar_path)
}

fn budget() -> ExecutionBudget {
    ExecutionBudget::within(Duration::from_secs(5))
}

const ANA: u128 = 0x0123_4567_89ab_cdef_0123_4567_89ab_cdef;

fn facade() -> Arc<EventFacade> {
    facade_with(None)
}

/// The facade, with the `sendCommandFeedback` rule when `feedback` is set.
fn facade_with(feedback: Option<bool>) -> Arc<EventFacade> {
    let mut server = ServerFacts::default();
    if let Some(on) = feedback {
        server
            .game_rules
            .insert("send_command_feedback".into(), GameRuleValue::Bool(on));
    }
    Arc::new(EventFacade {
        server: Arc::new(server),
        dedicated: true,
        players: vec![
            FacadePlayer {
                uuid: ANA,
                name: "Ana".into(),
                level: Some("minecraft:overworld".into()),
                entity_id: Some(7),
                session: 0,
                life: 0,
            },
            FacadePlayer {
                uuid: BOB,
                name: "Bob".into(),
                level: Some("minecraft:overworld".into()),
                entity_id: Some(8),
                session: 0,
                life: 0,
            },
        ],
        ..EventFacade::default()
    })
}

const BOB: u128 = 0x0fed_cba9_8765_4321_0fed_cba9_8765_4321;

fn source(level: u8, player: Option<u128>) -> CommandSource {
    CommandSource {
        name: if player.is_some() { "Ana" } else { "Server" }.into(),
        level,
        player,
        dimension: Some("minecraft:overworld".into()),
    }
}

#[test]
fn a_success_supplier_runs_inside_the_call_and_only_when_feedback_goes_somewhere() {
    let root = scratch("command-feedback");
    let Some(jar_path) = build_jar(&root) else {
        return;
    };
    let jar = NativeJar::from_path(&jar_path).unwrap();
    let entry = jar.discover_entrypoint("main").unwrap();
    let mut host = StdHost::new();
    jar.execute(&entry.class, entry.method.as_deref(), "main", &mut host)
        .unwrap();
    let listeners: Vec<_> = host
        .game_listeners(&jar, true)
        .listeners
        .into_iter()
        .filter(|l| l.event == GameEvent::RegisterCommands)
        .collect();
    let mut firing = EventFiring::new(
        GameEvent::RegisterCommands,
        facade(),
        EventContext::default(),
    );
    jar.run_game_listener(&listeners[0], &mut firing, &mut host, &budget())
        .unwrap();
    let commands: Vec<(String, horizon_native_java::Value)> = host
        .command_tree()
        .roots()
        .map(|(_, n)| (n.kind.name().to_owned(), n.command.clone().unwrap()))
        .collect();
    let mut run = |name: &str, player: Option<u128>, feedback: Option<bool>| {
        let command = &commands.iter().find(|(n, _)| n == name).unwrap().1;
        let before = host.unknown_host_call_count();
        let run = jar.run_command(
            command,
            &CommandInvocation {
                input: name.into(),
                source: source(4, player),
                arguments: Vec::new(),
            },
            facade_with(feedback),
            &mut host,
            &budget(),
        );
        let texts: Vec<String> = run.messages.iter().map(|m| m.text.text.clone()).collect();
        (run.outcome, texts, host.unknown_host_call_count() - before)
    };
    // The console accepts success: the supplier runs at the call (Java `a`, `n=0`).
    assert_eq!(
        run("late", None, None),
        (CommandOutcome::Returned(1), vec!["a".to_owned()], 0)
    );
    assert_eq!(
        run("latestate", None, None),
        (CommandOutcome::Returned(1), vec!["n=0".to_owned()], 0)
    );
    // A player with `sendCommandFeedback` off: the supplier never runs (`n=0`, no `x`).
    assert_eq!(
        run("calls", Some(ANA), Some(false)),
        (CommandOutcome::Returned(1), vec!["n=0".to_owned()], 0)
    );
    // ... and with it on, it runs before the next statement.
    assert_eq!(
        run("calls", Some(ANA), Some(true)),
        (
            CommandOutcome::Returned(1),
            vec!["x".to_owned(), "n=1".to_owned()],
            0
        )
    );
    // An unknown rule is counted, never guessed (a running command refuses it).
    let (outcome, _, counted) = run("calls", Some(ANA), None);
    assert!(
        counted > 0 && outcome != CommandOutcome::Returned(1),
        "{outcome:?}"
    );
}

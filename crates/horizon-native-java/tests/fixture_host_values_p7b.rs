//! The command surface's objects are typed handles (#156 P7b): Brigadier's
//! dispatcher, builders, nodes and argument types, a running command's
//! `CommandContext` and `CommandSourceStack`, and the permission values.
//!
//! `fixtures/host_values/p7b/demo/*.java` is compiled against the stubs beside
//! it (the libraries' signatures, left out of the jar so every call reaches the
//! runtime's models). Every expected string is what the same `demo` classes
//! printed on Java 25 (OpenJDK 25.0.5, `javac --release 21`, two runs alike)
//! against brigadier 1.3.11 and Minecraft 26.3's server classes:
//! `demo/Main.java` is that run's driver. It fires `Checks.fired` with a real
//! `CommandDispatcher`, runs `/probe`, `/rvarg 5`, `/rvforge 7` and `/rvreq` as
//! a game master console source, then fires it once per check. Here the jar
//! registers through Fabric's callback, the server's command-tree build fires
//! it, and the game runs the commands the same way (`/rvreq`'s `requires` and
//! command in one `CommandDispatch`). Each check is either answered as Java
//! 25 does, uncounted, or counted with exactly the calls named.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::{
    ExecutionBudget, StdHost,
    brigadier::{CommandArgument, CommandInvocation, CommandOutcome, CommandRun, CommandSource},
    events::{EventContext, EventFacade, EventFiring, GameEvent, GameListener},
};
use javac_fixture::Fixture;
use std::{collections::BTreeMap, sync::Arc, time::Duration};

const SOURCES: &[(&str, &str)] = &[
    (
        "demo/Checks.java",
        include_str!("fixtures/host_values/p7b/demo/Checks.java"),
    ),
    (
        "demo/Forgery.java",
        include_str!("fixtures/host_values/p7b/demo/Forgery.java"),
    ),
    (
        "demo/Probe.java",
        include_str!("fixtures/host_values/p7b/demo/Probe.java"),
    ),
    (
        "com/mojang/brigadier/Command.java",
        include_str!("fixtures/host_values/p7b/stubs/com/mojang/brigadier/Command.java"),
    ),
    (
        "com/mojang/brigadier/CommandDispatcher.java",
        include_str!("fixtures/host_values/p7b/stubs/com/mojang/brigadier/CommandDispatcher.java"),
    ),
    (
        "com/mojang/brigadier/RedirectModifier.java",
        include_str!("fixtures/host_values/p7b/stubs/com/mojang/brigadier/RedirectModifier.java"),
    ),
    (
        "com/mojang/brigadier/SingleRedirectModifier.java",
        include_str!(
            "fixtures/host_values/p7b/stubs/com/mojang/brigadier/SingleRedirectModifier.java"
        ),
    ),
    (
        "com/mojang/brigadier/arguments/ArgumentType.java",
        include_str!(
            "fixtures/host_values/p7b/stubs/com/mojang/brigadier/arguments/ArgumentType.java"
        ),
    ),
    (
        "com/mojang/brigadier/arguments/IntegerArgumentType.java",
        include_str!(
            "fixtures/host_values/p7b/stubs/com/mojang/brigadier/arguments/IntegerArgumentType.java"
        ),
    ),
    (
        "com/mojang/brigadier/arguments/StringArgumentType.java",
        include_str!(
            "fixtures/host_values/p7b/stubs/com/mojang/brigadier/arguments/StringArgumentType.java"
        ),
    ),
    (
        "com/mojang/brigadier/builder/ArgumentBuilder.java",
        include_str!(
            "fixtures/host_values/p7b/stubs/com/mojang/brigadier/builder/ArgumentBuilder.java"
        ),
    ),
    (
        "com/mojang/brigadier/builder/LiteralArgumentBuilder.java",
        include_str!(
            "fixtures/host_values/p7b/stubs/com/mojang/brigadier/builder/LiteralArgumentBuilder.java"
        ),
    ),
    (
        "com/mojang/brigadier/builder/RequiredArgumentBuilder.java",
        include_str!(
            "fixtures/host_values/p7b/stubs/com/mojang/brigadier/builder/RequiredArgumentBuilder.java"
        ),
    ),
    (
        "com/mojang/brigadier/context/CommandContext.java",
        include_str!(
            "fixtures/host_values/p7b/stubs/com/mojang/brigadier/context/CommandContext.java"
        ),
    ),
    (
        "com/mojang/brigadier/exceptions/CommandSyntaxException.java",
        include_str!(
            "fixtures/host_values/p7b/stubs/com/mojang/brigadier/exceptions/CommandSyntaxException.java"
        ),
    ),
    (
        "com/mojang/brigadier/tree/ArgumentCommandNode.java",
        include_str!(
            "fixtures/host_values/p7b/stubs/com/mojang/brigadier/tree/ArgumentCommandNode.java"
        ),
    ),
    (
        "com/mojang/brigadier/tree/CommandNode.java",
        include_str!("fixtures/host_values/p7b/stubs/com/mojang/brigadier/tree/CommandNode.java"),
    ),
    (
        "com/mojang/brigadier/tree/LiteralCommandNode.java",
        include_str!(
            "fixtures/host_values/p7b/stubs/com/mojang/brigadier/tree/LiteralCommandNode.java"
        ),
    ),
    (
        "com/mojang/brigadier/tree/RootCommandNode.java",
        include_str!(
            "fixtures/host_values/p7b/stubs/com/mojang/brigadier/tree/RootCommandNode.java"
        ),
    ),
    (
        "net/fabricmc/fabric/api/command/v2/CommandRegistrationCallback.java",
        include_str!(
            "fixtures/host_values/p7b/stubs/net/fabricmc/fabric/api/command/v2/CommandRegistrationCallback.java"
        ),
    ),
    (
        "net/fabricmc/fabric/api/event/Event.java",
        include_str!("fixtures/host_values/p7b/stubs/net/fabricmc/fabric/api/event/Event.java"),
    ),
    (
        "net/minecraft/commands/CommandBuildContext.java",
        include_str!(
            "fixtures/host_values/p7b/stubs/net/minecraft/commands/CommandBuildContext.java"
        ),
    ),
    (
        "net/minecraft/commands/CommandSourceStack.java",
        include_str!(
            "fixtures/host_values/p7b/stubs/net/minecraft/commands/CommandSourceStack.java"
        ),
    ),
    (
        "net/minecraft/commands/Commands.java",
        include_str!("fixtures/host_values/p7b/stubs/net/minecraft/commands/Commands.java"),
    ),
    (
        "net/minecraft/commands/SharedSuggestionProvider.java",
        include_str!(
            "fixtures/host_values/p7b/stubs/net/minecraft/commands/SharedSuggestionProvider.java"
        ),
    ),
    (
        "net/minecraft/server/permissions/Permission.java",
        include_str!(
            "fixtures/host_values/p7b/stubs/net/minecraft/server/permissions/Permission.java"
        ),
    ),
    (
        "net/minecraft/server/permissions/PermissionCheck.java",
        include_str!(
            "fixtures/host_values/p7b/stubs/net/minecraft/server/permissions/PermissionCheck.java"
        ),
    ),
    (
        "net/minecraft/server/permissions/PermissionProviderCheck.java",
        include_str!(
            "fixtures/host_values/p7b/stubs/net/minecraft/server/permissions/PermissionProviderCheck.java"
        ),
    ),
    (
        "net/minecraft/server/permissions/PermissionSet.java",
        include_str!(
            "fixtures/host_values/p7b/stubs/net/minecraft/server/permissions/PermissionSet.java"
        ),
    ),
    (
        "net/minecraft/server/permissions/PermissionSetSupplier.java",
        include_str!(
            "fixtures/host_values/p7b/stubs/net/minecraft/server/permissions/PermissionSetSupplier.java"
        ),
    ),
    (
        "net/minecraft/server/permissions/Permissions.java",
        include_str!(
            "fixtures/host_values/p7b/stubs/net/minecraft/server/permissions/Permissions.java"
        ),
    ),
];

fn budget() -> ExecutionBudget {
    ExecutionBudget::within(Duration::from_secs(5))
}

/// The game master console `/probe` and the others run as.
fn console() -> CommandSource {
    CommandSource {
        name: "Server".into(),
        level: 2,
        player: None,
        dimension: Some("minecraft:overworld".into()),
    }
}

fn facade() -> Arc<EventFacade> {
    Arc::new(EventFacade {
        dedicated: true,
        ..EventFacade::default()
    })
}

/// The jar after its entrypoint ran, the server built the command tree and a
/// game master console ran `/probe`, as `demo/Main.java` does on Java 25.
/// Each later firing of the registration callback runs the next check
/// (`Checks.NAMES`) against the statics the first one and `/probe` left, and
/// prints `name=answer`.
struct Server<'a> {
    fixture: &'a Fixture,
    host: StdHost,
    listener: GameListener,
}

impl<'a> Server<'a> {
    fn start(fixture: &'a Fixture) -> Self {
        let jar = &fixture.jar;
        let entry = jar.discover_entrypoint("main").expect("an entrypoint");
        let mut host = StdHost::new();
        jar.execute(&entry.class, entry.method.as_deref(), "main", &mut host)
            .expect("the entrypoint ran");
        let listener = host
            .game_listeners(jar, true)
            .listeners
            .into_iter()
            .find(|listener| listener.event == GameEvent::RegisterCommands)
            .expect("a registration callback");
        let mut server = Self {
            fixture,
            host,
            listener,
        };
        server.fire().expect("the callback ran");
        let tree = server.host.command_tree();
        let (_, probe) = tree
            .roots()
            .find(|(_, node)| node.kind.name() == "probe")
            .expect("/probe");
        let command = probe.command.clone().expect("/probe executes");
        let run = server.run(&command, "probe", Vec::new());
        assert_eq!(run.outcome, CommandOutcome::Returned(7), "{run:?}");
        // `/rvarg 5` keeps its context, then `/rvforge 7` reads others.
        for (name, n) in [("rvarg", 5), ("rvforge", 7)] {
            let tree = server.host.command_tree();
            let (_, root) = tree
                .roots()
                .find(|(_, node)| node.kind.name() == name)
                .expect("registered");
            let command = tree.nodes[root.children[0]]
                .command
                .clone()
                .expect("executes");
            let run = server.run(
                &command,
                &format!("{name} {n}"),
                vec![("n".into(), CommandArgument::Int(n))],
            );
            assert_eq!(run.outcome, CommandOutcome::Returned(1), "{name}: {run:?}");
        }
        // `/rvreq` as Brigadier dispatches it: its `requires` and its
        // command in one dispatch, with one source.
        let tree = server.host.command_tree();
        let (_, rvreq) = tree
            .roots()
            .find(|(_, node)| node.kind.name() == "rvreq")
            .expect("/rvreq");
        let (requirement, command) = (
            rvreq.requirement.clone().expect("requires"),
            rvreq.command.clone().expect("executes"),
        );
        let dispatch = server.host.command_dispatch();
        let allowed = jar
            .check_requirement_in(
                dispatch,
                &requirement,
                &console(),
                facade(),
                &mut server.host,
                &budget(),
            )
            .expect("the predicate ran");
        assert!(allowed);
        let run = jar.run_command_in(
            dispatch,
            &command,
            &CommandInvocation {
                input: "rvreq".into(),
                source: console(),
                arguments: Vec::new(),
            },
            facade(),
            &mut server.host,
            &budget(),
        );
        assert_eq!(run.outcome, CommandOutcome::Returned(1), "{run:?}");
        server
    }

    /// Run `command` as a game master console, a dispatch of its own.
    fn run(
        &mut self,
        command: &horizon_native_java::Value,
        input: &str,
        arguments: Vec<(String, CommandArgument)>,
    ) -> CommandRun {
        self.fixture.jar.run_command(
            command,
            &CommandInvocation {
                input: input.into(),
                source: console(),
                arguments,
            },
            facade(),
            &mut self.host,
            &budget(),
        )
    }

    fn fire(&mut self) -> Result<(), horizon_native_java::NativeError> {
        let mut firing = EventFiring::new(
            GameEvent::RegisterCommands,
            facade(),
            EventContext::default(),
        );
        self.fixture
            .jar
            .run_game_listener(&self.listener, &mut firing, &mut self.host, &budget())
            .map(|_| ())
    }

    /// Fire once more for check `name`: its answer and the calls it counted.
    /// A counted call that cannot go on (`getClass`, which needs the exact
    /// class) refuses the firing: the answer is then that refusal.
    fn check(&mut self, name: &str) -> (String, Vec<(String, usize)>) {
        let before: BTreeMap<String, usize> = self.host.unknown_host_calls().into_iter().collect();
        self.host.take_stdout();
        let fired = self.fire();
        let printed = self.host.take_stdout();
        let answer = match fired {
            Ok(()) => printed
                .strip_prefix(name)
                .and_then(|rest| rest.strip_prefix('='))
                .and_then(|rest| rest.strip_suffix('\n'))
                .unwrap_or_else(|| panic!("{name}: printed {printed:?}"))
                .to_owned(),
            Err(error) => format!("refused: {error}"),
        };
        let counted = self
            .host
            .unknown_host_calls()
            .into_iter()
            .filter_map(|(call, count)| {
                let new = count - before.get(&call).copied().unwrap_or(0);
                (new > 0).then_some((call, new))
            })
            .collect();
        (answer, counted)
    }
}

/// `(check, Java 25's answer)`: answered exactly, nothing counted.
const EXACT: &[(&str, &str)] = &[
    // What `/probe` saw: `getSource()` twice is one object (`==`, `equals`),
    // a `CommandSourceStack` and no `String`; the context equals itself only;
    // the source's permission set (a game master's) answers its checks.
    (
        "ran",
        "2truetruefalsefalsetruetruetruefalsefalsetrueprobe,truetruefalsefalse",
    ),
    // Nodes: `getRoot()` twice is one node; `toString` of the root and of
    // literals (one spelled like a node handle's text form); a node equals
    // itself and not a node of another kind or literal, a string or null;
    // `compareTo` puts literals first, then by name (directly, in
    // `Collections.sort`, in a `TreeSet`); `instanceof` their classes.
    (
        "nodes",
        "true<root><literal a,b><literal hbrnode:i0,rroot,r>a,btruetruefalsefalsefalsefalsefalsefalse,-77-11-11,truetruetruetruetruefalse,a,b|hbrnode:i0,rroot,r||n|3a,b",
    ),
    // Builders: two `literal("x")` are two objects; `then` answers the
    // builder itself; their classes; `getLiteral`/`getName`.
    ("builders", "falsefalsetruetruetruetruetruefalsefalsexw"),
    // An argument type is its factory's class and an `ArgumentType`.
    ("argumentTypes", "truetruetruefalsefalsefalsefalse"),
    // A level check, a permission, a `requires` predicate: their interfaces.
    ("permissions", "truetruetruefalsefalsefalse"),
    // Strings spelled like the old markers (`brigadier/node/0`,
    // `brigadier/requires/0`...) and like the handles' text forms stay
    // Strings: `instanceof String`, `getClass`, no command type, `equals`,
    // `hashCode`, concat, `HashMap` keys.
    (
        "spelled",
        "truejava.lang.Stringfalsefalsefalsefalsefalsefalsefalsefalsetrue54473928120,truejava.lang.Stringfalsefalsefalsefalsefalsefalsefalsefalsetrue125400368623,truejava.lang.Stringfalsefalsefalsefalsefalsefalsefalsefalsetrue88181165317,truejava.lang.Stringfalsefalsefalsefalsefalsefalsefalsefalsetrue-41497116218,truejava.lang.Stringfalsefalsefalsefalsefalsefalsefalsefalsetrue200562010916,truejava.lang.Stringfalsefalsefalsefalsefalsefalsefalsefalsetrue31878152219,truejava.lang.Stringfalsefalsefalsefalsefalsefalsefalsefalsetrue-33349272919,truejava.lang.Stringfalsefalsefalsefalsefalsefalsefalsefalsetrue190755308120,truejava.lang.Stringfalsefalsefalsefalsefalsefalsefalsefalsetrue160857462022,truejava.lang.Stringfalsefalsefalsefalsefalsefalsefalsefalsetrue35110043928,truejava.lang.Stringfalsefalsefalsefalsefalsefalsefalsefalsetrue56925438726,truejava.lang.Stringfalsefalsefalsefalsefalsefalsefalsefalsetrue990784145,truejava.lang.Stringfalsefalsefalsefalsefalsefalsefalsefalsetrue-60724631518,truejava.lang.Stringfalsefalsefalsefalsefalsefalsefalsefalsetrue-155503949012,truejava.lang.Stringfalsefalsefalsefalsefalsefalsefalsefalsetrue165091034120,2dn",
    ),
    // One object read twice and two made apart, on every identity path:
    // `IdentityHashMap`, `==`, `List.contains`, `Set.of`, `Objects.equals`.
    ("identities", "437truefalsetruetrue2falsetrue"),
    // Type switches, `Class.isInstance`, natural order (`List.sort(null)`,
    // `sorted()`, `reverseOrder`, a `TreeMap`) and host collections.
    (
        "generic",
        "literal:gen;root;builder;string;truefalsetruefalse,gen[gen, ][, gen]gen,[<literal gen>, <root>]1falsetrue",
    ),
    // A literal builder is no `RequiredArgumentBuilder`, and an integer
    // argument type no `StringArgumentType` (sibling classes).
    ("builderAsOther", "false"),
    ("argumentTypeAsOther", "false"),
    // Two builders made apart in a `HashSet` and as `HashMap` keys: two.
    ("hashed", "22true2"),
    // `/rvreq` in one dispatch: its `requires` predicate and its command see
    // one source object (`==`, `equals`), the predicate run once.
    ("required", "truetrue1"),
    // `/rvforge 7`: `getInteger` on a String forged as a context and on a
    // plain String throw (Java's cast); on the context `/rvarg 5` kept it
    // reads that run's `5`; `hasPermission(LEVEL_ALL).test` on a forged
    // source throws; on its own context it reads `7`.
    ("forged", "cce,cce,5,cce,7"),
    // `getArgument` on the kept context: the value as `Integer`, `int` and
    // `Number`; another class is brigadier's `IllegalArgumentException`, a
    // missing name too; `getInput` of its own run.
    (
        "argClass",
        "55,5,iae:Argument 'n' is defined as Integer, not class java.lang.String,iae:No such argument 'zz' exists on this command,rvarg 5",
    ),
    // `isFork`: a `redirect` with a modifier is not a fork, a `fork` is.
    ("forks", "falsetruefalse"),
    // A source kept past its run still answers for it.
    ("keptSourceName", "Server"),
    // A string cast to a builder, a predicate, a node, a permission set or a
    // source throws Java's `ClassCastException` at the cast: a `String` is no
    // Brigadier, game or JDK object (#167).
    ("used", "cceccecceccecce"),
];

/// `(check, Java 25's answer, the calls counted instead)`.
#[allow(clippy::type_complexity)]
const COUNTED: &[(&str, &str, &[(&str, usize)])] = &[
    // Two nodes of one literal, no children, no command: `CommandNode.equals`
    // compares their children and commands.
    (
        "nodeTwins",
        "true",
        &[("java/lang/Object#equals(Ljava/lang/Object;)Z", 1)],
    ),
    // `LiteralCommandNode.hashCode` hashes the children and the command.
    ("nodeHash", "3224", &[("java/lang/Object#hashCode()I", 1)]),
    // `ArgumentCommandNode.toString` prints its type's `toString`.
    (
        "argumentNodeText",
        "<argument n:integer(1, 9)>",
        &[("java/lang/Object#toString()Ljava/lang/String;", 1)],
    ),
    // `getClass()` needs #174's exact class surface (counted, then null).
    (
        "nodeClass",
        "com.mojang.brigadier.tree.LiteralCommandNode",
        &[("java/lang/Object#getClass()Ljava/lang/Class;", 1)],
    ),
    // `IntegerArgumentType`'s `equals`, `toString`, `hashCode` read its bounds.
    (
        "argumentTypeState",
        "trueinteger(1, 9)40",
        &[
            ("java/lang/Object#equals(Ljava/lang/Object;)Z", 1),
            ("java/lang/Object#hashCode()I", 1),
            ("java/lang/Object#toString()Ljava/lang/String;", 1),
        ],
    ),
    // A constant read twice: one object on Java, two handles here (several
    // constants share a level, so the kind is not canonical).
    (
        "permissionTwice",
        "truetrue",
        &[
            (
                "java/lang/Object#if_acmp(Ljava/lang/Object;Ljava/lang/Object;)Z",
                1,
            ),
            (
                "net/minecraft/server/permissions/PermissionCheck#equals(Ljava/lang/Object;)Z",
                1,
            ),
        ],
    ),
    (
        "sourceClass",
        "net.minecraft.commands.CommandSourceStack",
        &[("java/lang/Object#getClass()Ljava/lang/Class;", 1)],
    ),
];

#[test]
fn no_string_spells_a_command_object_and_they_answer_as_java_25() {
    let fixture = Fixture::build("p7b", SOURCES);
    let mut server = Server::start(&fixture);
    let mut wrong = Vec::new();
    // `Checks.NAMES`' order: the exact checks, then the counted ones.
    for (name, expected) in EXACT {
        let (value, unknown) = server.check(name);
        if value != *expected || !unknown.is_empty() {
            wrong.push(format!("{name}: {value:?} {unknown:?}"));
        }
    }
    for (name, java, calls) in COUNTED {
        let (value, unknown) = server.check(name);
        let expected = calls
            .iter()
            .map(|(call, count)| ((*call).to_owned(), *count))
            .collect::<Vec<_>>();
        if unknown != expected {
            wrong.push(format!("{name} (Java 25: {java}): {value:?} {unknown:?}"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

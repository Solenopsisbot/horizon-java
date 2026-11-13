//! The event objects against Java 25 (`tests/fixtures/host_game_events`, #156 P8b).
//!
//! `demo/HostGameEvents.java` is compiled against compile-only stubs and only `demo/*` ships. Each
//! probe takes the events as `Object`: this test hands it the runtime's handles;
//! `oracle/HostGameEventsOracle.java` handed the same probes NeoForge 26.3.0.101's and
//! loader-12.0.12's own events on OpenJDK 25.0.5+8 (two runs alike), and every expected string below
//! is what it printed. `Exact`: Java's answer, nothing counted; `CountedAs`: Java's answer, with a
//! check counted; `Counted`: counted (an identity hash, or a type only the class could rule out).

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use super::{game_event, lifecycle_event, lifecycle_parts};
use crate::{NativeJar, StdHost, Value};

const TICK: &str = "net/neoforged/neoforge/event/tick/ServerTickEvent$Post";
const SETUP: &str = "net/neoforged/fml/event/lifecycle/FMLCommonSetupEvent";
const PAYLOAD: &str = "net/neoforged/neoforge/network/event/RegisterPayloadHandlersEvent";
const CONTAINER: &str = "neoforge-mod-container/demo/1.0";

enum Outcome {
    Exact(&'static str),
    CountedAs(&'static str),
    Counted,
}
use Outcome::*;

fn java_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            java_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "java") {
            out.push(path);
        }
    }
}

/// The probe jar, compiled once per test process: only `demo/*`, never the stubs.
fn jar() -> &'static NativeJar {
    static JAR: OnceLock<NativeJar> = OnceLock::new();
    JAR.get_or_init(|| {
        use std::io::Write;
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
        let fixtures = [
            manifest.join("tests/fixtures/host_game_events"),
            manifest.join("../crates/horizon-native-java/tests/fixtures/host_game_events"),
        ]
        .into_iter()
        .find(|dir| dir.join("demo/HostGameEvents.java").is_file())
        .expect("the fixture is in crates/horizon-native-java/tests/fixtures/host_game_events");
        let scratch = std::env::temp_dir().join(format!(
            "horizon-native-java-host-game-events-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&scratch);
        let classes = scratch.join("classes");
        std::fs::create_dir_all(&classes).unwrap();
        let mut sources = Vec::new();
        for part in ["stubs", "demo"] {
            java_files(&fixtures.join(part), &mut sources);
        }
        let javac = std::env::var_os("JAVA_HOME")
            .map(|home| PathBuf::from(home).join("bin/javac"))
            .unwrap_or_else(|| "javac".into());
        let status = Command::new(javac)
            .args(["--release", "21", "-nowarn", "-d"])
            .arg(&classes)
            .args(&sources)
            .status()
            .expect("javac runs");
        assert!(status.success(), "javac failed on the game events fixture");
        let mut bytes = Vec::new();
        {
            let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut bytes));
            let options = zip::write::SimpleFileOptions::default();
            let mut class_files: Vec<PathBuf> = std::fs::read_dir(classes.join("demo"))
                .unwrap()
                .map(|e| e.unwrap().path())
                .collect();
            class_files.sort();
            for path in class_files {
                let name = path.file_name().unwrap().to_string_lossy().into_owned();
                zip.start_file(format!("demo/{name}"), options).unwrap();
                zip.write_all(&std::fs::read(&path).unwrap()).unwrap();
            }
            zip.finish().unwrap();
        }
        let _ = std::fs::remove_dir_all(&scratch);
        NativeJar::from_bytes(&bytes).unwrap()
    })
}

fn run(method: &str, args: &[Value]) -> (Result<Value, String>, Vec<(String, usize)>) {
    let descriptor = format!(
        "({})Ljava/lang/String;",
        "Ljava/lang/Object;".repeat(args.len())
    );
    let mut host = StdHost::new();
    let answer = jar()
        .invoke_method_with_args(
            "demo/HostGameEvents",
            method,
            &descriptor,
            None,
            args,
            &mut host,
        )
        .map_err(|error| error.to_string());
    (answer, host.unknown_host_calls())
}

#[test]
fn game_and_lifecycle_events_answer_as_java_25_or_count() {
    let tick = game_event(TICK);
    let setup = lifecycle_event(SETUP, Some(CONTAINER));
    let payload = lifecycle_event(PAYLOAD, None);
    let cases: Vec<(&str, Vec<Value>, Outcome)> = vec![
        // A string spelled like an old marker equals neither event, nor do they equal it.
        (
            "spelled",
            vec![tick.clone(), setup.clone()],
            Exact(
                "falsefalsefalsefalsetruegame-event/server_tick_end26,falsefalsefalsefalsetrueneoforge-lifecycle|net/neoforged/fml/event/lifecycle/FMLCommonSetupEvent|neoforge-mod-container/demo/1.0104,falsefalsefalsefalsetrueneoforge-lifecycle|net/neoforged/neoforge/network/event/RegisterPayloadHandlersEvent|85,0",
            ),
        ),
        // ... nor is one of their types (`word/rest` text waits for #156 P17's `is_plain_string`,
        // so a check is counted, with Java's answer).
        (
            "spelledTypes",
            vec![],
            CountedAs("falsefalsefalsefalse,falsefalsefalsefalse,falsefalsefalsefalse,"),
        ),
        // A game event is its listener's class: the table's chain to bus-8's `Event`; not a final
        // JDK class, not a jar class, not another event class (a complete chain); its exact class;
        // `Class.isInstance`/`isAssignableFrom`; a host list finds it.
        (
            "tick",
            vec![tick.clone()],
            Exact(
                "truetruetruefalsefalsefalse|net.neoforged.neoforge.event.tick.ServerTickEvent$Post|truetrue|true",
            ),
        ),
        // A NeoForge class is a Mixin target: an interface it does not list (`ICancellableEvent`
        // here) may be added, so `case ICancellableEvent` is counted; Java's case is `S`.
        ("tickSwitch", vec![tick.clone()], CountedAs("S")),
        ("tickInterfaces", vec![tick.clone()], Counted),
        // A firing's event is a fresh object: the same handle is one, two made alike are counted.
        (
            "identity",
            vec![tick.clone(), tick.clone()],
            Exact("truetrue1"),
        ),
        (
            "identity",
            vec![game_event(TICK), game_event(TICK)],
            Counted,
        ),
        // A stage posts one event per container: two handles of it are that one object.
        (
            "identity",
            vec![setup.clone(), lifecycle_event(SETUP, Some(CONTAINER))],
            Exact("truetrue1"),
        ),
        // `Object.toString`/`hashCode` (an identity hash), and no natural order (Java: `CCE`).
        ("text", vec![tick.clone()], Counted),
        ("sorted", vec![tick.clone()], Counted),
        // FML's classes sit outside the Mixin layer: the whole type is known, every answer exact.
        (
            "setup",
            vec![setup.clone()],
            Exact(
                "truetruetruetruetruefalsefalsefalse|net.neoforged.fml.event.lifecycle.FMLCommonSetupEvent|M",
            ),
        ),
        (
            "payload",
            vec![payload.clone()],
            Exact(
                "truetruetruefalse|net.neoforged.neoforge.network.event.RegisterPayloadHandlersEvent",
            ),
        ),
        // `RegisterPayloadHandlersEvent` is a NeoForge class: an interface it does not list is
        // counted (Java: `false`).
        ("payloadCancellable", vec![payload], Counted),
    ];
    let mut failures = Vec::new();
    for (method, args, outcome) in &cases {
        let (answer, unknown) = run(method, args);
        let ok = match outcome {
            Exact(expected) => {
                answer.as_ref() == Ok(&Value::Ref(Some((*expected).into()))) && unknown.is_empty()
            }
            CountedAs(expected) => {
                answer.as_ref() == Ok(&Value::Ref(Some((*expected).into()))) && !unknown.is_empty()
            }
            Counted => !unknown.is_empty(),
        };
        if !ok {
            failures.push(format!(
                "{method}({args:?})\n  native:  {answer:?}\n  unknown: {unknown:?}"
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// A loading-stage event's container (a mod's container marker, jar text) is held through the
/// escaping encoder: one with every separator reads back as itself and never aliases another.
#[test]
fn lifecycle_keys_never_alias_jar_text() {
    let containers = [
        None,
        Some(""),
        Some("a,b"),
        Some("a\\"),
        Some("\\,"),
        Some("|x|"),
        Some("rnull"),
        Some("null"),
        Some(CONTAINER),
    ];
    let mut seen = Vec::new();
    for class in [SETUP, PAYLOAD] {
        for container in containers {
            let event = lifecycle_event(class, container);
            assert_eq!(
                lifecycle_parts(&event),
                Some((class.to_owned(), container.map(str::to_owned)))
            );
            assert!(!seen.contains(&event), "{event:?} aliases another key");
            seen.push(event);
        }
    }
}

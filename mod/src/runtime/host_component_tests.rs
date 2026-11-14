//! Chat components against Java 25 (`tests/fixtures/host_components`, #156 P5a).
//!
//! `demo/HostComponents.java` makes its components through the real API in both runs: compiled
//! against the fixture's compile-only stubs here (only `demo/*` ships, so every call reaches the
//! runtime's model), and against Minecraft 26.3's server jar and libraries by
//! `oracle/HostComponentsOracle.java` on OpenJDK 25.0.5+8 (two runs alike), whose output is every
//! expected string below.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use crate::runtime::{NativeJar, StdHost, Value};

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
            manifest.join("tests/fixtures"),
            manifest.join("../crates/horizon-native-java/tests/fixtures"),
        ]
        .into_iter()
        .find(|dir| {
            dir.join("host_components/demo/HostComponents.java")
                .is_file()
        })
        .expect("the fixture is in crates/horizon-native-java/tests/fixtures/host_components");
        let scratch = std::env::temp_dir().join(format!(
            "horizon-native-java-host-components-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&scratch);
        let classes = scratch.join("classes");
        std::fs::create_dir_all(&classes).unwrap();
        let mut sources = Vec::new();
        for part in ["host_components/stubs", "host_components/demo"] {
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
        assert!(
            status.success(),
            "javac failed on the host components fixture"
        );
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

#[test]
fn components_answer_as_java_25_or_count() {
    let cases = [
        // Strings spelled like the old markers or the handle's key and text forms equal no
        // component.
        (
            "spelled",
            Exact(
                "falsefalsetruecomponent/literal/hi20,falsefalsetruecomponent/translatable/demo.key31,falsefalsetruecomponent/json/{\"text\":\"hi\"}28,falsefalsetrue{\"text\":\"hi\"}13,falsefalsetruehcomp:{\"text\":\"hi\"}19,0-1",
            ),
        ),
        (
            "encoded",
            Exact(
                "truejava.lang.String1960836082falsefalse,truejava.lang.String-2021455569falsefalse,13",
            ),
        ),
        // `word/rest` text is still not told from a marker before #156 P17: counted, with
        // Java's answer.
        (
            "spelledTypes",
            CountedAs("falsefalse,falsefalse,falsefalse,"),
        ),
        // Class and Class surface, identity, equality of bare texts and keys, `indexOf`/`contains`,
        // `isAssignableFrom`/`isInstance`.
        (
            "real",
            Exact(
                "truetruetruetruefalsefalse|net.minecraft.network.chat.MutableComponent|java.lang.Objectfalse|MutableComponent|falsetruetruetruefalsefalsetruefalse2|01false|truetruetrue",
            ),
        ),
        (
            "text",
            Exact("literal{hi}|translation{key='demo.key', args=[]}|hi|demo.key|literal{hi}"),
        ),
        ("copy", Exact("falsetruetrue2literal{hi}")),
        // `append`/`withStyle`/`withColor` change the receiver in place and return it; a sibling
        // changed after it was appended shows in its parent, and `copy` shares its siblings.
        ("appended", Exact("truehi!?1")),
        ("styled", Exact("truetruetruehi")),
        ("appendInPlace", Exact("truehi!s?~x1|hi!s?~false|k")),
        // Equality reads the state now: a changed component no longer equals what it was made
        // as, and a copy equals its source.
        ("changedEquals", Exact("truefalsetruefalse")),
        // Styled components compare their `Style`s (Java `falsetrue`): counted.
        ("styledEquals", Counted),
        // An interface a Mixin could add to a game class is counted; Java's case is `M`.
        ("typeSwitch", CountedAs("M")),
        // A hash set's answers follow from `equals` alone (equal objects hash alike).
        ("hashed", Exact("true1")),
        // A list prints its elements' text; `Map.of` finds an equal key (equal objects hash alike).
        (
            "listed",
            Exact("[literal{hi}, translation{key='demo.key', args=[]}]"),
        ),
        ("mapped", Exact("1")),
        // `Set.of` meets an equal component (`Set12`'s `equals`, `SetN`'s probe: equal objects
        // hash alike) and refuses it.
        ("duplicate", Exact("duplicate element: literal{a}")),
        ("duplicateN", Exact("duplicate element: literal{a}")),
        // `hashCode` combines contents, style and siblings (Java `132311680`) and there is no
        // natural order (Java `CCE`): counted.
        ("hash", Counted),
        ("sorted", Counted),
    ];
    let mut failures = Vec::new();
    for (method, outcome) in &cases {
        let mut host = StdHost::new();
        let answer = jar()
            .invoke_method_with_args(
                "demo/HostComponents",
                method,
                "()Ljava/lang/String;",
                None,
                &[],
                &mut host,
            )
            .map_err(|error| error.to_string());
        let unknown = host.unknown_host_calls();
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
                "{method}\n  native:  {answer:?}\n  unknown: {unknown:?}"
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// One content prints one JSON text whatever order it was built in, each allocation its own key;
/// jar text in it is escaped.
#[test]
fn a_component_key_is_its_sorted_json() {
    let a = crate::runtime::host_component::component(&serde_json::json!({"text": "a", "bold": true}));
    let b = crate::runtime::host_component::component(&serde_json::json!({"bold": true, "text": "a"}));
    let (Value::Host(a), Value::Host(b)) = (&a, &b) else {
        panic!("not handles")
    };
    assert_ne!(a.key(), b.key(), "two allocations");
    assert_eq!(
        crate::runtime::facade_text::json_now(&Value::Host(a.clone()), None),
        crate::runtime::facade_text::json_now(&Value::Host(b.clone()), None)
    );
    let (Some((_, a)), Some((_, b))) = (a.key().split_once(':'), b.key().split_once(':')) else {
        panic!("no serial")
    };
    assert_eq!(a, b, "one content, one JSON text");
    let tricky = crate::runtime::host_component::literal("\"},{\"text\":\"x");
    assert_eq!(
        crate::runtime::facade_text::json_now(&tricky, None)
            .as_ref()
            .and_then(crate::runtime::host_component::bare_literal)
            .as_deref(),
        Some("\"},{\"text\":\"x")
    );
}

/// A changed component lives in its host's table while something reaches it: a static field, or
/// a changed parent's siblings. The resident heap's collection drops one nothing reaches.
#[test]
fn changed_components_are_collected_with_the_heap() {
    use crate::runtime::host_component::{change, literal, marker};
    let argument = crate::runtime::facade_text::argument;
    let mut host = StdHost::new();
    let (kept, sibling, dropped) = (literal("kept"), literal("s"), literal("dropped"));
    change(&mut host.component_states, &sibling, &argument, |state| {
        state.siblings.push(literal("t"))
    })
    .unwrap();
    let held = sibling.clone();
    change(&mut host.component_states, &kept, &argument, |state| {
        state.siblings.push(held)
    })
    .unwrap();
    change(&mut host.component_states, &dropped, &argument, |state| {
        state.siblings.push(literal("u"))
    })
    .unwrap();
    host.persistent_static_fields
        .insert("demo/Main.TITLE".into(), kept.clone());
    let mut heap = crate::runtime::heap::JavaHeap::default();
    crate::runtime::heap::collect(&mut heap, &mut host, &[]);
    let row = |value: &Value| match value {
        Value::Host(handle) => host.component_states.contains_key(marker(handle)),
        _ => false,
    };
    assert!(row(&kept) && row(&sibling), "reached rows stay");
    assert!(!row(&dropped), "an unreached row goes");
    assert_eq!(
        crate::runtime::facade_text::json_now(&kept, Some(&host.component_states)),
        Some(
            serde_json::json!({"text": "kept", "extra": [{"text": "s", "extra": [{"text": "t"}]}]})
        )
    );
}

//! Iterators, enumerations, list iterators and collectors against Java 25
//! (`tests/fixtures/host_iterators`, #156 P14f).
//!
//! `demo/HostIterators.java` uses only the JDK, compiled here and run by
//! `oracle/HostIteratorsOracle.java` on OpenJDK 25.0.5+8 (two runs alike), whose output is every
//! expected string below.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use crate::runtime::{NativeJar, StdHost, Value};

enum Outcome {
    Exact(&'static str),
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
        .find(|dir| dir.join("host_iterators/demo/HostIterators.java").is_file())
        .expect("the fixture is in crates/horizon-native-java/tests/fixtures/host_iterators");
        let scratch = std::env::temp_dir().join(format!(
            "horizon-native-java-host-iterators-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&scratch);
        let classes = scratch.join("classes");
        std::fs::create_dir_all(&classes).unwrap();
        let mut sources = Vec::new();
        java_files(&fixtures.join("host_iterators/demo"), &mut sources);
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
            "javac failed on the host iterators fixture"
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
fn iterators_answer_as_java_25_or_count() {
    let cases = [
        // `Collectors.toList()` twice is two collectors; `joining`'s texts (a `,` in one), `toSet`, `toMap`.
        (
            "collectors",
            Exact(r#"false2true|[1, 2][3]|[x,;y][z]|2|{k1=2, k2=2}|aba-b"#),
        ),
        // `Collections.emptyIterator()`/`emptyEnumeration()`: one shared object each, empty.
        (
            "empties",
            Exact(
                r#"truefalsejava.util.NoSuchElementException: null|truefalsejava.util.NoSuchElementException: null|2"#,
            ),
        ),
        // `Collections.enumeration(c)` walks `c.iterator()`: two are two objects, and a change to `c` fails fast.
        (
            "enumerations",
            Exact(r#"falsetrueaa|java.util.ConcurrentModificationException: null|2"#),
        ),
        // Fail-fast `next()` after a change (`ArrayList`, `HashSet`); `Iterator.remove` is not modelled: counted.
        ("failFast", Counted),
        // An iterator's class (`ArrayList$Itr`) is not named here: `getClass()` and `instanceof ListIterator` counted.
        ("iteratorTypes", Counted),
        // Two `listIterator()` calls are two list iterators with their own cursors; `set`/`add`; fail-fast.
        (
            "listIterators",
            Exact(
                r#"falsefalse2|21[A, x, b, c]|java.util.ConcurrentModificationException: null|true"#,
            ),
        ),
        // `ClassLoader.getResources` of a missing name: a new empty enumeration per call.
        ("resources", Exact(r#"falsefalse2"#)),
        // Strings spelled like the old markers are Strings.
        (
            "spelled",
            Exact(
                r#"java.lang.Stringfalsefalsefalsefalse-1751405873110,java.lang.Stringfalsefalsefalsefalse-239962273115,java.lang.Stringfalsefalsefalsefalse1342349033122,java.lang.Stringfalsefalsefalsefalse1369441504114,java.lang.Stringfalsefalsefalsefalse897449811119,"#,
            ),
        ),
        // Two `iterator()` calls are two iterators with their own cursors (`==`, `equals`, `IdentityHashMap`, `contains`).
        (
            "twoIterators",
            Exact(r#"falsefalsetrue2falsetrue|31falsetrue|true"#),
        ),
        // `Vector.elements()` reads the vector live, no fail-fast; `NoSuchElementException("Vector Enumeration")`.
        (
            "vectorElements",
            Exact(r#"afalsetrueb|java.util.NoSuchElementException: Vector Enumeration|false"#),
        ),
    ];
    let mut failures = Vec::new();
    for (method, outcome) in &cases {
        let mut host = StdHost::new();
        let answer = jar()
            .invoke_method_with_args(
                "demo/HostIterators",
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

/// A host that keeps no enumerations makes none: `Collections.enumeration` is then counted, never
/// answered with a shared placeholder that two calls would see as one object (#225).
#[test]
fn a_host_without_enumerations_makes_none() {
    use crate::runtime::Host;
    let over = crate::runtime::host_iterator::EnumerationOver::Iterator(Value::Ref(None));
    assert_eq!(crate::runtime::NullHost.new_enumeration(over.clone()), None);
    let mut host = StdHost::new();
    let made = host.new_enumeration(over.clone());
    assert!(made.is_some());
    assert_ne!(made, host.new_enumeration(over));
}

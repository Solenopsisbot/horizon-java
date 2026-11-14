//! `UUID`, `Duration` and `URI` against Java 25 (`tests/fixtures/host_jdk_values`, #156 P1a).
//!
//! `demo/HostJdkValues.java` uses only the JDK, compiled here and run by
//! `oracle/HostJdkValuesOracle.java` on OpenJDK 25.0.5+8 (two runs alike), whose output is every
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
        .find(|dir| {
            dir.join("host_jdk_values/demo/HostJdkValues.java")
                .is_file()
        })
        .expect("the fixture is in crates/horizon-native-java/tests/fixtures/host_jdk_values");
        let scratch = std::env::temp_dir().join(format!(
            "horizon-native-java-host-jdk-values-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&scratch);
        let classes = scratch.join("classes");
        std::fs::create_dir_all(&classes).unwrap();
        let mut sources = Vec::new();
        java_files(&fixtures.join("host_jdk_values/demo"), &mut sources);
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
            "javac failed on the host JDK values fixture"
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
fn jdk_values_answer_as_java_25_or_count() {
    let cases = [
        // Strings spelled like the old markers equal no value.
        (
            "spelled",
            Exact(
                "falsefalseuuid/550e8400-e29b-41d4-a716-44665544000041,falsefalseduration/510,falsefalseuri/http://example.com/a24,-1",
            ),
        ),
        // A String is a String: no `instanceof` of these classes.
        (
            "spelledTypes",
            Exact("falsefalsefalse,falsefalsefalse,falsefalsefalse,"),
        ),
        // Value equality, Java's hash and text, the signed `compareTo` (`-1` for a high bit
        // set), hash and tree collections, identity of one object.
        (
            "uuids",
            Exact(
                "true550e8400-e29b-41d4-a716-446655440000|1170702770|-110|truetruejava.util.UUID|true7[80000000-0000-0000-0000-000000000000, 00000000-0000-0000-0000-000000000001, 550e8400-e29b-41d4-a716-446655440000]|1",
            ),
        ),
        // ISO-8601, value equality, Java's hash, `compareTo` as the nanos' difference, `ZERO`.
        (
            "durations",
            Exact(
                "PT1M30S|PT1.5S|PT-0.5S|PT48H|true90,360|4,-1|truetrue|true[PT0S, PT5S, PT1M30S]",
            ),
        ),
        (
            "uris",
            Exact("http://example.com/a?q=1|truehttpexample.com|1|truejava.net.URI"),
        ),
        // Two objects made separately (Java `falsefalse`), two URI texts with equal parts
        // (Java `true`), a URI's hash and order: counted.
        ("identity", Counted),
        ("uriCase", Counted),
        ("uriHash", Counted),
    ];
    let mut failures = Vec::new();
    for (method, outcome) in &cases {
        let mut host = StdHost::new();
        let answer = jar()
            .invoke_method_with_args(
                "demo/HostJdkValues",
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

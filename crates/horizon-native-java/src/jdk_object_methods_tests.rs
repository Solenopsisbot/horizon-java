//! JDK objects a jar builds with `new` against Java 25 (`tests/fixtures/jdk_object_overrides`,
//! #217).
//!
//! `demo/JdkObjects.java` uses only the JDK, compiled here and run by
//! `oracle/JdkObjectsOracle.java` on OpenJDK 25.0.5+8 (two runs alike), whose output is every
//! expected string below. Each probe is one member on one class: `equals` of two equal-valued
//! objects, `same` (`a.equals(a)`, `a == a`), `other` (`equals("x")`, `equals(null)`), `hash`
//! (two objects' hashes equal), `text` (`toString()`), `concat` (`"" + a`), `contains`
//! (`List.of(a).contains(b)`), `objects` (`Objects.equals(a, b)`), `compare`.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use crate::{NativeJar, StdHost, Value};

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
            dir.join("jdk_object_overrides/demo/JdkObjects.java")
                .is_file()
        })
        .expect("the fixture is in crates/horizon-native-java/tests/fixtures/jdk_object_overrides");
        let scratch = std::env::temp_dir().join(format!(
            "horizon-native-java-jdk-object-overrides-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&scratch);
        let classes = scratch.join("classes");
        std::fs::create_dir_all(&classes).unwrap();
        let mut sources = Vec::new();
        java_files(&fixtures.join("jdk_object_overrides/demo"), &mut sources);
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
            "javac failed on the JDK object overrides fixture"
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
fn jdk_objects_answer_as_java_25_or_count() {
    let cases = [
        // `new Object()`: `Object`'s members. `equals` is identity; `hashCode` and `toString` an identity hash.
        ("object_equals", Exact(r#"false"#)),
        ("object_same", Exact(r#"truetrue"#)),
        ("object_other", Exact(r#"falsefalse"#)),
        ("object_hash", Counted),
        ("object_text", Counted),
        ("object_concat", Counted),
        ("object_contains", Exact(r#"false"#)),
        ("object_objects", Exact(r#"false"#)),
        // `Random`, `CRC32`: listed, keep every `Object` member.
        ("random_equals", Exact(r#"false"#)),
        ("random_same", Exact(r#"truetrue"#)),
        ("random_other", Exact(r#"falsefalse"#)),
        ("random_hash", Counted),
        ("random_text", Counted),
        ("random_concat", Counted),
        ("random_contains", Exact(r#"false"#)),
        ("random_objects", Exact(r#"false"#)),
        ("crc_equals", Exact(r#"false"#)),
        ("crc_same", Exact(r#"truetrue"#)),
        ("crc_other", Exact(r#"falsefalse"#)),
        ("crc_hash", Counted),
        ("crc_text", Counted),
        ("crc_concat", Counted),
        ("crc_contains", Exact(r#"false"#)),
        ("crc_objects", Exact(r#"false"#)),
        // `Adler32` keeps every `Object` member; `DoubleAdder` its own `toString` (not modelled: counted), `Object.equals`.
        ("adler_equals", Exact(r#"false"#)),
        ("adler_same", Exact(r#"truetrue"#)),
        ("adler_other", Exact(r#"falsefalse"#)),
        ("adler_hash", Counted),
        ("adler_text", Counted),
        ("adler_concat", Counted),
        ("adler_contains", Exact(r#"false"#)),
        ("adler_objects", Exact(r#"false"#)),
        // `StringBuilder`: its own `toString` (modelled) and `compareTo` (counted); `Object.equals`.
        ("builder_equals", Exact(r#"false"#)),
        ("builder_same", Exact(r#"truetrue"#)),
        ("builder_other", Exact(r#"falsefalse"#)),
        ("builder_hash", Counted),
        ("builder_text", Exact(r#"ab"#)),
        ("builder_concat", Exact(r#"ab"#)),
        ("builder_contains", Exact(r#"false"#)),
        ("builder_objects", Exact(r#"false"#)),
        ("builder_compare", Counted),
        // `AtomicInteger`, `ReentrantLock`, `StringJoiner`: own `toString` (not modelled: counted), `Object.equals`.
        ("atomic_equals", Exact(r#"false"#)),
        ("atomic_same", Exact(r#"truetrue"#)),
        ("atomic_other", Exact(r#"falsefalse"#)),
        ("atomic_hash", Counted),
        ("atomic_text", Counted),
        ("atomic_concat", Counted),
        ("atomic_contains", Exact(r#"false"#)),
        ("atomic_objects", Exact(r#"false"#)),
        ("lock_equals", Exact(r#"false"#)),
        ("lock_same", Exact(r#"truetrue"#)),
        ("lock_other", Exact(r#"falsefalse"#)),
        ("lock_hash", Counted),
        ("lock_text", Counted),
        ("lock_concat", Counted),
        ("lock_contains", Exact(r#"false"#)),
        ("lock_objects", Exact(r#"false"#)),
        ("joiner_equals", Exact(r#"false"#)),
        ("joiner_same", Exact(r#"truetrue"#)),
        ("joiner_other", Exact(r#"falsefalse"#)),
        ("joiner_hash", Counted),
        ("joiner_text", Counted),
        ("joiner_concat", Counted),
        ("joiner_contains", Exact(r#"false"#)),
        ("joiner_objects", Exact(r#"false"#)),
        // `ZipEntry`: own `hashCode` and `toString`, `Object.equals`.
        ("zipEntry_equals", Exact(r#"false"#)),
        ("zipEntry_same", Exact(r#"truetrue"#)),
        ("zipEntry_other", Exact(r#"falsefalse"#)),
        ("zipEntry_hash", Counted),
        ("zipEntry_text", Counted),
        ("zipEntry_concat", Counted),
        ("zipEntry_contains", Exact(r#"false"#)),
        ("zipEntry_objects", Exact(r#"false"#)),
        // `BigDecimal`, `BigInteger`, `Date`, `BitSet`, `DecimalFormat`, `InetSocketAddress`: own `equals` no model answers: counted, also inside `List.contains`/`Objects.equals`.
        ("decimal_equals", Counted),
        ("decimal_same", Counted),
        ("decimal_other", Counted),
        ("decimal_hash", Counted),
        ("decimal_text", Counted),
        ("decimal_concat", Counted),
        ("decimal_contains", Counted),
        ("decimal_objects", Counted),
        ("decimal_compare", Counted),
        ("integer_equals", Counted),
        ("integer_same", Counted),
        ("integer_other", Counted),
        ("integer_hash", Counted),
        ("integer_text", Counted),
        ("integer_concat", Counted),
        ("integer_contains", Counted),
        ("integer_objects", Counted),
        ("integer_compare", Counted),
        ("date_equals", Counted),
        ("date_same", Counted),
        ("date_other", Counted),
        ("date_hash", Counted),
        ("date_text", Counted),
        ("date_concat", Counted),
        ("date_contains", Counted),
        ("date_objects", Counted),
        ("date_compare", Counted),
        ("bits_equals", Counted),
        ("bits_same", Counted),
        ("bits_other", Counted),
        ("bits_hash", Counted),
        ("bits_text", Counted),
        ("bits_concat", Counted),
        ("bits_contains", Counted),
        ("bits_objects", Counted),
        ("format_equals", Counted),
        ("format_same", Counted),
        ("format_other", Counted),
        ("format_hash", Counted),
        ("format_text", Counted),
        ("format_concat", Counted),
        ("format_contains", Counted),
        ("format_objects", Counted),
        ("address_equals", Counted),
        ("address_same", Counted),
        ("address_other", Counted),
        ("address_hash", Counted),
        ("address_text", Counted),
        ("address_concat", Counted),
        ("address_contains", Counted),
        ("address_objects", Counted),
        // `AbstractMap.SimpleEntry`: its JDK skeleton answers all of them, in a collection too.
        ("entry_equals", Exact(r#"true"#)),
        ("entry_same", Exact(r#"truetrue"#)),
        ("entry_other", Exact(r#"falsefalse"#)),
        ("entry_hash", Exact(r#"true"#)),
        ("entry_text", Exact(r#"k=v"#)),
        ("entry_concat", Exact(r#"k=v"#)),
        ("entry_contains", Exact(r#"true"#)),
        ("entry_objects", Exact(r#"true"#)),
        ("adder_equals", Exact(r#"false"#)),
        ("adder_same", Exact(r#"truetrue"#)),
        ("adder_other", Exact(r#"falsefalse"#)),
        ("adder_hash", Counted),
        ("adder_text", Counted),
        ("adder_concat", Counted),
        ("adder_contains", Exact(r#"false"#)),
        ("adder_objects", Exact(r#"false"#)),
        // `Exception`: `Throwable.toString` (modelled), `Object.equals`.
        ("exception_equals", Exact(r#"false"#)),
        ("exception_same", Exact(r#"truetrue"#)),
        ("exception_other", Exact(r#"falsefalse"#)),
        ("exception_hash", Counted),
        ("exception_text", Exact(r#"java.lang.Exception: m"#)),
        ("exception_concat", Exact(r#"java.lang.Exception: m"#)),
        ("exception_contains", Exact(r#"false"#)),
        ("exception_objects", Exact(r#"false"#)),
        // Natural order: `Random` is no `Comparable` (Java's `ClassCastException`), `BigDecimal`'s `compareTo` is counted. A jar subclass runs the JDK class it extends (`Random`: identity; `BigDecimal`: counted), a call site typed `Object` too. A `HashSet` keeps its model.
        (
            "sorted_random",
            Exact(
                r#"java.lang.ClassCastException: class java.util.Random cannot be cast to class java.lang.Comparable (java.util.Random and java.lang.Comparable are in module java.base of loader 'bootstrap')"#,
            ),
        ),
        ("sorted_decimal", Counted),
        ("sub_random_equals", Exact(r#"falsefalse"#)),
        ("sub_decimal_equals", Counted),
        ("sub_decimal_objects", Counted),
        ("as_object_equals", Counted),
        ("set_objects", Exact(r#"true"#)),
        ("set_equals", Exact(r#"true"#)),
    ];
    let mut failures = Vec::new();
    for (method, outcome) in &cases {
        let mut host = StdHost::new();
        let answer = jar()
            .invoke_method_with_args(
                "demo/JdkObjects",
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

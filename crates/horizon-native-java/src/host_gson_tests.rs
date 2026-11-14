//! Gson's `FormattingStyle` and Guava's `TypeToken` against Java 25
//! (`tests/fixtures/host_library_values`, #156 P5b).
//!
//! `demo/HostLibraryValues.java` makes its styles and tokens through the real API in both runs:
//! compiled against the fixture's compile-only stubs here (only `demo/*` ships, so every call
//! reaches the runtime's model), and against gson 2.14.0 and guava 33.6.0 (the 26.3 server's
//! libraries) by `oracle/HostLibraryValuesOracle.java` on OpenJDK 25.0.5+8 (two runs alike but
//! for the identity hashes), whose output is every expected string below.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use crate::{NativeJar, StdHost, Value};

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
            dir.join("host_library_values/demo/HostLibraryValues.java")
                .is_file()
        })
        .expect("the fixture is in crates/horizon-native-java/tests/fixtures/host_library_values");
        let scratch = std::env::temp_dir().join(format!(
            "horizon-native-java-host-library-values-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&scratch);
        let classes = scratch.join("classes");
        std::fs::create_dir_all(&classes).unwrap();
        let mut sources = Vec::new();
        for part in ["host_library_values/stubs", "host_library_values/demo"] {
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
            "javac failed on the host library values fixture"
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
fn styles_and_tokens_answer_as_java_25_or_count() {
    let cases = [
        // Strings spelled like the old markers are Strings: equal to no style or token.
        (
            "spelled",
            Exact(
                "falsefalsefalsefalseformatting-style/0a/2020/126,falsefalsefalsefalsegson-style/newline=0a;indent=2020;space=141,falsefalsefalsefalsetypetoken/java/lang/String26,2-1",
            ),
        ),
        // `word/rest` text is still not told from a marker before #156 P17 (`instanceof String`,
        // its hash): counted. Java: `true-313121738falsefalse,true-928762901falsefalse,
        // true17512469falsefalse,`.
        ("spelledTypes", Counted),
        // The constants are one object each, every `with*` a new one (identity `equals`); the
        // getters, the class surface, and the style reaching `toJson`.
        (
            "styles",
            Exact(
                "truetruefalsefalsefalse3|41truetrue|truefalsecom.google.gson.FormattingStylejava.lang.Object|1false|[\n    1,\n    2\n]",
            ),
        ),
        // `TypeToken.of` is a new token each time, equal by type; its class is exact.
        (
            "tokens",
            Exact(
                "falsetruefalsefalse2|truejava.lang.String|java.lang.String|java.lang.Integer|truefalsecom.google.common.reflect.TypeToken$SimpleTypeToken|com.google.common.reflect.TypeToken|1true",
            ),
        ),
        // Identity hashes and `Object.toString` (Java `com.google.gson.FormattingStyle@…`, a
        // class's identity hash for a token): counted.
        ("styleText", Counted),
        ("tokenHash", Counted),
        // Gson's `TypeToken` (#156 P5b): its type is Gson's own canonical copy of the JDK node,
        // equal both ways, not `==`, Gson's class and text.
        (
            "gsonSubclass",
            Exact(
                "java.util.List<java.lang.String>|truetruetruefalse|com.google.gson.internal.GsonTypes$ParameterizedTypeImpl|truetrue|java.util.List|java.util.List<java.lang.String>|truefalsefalse",
            ),
        ),
        (
            "gsonGet",
            Exact(
                "truetruejava.lang.String|com.google.gson.internal.GsonTypes$GenericArrayTypeImpl|java.lang.String[]|[Ljava.lang.String;|int[]|truetruetrue",
            ),
        ),
        (
            "gsonNested",
            Exact(
                "java.util.Map<java.lang.String, java.util.List<java.lang.Integer>>|com.google.gson.internal.GsonTypes$ParameterizedTypeImpl|truetruetrue|truenull|true",
            ),
        ),
        (
            "gsonWildcards",
            Exact(
                "java.util.List<? extends java.lang.Number>|java.util.List<?>|java.util.List<? super java.lang.Integer>",
            ),
        ),
        (
            "gsonMember",
            Exact(
                "demo.HostLibraryValues$Outer$Inner<java.lang.Integer>|demo.HostLibraryValues$Outer<java.lang.String>$Inner<java.lang.Integer>|truetrue",
            ),
        ),
        // Classes' identity hashes: counted.
        ("gsonHash", Counted),
        // An interface a library class could gain is counted; Java's case is `F`.
        ("typeSwitch", CountedAs("F")),
    ];
    let mut failures = Vec::new();
    for (method, outcome) in &cases {
        let mut host = StdHost::new();
        let answer = jar()
            .invoke_method_with_args(
                "demo/HostLibraryValues",
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

/// A style's key escapes jar text: a newline or indent holding the key's separators reads back
/// as itself, and two styles never share a key.
#[test]
fn a_style_key_round_trips_any_text() {
    use crate::host_gson::{Style, formatting_style, style_of};
    for text in [",", "\\", "a,b\\,c", "[I#0", "\u{0}"] {
        let style = Style {
            newline: text.to_owned(),
            indent: format!("{text}{text}"),
            space_after_separators: true,
        };
        let (a, b) = (
            formatting_style(&style, None),
            formatting_style(&style, None),
        );
        assert_eq!(style_of(&a), Some(style.clone()), "{text:?}");
        assert_ne!(a, b, "two allocations");
    }
}

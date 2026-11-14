//! JDK constants and small values against Java 25 (`tests/fixtures/host_jdk_constants`,
//! #156 P1b).
//!
//! `demo/HostJdkConstants.java` uses only the JDK, compiled here and run by
//! `oracle/HostJdkConstantsOracle.java` on OpenJDK 25.0.5+8 (two runs alike but for an identity
//! hash), whose output is every expected string below.

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
            dir.join("host_jdk_constants/demo/HostJdkConstants.java")
                .is_file()
        })
        .expect("the fixture is in crates/horizon-native-java/tests/fixtures/host_jdk_constants");
        let scratch = std::env::temp_dir().join(format!(
            "horizon-native-java-host-jdk-constants-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&scratch);
        let classes = scratch.join("classes");
        std::fs::create_dir_all(&classes).unwrap();
        let mut sources = Vec::new();
        java_files(&fixtures.join("host_jdk_constants/demo"), &mut sources);
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
            "javac failed on the host JDK constants fixture"
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
fn jdk_constants_answer_as_java_25_or_count() {
    let cases = [
        // Strings spelled like the old markers are Strings.
        (
            "spelled",
            Exact(
                "falsefalsetime-unit/SECONDS17,falsefalsecoding-error-action/REPORT26,falsefalseunicode-block/BASIC_LATIN25,falsefalselocale/US9,falsefalseregex-pattern/a+16,-1",
            ),
        ),
        // One object per constant, their text, `Enum.compareTo` (ordinals' difference).
        (
            "constants",
            Exact(
                "trueSECONDS-32false5000|trueREPORTfalse|trueBASIC_LATIN|truejava.util.concurrent.TimeUnit|21",
            ),
        ),
        // `BaseLocale`'s hash, text, tag, equality.
        (
            "locales",
            Exact("en_US|en-US|3118343,3241,0||en|truefalsefalse|USen|truejava.util.Locale"),
        ),
        // One object per `compile`.
        (
            "patterns",
            Exact("a+|a+|truefalsefalsetrue|java.util.regex.Pattern|false"),
        ),
        // Java 25's locale cache: the constants are the objects `Locale.of`/`forLanguageTag`
        // answer; old ISO codes answer their new spelling.
        (
            "localeIdentity",
            Exact(
                "truetruetruetruetruetruetruetruetruetruetruefalsetrue|heyiid|fr_CA|zh_TW3079362|title|en-001",
            ),
        ),
        // A key no constant holds: the cache keeps it softly, so `==` is counted.
        ("cachedIdentity", Counted),
        // Turkish case tailoring (`"TITLE"` is `"tıtle"`): counted.
        ("turkish", Counted),
        // An identity hash.
        ("unitHash", Counted),
    ];
    let mut failures = Vec::new();
    for (method, outcome) in &cases {
        let mut host = StdHost::new();
        let answer = jar()
            .invoke_method_with_args(
                "demo/HostJdkConstants",
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

/// NeoForge's `ArtifactVersion`: its text, equal for one text, counted for two (Maven's
/// `ComparableVersion` makes `1.0` equal `1.0.0`); a String spelled like the old marker is none.
#[test]
fn an_artifact_version_is_its_text() {
    use crate::runtime::host_jdk_constant::{artifact_version, artifact_version_text};
    let version = artifact_version("1.0");
    assert_eq!(artifact_version_text(&version), Some("1.0"));
    assert_eq!(
        artifact_version_text(&Value::Ref(Some("artifact-version/1.0".into()))),
        None
    );
    let Value::Host(handle) = &version else {
        panic!("not a handle")
    };
    let mut nested = crate::runtime::HostNested(None);
    assert_eq!(
        handle
            .equals(&artifact_version("1.0"), &mut nested)
            .unwrap(),
        Some(true)
    );
    assert_eq!(
        handle
            .equals(&artifact_version("1.0.0"), &mut nested)
            .unwrap(),
        None
    );
    assert_eq!(
        handle
            .equals(&Value::Ref(Some("1.0".into())), &mut nested)
            .unwrap(),
        Some(false)
    );
    assert_eq!(
        handle.to_string(&mut nested).unwrap().as_deref(),
        Some("1.0")
    );
}

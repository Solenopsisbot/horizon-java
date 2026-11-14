//! DataFixerUpper's `Pair` and opaque codecs against Java 25 (`tests/fixtures/host_codecs`,
//! #156 P5c).
//!
//! `demo/HostCodecs.java` makes its pairs through the real API in both runs: compiled against
//! the fixture's compile-only stubs here (only `demo/*` ships, so every call reaches the
//! runtime's model), and against DataFixerUpper 10.0.21 (the 26.3 server's library, with guava
//! and fastutil) by `oracle/HostCodecsOracle.java` on OpenJDK 25.0.5+8 (two runs alike but for
//! an identity hash), whose output is every expected string below.

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
        .find(|dir| dir.join("host_codecs/demo/HostCodecs.java").is_file())
        .expect("the fixture is in crates/horizon-native-java/tests/fixtures/host_codecs");
        let scratch = std::env::temp_dir().join(format!(
            "horizon-native-java-host-codecs-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&scratch);
        let classes = scratch.join("classes");
        std::fs::create_dir_all(&classes).unwrap();
        let mut sources = Vec::new();
        for part in ["host_codecs/stubs", "host_codecs/demo"] {
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
        assert!(status.success(), "javac failed on the host codecs fixture");
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
fn pairs_and_codecs_answer_as_java_25_or_count() {
    let cases = [
        // Strings spelled like the old markers are Strings: equal to no pair.
        (
            "spelled",
            Exact(
                "falsefalsedfu-pair/i:1,a14,falsefalsenf-codec/010,falsefalsenf-codec/compound-tag21,falsefalsecom/mojang/serialization/Codec#codec36,falsefalsecodec/opaque12,0-1",
            ),
        ),
        // `word/rest` text is still not told from a marker before #156 P17: counted. Java:
        // `truefalsefalse,` five times.
        ("spelledTypes", Counted),
        // Values back, `equals` by them (`instanceof Pair`, `Objects.equals`), a new object per
        // `of`, `"(f, s)"`, Guava's `Objects.hashCode`, the class surface, hash collections.
        (
            "pairs",
            Exact(
                "1aa1|falsetruefalsetruefalsefalse2|(1, a)|(null, null)|1089,961|truetruefalsecom.mojang.datafixers.util.Pairjava.lang.Object|true71",
            ),
        ),
        ("nested", Exact("((2, b), [3, 4])|36770|true")),
        // An identity hash held: counted.
        ("identityHeld", Counted),
        // An unmodelled codec constant (Java `truetrue`): counted.
        ("opaqueCodec", Counted),
    ];
    let mut failures = Vec::new();
    for (method, outcome) in &cases {
        let mut host = StdHost::new();
        let answer = jar()
            .invoke_method_with_args(
                "demo/HostCodecs",
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

/// A String is no codec: a payload registered with one names no codec, and the attachment
/// model's codec nodes are reached only through their handles.
#[test]
fn a_string_is_not_a_codec() {
    use crate::host_codec::{modelled_codec, modelled_codec_key, opaque_codec, opaque_codec_type};
    for text in [
        "codec/opaque",
        "nf-codec/0",
        "nf-codec/compound-tag",
        "com/mojang/serialization/Codec#codec",
    ] {
        let forged = Value::Ref(Some(text.into()));
        assert_eq!(opaque_codec_type(&forged), None, "{text}");
        assert_eq!(modelled_codec_key(&forged), None, "{text}");
    }
    let codec = opaque_codec("net/minecraft/network/codec/StreamCodec");
    assert_eq!(
        opaque_codec_type(&codec),
        Some("net/minecraft/network/codec/StreamCodec")
    );
    assert_eq!(
        modelled_codec_key(&modelled_codec("map:3".into())),
        Some("map:3")
    );
}

/// A pair's key escapes the values it holds: text with the key's separators reads back as
/// itself, and two pairs of one content are two objects.
#[test]
fn a_pair_key_round_trips_any_value() {
    use crate::host_codec::{pair, pair_parts};
    for text in [",", "\\", "a,b\\,c", "[I#0", "dfu-pair/x,y"] {
        let first = Value::Ref(Some(text.into()));
        let second = crate::host_codec::opaque_codec("com/mojang/serialization/Codec");
        let (a, b) = (pair(&first, &second), pair(&first, &second));
        assert_eq!(
            pair_parts(&a),
            Some((first.clone(), second.clone())),
            "{text:?}"
        );
        assert_ne!(a, b, "two allocations");
    }
}

//! NeoForge's attachment types and builders against Java 25 (`tests/fixtures/host_attachments`,
//! #156 P8c).
//!
//! `demo/HostAttachments.java` builds its types through the real API in both runs: compiled
//! against the attachments fixture's compile-only stubs here (only `demo/*` ships, so every call
//! reaches the runtime's model), and against NeoForge 26.3.0.101 by
//! `oracle/HostAttachmentsOracle.java` on OpenJDK 25.0.5+8 (two runs alike), whose output is every
//! expected string below. The same oracle re-ran `tests/fixtures/attachments/oracle.jsonl`'s
//! `encode/carry-state` and `builder/checks` (recorded on OpenJDK 27): Java 25 gives the same answers.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use crate::host_ref::HostKind;
use crate::{MethodRef, NativeJar, StdHost, Value};

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
            dir.join("host_attachments/demo/HostAttachments.java")
                .is_file()
        })
        .expect("the fixture is in crates/horizon-native-java/tests/fixtures/host_attachments");
        let scratch = std::env::temp_dir().join(format!(
            "horizon-native-java-host-attachments-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&scratch);
        let classes = scratch.join("classes");
        std::fs::create_dir_all(&classes).unwrap();
        let mut sources = Vec::new();
        for part in [
            "attachments/stubs",
            "host_attachments/stubs",
            "host_attachments/demo",
        ] {
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
            "javac failed on the host attachments fixture"
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
fn attachment_types_answer_as_java_25_or_count() {
    let cases = [
        // Strings spelled like the old markers or the handles' text forms equal no type.
        (
            "spelled",
            Exact(
                "falsefalsetruenf-attachment/net/neoforged/neoforge/attachment/AttachmentType/064,falsefalsetruenf-attachment/net/neoforged/neoforge/attachment/AttachmentType$Builder/072,falsefalsetruehnatt:07,falsefalsetruehnattb:08,0",
            ),
        ),
        (
            "encoded",
            Exact(
                "truejava.lang.String1053459537falsefalse,truejava.lang.String-1702453923falsefalse,7",
            ),
        ),
        // `word/rest` text is still not told from a marker before #156 P17: counted, with
        // Java's answer.
        ("spelledTypes", CountedAs("falsefalse,falsefalse,")),
        // Two types and a builder: exact class and Class surface, identity, the equality paths,
        // `isAssignableFrom`/`isInstance`.
        (
            "real",
            Exact(
                "truetruefalsefalsefalsefalse|net.neoforged.neoforge.attachment.AttachmentType|net.neoforged.neoforge.attachment.AttachmentType$Builder|java.lang.Objectjava.lang.Objectfalse|AttachmentType|falsefalsetrue2|falsetrue01|truefalse",
            ),
        ),
        // `return this`: the builder is one object through its chain.
        ("chain", Exact("truetrue1true")),
        // An interface a Mixin could add to a NeoForge class is counted; Java's case is `A`.
        ("typeSwitch", CountedAs("A")),
        // Object's text and hash (an identity hash), the nested `Builder`'s simple name (Java
        // `Builder`), `getInterfaces` (Java `0`), and no natural order (Java `CCE`): counted.
        ("text", Counted),
        ("builderSimpleName", Counted),
        ("interfaces", Counted),
        ("sorted", Counted),
    ];
    let mut failures = Vec::new();
    for (method, outcome) in &cases {
        let mut host = StdHost::new();
        let answer = jar()
            .invoke_method_with_args(
                "demo/HostAttachments",
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

/// The bridge models a player's attachments only: a level's or a chunk's `getData`/`setData`/
/// `hasData`/`removeData` with a real type is counted or refused (the façade's error), never
/// answered.
#[test]
fn a_non_player_holder_call_is_counted() {
    let mut host = StdHost::new();
    let made = crate::host_attachment::mint(HostKind::AttachmentType);
    for (name, descriptor, args) in [
        (
            "getData",
            "(Lnet/neoforged/neoforge/attachment/AttachmentType;)Ljava/lang/Object;",
            vec![made.clone()],
        ),
        (
            "setData",
            "(Lnet/neoforged/neoforge/attachment/AttachmentType;Ljava/lang/Object;)Ljava/lang/Object;",
            vec![made.clone(), Value::Int(1)],
        ),
        (
            "hasData",
            "(Lnet/neoforged/neoforge/attachment/AttachmentType;)Z",
            vec![made.clone()],
        ),
        (
            "removeData",
            "(Lnet/neoforged/neoforge/attachment/AttachmentType;)Ljava/lang/Object;",
            vec![made.clone()],
        ),
    ] {
        for owner in [
            "net/minecraft/world/level/Level",
            "net/minecraft/world/level/chunk/LevelChunk",
        ] {
            let before: usize = host.unknown_host_calls().iter().map(|(_, n)| n).sum();
            let r = MethodRef {
                owner: owner.into(),
                name: name.into(),
                descriptor: descriptor.into(),
            };
            let receiver = Value::Ref(Some("game/level/minecraft:overworld".into()));
            let answer = crate::Host::invoke_instance(&mut host, &r, &receiver, &args);
            let after: usize = host.unknown_host_calls().iter().map(|(_, n)| n).sum();
            assert!(
                answer.is_err() || after == before + 1,
                "{owner}#{name}: {answer:?}"
            );
        }
    }
}

/// A type's key names one allocation: two built alike are two objects; a builder's chain keeps
/// one.
#[test]
fn each_type_and_builder_is_one_object() {
    let (Value::Host(a), Value::Host(b)) = (
        crate::host_attachment::mint(HostKind::AttachmentType),
        crate::host_attachment::mint(HostKind::AttachmentType),
    ) else {
        panic!("not handles")
    };
    assert_eq!(a.same_object(&b), Some(false));
    let Value::Host(again) = crate::host_attachment::handle(HostKind::AttachmentType, a.key())
    else {
        panic!("not a handle")
    };
    assert_eq!(a.same_object(&again), Some(true));
}

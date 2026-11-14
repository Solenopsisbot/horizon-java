//! The host's map entries against Java 25 (`tests/fixtures/host_map_entries`, #156 P14e).
//!
//! `demo/HostMapEntries.java` uses only the JDK, compiled here and run by
//! `oracle/HostMapEntriesOracle.java` on OpenJDK 25.0.5+8 (two runs alike), whose output is
//! every expected string below.

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
            dir.join("host_map_entries/demo/HostMapEntries.java")
                .is_file()
        })
        .expect("the fixture is in crates/horizon-native-java/tests/fixtures/host_map_entries");
        let scratch = std::env::temp_dir().join(format!(
            "horizon-native-java-host-map-entries-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&scratch);
        let classes = scratch.join("classes");
        std::fs::create_dir_all(&classes).unwrap();
        let mut sources = Vec::new();
        java_files(&fixtures.join("host_map_entries/demo"), &mut sources);
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
            "javac failed on the host map entries fixture"
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
fn map_entries_answer_as_java_25_or_count() {
    let cases = [
        // Entries in a `HashSet`, a `List`'s text, a `HashMap` key, `Map.of`, `indexOf`.
        ("collections", Exact(r#"2|[a=1, q=0]|x|{k=v=1}|1"#)),
        // #212: a node whose key was removed keeps its value; `setValue` updates it, not the map.
        ("detached", Exact(r#"1,2,{b=2},a=2,99"#)),
        // A `HashMap` node: its class, write-through `setValue`, `entrySet().contains`, `copyOf` a holder.
        (
            "hashNode",
            Exact(
                r#"java.util.HashMap$Node|a=10|1|{a=10, b=2}|107|truetruetrue|java.util.KeyValueHolder"#,
            ),
        ),
        // `Map.entry`: `KeyValueHolder`, `k=v`, `hash(k) ^ hash(v)`, equals a `SimpleEntry` both ways, immutable, no `null`.
        (
            "holder",
            Exact(
                r#"a=b|java.util.KeyValueHolder|3,3|truetruefalsefalse|java.lang.UnsupportedOperationException: not supported|java.lang.NullPointerException: null|java.lang.NullPointerException: null|trueab"#,
            ),
        ),
        // Two `Map.entry` calls are two objects; `copyOf` of a holder is itself.
        (
            "holderIdentity",
            Exact(r#"falsetrue2truetrue|java.util.KeyValueHoldertrue"#),
        ),
        // `LinkedHashMap` nodes, in order, written through.
        (
            "linked",
            Exact(
                r#"java.util.LinkedHashMap$Entryz=11;java.util.LinkedHashMap$Entryy=22;|{z=10, y=20}"#,
            ),
        ),
        // A node sees a later `put` of its key and writes through after other keys were added.
        ("livePut", Exact(r#"5|a=5|5|{a=6, b=2, c=3}"#)),
        // Two iterators' first entry is one node, for `HashMap`, `LinkedHashMap` and `TreeMap`.
        ("nodeIdentity", Exact(r#"true1,true1,true1,"#)),
        // A `null` key and value.
        ("nulls", Exact(r#"null=null|0|true|nullnull"#)),
        // A removed key put again is a new node.
        ("reput", Exact(r#"1,false,a=7,{a=7, b=2},false"#)),
        // `Map.Entry.comparingByValue/Key`; two `entrySet()` copies share the nodes.
        ("sorting", Exact(r#"[y=1, z=2, x=3]|[x=3, y=1, z=2]|true"#)),
        // A String spelled like the old marker is a String (class, `instanceof`, `equals`, hash, `HashMap` key).
        (
            "spelled",
            Exact(r#"java.lang.Stringfalsefalsefalse160235519|map-entry/ra,rb|115|2"#),
        ),
        // A String spelled like an entry handle's key is a String.
        (
            "spelledKey",
            Exact(
                r#"java.lang.Stringfalsefalse1700335908|java.lang.Stringfalsejava/util/HashMap$Node#node2|59"#,
            ),
        ),
        // `TreeMap` nodes write through; `firstEntry`/`ceilingEntry` are immutable snapshots, new per call; `pollFirstEntry`.
        (
            "tree",
            Exact(
                r#"java.util.TreeMap$Entry,java.util.AbstractMap$SimpleImmutableEntry|{a=9, b=2}|a=1|java.lang.UnsupportedOperationException: null|falsetruetrue|b=2null|a=9{b=2}"#,
            ),
        ),
        // `TreeMap.deleteEntry` of a node with two children: the successor's mapping moves into it, the successor's node is detached.
        (
            "treeDelete",
            Exact(
                r#"[a=0, b=1, c=2, e=4, e=4, f=5, g=6]|{a=0, b=1, c=2, e=4, f=5, g=6}|44|truefalse|4{a=0, b=1, c=2, e=30, f=5, g=6}"#,
            ),
        ),
        // A bin of colliding keys turns into a tree, replacing its nodes: counted.
        ("treeified", Counted),
        // A type switch `case Map.Entry`, `Class.isInstance`, no `Comparable`, `isAssignableFrom`.
        ("types", Exact(r#"entry:a|truetruefalse|true"#)),
        // A key removed through `keySet()` (the view's `remove` is not modelled): counted.
        ("viewRemove", Counted),
        // `ConcurrentHashMap`: a new `MapEntry` per `next()` keeping its own value; `setValue` sets it and `put`s; no `null`.
        (
            "concurrentEntries",
            Exact(
                r#"java.util.concurrent.ConcurrentHashMap$MapEntry,falsefalse,1,1,7,a=7,java.lang.NullPointerException: null,7"#,
            ),
        ),
        // `EnumMap`: a new entry per `next()` reading and writing the map; `null` allowed.
        (
            "enumMapEntries",
            Exact(r#"java.util.EnumMap$EntryIterator$Entry,falsetrue,5,5,7,LEFT=7,7,null"#),
        ),
        // `Hashtable`: its nodes; no `null`.
        (
            "hashtableEntries",
            Exact(
                r#"java.util.Hashtable$Entry,truetrue,5,5,7,a=7,java.lang.NullPointerException: null,7"#,
            ),
        ),
        // `IdentityHashMap`: a new entry per `next()` reading and writing the map (its own
        // `get`/`put` of `String` keys, whose identity is unknown, are counted).
        ("identityEntries", Counted),
        // `ConcurrentSkipListMap`: immutable snapshots, new per `next()`.
        (
            "skipListEntries",
            Exact(
                r#"java.util.AbstractMap$SimpleImmutableEntry,a=1,java.lang.UnsupportedOperationException: null,false"#,
            ),
        ),
        // `WeakHashMap`: its nodes.
        (
            "weakEntries",
            Exact(r#"java.util.WeakHashMap$Entry,truetrue,5,5,7,a=7,7,null"#),
        ),
        // `IdentityHashMap` entries compare key and value by `==`; record keys and values
        // make identity decidable (#221 review B1).
        ("identityKeyed", Exact(r#"falsetruefalse|falsetrue|true"#)),
        // Their hash is `System.identityHashCode(k) ^ System.identityHashCode(v)`: counted.
        ("identityHash", Counted),
        // A map caches its `entrySet()` view: one object per map, following the map (B2).
        ("entrySetSame", Exact(r#"truetruetrue3[a=1, b=2, c=3]"#)),
        // An unmodifiable view's entry reads the wrapped map's node live.
        ("unmodifiableLive", Exact(r#"2|a=2"#)),
    ];
    let mut failures = Vec::new();
    for (method, outcome) in &cases {
        let mut host = StdHost::new();
        let answer = jar()
            .invoke_method_with_args(
                "demo/HostMapEntries",
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

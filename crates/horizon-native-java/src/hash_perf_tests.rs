//! A hash collection member costs the same at any size (#203).
//!
//! Before #203 every `HashMap`/`HashSet` member reconciled the bucket table with the host's
//! list (an O(n) pass, O(n²) with linear table lookups) and copied the list to read one value:
//! about 1 µs per entry per call, so 1M `merge`s into 1000 keys took 18 minutes. Now a member
//! reads and writes the list in place at its key's position while the table mirrors the list
//! (`collection_store.rs`, `hash_table.rs`). These loops run the same number of calls on a small
//! and a large map: the large one may not cost more than a few times the small one (an O(n)
//! member would cost ~60× at these sizes), under a bound generous enough for a debug build on a
//! loaded machine. On the `fast` profile 1M merges into 1000 keys take about 7 s.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use crate::{NativeJar, StdHost, Value};

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
        .find(|dir| dir.join("hash_perf/demo/HashPerf.java").is_file())
        .expect("the fixture is in crates/horizon-native-java/tests/fixtures/hash_perf");
        let scratch = std::env::temp_dir().join(format!(
            "horizon-native-java-hash-perf-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&scratch);
        let classes = scratch.join("classes");
        std::fs::create_dir_all(&classes).unwrap();
        let mut sources = Vec::new();
        java_files(&fixtures.join("hash_perf/demo"), &mut sources);
        let javac = std::env::var_os("JAVA_HOME")
            .map(|home| PathBuf::from(home).join("bin/javac"))
            .unwrap_or_else(|| "javac".into());
        let status = Command::new(javac)
            .args(["--release", "21", "-nowarn", "-d"])
            .arg(&classes)
            .args(&sources)
            .status()
            .expect("javac runs");
        assert!(status.success(), "javac failed on the hash perf fixture");
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

/// Run `demo/HashPerf.<method>(ops, keys)`: its answer and how long it took.
fn timed(method: &str, ops: i32, keys: i32) -> (Value, Duration) {
    let mut host = StdHost::new();
    let start = Instant::now();
    let answer = jar()
        .invoke_method_with_args(
            "demo/HashPerf",
            method,
            "(II)I",
            None,
            &[Value::Int(ops), Value::Int(keys)],
            &mut host,
        )
        .unwrap();
    assert!(
        host.unknown_host_calls().is_empty(),
        "{method}: {:?}",
        host.unknown_host_calls()
    );
    (answer, start.elapsed())
}

#[test]
fn a_hash_member_costs_the_same_at_any_size() {
    const OPS: i32 = 20_000;
    // Java 25's answers (`ops`, `keys`): 64 and 4096 keys.
    for (method, small_answer, large_answer) in [
        ("merges", 2297, 126_981),
        ("putGets", 199_990_064, 199_994_096),
        ("setOps", 20_064, 24_096),
    ] {
        let (small, small_time) = timed(method, OPS, 64);
        let (large, large_time) = timed(method, OPS, 4096);
        assert_eq!(small, Value::Int(small_answer), "{method} at 64 keys");
        assert_eq!(large, Value::Int(large_answer), "{method} at 4096 keys");
        // An O(n) member would make the 4096-key loop ~60× the 64-key one.
        assert!(
            large_time < small_time * 6 + Duration::from_millis(500),
            "{method}: {small_time:?} at 64 keys, {large_time:?} at 4096"
        );
        assert!(
            large_time < Duration::from_secs(60),
            "{method}: {large_time:?} for {OPS} calls"
        );
    }
}

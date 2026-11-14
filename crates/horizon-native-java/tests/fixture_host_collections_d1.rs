//! Collections the host used to answer as text literals are objects of their
//! own (#156 P14d1). A literal was its text, so every equal literal was one
//! "object", and a write to one created a row every equal literal then shared.
//! Now a map's `keySet()`/`values()`/`entrySet()` is one object per map that
//! follows it, two maps' views are two objects, and `System.getProperties()`
//! and `System.getenv()` are one object each, the environment's
//! unmodifiable as on a JVM.
//!
//! `fixtures/host_collections_d1/demo/Probe.java` runs unchanged on both
//! sides: its own `main` on Java 25 (its output is `java25.txt`), its static
//! probes in Horizon.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

/// What `java -cp . demo.Probe` printed on Java 25 (Temurin 25.0.5, `javac
/// --release 21`), the same on two runs.
const ORACLE: &str = include_str!("fixtures/host_collections_d1/java25.txt");

const PROBE: &str = include_str!("fixtures/host_collections_d1/demo/Probe.java");

fn expected(method: &str) -> &'static str {
    ORACLE
        .lines()
        .find_map(|line| line.strip_prefix(method)?.strip_prefix('='))
        .unwrap_or_else(|| panic!("no oracle line for {method}"))
}

/// Each probe answers as Java 25 did, with nothing counted:
/// - `mapViews`: for a `HashMap`, a `LinkedHashMap` and a `TreeMap`, each
///   view is the same object on every call and sees a later `put`; another
///   map's equal view is equal but not the same object;
/// - `system`: `getProperties()` and `getenv()` are one object each;
///   `getenv()` is a `Collections$UnmodifiableMap` whose `put` throws.
#[test]
fn host_answered_collections_are_objects() {
    let fixture =
        javac_fixture::Fixture::build("host-collections-d1", &[("demo/Probe.java", PROBE)]);
    let mut failures = Vec::new();
    for method in ["mapViews", "system"] {
        let mut host = fixture.jar.seed_host(horizon_native_java::StdHost::new());
        let answer = fixture.jar.invoke_method_with_args(
            "demo/Probe",
            method,
            "()Ljava/lang/String;",
            None,
            &[],
            &mut host,
        );
        let text = match answer {
            Ok(horizon_native_java::Value::Ref(Some(text))) => text,
            other => format!("{other:?}"),
        };
        let unknown = host.unknown_host_calls();
        if text != expected(method) || !unknown.is_empty() {
            failures.push(format!(
                "{method}\n  java:   {}\n  native: {text}\n  unknown: {unknown:?}",
                expected(method)
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

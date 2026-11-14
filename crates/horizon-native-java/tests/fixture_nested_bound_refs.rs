//! A bound method reference called through another bound reference throws
//! what the inner call throws, at any depth (#224): `Supplier n = it::next;
//! cls(n::get)` past the end throws `NoSuchElementException`, exactly as the
//! lambda form `() -> n.get()` and the direct call do. The bound-callback
//! path returned normally instead, silently.
//!
//! `fixtures/nested_bound_refs/demo/Probe.java` runs unchanged on both sides:
//! its own `main` on Java 25 (its output is `java25.txt`), its static probes
//! in Horizon.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

/// What `java -cp . demo.Probe` printed on Java 25 (Temurin 25.0.5, `javac
/// --release 21`), the same on two runs.
const ORACLE: &str = include_str!("fixtures/nested_bound_refs/java25.txt");

const PROBE: &str = include_str!("fixtures/nested_bound_refs/demo/Probe.java");

fn expected(method: &str) -> &'static str {
    ORACLE
        .lines()
        .find_map(|line| line.strip_prefix(method)?.strip_prefix('='))
        .unwrap_or_else(|| panic!("no oracle line for {method}"))
}

/// Each probe answers as Java 25 did, with nothing counted:
/// - `iterator`: `it::next` past the end through `n::get`, `() -> n.get()`
///   and `n` itself;
/// - `listIterator`: a list iterator's `li::previous` at its start, the same
///   three ways;
/// - `checked`: a bound reference to an instance method throwing
///   `IOException` and a static one throwing `Exception`, each through a
///   `Callable` bound to a `Callable`, a lambda and directly;
/// - `nested`: two and three levels of bound references over the throwing
///   call, and over a normal one (which answers, in order).
#[test]
fn a_bound_reference_through_a_bound_reference_throws_what_the_inner_call_throws() {
    let fixture = javac_fixture::Fixture::build("nested-bound-refs", &[("demo/Probe.java", PROBE)]);
    let mut failures = Vec::new();
    for method in ["iterator", "listIterator", "checked", "nested"] {
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

//! Guava's `Joiner.join(Iterator)` walks the iterator through its own `hasNext`/`next` (#225):
//! it consumes the iterator, fails fast as that iterator does (#222's guard), runs a jar's own
//! iterator, and stops at a `null` part where Java's `toString(part)` throws.
//!
//! `fixtures/joiner_iterator/demo/Probe.java` runs unchanged on both sides: its own `main` on
//! Java 25 against the real Guava 33.6.0 the 26.3 game ships (its output is `java25.txt`), its
//! static probes in Horizon. The Guava stub here only lets `javac` compile the probe and is never
//! packed.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

/// What `java -cp .:guava-33.6.0-jre.jar demo.Probe` printed on OpenJDK 25.0.5+8
/// (`javac --release 21`), the same on two runs.
const ORACLE: &str = include_str!("fixtures/joiner_iterator/java25.txt");

const PROBE: &str = include_str!("fixtures/joiner_iterator/demo/Probe.java");

const JOINER_STUB: &str = r#"package com.google.common.base;

public class Joiner {
    public static Joiner on(String separator) { throw new AssertionError(); }
    public final String join(Iterable<?> parts) { throw new AssertionError(); }
    public final String join(java.util.Iterator<?> parts) { throw new AssertionError(); }
}
"#;

fn expected(method: &str) -> &'static str {
    ORACLE
        .lines()
        .find_map(|line| line.strip_prefix(method)?.strip_prefix('='))
        .unwrap_or_else(|| panic!("no oracle line for {method}"))
}

/// Every probe answers as Java 25 with Guava 33.6.0 did, with nothing counted. On main the
/// iterator stayed where it was (`consumes`: `true`), a changed list joined without
/// `ConcurrentModificationException`, and a jar's own iterator was counted.
#[test]
fn joiner_walks_the_iterator_it_joins() {
    let fixture = javac_fixture::Fixture::build(
        "joiner-iterator",
        &[
            ("demo/Probe.java", PROBE),
            ("com/google/common/base/Joiner.java", JOINER_STUB),
        ],
    );
    let mut failures = Vec::new();
    for method in ["consumes", "failsFast", "jarIterator", "nullPart"] {
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

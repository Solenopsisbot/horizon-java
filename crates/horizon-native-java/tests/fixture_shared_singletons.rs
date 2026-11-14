//! Objects the JDK and Guava share stay one object in Horizon (#222's review):
//! every `Collections` empty collection's iterator (`emptyList()`, `emptySet()`,
//! and `emptyMap()`'s `keySet()`/`entrySet()`/`values()`, all `emptySet()`) is
//! `Collections.emptyIterator()`'s one object, and Guava's `toImmutableList()`/
//! `toImmutableSet()` are its `TO_IMMUTABLE_LIST`/`TO_IMMUTABLE_SET` constants,
//! while `Collectors.toList()` and `List.of().iterator()` are new each call.
//! `toImmutableSet()` keeps each element once, in encounter order.
//!
//! `fixtures/shared_singletons/demo/Probe.java` runs unchanged on both sides:
//! its own `main` on Java 25 against the real Guava 33.6.0 the 26.3 game ships
//! (its output is `java25.txt`), its static probes in Horizon. The Guava stubs
//! here only let `javac` compile the probe and are never packed.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

/// What `java -cp .:guava-33.6.0-jre.jar demo.Probe` printed on OpenJDK
/// 25.0.5+8 (`javac --release 21`), the same on two runs.
const ORACLE: &str = include_str!("fixtures/shared_singletons/java25.txt");

const PROBE: &str = include_str!("fixtures/shared_singletons/demo/Probe.java");

const IMMUTABLE_LIST_STUB: &str = r#"package com.google.common.collect;

public abstract class ImmutableList<E> extends java.util.AbstractList<E> {
    public static <E> java.util.stream.Collector<E, ?, ImmutableList<E>> toImmutableList() {
        throw new AssertionError();
    }
}
"#;

const IMMUTABLE_SET_STUB: &str = r#"package com.google.common.collect;

public abstract class ImmutableSet<E> extends java.util.AbstractSet<E> {
    public static <E> java.util.stream.Collector<E, ?, ImmutableSet<E>> toImmutableSet() {
        throw new AssertionError();
    }
}
"#;

fn expected(method: &str) -> &'static str {
    ORACLE
        .lines()
        .find_map(|line| line.strip_prefix(method)?.strip_prefix('='))
        .unwrap_or_else(|| panic!("no oracle line for {method}"))
}

/// Every probe answers as Java 25 did, with nothing counted. On #222's first
/// head, `emptyIterators` and `collectors` answered `false` where Java shares
/// one object, and `immutableSet` kept the duplicates. (A Guava set's
/// `toString` is still counted, as on main, so the probe iterates it.)
#[test]
fn shared_iterators_and_collectors_stay_one_object() {
    let fixture = javac_fixture::Fixture::build(
        "shared-singletons",
        &[
            ("demo/Probe.java", PROBE),
            (
                "com/google/common/collect/ImmutableList.java",
                IMMUTABLE_LIST_STUB,
            ),
            (
                "com/google/common/collect/ImmutableSet.java",
                IMMUTABLE_SET_STUB,
            ),
        ],
    );
    let mut failures = Vec::new();
    for method in [
        "emptyIterators",
        "freshIterators",
        "collectors",
        "immutableSet",
    ] {
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

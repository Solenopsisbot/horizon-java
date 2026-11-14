//! Guava's immutable collections the host makes print, compare and hash as Guava's do (#223):
//! `ImmutableCollection` extends `AbstractCollection`, so `toString` is `[e1, e2]` with each
//! element's own text, and an `ImmutableMap` prints `{k=v}`; `equals`/`hashCode` follow the
//! `List`/`Set`/`Map` contracts against the JDK's own collections.
//!
//! `fixtures/guava_text_forms/demo/Probe.java` runs unchanged on both sides: its own `main` on
//! Java 25 against the real Guava 33.6.0 the 26.3 game ships (its output is `java25.txt`), its
//! static probes in Horizon. The Guava stubs here only let `javac` compile the probe and are never
//! packed.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

/// What `java -cp .:guava-33.6.0-jre.jar demo.Probe` printed on OpenJDK 25.0.5+8
/// (`javac --release 21`), the same on two runs.
const ORACLE: &str = include_str!("fixtures/guava_text_forms/java25.txt");

const PROBE: &str = include_str!("fixtures/guava_text_forms/demo/Probe.java");

const IMMUTABLE_LIST_STUB: &str = r#"package com.google.common.collect;

public abstract class ImmutableList<E> extends java.util.AbstractList<E> {
    public static <E> ImmutableList<E> of() { throw new AssertionError(); }
    public static <E> ImmutableList<E> of(E e1) { throw new AssertionError(); }
    public static <E> ImmutableList<E> of(E e1, E e2) { throw new AssertionError(); }
    public static <E> ImmutableList<E> copyOf(java.util.Collection<? extends E> elements) {
        throw new AssertionError();
    }
    public static <E> java.util.stream.Collector<E, ?, ImmutableList<E>> toImmutableList() {
        throw new AssertionError();
    }
}
"#;

const IMMUTABLE_SET_STUB: &str = r#"package com.google.common.collect;

public abstract class ImmutableSet<E> extends java.util.AbstractSet<E> {
    public static <E> ImmutableSet<E> of() { throw new AssertionError(); }
    public static <E> ImmutableSet<E> of(E e1) { throw new AssertionError(); }
    public static <E> ImmutableSet<E> of(E e1, E e2) { throw new AssertionError(); }
    public static <E> java.util.stream.Collector<E, ?, ImmutableSet<E>> toImmutableSet() {
        throw new AssertionError();
    }
}
"#;

const IMMUTABLE_MAP_STUB: &str = r#"package com.google.common.collect;

public abstract class ImmutableMap<K, V> extends java.util.AbstractMap<K, V> {
    public static <K, V> ImmutableMap<K, V> of() { throw new AssertionError(); }
    public static <K, V> ImmutableMap<K, V> of(K k1, V v1) { throw new AssertionError(); }
    public static <K, V> ImmutableMap<K, V> of(K k1, V v1, K k2, V v2) { throw new AssertionError(); }
}
"#;

fn expected(method: &str) -> &'static str {
    ORACLE
        .lines()
        .find_map(|line| line.strip_prefix(method)?.strip_prefix('='))
        .unwrap_or_else(|| panic!("no oracle line for {method}"))
}

/// Every probe answers as Java 25 with Guava 33.6.0 did, with nothing counted.
#[test]
fn guava_collections_print_compare_and_hash_as_guava() {
    let fixture = javac_fixture::Fixture::build(
        "guava-text-forms",
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
            (
                "com/google/common/collect/ImmutableMap.java",
                IMMUTABLE_MAP_STUB,
            ),
        ],
    );
    let mut failures = Vec::new();
    for method in ["listText", "nested", "setAndMapText", "jdkEquality"] {
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

/// A jar object's own `toString` inside a host collection is the jar's code, which the host's
/// collection text cannot run: counted, for Guava's collections as for the JDK's, never printed
/// silently.
#[test]
fn a_jar_element_inside_is_counted() {
    let fixture = javac_fixture::Fixture::build(
        "guava-text-forms-jar-elements",
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
            (
                "com/google/common/collect/ImmutableMap.java",
                IMMUTABLE_MAP_STUB,
            ),
        ],
    );
    let mut host = fixture.jar.seed_host(horizon_native_java::StdHost::new());
    let answer = fixture.jar.invoke_method_with_args(
        "demo/Probe",
        "jarElements",
        "()Ljava/lang/String;",
        None,
        &[],
        &mut host,
    );
    assert!(answer.is_ok(), "{answer:?}");
    assert_eq!(expected("jarElements"), "[P[x=1]]|[P[x=2]]");
    assert!(
        !host.unknown_host_calls().is_empty(),
        "the jar elements' text is counted"
    );
}

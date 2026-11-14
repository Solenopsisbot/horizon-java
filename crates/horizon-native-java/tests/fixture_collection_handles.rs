//! Collections the host makes are objects of their own (#156 P14c): an
//! unmodifiable collection is the class Java 25 makes for its factory
//! (`List12`/`ListN`, `Set12`/`SetN`, `Map1`/`MapN`, `Collections$EmptyList`,
//! `Collections$SingletonMap`, the unmodifiable views, Guava's
//! `RegularImmutableList`/`SingletonImmutableList`); `==` tells two apart while
//! the JDK's shared empties stay one object each; a write to one host-made list
//! never shows in another; and a `String` spelled like the host's old marker
//! text (`java/util/ArrayList#neutral0`, `…ListN#immutable0`) is a `String`.
//!
//! `fixtures/collection_handles/demo/Probe.java` runs unchanged on both sides:
//! its own `main` on Java 25 against the real Guava 33.6.0 the 26.3 game ships
//! (its output is `java25.txt`), its static probes in Horizon. The Guava stubs
//! here only let `javac` compile the probe and are never packed.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

/// What `java -cp .:guava-33.6.0-jre.jar demo.Probe` printed on Java 25
/// (Temurin 25.0.5, `javac --release 21`), the same on two runs.
const ORACLE: &str = include_str!("fixtures/collection_handles/java25.txt");

const PROBE: &str = include_str!("fixtures/collection_handles/demo/Probe.java");

const IMMUTABLE_LIST_STUB: &str = r#"package com.google.common.collect;

public abstract class ImmutableList<E> extends java.util.AbstractList<E> {
    public static <E> ImmutableList<E> of() { throw new AssertionError(); }
    public static <E> ImmutableList<E> of(E e1) { throw new AssertionError(); }
    public static <E> ImmutableList<E> of(E e1, E e2) { throw new AssertionError(); }
    public static <E> ImmutableList<E> copyOf(java.util.Collection<? extends E> elements) {
        throw new AssertionError();
    }
}
"#;

const IMMUTABLE_SET_STUB: &str = r#"package com.google.common.collect;

public abstract class ImmutableSet<E> extends java.util.AbstractSet<E> {
    public static <E> ImmutableSet<E> copyOf(java.util.Collection<? extends E> elements) {
        throw new AssertionError();
    }
}
"#;

const LISTS_STUB: &str = r#"package com.google.common.collect;

public final class Lists {
    public static <E> java.util.ArrayList<E> newArrayList(Iterable<? extends E> elements) {
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

/// Every probe answers as Java 25 did, with nothing counted:
/// - `jdkClasses`/`guavaClasses`: `getClass()` of each factory's answer;
/// - `behaves`: reads, `instanceof`, a refused mutator and a view reading
///   through its source, as before;
/// - `identity`: two `List.of(1)` are two objects (`==`, `IdentityHashMap`)
///   and equal (`HashMap`); two views of one source are two objects; each
///   shared empty is one object, and `List.of()` is not `emptyList()` nor a
///   `Stream.toList()`;
/// - `writes`: two `Lists.newArrayList(..)` never share a write;
/// - `strings`: a `String` spelling a host-made collection's key, the live
///   collections' keys among them, is a `String` (`getClass`, `instanceof`,
///   `equals` both ways, `hashCode`, a `HashMap` key, `valueOf`).
#[test]
fn host_made_collections_are_objects_of_java_25s_classes() {
    let fixture = javac_fixture::Fixture::build(
        "collection-handles",
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
            ("com/google/common/collect/Lists.java", LISTS_STUB),
        ],
    );
    let mut failures = Vec::new();
    for method in [
        "jdkClasses",
        "guavaClasses",
        "behaves",
        "identity",
        "writes",
        "strings",
    ] {
        let mut host = fixture.jar.seed_host(horizon_native_java::StdHost::new());
        let text = fixture.text_on(&mut host, method);
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

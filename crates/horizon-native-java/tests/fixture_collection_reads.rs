//! A host collection hands back the `Value`s it stored (#156 P14a): one
//! object put into a list, deque, queue, sorted set or map and read back on any
//! path is that object, so `==` and `IdentityHashMap` answer as Java 25 does.
//! Main read every collection as its elements' encoded text and decoded it
//! again, which made a handle a new construction on each read (identity
//! counted).
//!
//! `fixtures/collection_reads/demo/Reads.java` runs unchanged on both sides:
//! its own `main` on Java 25, its static probes in Horizon (`demo.Probe`).

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

/// What `java -cp . demo.Reads` printed on Java 25 (Temurin 25.0.5, `javac
/// --release 21`), the same on two runs.
const ORACLE: &str = "\
sequences=TTTTTTTTTTTTTT1c
maps=TTTTtrue
";

const READS: &str = include_str!("fixtures/collection_reads/demo/Reads.java");
const PROBE: &str = include_str!("fixtures/collection_reads/demo/Probe.java");

fn expected(method: &str) -> &'static str {
    ORACLE
        .lines()
        .find_map(|line| line.strip_prefix(method)?.strip_prefix('='))
        .unwrap_or_else(|| panic!("no oracle line for {method}"))
}

/// One `Path` (a handle no key makes canonical) read back through every
/// path: `get`, `getLast`, an iterator, a linked list's `getFirst`, a deque, a priority
/// queue, a tree set, an unmodifiable view, `subList`, a copy, `toArray`,
/// `reversed()`, a stream's `toList()` (a lambda's captures are #156 P15's, an
/// `Optional`'s held value is its key, P0's); a hash map's `get` and
/// `getOrDefault`, a tree map's first key and a value, and `IdentityHashMap`
/// keyed by what the map handed back. Every read is the stored object, as on
/// Java 25, with nothing counted. (A map view's copy and an entry are inline
/// text until #156 P14c-P14e.)
#[test]
fn a_collection_hands_back_the_object_it_stored() {
    for method in ["sequences", "maps"] {
        let fixture = javac_fixture::Fixture::build(
            method,
            &[("demo/Reads.java", READS), ("demo/Probe.java", PROBE)],
        );
        let mut host = fixture.jar.seed_host(horizon_native_java::StdHost::new());
        let text = fixture.text_on(&mut host, method);
        assert_eq!(
            text,
            expected(method),
            "{method}: {:?}",
            host.unknown_host_calls()
        );
        assert_eq!(
            host.unknown_host_calls(),
            Vec::<(String, usize)>::new(),
            "{method}"
        );
    }
}

//! A hash collection keeps the keys it was given (#156 P14b): the same
//! object put into a `HashSet`, `HashMap`, `Set.of`/`Map.of` or
//! `IdentityHashMap` twice is one key, as on Java 25. Main kept a hash table's
//! and a probe table's keys as encoded text and decoded them for each lookup,
//! so a stored handle came back a new construction: `p.key == key` could not
//! be proved for a handle with an identity hash (a `MethodHandle`), and the
//! second `add`, `put` and `Set.of(h, h)` were counted (#175's cost).
//!
//! `fixtures/hash_identity/demo/Identity.java` runs unchanged on both sides:
//! its own `main` on Java 25, its static probes in Horizon (`demo.Probe`).

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

/// What `java -cp . demo.Identity` printed on Java 25 (Temurin 25.0.5,
/// `javac --release 21`), the same on two runs.
const ORACLE: &str = "\
tables=TFTF2T11/2/2/1T1F
probes=IIII23TT8T
identity=1/2/1T20
separate=FT22
";

const IDENTITY: &str = include_str!("fixtures/hash_identity/demo/Identity.java");
const PROBE: &str = include_str!("fixtures/hash_identity/demo/Probe.java");

fn expected(method: &str) -> &'static str {
    ORACLE
        .lines()
        .find_map(|line| line.strip_prefix(method)?.strip_prefix('='))
        .unwrap_or_else(|| panic!("no oracle line for {method}"))
}

fn run(method: &str) -> (String, Vec<(String, usize)>) {
    let fixture = javac_fixture::Fixture::build(
        method,
        &[("demo/Identity.java", IDENTITY), ("demo/Probe.java", PROBE)],
    );
    let mut host = fixture.jar.seed_host(horizon_native_java::StdHost::new());
    let text = fixture.text_on(&mut host, method);
    (text, host.unknown_host_calls())
}

/// One `MethodHandle` (identity `equals` and hash) and one `Path` (value
/// hash), each added twice: `HashSet`/`LinkedHashSet` keep one, a second
/// `HashMap.put` answers the first value, `computeIfAbsent` finds it, and
/// `remove` takes it out. `Set.of(h, h)`, `Set.of(h, p, h)`, `Map.of(h, 1, h,
/// 2)` and `Map.of(h, 1, p, 2, h, 3)` throw "duplicate element"/"duplicate
/// key"; `Set12`, `SetN` and `MapN` find the handle. `IdentityHashMap.put`
/// twice is one entry. Every answer is Java 25's, with nothing counted.
#[test]
fn the_same_object_added_twice_is_one_key() {
    for method in ["tables", "probes", "identity"] {
        let (text, unknown) = run(method);
        assert_eq!(text, expected(method), "{method}: {unknown:?}");
        assert_eq!(unknown, Vec::<(String, usize)>::new(), "{method}");
    }
}

/// Two handles made by separate `findStatic` calls are two objects on Java 25
/// (`h == g` is false, both are kept, `Set.of(h, g)` has two). Their equal
/// keys never prove either answer here, so the second `add` is counted (#175),
/// never merged by key.
#[test]
fn separately_made_handles_stay_counted() {
    assert_eq!(expected("separate"), "FT22");
    let (_, unknown) = run("separate");
    assert!(!unknown.is_empty(), "separate: nothing counted");
}

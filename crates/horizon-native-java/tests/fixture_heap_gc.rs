//! A jar's objects are collected between callbacks once nothing reaches
//! them (#197). Every object a jar made used to stay a root through the
//! host's call log (the `Object.<init>` each `new` logs), so a callback that
//! allocated one object with a field died at its 65,536th run, the resident
//! heap's bound on live fields: about 55 minutes of one allocation per tick.
//!
//! `fixtures/heap_gc/demo/Probe.java` runs unchanged on both sides: its own
//! `main` on Java 25 (`setup`, 100,000 `tick`s, `check`), the same calls one
//! by one through one Horizon host.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::{StdHost, Value};

/// What `java -cp . demo.Probe` printed on Java 25 (Temurin 25.0.5, `javac
/// --release 21`), the same on two runs.
const ORACLE: &str =
    "100000|705082704|7shared8second9third|truetruetrueid|11capturedtrue|12viewed|13frozen";

const PROBE: &str = include_str!("fixtures/heap_gc/demo/Probe.java");

/// Callbacks run: well past the old 65,536-object limit.
const TICKS: usize = 100_000;

fn call(fixture: &javac_fixture::Fixture, host: &mut StdHost, method: &str) -> String {
    match fixture.jar.invoke_method_with_args(
        "demo/Probe",
        method,
        "()Ljava/lang/String;",
        None,
        &[],
        host,
    ) {
        Ok(Value::Ref(Some(text))) => text,
        other => panic!("{method}: {other:?}"),
    }
}

/// `setup` keeps one object through a static, a `HashMap`, an `Object[]` and
/// an `IdentityHashMap` key, a chain behind it reached only through its
/// fields, an object a lambda captured, one behind an unmodifiable view of a
/// jar list and one in a `List.of`. Then 100,000 callbacks each allocate two
/// objects with fields and drop them. `check` reads every kept object's
/// fields and compares the references to the shared one with `==` and the
/// `IdentityHashMap`: Java 25's answer, nothing counted. Collections ran
/// between the callbacks (the live fields stay far below the run's
/// allocations), and the long run's time is printed.
#[test]
fn objects_nothing_reaches_are_collected_and_kept_ones_stay_identical() {
    let fixture = javac_fixture::Fixture::build("heap-gc", &[("demo/Probe.java", PROBE)]);
    let mut host = fixture.jar.seed_host(StdHost::new());
    assert_eq!(call(&fixture, &mut host, "setup"), "ok");
    let start = std::time::Instant::now();
    let mut most = 0;
    for _ in 0..TICKS {
        assert_eq!(call(&fixture, &mut host, "tick"), "t");
        most = most.max(host.resident_heap().instance_fields());
    }
    let elapsed = start.elapsed();
    println!(
        "{TICKS} allocating callbacks in {elapsed:?} ({:?} each); at most {most} live fields",
        elapsed / TICKS as u32
    );
    assert_eq!(call(&fixture, &mut host, "check"), ORACLE);
    assert_eq!(host.unknown_host_calls(), Vec::<(String, usize)>::new());
    // Two objects with three fields each per callback: without collection
    // the heap would hold about 600,000 fields.
    assert!(most < 5_000, "{most} live fields: nothing was collected");
}

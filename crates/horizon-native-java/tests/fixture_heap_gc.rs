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

/// Java 25's `held=` line: the objects the caller kept across the run.
const HELD: &str = "41held|42retlist|43retlambda";

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

/// `demo.Probe.method()` answering an object the caller keeps.
fn make(fixture: &javac_fixture::Fixture, host: &mut StdHost, method: &str) -> Value {
    fixture
        .jar
        .invoke_method_with_args(
            "demo/Probe",
            method,
            "()Ljava/lang/Object;",
            None,
            &[],
            host,
        )
        .unwrap_or_else(|error| panic!("{method}: {error}"))
}

/// `demo.Probe.method(value)` reading an object the caller kept.
fn read(
    fixture: &javac_fixture::Fixture,
    host: &mut StdHost,
    method: &str,
    value: &Value,
) -> Result<String, String> {
    match fixture.jar.invoke_method_with_args(
        "demo/Probe",
        method,
        "(Ljava/lang/Object;)Ljava/lang/String;",
        None,
        std::slice::from_ref(value),
        host,
    ) {
        Ok(Value::Ref(Some(text))) => Ok(text),
        other => Err(format!("{other:?}")),
    }
}

/// The caller gets an object back past 3,000 garbage objects and reads it
/// in the next call (Java 25: `40returned`): an invocation's result is a
/// root of its own store. It retains a held object, a returned `ArrayList`
/// and a returned bound method reference across the run (Java 25:
/// [`HELD`]). An object and a `StringBuilder` it did not retain are
/// collected, and reading either fails by name rather than answering `0null`
/// (#226 review).
///
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
    // Objects only the caller holds (#226 review B1): returned past 3,000
    // garbage objects, and kept across every later callback.
    let returned = make(&fixture, &mut host, "makeReturned");
    assert_eq!(
        read(&fixture, &mut host, "read", &returned),
        Ok("40returned".into())
    );
    // Kept across later calls: retained before the next call, as an
    // embedder holding them must (a later call's store may collect).
    let mut keep = |method: &str, host: &mut StdHost| {
        let value = make(&fixture, host, method);
        host.retain(&value);
        value
    };
    let held = keep("makeHeld", &mut host);
    let list = keep("makeReturnedList", &mut host);
    let lambda = keep("makeReturnedLambda", &mut host);
    // Not retained: collected, and then never read as defaults.
    let dropped = make(&fixture, &mut host, "makeHeld");
    let builder = make(&fixture, &mut host, "makeBuilder");
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
    let held = [
        read(&fixture, &mut host, "read", &held),
        read(&fixture, &mut host, "readList", &list),
        read(&fixture, &mut host, "readSupplier", &lambda),
    ]
    .map(Result::unwrap)
    .join("|");
    assert_eq!(held, HELD);
    for (method, value) in [("read", &dropped), ("readBuilder", &builder)] {
        let freed = read(&fixture, &mut host, method, value).unwrap_err();
        assert!(
            freed.contains("was collected while still reachable"),
            "{method}: a collected object read as {freed}"
        );
    }
    assert_eq!(host.retained_count(), 3);
    assert_eq!(call(&fixture, &mut host, "check"), ORACLE);
    assert_eq!(host.unknown_host_calls(), Vec::<(String, usize)>::new());
    // Two objects with three fields each per callback: without collection
    // the heap would hold about 600,000 fields.
    assert!(most < 5_000, "{most} live fields: nothing was collected");
}

/// An embedder calling a method that returns a new object on every call, and
/// keeping none of them, keeps nothing: each result is a root only of its
/// own call's store (#226 review: pinned for ever, they reached the bound).
#[test]
fn results_the_embedder_does_not_retain_are_collected() {
    let fixture = javac_fixture::Fixture::build("heap-gc-results", &[("demo/Probe.java", PROBE)]);
    let mut host = fixture.jar.seed_host(StdHost::new());
    let mut most = 0;
    for _ in 0..20_000 {
        make(&fixture, &mut host, "makeHeld");
        most = most.max(host.resident_heap().instance_fields());
    }
    assert_eq!(host.retained_count(), 0);
    assert!(most < 5_000, "{most} live fields: results stayed rooted");
}

//! A jar's static fields are per class for the whole run (#184): what one
//! Mixin `@Inject` callback writes, the next callback of the same target, a
//! callback on another target, the jar's entrypoint and a later plain call all
//! see, as in a JVM. `<clinit>` runs once, and objects a static reaches (a
//! list, a jar object's field) keep their state.
//!
//! `fixtures/statics_callbacks/demo/*.java` is compiled against sponge-mixin's
//! signatures (the p7 stubs, left out of the jar). The expected lines are what
//! the same classes printed on OpenJDK 25.0.5 (`javac --release 21`) driven by
//! `fixtures/statics_callbacks/Drive.java.txt`, which runs each target's HEAD
//! callbacks in Mixin's (declaration) order, then its body.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::{Host, MethodRef, StdHost, Value};
use javac_fixture::Fixture;

const SOURCES: &[(&str, &str)] = &[
    (
        "demo/Counter.java",
        include_str!("fixtures/statics_callbacks/demo/Counter.java"),
    ),
    (
        "demo/CounterMixin.java",
        include_str!("fixtures/statics_callbacks/demo/CounterMixin.java"),
    ),
    (
        "demo/Probe.java",
        include_str!("fixtures/statics_callbacks/demo/Probe.java"),
    ),
    (
        "demo/Target.java",
        include_str!("fixtures/statics_callbacks/demo/Target.java"),
    ),
    (
        "log/Probe.java",
        include_str!("fixtures/host_values/p7/log/Probe.java"),
    ),
    (
        "org/spongepowered/asm/mixin/Mixin.java",
        include_str!("fixtures/host_values/p7/stubs/org/spongepowered/asm/mixin/Mixin.java"),
    ),
    (
        "org/spongepowered/asm/mixin/injection/At.java",
        include_str!("fixtures/host_values/p7/stubs/org/spongepowered/asm/mixin/injection/At.java"),
    ),
    (
        "org/spongepowered/asm/mixin/injection/Inject.java",
        include_str!(
            "fixtures/host_values/p7/stubs/org/spongepowered/asm/mixin/injection/Inject.java"
        ),
    ),
    (
        "org/spongepowered/asm/mixin/injection/callback/CallbackInfo.java",
        include_str!(
            "fixtures/host_values/p7/stubs/org/spongepowered/asm/mixin/injection/callback/CallbackInfo.java"
        ),
    ),
    (
        "org/spongepowered/asm/mixin/injection/callback/Cancellable.java",
        include_str!(
            "fixtures/host_values/p7/stubs/org/spongepowered/asm/mixin/injection/callback/Cancellable.java"
        ),
    ),
];

fn target(name: &str) -> MethodRef {
    MethodRef {
        owner: "demo/Target".into(),
        name: name.into(),
        descriptor: "()V".into(),
    }
}

/// What `log/Probe.mark` was handed, in order.
fn marks(host: &StdHost) -> Vec<String> {
    host.invocations()
        .iter()
        .filter(|invocation| {
            invocation.method.owner == "log/Probe" && invocation.method.name == "mark"
        })
        .filter_map(|invocation| match invocation.args.first() {
            Some(Value::Ref(Some(text))) => Some(text.clone()),
            _ => None,
        })
        .collect()
}

/// Two callbacks on one target, a callback on another target, the jar's
/// entrypoint, the first target again, then a later plain call: one run.
#[test]
fn statics_carry_across_callbacks_entrypoints_and_calls_as_jdk_25_does() {
    let fixture = Fixture::build("statics-across-callbacks", SOURCES);
    let jar = &fixture.jar;
    let mut host = jar.seed_host(StdHost::new());
    jar.execute_discovered_mixins(&target("twice"), None, &[], &mut host)
        .expect("twice");
    jar.execute_discovered_mixins(&target("other"), None, &[], &mut host)
        .expect("other");
    let entry = jar.discover_entrypoint("main").expect("main entrypoint");
    jar.execute(&entry.class, entry.method.as_deref(), "main", &mut host)
        .expect("entrypoint");
    jar.execute_discovered_mixins(&target("twice"), None, &[], &mut host)
        .expect("twice again");
    let read = jar
        .invoke_method_with_args(
            "demo/Probe",
            "read",
            "()Ljava/lang/String;",
            None,
            &[],
            &mut host,
        )
        .expect("read");
    assert_eq!(
        marks(&host),
        [
            "clinit",
            "a:1",
            "b:1:[a1]:1",
            "target:1",
            "c:1:1:1",
            "other:1",
            "init:11:1",
            "a:12",
            "b:12:[a1, a12]:2",
            "target:12",
        ]
    );
    assert_eq!(read, Value::Ref(Some("12:[a1, a12]:2".into())));
    // Only the marks themselves reach the host unanswered (`log/Probe` is a
    // stub outside the jar, there to be observed).
    let unknown: Vec<_> = host
        .unknown_host_calls()
        .into_iter()
        .filter(|(call, _)| !call.starts_with("log/Probe#"))
        .collect();
    assert_eq!(unknown, []);
}

/// Generated classes: 5000 `ConstantValue` statics, and 2000 mutable statics
/// set in `<clinit>`.
fn wide_sources() -> Vec<(&'static str, String)> {
    let mut consts = String::from("package demo;\n\npublic final class Consts {\n");
    for i in 0..5000 {
        consts.push_str(&format!("  public static final int C{i} = {i};\n"));
    }
    consts.push_str(
        "  public static int get() { return 1; }\n  public static int reflect() throws Exception { return Consts.class.getField(\"C4999\").getInt(null) + Consts.class.getField(\"C7\").getInt(null); }\n}\n",
    );
    let mut many = String::from("package demo;\n\npublic final class Many {\n");
    for i in 0..2000 {
        many.push_str(&format!("  public static int M{i} = {i};\n"));
    }
    many.push_str("  public static int bump() { M0++; M1999++; return M0 + M1999; }\n}\n");
    vec![("demo/Consts.java", consts), ("demo/Many.java", many)]
}

fn call_int(fixture: &Fixture, host: &mut StdHost, class: &str, method: &str) -> Value {
    fixture
        .jar
        .invoke_method_with_args(class, method, "()I", None, &[], host)
        .unwrap_or_else(|error| panic!("{class}.{method}: {error}"))
}

/// #195 review: a class with many compile-time constants, or many mutable
/// statics, still runs and keeps its state. A `ConstantValue` static is
/// never stored (preparation's constant answers a read), so constants do not
/// count toward the host's bound, and the bound is far above any real jar.
/// OpenJDK 25.0.5 for the same classes: `1,1,5006` and `2001,2003`.
#[test]
fn many_constants_and_many_statics_keep_running_as_jdk_25_does() {
    let generated = wide_sources();
    let sources: Vec<(&str, &str)> = generated
        .iter()
        .map(|(path, text)| (*path, text.as_str()))
        .collect();
    let fixture = Fixture::build("statics-wide", &sources);
    let mut host = fixture.jar.seed_host(StdHost::new());
    assert_eq!(
        call_int(&fixture, &mut host, "demo/Consts", "get"),
        Value::Int(1)
    );
    assert_eq!(
        call_int(&fixture, &mut host, "demo/Consts", "get"),
        Value::Int(1)
    );
    assert_eq!(
        call_int(&fixture, &mut host, "demo/Consts", "reflect"),
        Value::Int(5006)
    );
    assert_eq!(
        call_int(&fixture, &mut host, "demo/Many", "bump"),
        Value::Int(2001)
    );
    assert_eq!(
        call_int(&fixture, &mut host, "demo/Many", "bump"),
        Value::Int(2003)
    );
    // No constant was stored as persistent state; the 2000 statics were.
    let (statics, _) = host.persistent_class_state();
    assert!(!statics.keys().any(|key| key.starts_with("demo/Consts#C")));
    assert_eq!(
        statics
            .keys()
            .filter(|key| key.starts_with("demo/Many#M"))
            .count(),
        2000
    );
    assert_eq!(host.unknown_host_calls(), []);
}

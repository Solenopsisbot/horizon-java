//! Host objects are typed handles (#156 P7): the config libraries' objects
//! (cloth-config's `AutoConfig` holders, YACL's builders, SuperMartijn642's
//! configlib builders and values) and Mixin's (`CallbackInfo`,
//! `MixinEnvironment.Side`).
//!
//! `fixtures/host_values/p7/demo/*.java` is compiled against the stubs beside
//! it (the libraries' signatures, left out of the jar so every call reaches the
//! runtime's models). Every expected string is what the same `demo` classes
//! printed on Java 25 (OpenJDK 25.0.5, `javac --release 21`, two runs alike)
//! against the real libraries: cloth-config 26.3.159, yet_another_config_lib_v3
//! 3.9.7, supermartijn642configlib 1.1.8a, sponge-mixin 0.17.5+mixin.0.8.7
//! (`demo/Main.java` and `demo/MixinMain.java` are that run's drivers; for a
//! callback the second builds the object Mixin's injector builds for the
//! target). Each method is either answered as
//! Java 25 does, uncounted, or counted with exactly the calls named.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::{MethodRef, StdHost, Value};
use javac_fixture::Fixture;

const SOURCES: &[(&str, &str)] = &[
    (
        "demo/Configs.java",
        include_str!("fixtures/host_values/p7/demo/Configs.java"),
    ),
    (
        "demo/Forgery.java",
        include_str!("fixtures/host_values/p7/demo/Forgery.java"),
    ),
    (
        "demo/InfoMixin.java",
        include_str!("fixtures/host_values/p7/demo/InfoMixin.java"),
    ),
    (
        "demo/Observe.java",
        include_str!("fixtures/host_values/p7/demo/Observe.java"),
    ),
    (
        "demo/Sides.java",
        include_str!("fixtures/host_values/p7/demo/Sides.java"),
    ),
    (
        "demo/Target.java",
        include_str!("fixtures/host_values/p7/demo/Target.java"),
    ),
    (
        "log/Probe.java",
        include_str!("fixtures/host_values/p7/log/Probe.java"),
    ),
    (
        "com/supermartijn642/configlib/api/ConfigBuilders.java",
        include_str!(
            "fixtures/host_values/p7/stubs/com/supermartijn642/configlib/api/ConfigBuilders.java"
        ),
    ),
    (
        "com/supermartijn642/configlib/api/IConfigBuilder.java",
        include_str!(
            "fixtures/host_values/p7/stubs/com/supermartijn642/configlib/api/IConfigBuilder.java"
        ),
    ),
    (
        "dev/isxander/yacl3/config/v2/api/ConfigClassHandler.java",
        include_str!(
            "fixtures/host_values/p7/stubs/dev/isxander/yacl3/config/v2/api/ConfigClassHandler.java"
        ),
    ),
    (
        "me/shedaniel/autoconfig/annotation/Config.java",
        include_str!(
            "fixtures/host_values/p7/stubs/me/shedaniel/autoconfig/annotation/Config.java"
        ),
    ),
    (
        "me/shedaniel/autoconfig/AutoConfig.java",
        include_str!("fixtures/host_values/p7/stubs/me/shedaniel/autoconfig/AutoConfig.java"),
    ),
    (
        "me/shedaniel/autoconfig/ConfigData.java",
        include_str!("fixtures/host_values/p7/stubs/me/shedaniel/autoconfig/ConfigData.java"),
    ),
    (
        "me/shedaniel/autoconfig/ConfigHolder.java",
        include_str!("fixtures/host_values/p7/stubs/me/shedaniel/autoconfig/ConfigHolder.java"),
    ),
    (
        "me/shedaniel/autoconfig/serializer/ConfigSerializer.java",
        include_str!(
            "fixtures/host_values/p7/stubs/me/shedaniel/autoconfig/serializer/ConfigSerializer.java"
        ),
    ),
    (
        "org/spongepowered/asm/mixin/injection/At.java",
        include_str!("fixtures/host_values/p7/stubs/org/spongepowered/asm/mixin/injection/At.java"),
    ),
    (
        "org/spongepowered/asm/mixin/injection/callback/CallbackInfo.java",
        include_str!(
            "fixtures/host_values/p7/stubs/org/spongepowered/asm/mixin/injection/callback/CallbackInfo.java"
        ),
    ),
    (
        "org/spongepowered/asm/mixin/injection/callback/CallbackInfoReturnable.java",
        include_str!(
            "fixtures/host_values/p7/stubs/org/spongepowered/asm/mixin/injection/callback/CallbackInfoReturnable.java"
        ),
    ),
    (
        "org/spongepowered/asm/mixin/injection/callback/CancellationException.java",
        include_str!(
            "fixtures/host_values/p7/stubs/org/spongepowered/asm/mixin/injection/callback/CancellationException.java"
        ),
    ),
    (
        "org/spongepowered/asm/mixin/injection/callback/Cancellable.java",
        include_str!(
            "fixtures/host_values/p7/stubs/org/spongepowered/asm/mixin/injection/callback/Cancellable.java"
        ),
    ),
    (
        "org/spongepowered/asm/mixin/injection/Inject.java",
        include_str!(
            "fixtures/host_values/p7/stubs/org/spongepowered/asm/mixin/injection/Inject.java"
        ),
    ),
    (
        "org/spongepowered/asm/mixin/MixinEnvironment.java",
        include_str!(
            "fixtures/host_values/p7/stubs/org/spongepowered/asm/mixin/MixinEnvironment.java"
        ),
    ),
    (
        "org/spongepowered/asm/mixin/Mixin.java",
        include_str!("fixtures/host_values/p7/stubs/org/spongepowered/asm/mixin/Mixin.java"),
    ),
];

fn fixture(test: &str) -> Fixture {
    Fixture::build(test, SOURCES)
}

/// Run `String <class>.<method>()` on a fresh host: its answer and the calls
/// it counted.
fn run(fixture: &Fixture, class: &str, method: &str) -> (String, Vec<(String, usize)>) {
    let mut host = fixture.jar.seed_host(StdHost::new());
    let value = fixture
        .jar
        .invoke_method_with_args(class, method, "()Ljava/lang/String;", None, &[], &mut host)
        .unwrap_or_else(|error| panic!("{method} failed: {error}"));
    let text = match value {
        Value::Ref(Some(text)) => text,
        other => panic!("{method} answered {other:?}"),
    };
    (text, host.unknown_host_calls())
}

/// What the Mixin injections on `demo/Target.<name>` left: the callbacks'
/// marks (`log/Probe.mark`), the objects they were handed (`log/Probe.see`),
/// the calls they counted and the target's result.
struct Injected {
    marks: Vec<String>,
    seen: Vec<Value>,
    unknown: Vec<(String, usize)>,
    result: Value,
}

fn inject(fixture: &Fixture, name: &str, descriptor: &str) -> Injected {
    let mut host = StdHost::new();
    let target = MethodRef {
        owner: "demo/Target".into(),
        name: name.into(),
        descriptor: descriptor.into(),
    };
    let report = fixture
        .jar
        .execute_discovered_mixins(&target, None, &[], &mut host)
        .unwrap_or_else(|error| panic!("{name} failed: {error}"));
    let probed = |method: &str| -> Vec<Value> {
        host.invocations()
            .iter()
            .filter(|invocation| {
                invocation.method.owner == "log/Probe" && invocation.method.name == method
            })
            .filter_map(|invocation| invocation.args.first().cloned())
            .collect()
    };
    let marks = probed("mark")
        .into_iter()
        .filter_map(|value| match value {
            Value::Ref(Some(text)) => Some(text),
            _ => None,
        })
        .collect();
    let seen = probed("see");
    let unknown = host
        .unknown_host_calls()
        .into_iter()
        .filter(|(call, _)| !call.starts_with("log/Probe#"))
        .collect();
    Injected {
        marks,
        seen,
        unknown,
        result: report.target_result,
    }
}

/// `(class, method, Java 25's answer)`: answered exactly, nothing counted.
const EXACT: &[(&str, &str, &str)] = &[
    // Strings spelled like the old markers (`config-holder/...`,
    // `mixin-environment`, `mixin-side/CLIENT`...) and like the handles' text
    // forms stay Strings: `instanceof String`, `getClass`, the library types,
    // `equals`, `hashCode`, concat, `HashMap` keys.
    (
        "demo/Forgery",
        "spelled",
        "truejava.lang.Stringfalsefalsefalsefalsefalsetrue188654713330,truejava.lang.Stringfalsefalsefalsefalsefalsetrue157259376431,truejava.lang.Stringfalsefalsefalsefalsefalsetrue-167854903917,truejava.lang.Stringfalsefalsefalsefalsefalsetrue-167788972017,truejava.lang.Stringfalsefalsefalsefalsefalsetrue52555321837,truejava.lang.Stringfalsefalsefalsefalsefalsetrue64283662935,truejava.lang.Stringfalsefalsefalsefalsefalsetrue165547604438,truejava.lang.Stringfalsefalsefalsefalsefalsetrue190218823117,truejava.lang.Stringfalsefalsefalsefalsefalsetrue-162998266517,truejava.lang.Stringfalsefalsefalsefalsefalsetrue188777671223,truejava.lang.Stringfalsefalsefalsefalsefalsetrue134838505514,truejava.lang.Stringfalsefalsefalsefalsefalsetrue94414144527,2estrue",
    ),
    // `MixinEnvironment.Side`: `name`/`ordinal`/`toString`, `compareTo` by
    // ordinal, `==`/`equals`, `instanceof`, `TreeSet` order, lists,
    // `IdentityHashMap` (one object per constant).
    (
        "demo/Sides",
        "sides",
        "CLIENT1CLIENT|-1,2,0|truetruefalsetruetruetruefalse|[UNKNOWN, CLIENT, SERVER]true-11|CLIENTSERVER",
    ),
    // `AutoConfig`: one holder per class (`==`, `equals`, `List.contains`),
    // its config's defaults, `instanceof ConfigHolder`.
    (
        "demo/Configs",
        "autoConfig",
        "truetruetruefalsefalse7truetrue1false",
    ),
    ("demo/Configs", "autoConfigHashed", "2"),
    // YACL builders and configlib builders/values: their interfaces, a
    // builder step answering the builder itself, two builders or two values
    // two objects, also in `HashSet`, `HashMap`, `List.contains`, `Set.of`.
    ("demo/Configs", "yaclBuilder", "truefalsetrue"),
    ("demo/Configs", "yaclTwoBuilders", "falsefalse"),
    ("demo/Configs", "yaclBuildersInCollections", "22false2"),
    (
        "demo/Configs",
        "configlib",
        "truetruefalse,truefalsefalsetrue1",
    ),
    ("demo/Configs", "configlibTwoBuilders", "falsefalse"),
];

/// `(class, method, Java 25's answer, the calls counted instead)`.
#[allow(clippy::type_complexity)]
const COUNTED: &[(&str, &str, &str, &[(&str, usize)])] = &[
    // An object against another library's interface: only the interface the
    // call answered is known, not the implementation class.
    (
        "demo/Configs",
        "crossTypes",
        "falsefalsefalsefalse",
        &[
            (
                "com/supermartijn642/configlib/api/IConfigBuilder#<instanceof>(Ljava/lang/Object;)Z",
                1,
            ),
            (
                "dev/isxander/yacl3/config/v2/api/ConfigClassHandler#<instanceof>(Ljava/lang/Object;)Z",
                2,
            ),
            (
                "java/util/function/Supplier#<instanceof>(Ljava/lang/Object;)Z",
                1,
            ),
        ],
    ),
];

#[test]
fn no_string_spells_a_config_or_mixin_object_and_they_answer_as_java_25() {
    let fixture = fixture("p7-exact");
    let mut wrong = Vec::new();
    for (class, method, expected) in EXACT {
        let (value, unknown) = run(&fixture, class, method);
        if value != *expected || !unknown.is_empty() {
            wrong.push(format!("{method}: {value:?} {unknown:?}"));
        }
    }
    for (class, method, java, calls) in COUNTED {
        let (value, unknown) = run(&fixture, class, method);
        let expected = calls
            .iter()
            .map(|(call, count)| ((*call).to_owned(), *count))
            .collect::<Vec<_>>();
        if unknown != expected {
            wrong.push(format!("{method} (Java 25: {java}): {value:?} {unknown:?}"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// The `CallbackInfo` an injection point hands its callbacks, as each callback
/// observes it, against the object Mixin's injector builds for the target on
/// Java 25 (`demo/MixinMain.java`): a `CallbackInfoReturnable` for `count()I`,
/// a `CallbackInfo` for `run()V`; `cancel()`/`isCancelled()` act on it. On an
/// `@Inject` that is not cancellable, `cancel()` and `setReturnValue` throw
/// Mixin's `CancellationException` (a `RuntimeException`) and the target's
/// result stays. Its `hashCode` is the identity hash: counted.
#[test]
fn a_callback_info_answers_as_the_one_mixin_builds() {
    let fixture = fixture("p7-mixin");
    let count = inject(&fixture, "count", "()I");
    assert_eq!(count.marks, ["truetruetruetruetruefalsefalse", "truefalse"]);
    assert_eq!(count.unknown, []);
    let run = inject(&fixture, "run", "()V");
    assert_eq!(
        run.marks,
        [
            "truefalsetruetruefalse",
            "org.spongepowered.asm.mixin.injection.callback.CancellationException:The call run is not cancellable.",
            "runtime:true",
            "false",
        ]
    );
    assert_eq!(run.unknown, []);
    let strict = inject(&fixture, "strict", "()I");
    assert_eq!(
        strict.marks,
        ["6", "The call strict is not cancellable.", "false6"]
    );
    assert_eq!(strict.unknown, []);
    assert_eq!(strict.result, Value::Int(6));
    let hash = inject(&fixture, "hash", "()V");
    assert_eq!(hash.marks, ["true"]);
    assert_eq!(
        hash.unknown,
        [("java/lang/Object#hashCode()I".to_owned(), 2)]
    );
}

/// Mixin keeps one `CallbackInfo` per `@Inject` (`CallbackInjector`'s
/// `callbackInfoVar`, sponge-mixin 0.17.5), so the two injectors on
/// `twice()V` are handed two objects (Java 25: `first == second` is `false`).
/// The objects are compared as the callbacks hand them to a host call: two
/// handles with different keys, which `==`, `IdentityHashMap` and `equals`
/// answer as two objects. (A jar static carries from one callback to the
/// next since #184: `fixture_statics_across_callbacks.rs`.)
#[test]
fn each_inject_gets_its_own_callback_info() {
    let fixture = fixture("p7-twice");
    let twice = inject(&fixture, "twice", "()V");
    assert_eq!(twice.unknown, []);
    let [Value::Host(first), Value::Host(second)] = twice.seen.as_slice() else {
        panic!("two callback infos, got {:?}", twice.seen);
    };
    assert_eq!(first.kind(), second.kind());
    assert_ne!(first.key(), second.key());
}

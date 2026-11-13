//! `MethodType`, `MethodHandle`, `MethodHandles.Lookup` and the call sites
//! `LambdaMetafactory` builds are typed handles (#156 P10b).
//!
//! They used to be text a jar `String` could spell (`method-type/…`,
//! `method-handle/…`, `java/lang/invoke/MethodHandle#mh…`,
//! `java/lang/invoke/ConstantCallSite#cs…`, `method-handles-lookup`).
//! `fixtures/host_values/Invoke.java` checks that strings spelled like those
//! stay `String`s, that the real objects print, compare, cast and work as
//! Java's do, every identity path, and the generic paths (type switches,
//! patterns, `Class.isInstance`, host collections). Each expected string is
//! what its `main` printed on OpenJDK 25.0.5 (`javac --release 21`, two runs
//! alike). `Counted` cases rest on something the run cannot know (an
//! identity hash, a handle's exact class, `lookup()`'s caller) and must count
//! it.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::{StdHost, Value};
use javac_fixture::Fixture;

enum Outcome {
    Exact(&'static str),
    /// Java's answer, with only the named calls counted: calls this PR's
    /// kinds do not own (a proxy's `checkcast`).
    ExactCounting(&'static str, &'static [&'static str]),
    Counted,
    /// Java throws this (class, message) out of the method.
    Throws(&'static str, &'static str),
}
use Outcome::*;

const CASES: &[(&str, Outcome)] = &[
    // Strings stay Strings: no handle `instanceof`, `equals` both ways,
    // concat; the bare word and the encodings also `instanceof String`,
    // `getClass`, `hashCode`, a `HashMap` key, and `(MethodType) "hmtype:…"`
    // throws Java's exception. (`word/rest` text and text with a `#` are still
    // read by `is_plain_string` / the `class#n` rule until #156 P17 / P12, so
    // those skip the checks the rules decide: `#` text skips `instanceof`.)
    (
        "spelled",
        Exact(concat!(
            "falsefalsefalsefalsefalsefalsefalsefalse|method-type/(I)Ljava/lang/String;,",
            "falsefalsefalsefalse|method-handle/virtual/-/reflect-method/demo/Invoke#s(I)Ljava/lang/String;,",
            "falsefalsefalsefalse|java/lang/invoke/MethodHandle#mh{},",
            "falsefalsefalsefalse|java/lang/invoke/ConstantCallSite#cs{},",
            "falsefalsefalsefalsefalsefalsefalsefalse|method-handles-lookup,truejava.lang.String-1958289240,",
            "falsefalsefalsefalsefalsefalsefalsefalse|hmtype:(I)Ljava/lang/String;,truejava.lang.String-707034953,",
            "falsefalsefalsefalsefalsefalsefalsefalse|hmhlk:rpublic,null,truejava.lang.String1521939184,",
            "4ss|class java.lang.String cannot be cast to class java.lang.invoke.MethodType ",
            "(java.lang.String and java.lang.invoke.MethodType are in module java.base of loader ",
            "'bootstrap')|ok1"
        )),
    ),
    // Interned, simple names (`Map.Entry[]` is `Entry[]`), queries, class and
    // Java 25's supertypes.
    (
        "types",
        Exact(
            "(int)String|(int[],List,String[][])Entry[]|()void|truetruefalse|1java.lang.Stringint(I)Ljava/lang/String;|(int)Object|java.lang.invoke.MethodTypetruetruetruefalse",
        ),
    ),
    // `MethodHandle` + its type; `invoke`/`invokeExact`/`invokeWithArguments`
    // and `bindTo`; equal only to itself.
    (
        "handles",
        Exact(
            "MethodHandle(int)String|MethodHandle(Invoke,String)String|MethodHandle(String)String|(int)Stringtrue|s5vxs7s3|truefalse|truetrue",
        ),
    ),
    (
        "lookups",
        Exact(
            "java.lang.Object/publicLookup|truetrue|demo.Invoke|java.lang.invoke.MethodHandles$Lookuptrue",
        ),
    ),
    // The same handle on both sides; interned method types in an
    // `IdentityHashMap` are one key.
    ("sameHandle", Exact("truetruetrue2")),
    ("twoHandles", Counted),
    // Two lookups' handles have equal keys: maybe one object, maybe two
    // (Java: `false`); counted.
    ("twoHandlesEqual", Counted),
    ("twoLookups", Counted),
    ("handleClass", Counted),
    ("typeHash", Counted),
    ("callerLookup", Counted),
    (
        "generic",
        Exact("type1,handle,lookup,other,|truetruefalse|[(int)String]true|10"),
    ),
    // Interned method types: equal keys are one object (Java: `1`).
    ("typeSet", Exact("1")),
    // A proxy's default method through `privateLookupIn`/`unreflectSpecial`/
    // `bindTo`/`invokeWithArguments`, and a virtual handle on the proxy (its
    // handler answers).
    (
        "proxies",
        ExactCounting(
            "42|21|1",
            &["demo/Invoke$Cfg#<checkcast>(Ljava/lang/Object;)Ldemo/Invoke$Cfg;"],
        ),
    ),
    ("metafactory", Exact("hi|()Supplier|true")),
    // Separately made handles, lookups and call sites with equal keys are
    // two objects to every collection (Java: 2, false, false-1, 2, 2, 2, 2).
    // Identity `equals` cannot tell them from one, so each is counted, never
    // merged by key (#175 review M1).
    ("handleSet", Counted),
    ("handleMapKey", Counted),
    ("handleListContains", Counted),
    ("handleSetOf", Counted),
    ("handleMapOf", Counted),
    ("lookupSet", Counted),
    ("siteSet", Counted),
    // `unreflectSpecial` types the receiver as the special caller (M2).
    (
        "specialCaller",
        Exact("MethodHandle(TopImpl)String|(TopImpl)String|dflt|MethodHandle()String|dflt"),
    ),
    // `privateLookupIn`: a primitive or array target, or a caller without
    // full privilege, throws Java's exception; a jar class is its lookup (M3).
    (
        "privateLookups",
        Exact(concat!(
            "java.lang.IllegalArgumentException:int is a primitive class",
            "|java.lang.IllegalArgumentException:class [Ljava.lang.String; is an array class",
            "|java.lang.IllegalAccessException:caller does not have PRIVATE and MODULE lookup mode",
            "|java.lang.IllegalArgumentException:int is a primitive class|demo.TopImpl"
        )),
    ),
    // A JDK class: the launch's modules decide (Java: `IllegalAccessException:
    // module java.base does not open java.lang to unnamed module @…`); counted.
    ("privateLookupJdk", Counted),
    // Nothing to drop or insert: the very same handle.
    ("noOpCombinators", Exact("truetruetruetrue")),
    // `methodType(Class, List)` reads the list; a `void` parameter throws.
    (
        "typeFromList",
        Exact("(int)String|true|java.lang.IllegalArgumentException:parameter type cannot be void"),
    ),
    (
        "wrongArity",
        Throws(
            "java/lang/invoke/WrongMethodTypeException",
            "cannot convert MethodHandle(int)String to (Object,Object)Object",
        ),
    ),
    ("siteClass", Exact("java.lang.invoke.ConstantCallSite")),
    // Java 25's helpful messages for null arguments.
    (
        "nullArguments",
        Exact(concat!(
            "java.lang.NullPointerException:Cannot invoke \"java.lang.Class.isPrimitive()\" because \"targetClass\" is null",
            "|java.lang.NullPointerException:Cannot read field \"allowedModes\" because \"caller\" is null",
            "|java.lang.NullPointerException:Cannot invoke \"Object.hashCode()\" because \"this.rtype\" is null",
            "|java.lang.NullPointerException:Cannot read the array length because \"ptypes\" is null"
        )),
    ),
];

#[test]
fn invoke_handles_answer_as_java_25_or_count() {
    let fixture = Fixture::build(
        "host-values-invoke",
        &[(
            "demo/Invoke.java",
            include_str!("fixtures/host_values/Invoke.java"),
        )],
    );
    let mut failures = Vec::new();
    for (method, outcome) in CASES {
        let mut host = fixture.jar.seed_host(StdHost::new());
        let answer = fixture.jar.invoke_method_with_args(
            "demo/Invoke",
            method,
            "()Ljava/lang/String;",
            None,
            &[],
            &mut host,
        );
        let unknown = host.unknown_host_calls();
        let ok = match outcome {
            Exact(expected) => {
                matches!(&answer, Ok(Value::Ref(Some(text))) if text == expected)
                    && unknown.is_empty()
            }
            ExactCounting(expected, calls) => {
                matches!(&answer, Ok(Value::Ref(Some(text))) if text == expected)
                    && unknown
                        .iter()
                        .all(|(call, _)| calls.contains(&call.as_str()))
            }
            Counted => !unknown.is_empty(),
            Throws(class, message) => matches!(
                &answer,
                Err(horizon_native_java::NativeError::UncaughtException {
                    class: thrown,
                    message: Some(text),
                    ..
                }) if thrown == class && text == message
            ),
        };
        if !ok {
            let expected = match outcome {
                Exact(text) => format!("exact {text}"),
                ExactCounting(text, calls) => format!("exact {text}, counting only {calls:?}"),
                Counted => "counted".to_owned(),
                Throws(class, message) => format!("throws {class}: {message}"),
            };
            failures.push(format!(
                "{method}\n  want:    {expected}\n  native:  {answer:?}\n  unknown: {unknown:?}"
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} cases:\n{}",
        failures.len(),
        CASES.len(),
        failures.join("\n")
    );
}

//! #196's review (#156 P10c), against Java 25.
//!
//! `fixtures/reflect_type_review/Probe.java`'s expected strings are what
//! OpenJDK 25.0.5 printed (`javac --release 21`, two runs alike):
//!
//! * `owners`: `Outer<String>.Mid.Deep<Long>`'s owner is `Outer<String>.Mid`,
//!   a `ParameterizedType` with no arguments whose owner is `Outer<String>`
//!   (the signature's inner-class chain, as the JDK's `Reifier` builds it).
//! * `unannotated`: a member with no type-use annotations has none, exactly.
//! * `classEqualsOther`, `castPlain`: a class's `Type.equals` of another class
//!   or `null`, and a plain String cast to `ParameterizedType`/`Type` (CCE).
//!
//! Counted, where Java 25 answers what Horizon cannot read or tell apart:
//! * `annotatedField`/`annotatedArgument`: a `RUNTIME` `TYPE_USE` annotation
//!   (Java printed `1true,@demo.Probe.TU() java.lang.String` and
//!   `java.util.List<java.lang.String>,1`): the annotation reads and the
//!   annotated text, never "none";
//! * `classEqualsSpelling`: `String.class.equals("java/lang/String")` (Java
//!   `false`): a class marker and a String with its spelling are both text
//!   until #156 P13.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::StdHost;
use javac_fixture::Fixture;

const PROBE: &str = include_str!("fixtures/reflect_type_review/Probe.java");

fn fixture() -> Fixture {
    Fixture::build("reflect-type-review", &[("demo/Probe.java", PROBE)])
}

fn run(fixture: &Fixture, method: &str) -> (String, Vec<String>) {
    let mut host = fixture.jar.seed_host(StdHost::new());
    let text = fixture.text_on(&mut host, method);
    let counted = host
        .unknown_host_calls()
        .into_iter()
        .map(|(call, _)| call)
        .collect();
    (text, counted)
}

#[test]
fn answers_as_java_25() {
    let fixture = fixture();
    for (method, java) in [
        (
            "owners",
            "demo.Probe<T, E>$Outer<java.lang.String>$Mid$Deep<java.lang.Long>;\
             demo.Probe<T, E>$Outer<java.lang.String>$Mid;demo.Probe$Outer$Mid,0,\
             demo.Probe<T, E>$Outer<java.lang.String>;true",
        ),
        ("unannotated", "0false,java.lang.String,0"),
        ("classEqualsOther", "falsefalse"),
        ("castPlain", "CCECCE"),
    ] {
        let (text, counted) = run(&fixture, method);
        assert_eq!(text, java, "{method}");
        assert_eq!(counted, Vec::<String>::new(), "{method}");
    }
}

#[test]
fn unread_type_use_annotations_and_a_spelled_class_are_counted() {
    let fixture = fixture();
    for (method, calls) in [
        (
            "annotatedField",
            &[
                "java/lang/reflect/AnnotatedType#getAnnotations()[Ljava/lang/annotation/Annotation;",
                "java/lang/reflect/AnnotatedType#isAnnotationPresent(Ljava/lang/Class;)Z",
            ][..],
        ),
        (
            "annotatedArgument",
            &["java/lang/reflect/AnnotatedType#getAnnotations()[Ljava/lang/annotation/Annotation;"]
                [..],
        ),
        (
            "classEqualsSpelling",
            &["java/lang/reflect/Type#equals(Ljava/lang/Object;)Z"][..],
        ),
    ] {
        let (_, counted) = run(&fixture, method);
        for call in calls {
            assert!(
                counted.iter().any(|c| c == call),
                "{method}: {call} not counted: {counted:?}"
            );
        }
    }
}

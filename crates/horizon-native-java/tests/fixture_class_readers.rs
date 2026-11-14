//! The readers of a `Class` value (#156 P13a): `Class`'s own members that read
//! the name, and APIs that take a `Class` (`Array.newInstance`,
//! `Enum.valueOf`, `isAnnotationPresent`/`getAnnotation`,
//! `MethodType.methodType`/`changeReturnType`, `Lookup.findStatic`). P13a
//! routes every such reader through `host_class.rs`; P13b makes the classes
//! handles, and these answers must stay Java 25's.
//!
//! The expected strings are what OpenJDK 25.0.5 printed for
//! `fixtures/class_readers/Probe.java` (`javac --release 21`, two runs alike).

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::StdHost;
use javac_fixture::Fixture;

const PROBE: &str = include_str!("fixtures/class_readers/Probe.java");

#[test]
fn class_readers_answer_as_java_25() {
    let fixture = Fixture::build("class-readers", &[("demo/Probe.java", PROBE)]);
    for (method, java) in [
        (
            "members",
            "java.lang.String,Tagged,demo,[I,truefalse,truefalse,true",
        ),
        (
            "arguments",
            "GREEN,2,truetrue,true,(String)String,abab,(String)int",
        ),
    ] {
        let mut host = fixture.jar.seed_host(StdHost::new());
        assert_eq!(fixture.text_on(&mut host, method), java, "{method}");
        assert_eq!(
            host.unknown_host_calls(),
            Vec::<(String, usize)>::new(),
            "{method}"
        );
    }
}

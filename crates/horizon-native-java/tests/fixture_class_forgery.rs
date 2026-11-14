//! A `String` spelled like a class is that `String`, never the `Class`
//! (#156 P13c).
//!
//! `fixtures/class_forgery/Probe.java` is the spec. Its expected strings are
//! what OpenJDK 25.0.5 printed for `demo.Probe` (`javac --release 21`, two runs
//! alike):
//!
//! * `forged`: 12 spellings of a class: internal and binary names of a JDK
//!   class, a jar class and an interface, primitives, arrays in descriptor and
//!   binary form, `class java.lang.String`, `java/lang/Class`. Each of 12 checks
//!   counts the texts that behave as the `String` they are: `instanceof`
//!   `Class`/`Type`, `(Class<?>)`/`(Type)` casts, `Class.isInstance`,
//!   `getClass`, `equals` both ways against the class, `hashCode`, text,
//!   collection lookups keyed by the class (`HashSet`, `HashMap`,
//!   `IdentityHashMap`, `ArrayList`, `List.of`, `Set.of`, `Map.of`), `==`, a
//!   type switch, `Method.invoke` with the text as the receiver, and
//!   `CharSequence`. On a JVM all 12 pass all 12 checks.
//! * `forName`: `Class.forName` resolves a binary name, the one way text
//!   names a class.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::StdHost;
use javac_fixture::Fixture;

const PROBE: &str = include_str!("fixtures/class_forgery/Probe.java");

const FORGED: &str = "instanceof=12,cast=12,isInstance=12,getClass=12,equals=12,hashCode=12,\
                      text=12,collections=12,identity=12,switch=12,reflection=12,charSequence=12";

fn fixture() -> Fixture {
    Fixture::build("class-forgery", &[("demo/Probe.java", PROBE)])
}

#[test]
fn a_string_spelled_like_a_class_is_only_that_string() {
    let fixture = fixture();
    let mut host = fixture.jar.seed_host(StdHost::new());
    assert_eq!(fixture.text_on(&mut host, "forged"), FORGED);
    assert_eq!(host.unknown_host_calls(), Vec::<(String, usize)>::new());
}

#[test]
fn class_for_name_resolves_a_binary_name() {
    let fixture = fixture();
    let mut host = fixture.jar.seed_host(StdHost::new());
    assert_eq!(fixture.text_on(&mut host, "forName"), "truetruetruetrue");
    assert_eq!(host.unknown_host_calls(), Vec::<(String, usize)>::new());
}

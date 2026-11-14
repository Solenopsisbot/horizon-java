//! A `String` spelling an object's key is that `String` whatever the key's
//! package (#156 P12b2, #198 review F1). A jar class in a package named like a
//! host marker (`map`, `list`, `set`, `stream`, `optional`, `type`, ...) has
//! keys that read as that marker's text, and its object was reachable through a
//! `String` spelling the key: a method reference, `MethodHandle` or lambda read
//! and wrote its field, the type switch threw. And `Class.cast` of a `String`
//! to a class it is not throws Java's `ClassCastException` (F2), with Java's
//! messages for it and for `Method.invoke` on a `String` (F4).
//!
//! `fixtures/string_keys/` runs unchanged on both sides: `demo.Probe`'s `main`
//! on Java 25 (its output is `java25.txt`), its static probes in Horizon.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

/// What `java -cp . demo.Probe` printed on Java 25 (Temurin 25.0.5, `javac
/// --release 21`), the same on two runs.
const ORACLE: &str = include_str!("fixtures/string_keys/java25.txt");

const PACKAGES: &[&str] = &["map", "list", "set", "stream", "optional", "type", "component", "uuid", "codec", "iterator", "duration", "locale", "list_render", "set_rows"];

const SOURCES: &[(&str, &str)] = &[
    ("demo/Probe.java", include_str!("fixtures/string_keys/demo/Probe.java")),
    ("map/Thing.java", include_str!("fixtures/string_keys/map/Thing.java")),
    ("list/Thing.java", include_str!("fixtures/string_keys/list/Thing.java")),
    ("set/Thing.java", include_str!("fixtures/string_keys/set/Thing.java")),
    ("stream/Thing.java", include_str!("fixtures/string_keys/stream/Thing.java")),
    ("optional/Thing.java", include_str!("fixtures/string_keys/optional/Thing.java")),
    ("type/Thing.java", include_str!("fixtures/string_keys/type/Thing.java")),
    ("component/Thing.java", include_str!("fixtures/string_keys/component/Thing.java")),
    ("uuid/Thing.java", include_str!("fixtures/string_keys/uuid/Thing.java")),
    ("codec/Thing.java", include_str!("fixtures/string_keys/codec/Thing.java")),
    ("iterator/Thing.java", include_str!("fixtures/string_keys/iterator/Thing.java")),
    ("duration/Thing.java", include_str!("fixtures/string_keys/duration/Thing.java")),
    ("locale/Thing.java", include_str!("fixtures/string_keys/locale/Thing.java")),
    ("list/render/Thing.java", include_str!("fixtures/string_keys/list/render/Thing.java")),
    ("set/rows/Thing.java", include_str!("fixtures/string_keys/set/rows/Thing.java")),
];

fn expected(method: &str) -> &'static str {
    ORACLE
        .lines()
        .find_map(|line| line.strip_prefix(method)?.strip_prefix('='))
        .unwrap_or_else(|| panic!("no oracle line for {method}"))
}

fn run(fixture: &javac_fixture::Fixture, method: &str) -> (String, Vec<(String, usize)>) {
    let mut host = fixture.jar.seed_host(horizon_native_java::StdHost::new());
    let text = fixture.text_on(&mut host, method);
    (text, host.unknown_host_calls())
}

/// For each package, a `String` spelling `<package>/Thing#n` (n = 0..3, the
/// live object's key among them): `instanceof`, `Class.isInstance`,
/// `getClass`, `checkcast` and `Class.cast` (both throw), `String.valueOf`, a
/// type switch (`case String`), `equals` both ways, and the object's field
/// untouched. Every answer is Java 25's, with nothing counted.
#[test]
fn a_string_spelling_a_key_in_any_package_is_a_string() {
    let fixture = javac_fixture::Fixture::build("string-keys", SOURCES);
    for package in PACKAGES {
        let method = format!("{package}Exact");
        let (text, unknown) = run(&fixture, &method);
        assert_eq!(text, expected(&method), "{method}: {unknown:?}");
        assert_eq!(unknown, Vec::<(String, usize)>::new(), "{method}");
    }
}

/// The same `String`s handed to a method reference, a `MethodHandle` and a
/// lambda typed to the class: Java throws `ClassCastException` at each. The
/// runtime does not cast a functional receiver (a follow-up, #198 review F3),
/// so a body that reads or writes the field is counted; it never reads or
/// writes the live object's (its `value` stays 42).
#[test]
fn a_string_spelling_a_key_never_reaches_the_object() {
    let fixture = javac_fixture::Fixture::build("string-keys", SOURCES);
    for package in PACKAGES {
        let method = format!("{package}Calls");
        let (text, unknown) = run(&fixture, &method);
        if unknown.is_empty() {
            assert_eq!(text, expected(&method), "{method}");
        } else {
            assert!(text.ends_with("|42"), "{method}: {text} {unknown:?}");
            assert!(!text.contains("=42"), "{method}: read the object: {text}");
        }
    }
}

/// `Class.cast` of a `String` to `ArrayList`, `StringBuilder`, `HashMap`,
/// `RuntimeException`, `Runnable`, `List` and a jar class throws `Cannot cast
/// java.lang.String to ...`; to `CharSequence`, `Comparable`, `Object` and
/// `String` it passes; a real object passes its own class. `Method.invoke` of
/// a jar method on a `String` throws Java 25's `IllegalArgumentException`.
#[test]
fn class_cast_of_a_string_answers_as_java_25() {
    let fixture = javac_fixture::Fixture::build("string-keys", SOURCES);
    let (text, unknown) = run(&fixture, "casts");
    assert_eq!(text, expected("casts"), "{unknown:?}");
    assert_eq!(unknown, Vec::<(String, usize)>::new());
}

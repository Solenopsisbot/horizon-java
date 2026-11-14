//! `java.lang.reflect.Type` nodes and `AnnotatedType`s are typed handles
//! (`Value::Host`, #156 P10c, `host_reflect_type.rs`), on a `javac` fixture
//! reflecting over its own generic class.
//!
//! `fixtures/reflect_type/demo/Types.java` runs unchanged on both sides: its
//! own `main` on Java 25 (see [`ORACLE`]), its static probes in Horizon
//! (through `demo.Probe`).

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

/// What `java -cp . demo.Types` printed on Java 25 (Temurin 25.0.5, `javac
/// --release 21`), the same on two runs.
const ORACLE: &str = "\
texts=Pjava.util.List<java.lang.String>|java.util.List<java.lang.String>;Pjava.util.Map<java.lang.String, ? extends java.lang.Number>|java.util.Map<java.lang.String, ? extends java.lang.Number>;VT|T;AT[]|T[];Ajava.util.List<java.lang.String>[]|java.util.List<java.lang.String>[];CC:java.lang.String[]|java.lang.String[];CC:int|int;Pjava.util.Map$Entry<java.lang.String, java.lang.Integer>|java.util.Map$Entry<java.lang.String, java.lang.Integer>;Pdemo.Types$Outer<java.lang.String>$Inner<java.lang.Integer>|demo.Types$Outer<java.lang.String>$Inner<java.lang.Integer>;Pjava.util.List<?>|java.util.List<?>;Pjava.util.List<?>|java.util.List<?>;Pjava.util.List<? super java.lang.Integer>|java.util.List<? super java.lang.Integer>;java.util.ArrayList<T>;java.lang.Comparable<demo.Types<T>>;U;java.util.List<? super U>,T,
structure=truetruetrue1|true01truetrue|Ttrue1java.lang.Comparable<T>|truejava.util.List<java.lang.String>|demo.Types$Outer<java.lang.String>true|truetrue|U1true
equality=truetruefalsefalsefalsefalsetruetruetruefalsetruefalsetrue1true|truefalsefalsetruefalsetruefalsetruetruefalse
annotated=truetrue0truetruetruefalsetruetrue|java.util.List<java.lang.String>|java.lang.String[]|T|int|java.util.Map<java.lang.String, ? extends java.lang.Number>
identity=falsetrue1falseIAEinterface java.util.Map|true
spelled=falsefalsetype/P43:14:java/util/List1:C19:16:java/lang/String-!,falsefalseannotated-type/java/lang/String!,falsefalsetype/V4:1:T0:!,falsefalsehrtyp:rP,rjava/util/List,n,rjava/lang/String!,falsefalsehrann:rjava/lang/String!,5sfalse
forged=CCECCE
";

const TYPES: &str = include_str!("fixtures/reflect_type/demo/Types.java");
const PROBE: &str = include_str!("fixtures/reflect_type/demo/Probe.java");

fn expected(method: &str) -> &'static str {
    ORACLE
        .lines()
        .find_map(|line| line.strip_prefix(method)?.strip_prefix('='))
        .unwrap_or_else(|| panic!("no oracle line for {method}"))
}

/// Run `method`: its text and what it counted.
fn run(method: &str) -> (String, Vec<(String, usize)>) {
    let fixture = javac_fixture::Fixture::build(
        method,
        &[("demo/Types.java", TYPES), ("demo/Probe.java", PROBE)],
    );
    let mut host = fixture.jar.seed_host(horizon_native_java::StdHost::new());
    let text = fixture.text_on(&mut host, method);
    (text, host.unknown_host_calls())
}

/// Every shape a field, a method, a superclass or an interface declares
/// answers as Java 25 does, with nothing counted:
/// * `toString`/`getTypeName` (`Map<java.lang.String, ? extends
///   java.lang.Number>`, `T[]`, `Outer<String>$Inner<Integer>`, `List<?>`
///   for `? extends Object`) and a type switch over the shapes;
/// * the parts (raw type, arguments, owner, bounds from the declaration, a
///   generic array's component, a method's own type variable);
/// * `equals` as each JDK implementation specifies it (structural; a type
///   variable by its declaration and name) and exact `instanceof`;
/// * `Field.getAnnotatedType`/`Method.getAnnotatedReturnType`.
#[test]
fn types_answer_as_java_25() {
    for method in ["texts", "structure", "equality"] {
        let (text, unknown) = run(method);
        assert_eq!(text, expected(method), "{method}: {unknown:?}");
        assert_eq!(unknown, Vec::<(String, usize)>::new(), "{method}");
    }
    // `annotated` compares a type argument with `String.class`: since #156
    // P13b1 both are the `Class` object, so `Type.equals` is exact (it was
    // counted while classes were text, #196's review).
    let (text, unknown) = run("annotated");
    assert_eq!(text, expected("annotated"));
    assert_eq!(unknown, Vec::<(String, usize)>::new());
}

/// Two lookups' types are two objects whose hashes hash `Class` identity
/// hashes, a nested class's owner is its declaring class, and a method's
/// type variable's declaration is a `Method` object: Java 25's answers or
/// counted, never a silent one from equal keys.
#[test]
fn what_only_a_jvm_knows_is_java_25s_or_counted() {
    let (text, unknown) = run("identity");
    assert!(
        text == expected("identity") || !unknown.is_empty(),
        "{text}"
    );
}

/// A string spelled like an old `type/` or `annotated-type/` marker, or like
/// a handle's key, is no `Type`: `equals` both ways, concatenation and
/// `HashMap` keying are `String`'s (an old `word/rest` marker's hash and
/// `instanceof` are still read as a marker until #156 P12/P17: counted).
#[test]
fn no_string_spells_a_type() {
    let (text, unknown) = run("spelled");
    assert_eq!(text, expected("spelled"), "{unknown:?}");
    assert!(
        unknown
            .iter()
            .all(|(call, _)| call.ends_with("#hashCode()I")
                || call.ends_with("#<instanceof>(Ljava/lang/Object;)Z")),
        "{unknown:?}"
    );
}

/// A `String` cast to `ParameterizedType` or `TypeVariable` is a
/// `ClassCastException` on Java 25 (main answered the cast `instanceof`
/// `true` and decoded the text). Horizon's cast of text is a counted pass
/// until #156 P17; whatever it answers, the string never drives the type it
/// spells.
#[test]
fn a_string_cast_to_a_type_is_never_the_type() {
    let (text, unknown) = run("forged");
    if text != expected("forged") {
        assert!(!unknown.is_empty(), "{text}");
        assert!(
            !text.contains("java") && text != "CCET",
            "{text} {unknown:?}"
        );
    }
}

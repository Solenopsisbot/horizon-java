//! The client's value objects keep Java's text and equality as host handles
//! (#156 P16d): `Style`, `TextColor`, `KeyMapping.Category` and
//! `java.awt.Color` answer `toString`, `hashCode`, `equals`, `==` and the
//! collections as vanilla 26.3 does, and text spelled like their old markers
//! stays text.
//!
//! `fixtures/client_values/demo/Probe.java` asks the questions of the oracle
//! that ran them against the real 26.3 client classes on OpenJDK 25.0.5+8
//! (`javac --release 21`; `java.txt` is its output for the probe's lines).
//! The game stubs here only let `javac` compile the probe and are never
//! packed, so every call reaches the host's model, as for a real mod.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::StdHost;

const ORACLE: &str = include_str!("fixtures/client_values/java25.txt");
const PROBE: &str = include_str!("fixtures/client_values/demo/Probe.java");

const STUBS: &[(&str, &str)] = &[
    (
        "net/minecraft/ChatFormatting.java",
        "package net.minecraft; public enum ChatFormatting { DARK_BLUE, GOLD, RED, BOLD, RESET }",
    ),
    (
        "net/minecraft/resources/Identifier.java",
        "package net.minecraft.resources; public final class Identifier { \
         public static Identifier fromNamespaceAndPath(String n, String p) { return null; } \
         public static Identifier withDefaultNamespace(String p) { return null; } }",
    ),
    (
        "net/minecraft/network/chat/TextColor.java",
        "package net.minecraft.network.chat; public final class TextColor { \
         public static TextColor fromRgb(int rgb) { return null; } \
         public static TextColor fromLegacyFormat(net.minecraft.ChatFormatting f) { return null; } \
         public int getValue() { return 0; } }",
    ),
    (
        "net/minecraft/network/chat/Style.java",
        "package net.minecraft.network.chat; import net.minecraft.ChatFormatting; \
         public class Style { public static final Style EMPTY = null; \
         public Style withColor(TextColor c) { return null; } \
         public Style withColor(ChatFormatting c) { return null; } \
         public Style withColor(int c) { return null; } \
         public Style withBold(Boolean b) { return null; } \
         public Style withItalic(Boolean b) { return null; } \
         public Style withUnderlined(Boolean b) { return null; } \
         public Style withStrikethrough(Boolean b) { return null; } \
         public Style withObfuscated(Boolean b) { return null; } \
         public Style applyFormat(ChatFormatting f) { return null; } \
         public Style applyLegacyFormat(ChatFormatting f) { return null; } \
         public boolean isEmpty() { return false; } }",
    ),
    (
        "net/minecraft/client/KeyMapping.java",
        "package net.minecraft.client; import net.minecraft.resources.Identifier; \
         public class KeyMapping { public record Category(Identifier id) { \
         public static final Category MISC = null; \
         public static Category register(Identifier id) { return null; } } }",
    ),
];

fn fixture() -> javac_fixture::Fixture {
    let mut sources = STUBS.to_vec();
    sources.push(("demo/Probe.java", PROBE));
    javac_fixture::Fixture::build("client-values", &sources)
}

/// Every line the probe prints is Java 25's, and none of them is counted.
#[test]
fn client_values_answer_as_java() {
    let fixture = fixture();
    let mut host = fixture.jar.seed_host(StdHost::new());
    let printed = fixture.text_on(&mut host, "run");
    for (line, expected) in printed.lines().zip(ORACLE.lines()) {
        assert_eq!(line, expected);
    }
    assert_eq!(printed.lines().count(), ORACLE.lines().count());
    assert_eq!(host.unknown_host_calls(), Vec::<(String, usize)>::new());
}

/// What the model cannot know is counted, never answered as Java's opposite
/// silently: two styles built alike may be one object or two (Java: two);
/// a jar's own `new Category(id)` is no handle (Java: `equals` the
/// registered one); a second registration of an id throws in Java.
#[test]
fn identities_the_model_cannot_know_are_counted() {
    let fixture = fixture();
    let mut host = fixture.jar.seed_host(StdHost::new());
    fixture.text_on(&mut host, "twoBold");
    fixture.text_on(&mut host, "newCategory");
    let counted: Vec<String> = host
        .unknown_host_calls()
        .into_iter()
        .map(|(call, _)| call)
        .collect();
    assert_eq!(
        counted,
        [
            "java/lang/Object#if_acmp(Ljava/lang/Object;Ljava/lang/Object;)Z",
            "net/minecraft/client/KeyMapping$Category#equals(Ljava/lang/Object;)Z",
        ]
    );
    let twice = fixture.jar.invoke_method_with_args(
        "demo/Probe",
        "registerTwice",
        "()Ljava/lang/String;",
        None,
        &[],
        &mut host,
    );
    assert!(twice.is_err(), "{twice:?}");
}

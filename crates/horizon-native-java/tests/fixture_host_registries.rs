//! Registries, registry keys and NeoForge holders are typed handles
//! (`Value::Host`, #156 P4b, `host_registry.rs`).
//!
//! `registry/<id>` used to be both a registry (`BuiltInRegistries.ITEM`) and
//! its key (`Registries.ITEM`), and `holder/…`/`registered/…` named a holder
//! and its value by text a jar `String` could spell. The stubs exist only for
//! `javac`: only `demo/` ships, so every call reaches the host's model as a
//! real mod's does.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::StdHost;

/// `java` (Temurin 25.0.5) running this `Probe`, compiled with `javac
/// --release 21` against the real 26.3 `server-game.jar` and its libraries
/// plus NeoForge 26.3.0.101's universal jar and its libraries (`main`, left
/// out here, ran `Bootstrap.bootStrap()` first), printed these lines, the
/// same on two runs but for the identity hashes; pasted verbatim.
const EXPECTED: &str = "\
spelled=falsefalsefalsefalsefalsefalsefalseregistry/minecraft:item,falsefalsefalsefalsefalsefalsefalseholder/deferred-register/registry/minecraft:item/demo/thing,falsefalsefalsefalsefalsefalsefalseregistered/deferred-register/registry/minecraft:item/demo/thing,falsefalsefalsefalsefalsefalsefalsehreg:minecraft:item,falsefalsefalsefalsefalsefalsefalsehhold:1,CCE\n\
spelledStrings=Stringtrue-895996210,Stringtrue1353332634,Stringtrue-1209087779,4rs\n\
identity=4|2476|truefalsetrue\n\
keys=ResourceKey[minecraft:root / minecraft:item]|minecraft:item|minecraft:root|truefalsetrue|net.minecraft.resources.ResourceKey|ResourceKey[minecraft:root / minecraft:item]true|minecraft:block|[ResourceKey[minecraft:root / minecraft:item], ResourceKey[minecraft:root / minecraft:block]]true|ResourceKey[minecraft:root / demo:things]\n\
registries=truefalsetruefalse|ResourceKey[minecraft:root / minecraft:item]true|truetruetruefalsefalsetrue|true1\n\
holders=DeferredHolder{ResourceKey[minecraft:item / demo:thing]}|demo:thing|falsefalsefalsefalse|truetruetruefalse2|truetruetruefalsefalse|ResourceKey[minecraft:item / demo:thing]|[DeferredHolder{ResourceKey[minecraft:item / demo:thing]}, DeferredHolder{ResourceKey[minecraft:item / demo:other]}]\n\
types=defaulted:minecraft:itemtruefalsefalsefalse,key:minecraft:itemfalsefalsetruefalse,holder:demo:tfalsetruefalsetrue,stringfalsefalsefalsefalse,trueResourceKey[minecraft:root / minecraft:item]\n\
holderIdentity=false\n\
holderHash=1053036435\n\
registryHash=404222443\n\
registryText=Registry[ResourceKey[minecraft:root / minecraft:item] (Stable)]\n\
registryClass=class net.minecraft.core.DefaultedMappedRegistry\n\
stringCast=CCE\n\
";

const IDENTIFIER_STUB: &str = r#"package net.minecraft.resources;

public final class Identifier {
    public static Identifier parse(String id) { return null; }
}
"#;

const RESOURCE_KEY_STUB: &str = r#"package net.minecraft.resources;

public class ResourceKey<T> {
    public static <T> ResourceKey<T> create(ResourceKey<?> registry, Identifier id) { return null; }
    public static <T> ResourceKey<T> createRegistryKey(Identifier id) { return null; }
    public Identifier identifier() { return null; }
    public Identifier registry() { return null; }
    public ResourceKey<?> registryKey() { return null; }
}
"#;

const REGISTRIES_STUB: &str = r#"package net.minecraft.core.registries;

import net.minecraft.resources.ResourceKey;

public final class Registries {
    public static final ResourceKey<?> ITEM = null;
    public static final ResourceKey<?> BLOCK = null;
}
"#;

const BUILT_IN_REGISTRIES_STUB: &str = r#"package net.minecraft.core.registries;

import net.minecraft.core.DefaultedRegistry;

public final class BuiltInRegistries {
    public static final DefaultedRegistry<?> ITEM = null;
    public static final DefaultedRegistry<?> BLOCK = null;
}
"#;

const REGISTRY_STUB: &str = r#"package net.minecraft.core;

public interface Registry<T> extends Iterable<T> {
    net.minecraft.resources.ResourceKey<?> key();
}
"#;

const DEFAULTED_REGISTRY_STUB: &str = r#"package net.minecraft.core;

public interface DefaultedRegistry<T> extends Registry<T> {}
"#;

const WRITABLE_REGISTRY_STUB: &str = r#"package net.minecraft.core;

public interface WritableRegistry<T> extends Registry<T> {}
"#;

const HOLDER_STUB: &str = r#"package net.minecraft.core;

public interface Holder<T> {
    java.util.Optional<net.minecraft.resources.ResourceKey<T>> unwrapKey();
}
"#;

const DEFERRED_REGISTER_STUB: &str = r#"package net.neoforged.neoforge.registries;

import java.util.Collection;
import java.util.function.Supplier;
import net.minecraft.resources.ResourceKey;

public class DeferredRegister<T> {
    public static <T> DeferredRegister<T> create(ResourceKey<?> registry, String namespace) { return null; }
    public <I extends T> DeferredHolder<T, I> register(String name, Supplier<? extends I> value) { return null; }
    public Collection<DeferredHolder<T, ? extends T>> getEntries() { return null; }
}
"#;

const DEFERRED_HOLDER_STUB: &str = r#"package net.neoforged.neoforge.registries;

public class DeferredHolder<R, T extends R> {
    public net.minecraft.resources.Identifier getId() { return null; }
}
"#;

/// The probe the oracle ran (its `main` left out).
const PROBE: &str = r##"package demo;

import java.util.*;
import java.util.function.Supplier;
import net.minecraft.core.DefaultedRegistry;
import net.minecraft.core.Holder;
import net.minecraft.core.Registry;
import net.minecraft.core.WritableRegistry;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.core.registries.Registries;
import net.minecraft.resources.Identifier;
import net.minecraft.resources.ResourceKey;
import net.neoforged.neoforge.registries.DeferredHolder;
import net.neoforged.neoforge.registries.DeferredRegister;

@SuppressWarnings({"rawtypes", "unchecked"})
public final class Probe {
    static Identifier id(String text) { return Identifier.parse(text); }

    static String cast(Object o) {
        try {
            return String.valueOf(((ResourceKey) o).identifier());
        } catch (ClassCastException e) {
            return "CCE";
        }
    }

    /** Strings spelled like the old markers stay Strings. */
    public static String spelled() {
        Object[] texts = {"registry/minecraft:item", "holder/deferred-register/registry/minecraft:item/demo/thing",
            "registered/deferred-register/registry/minecraft:item/demo/thing", "hreg:minecraft:item", "hhold:1"};
        Object item = BuiltInRegistries.ITEM;
        Object key = Registries.ITEM;
        StringBuilder out = new StringBuilder();
        for (Object t : texts) {
            out.append(t instanceof Registry).append(t instanceof ResourceKey).append(t instanceof Holder)
                .append(t instanceof Supplier).append(t.equals(item)).append(item.equals(t)).append(key.equals(t))
                .append(t).append(',');
        }
        return out.append(cast("hreg:minecraft:item")).toString();
    }

    /** Registry keys: Registries.X is ResourceKey[minecraft:root / X], interned. */
    public static String keys() {
        ResourceKey k = Registries.ITEM;
        return k + "|" + k.identifier() + "|" + k.registry() + "|" + (k == ResourceKey.createRegistryKey(id("item")))
            + (k == Registries.BLOCK) + k.equals(Registries.ITEM) + "|" + ((Object) k).getClass().getName()
            + "|" + ResourceKey.create(Registries.ITEM, id("a:x")).registryKey() + (ResourceKey.create(Registries.ITEM, id("a:x")).registryKey() == Registries.ITEM)
            + "|" + cast(Registries.BLOCK) + "|" + List.of(Registries.ITEM, Registries.BLOCK) + List.of(Registries.ITEM).contains(Registries.ITEM)
            + "|" + ResourceKey.createRegistryKey(id("demo:things"));
    }

    /** Registries: one object per registry; its key; its class is an implementation. */
    public static String registries() {
        Registry item = BuiltInRegistries.ITEM;
        Object o = item;
        return (item == BuiltInRegistries.ITEM) + "" + (o == BuiltInRegistries.BLOCK) + item.equals(BuiltInRegistries.ITEM)
            + o.equals(BuiltInRegistries.BLOCK) + "|" + item.key() + (item.key() == Registries.ITEM) + "|"
            + (o instanceof Registry) + (o instanceof DefaultedRegistry) + (o instanceof WritableRegistry)
            + (o instanceof Supplier) + (o instanceof Holder) + (BuiltInRegistries.ITEM instanceof Iterable)
            + "|" + List.of(item).contains(BuiltInRegistries.ITEM) + List.of(item, BuiltInRegistries.BLOCK).indexOf(BuiltInRegistries.BLOCK);
    }

    static DeferredHolder holder(DeferredRegister register, String name) {
        return register.register(name, () -> "value");
    }

    /** NeoForge holders: each register() makes one holder object; getEntries lists those same objects. */
    public static String holders() {
        DeferredRegister r1 = DeferredRegister.create(Registries.ITEM, "demo");
        DeferredHolder a = holder(r1, "thing");
        DeferredHolder c = holder(r1, "other");
        DeferredHolder b = holder(DeferredRegister.create(Registries.BLOCK, "demo"), "thing");
        List entries = new ArrayList(r1.getEntries());
        Object o = a;
        return a + "|" + a.getId() + "|" + (a == b) + a.equals(b) + (a == c) + a.equals(c) + "|" + (entries.get(0) == a)
            + (entries.get(1) == c) + entries.contains(a) + entries.contains(b) + entries.size() + "|"
            + (o instanceof Holder) + (o instanceof Supplier) + (o instanceof DeferredHolder) + (o instanceof Registry)
            + (o instanceof ResourceKey) + "|" + ((Holder) a).unwrapKey().get() + "|" + List.of(a, c);
    }

    /** Strings spelled like a handle's text are Strings: class, hash, map keys. */
    public static String spelledStrings() {
        Object[] texts = {"hreg:minecraft:item", "hhold:i0,i0,null", "hrgd:0"};
        StringBuilder out = new StringBuilder();
        Map<Object, String> m = new HashMap<>();
        m.put(BuiltInRegistries.ITEM, "r");
        for (Object t : texts) {
            out.append(t.getClass().getSimpleName()).append(t instanceof String).append(t.hashCode()).append(',');
            m.put(t, "s");
        }
        return out.append(m.size()).append(m.get(BuiltInRegistries.ITEM)).append(m.get("hreg:minecraft:item")).toString();
    }

    /** Identity: one registry and one registry key per id; one holder per register call. */
    public static String identity() {
        DeferredRegister r = DeferredRegister.create(Registries.ITEM, "demo");
        DeferredHolder a = holder(r, "a");
        DeferredHolder b = holder(DeferredRegister.create(Registries.ITEM, "demo"), "a");
        Map<Object, Integer> m = new IdentityHashMap<>();
        m.put(BuiltInRegistries.ITEM, 1);
        m.put(BuiltInRegistries.ITEM, 2);
        m.put(Registries.ITEM, 3);
        m.put(ResourceKey.createRegistryKey(id("item")), 4);
        m.put(a, 5);
        m.put(b, 6);
        m.put(r.getEntries().iterator().next(), 7);
        return m.size() + "|" + m.get(BuiltInRegistries.ITEM) + m.get(Registries.ITEM) + m.get(a) + m.get(b) + "|"
            + (r.getEntries().iterator().next() == a) + (a == b) + (BuiltInRegistries.ITEM == BuiltInRegistries.ITEM);
    }

    /** Type switches and Class checks on the handles. */
    public static String types() {
        Object[] values = {BuiltInRegistries.ITEM, Registries.ITEM, holder(DeferredRegister.create(Registries.ITEM, "demo"), "t"), "registry/minecraft:item"};
        StringBuilder out = new StringBuilder();
        for (Object o : values) {
            String kind = switch (o) {
                case DefaultedRegistry r -> "defaulted:" + r.key().identifier();
                case Registry r -> "registry";
                case ResourceKey k -> "key:" + k.identifier();
                case DeferredHolder h -> "holder:" + h.getId();
                case String s -> "string";
                default -> "other";
            };
            out.append(kind).append(Registry.class.isInstance(o)).append(Holder.class.isInstance(o))
                .append(ResourceKey.class.isInstance(o)).append(Supplier.class.isInstance(o)).append(',');
        }
        Object key = Registries.ITEM;
        return out.append(ResourceKey.class.isAssignableFrom(key.getClass())).append(ResourceKey.class.cast(key)).toString();
    }

    /** Two registers' holders for one id are two objects. */
    public static String holderIdentity() {
        return "" + (holder(DeferredRegister.create(Registries.ITEM, "demo"), "x") == holder(DeferredRegister.create(Registries.ITEM, "demo"), "x"));
    }

    public static String holderHash() {
        return "" + holder(DeferredRegister.create(Registries.ITEM, "demo"), "x").hashCode();
    }

    public static String registryHash() {
        return "" + BuiltInRegistries.ITEM.hashCode();
    }

    public static String registryText() {
        String text = String.valueOf(BuiltInRegistries.ITEM);
        return text == null ? "null" : text;
    }

    /** A registry's class is an implementation (`DefaultedMappedRegistry` here). */
    public static String registryClass() {
        Object c = BuiltInRegistries.ITEM.getClass();
        return String.valueOf(c);
    }

    /** A String spelled like the old registry marker is not a ResourceKey: the cast throws. */
    public static String stringCast() {
        return cast("registry/minecraft:item");
    }
}
"##;

fn fixture(test: &str) -> javac_fixture::Fixture {
    javac_fixture::Fixture::build(
        test,
        &[
            ("net/minecraft/resources/Identifier.java", IDENTIFIER_STUB),
            (
                "net/minecraft/resources/ResourceKey.java",
                RESOURCE_KEY_STUB,
            ),
            (
                "net/minecraft/core/registries/Registries.java",
                REGISTRIES_STUB,
            ),
            (
                "net/minecraft/core/registries/BuiltInRegistries.java",
                BUILT_IN_REGISTRIES_STUB,
            ),
            ("net/minecraft/core/Registry.java", REGISTRY_STUB),
            (
                "net/minecraft/core/DefaultedRegistry.java",
                DEFAULTED_REGISTRY_STUB,
            ),
            (
                "net/minecraft/core/WritableRegistry.java",
                WRITABLE_REGISTRY_STUB,
            ),
            ("net/minecraft/core/Holder.java", HOLDER_STUB),
            (
                "net/neoforged/neoforge/registries/DeferredRegister.java",
                DEFERRED_REGISTER_STUB,
            ),
            (
                "net/neoforged/neoforge/registries/DeferredHolder.java",
                DEFERRED_HOLDER_STUB,
            ),
            ("demo/Probe.java", PROBE),
        ],
    )
}

/// The JVM's line for `method` in [`EXPECTED`].
fn expected(method: &str) -> &'static str {
    EXPECTED
        .lines()
        .find_map(|line| line.strip_prefix(method)?.strip_prefix('='))
        .unwrap_or_else(|| panic!("no expected line for {method}"))
}

/// Run `method`: its text and the unknown calls it counted.
fn run(fixture: &javac_fixture::Fixture, method: &str) -> (String, Vec<(String, usize)>) {
    let mut host = fixture.jar.seed_host(StdHost::new());
    let text = fixture.text_on(&mut host, method);
    (text, host.unknown_host_calls())
}

/// What `#163` leaves unknown: `instanceof` an interface the handle's
/// declared types do not have (a registry is no `Supplier`, but another
/// jar's Mixin could add one), which is counted, never answered.
const UNDECLARED_INTERFACES: &[&str] = &[
    "java/util/function/Supplier",
    "net/minecraft/core/Holder",
    "net/minecraft/core/Registry",
    "net/minecraft/core/DefaultedRegistry",
];

fn only_undeclared_interfaces(unknown: &[(String, usize)]) -> bool {
    unknown.iter().all(|(call, _)| {
        call.strip_suffix("#<instanceof>(Ljava/lang/Object;)Z")
            .is_some_and(|owner| UNDECLARED_INTERFACES.contains(&owner))
    })
}

/// Registries, registry keys and NeoForge holders answer as 26.3 does on
/// Java 25: `Registries.X` is the interned `ResourceKey[minecraft:root / X]`
/// and `registry.key()` that same key; one registry per key; one holder per
/// `register` call, which `getEntries` hands back, `equals` by identity,
/// printed `DeferredHolder{<key>}`; type switches and `Class` checks reach
/// the kinds' rows; `IdentityHashMap` and `==` follow identity; strings
/// spelled like a handle's text are Strings. A registry's and a holder's
/// class is an implementation the host does not know, so only an `instanceof`
/// an interface they do not declare is counted (`#163`).
#[test]
fn registries_and_holders_answer_as_java_25() {
    let fixture = fixture("host-registries");
    for method in [
        "spelledStrings",
        "identity",
        "keys",
        "registries",
        "holders",
        "types",
        "holderIdentity",
    ] {
        let (text, unknown) = run(&fixture, method);
        assert_eq!(text, expected(method), "{method}: {unknown:?}");
        assert!(
            only_undeclared_interfaces(&unknown),
            "{method}: {unknown:?}"
        );
    }
}

/// A string spelled like the old `registry/`, `holder/`, `registered/`
/// markers is none of those objects. Its `instanceof` against a game type is
/// still counted, because `word/rest` text is classified by
/// `is_plain_string` until #156 P17.
#[test]
fn a_string_spelled_like_an_old_marker_is_not_a_registry_or_holder() {
    let fixture = fixture("host-registries-spelled");
    let (text, unknown) = run(&fixture, "spelled");
    assert_eq!(text, expected("spelled"));
    assert!(
        unknown
            .iter()
            .all(|(call, _)| call.ends_with("#<instanceof>(Ljava/lang/Object;)Z")),
        "{unknown:?}"
    );
}

/// What only a JVM knows is counted, never answered: a holder's and a
/// registry's identity hashes, a registry's lifecycle in its `toString`, its
/// implementation class, and (until P17 classifies `word/rest` text) the cast
/// of the String `"registry/minecraft:item"` to `ResourceKey`, which Java 25
/// refuses with a `ClassCastException` and the host no longer lets through.
#[test]
fn what_only_a_jvm_knows_is_counted() {
    let fixture = fixture("host-registries-counted");
    for (method, call) in [
        ("holderHash", "#hashCode()I"),
        ("registryHash", "#hashCode()I"),
        ("registryText", "#toString()Ljava/lang/String;"),
        ("registryClass", "#getClass()Ljava/lang/Class;"),
        (
            "stringCast",
            "net/minecraft/resources/ResourceKey#<checkcast>(Ljava/lang/Object;)Lnet/minecraft/resources/ResourceKey;",
        ),
    ] {
        let (_, unknown) = run(&fixture, method);
        assert!(
            unknown.iter().any(|(counted, _)| counted.ends_with(call)),
            "{method}: {unknown:?}"
        );
    }
}

//! Registry and holder answers that were silently wrong (#182): what
//! `DeferredRegister.getEntries()` is, walking a registry whose game entries
//! the host does not hold, and `FabricRegistryBuilder.from` of a registry the
//! jar built itself. Each answer is now Java 25's, or counted. The stubs exist
//! only for `javac`: only `demo/` ships, so every call reaches the host's
//! model as a real mod's does.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::StdHost;

/// `java` (Temurin 25.0.5) running this `Probe`'s first eight methods,
/// compiled with `javac --release 21` against the real 26.3
/// `server-game.jar` and its libraries, NeoForge 26.3.0.101's universal jar
/// and its libraries, and fabric-registry-sync 8.0.4 (`main`, left out here,
/// ran `SharedConstants.tryDetectVersion()` and `Bootstrap.bootStrap()`
/// first), printed these lines, the same on two runs; pasted verbatim.
const EXPECTED: &str = "\
registers=falsefalse11truetruefalse\n\
registerClass=net.neoforged.neoforge.registries.DeferredRegistertrue\n\
spelledRegister=CCE\n\
entries=falsetruetruetrue22truetrue|truetruefalse|UOEUOEUOE|2|[DeferredHolder{ResourceKey[minecraft:item / demo:x]}, DeferredHolder{ResourceKey[minecraft:item / demo:y]}]\n\
entriesIteratorRemove=UOE\n\
entriesEmpty=true0[]falsetruefalse\n\
entriesClass=java.util.Collections$UnmodifiableSet\n\
vanillaIterates=truetrue\n\
";

const STUBS: &[(&str, &str)] = &[
    (
        "net/minecraft/resources/Identifier.java",
        "package net.minecraft.resources;\n\
         public final class Identifier {\n\
         public static Identifier fromNamespaceAndPath(String namespace, String path) { return null; }\n}\n",
    ),
    (
        "net/minecraft/resources/ResourceKey.java",
        "package net.minecraft.resources;\n\
         public class ResourceKey<T> {\n\
         public static <T> ResourceKey<T> createRegistryKey(Identifier id) { return null; }\n}\n",
    ),
    (
        "net/minecraft/core/registries/Registries.java",
        "package net.minecraft.core.registries;\n\
         public final class Registries {\n\
         public static final net.minecraft.resources.ResourceKey<?> ITEM = null;\n\
         public static final net.minecraft.resources.ResourceKey<?> BLOCK = null;\n}\n",
    ),
    (
        "net/minecraft/core/registries/BuiltInRegistries.java",
        "package net.minecraft.core.registries;\n\
         public final class BuiltInRegistries {\n\
         public static final net.minecraft.core.DefaultedRegistry<?> ITEM = null;\n\
         public static final net.minecraft.core.DefaultedRegistry<?> BLOCK = null;\n}\n",
    ),
    (
        "net/minecraft/core/Registry.java",
        "package net.minecraft.core;\n\
         public interface Registry<T> extends Iterable<T> {\n\
         static <T> T register(Registry<? super T> registry, net.minecraft.resources.Identifier id, T value) { return value; }\n}\n",
    ),
    (
        "net/minecraft/core/DefaultedRegistry.java",
        "package net.minecraft.core;\npublic interface DefaultedRegistry<T> extends Registry<T> {}\n",
    ),
    (
        "net/minecraft/core/WritableRegistry.java",
        "package net.minecraft.core;\npublic interface WritableRegistry<T> extends Registry<T> {}\n",
    ),
    (
        "com/mojang/serialization/Lifecycle.java",
        "package com.mojang.serialization;\npublic class Lifecycle {}\n",
    ),
    (
        "net/minecraft/core/MappedRegistry.java",
        "package net.minecraft.core;\n\
         public class MappedRegistry<T> implements WritableRegistry<T> {\n\
         public MappedRegistry(net.minecraft.resources.ResourceKey<? extends Registry<T>> key, com.mojang.serialization.Lifecycle lifecycle) {}\n\
         public java.util.Iterator<T> iterator() { return null; }\n}\n",
    ),
    (
        "net/neoforged/neoforge/registries/DeferredRegister.java",
        "package net.neoforged.neoforge.registries;\n\
         public class DeferredRegister<T> {\n\
         public static <T> DeferredRegister<T> create(net.minecraft.resources.ResourceKey<?> registry, String namespace) { return null; }\n\
         public <I extends T> DeferredHolder<T, I> register(String name, java.util.function.Supplier<? extends I> value) { return null; }\n\
         public java.util.Collection<DeferredHolder<T, ? extends T>> getEntries() { return null; }\n}\n",
    ),
    (
        "net/neoforged/neoforge/registries/DeferredHolder.java",
        "package net.neoforged.neoforge.registries;\npublic class DeferredHolder<R, T extends R> {}\n",
    ),
    (
        "net/fabricmc/fabric/api/event/registry/FabricRegistryBuilder.java",
        "package net.fabricmc.fabric.api.event.registry;\n\
         public final class FabricRegistryBuilder<T, R extends net.minecraft.core.WritableRegistry<T>> {\n\
         public static <T> FabricRegistryBuilder<T, net.minecraft.core.MappedRegistry<T>> create(net.minecraft.resources.ResourceKey<?> key) { return null; }\n\
         public static <T, R extends net.minecraft.core.WritableRegistry<T>> FabricRegistryBuilder<T, R> from(R registry) { return null; }\n\
         public R buildAndRegister() { return null; }\n}\n",
    ),
];

/// The probe. The oracle ran its first eight methods; the Fabric ones need
/// Fabric's Mixins (`buildAndRegister` registers through an accessor), so
/// their Java answer is fabric-registry-sync 8.0.4's bytecode: `from(r)` is
/// `new FabricRegistryBuilder(r)` and `buildAndRegister()` ends
/// `getfield registry; areturn` (@69–72), the very object `from` was given.
const PROBE: &str = r##"package demo;

import java.util.*;
import net.minecraft.core.MappedRegistry;
import net.minecraft.core.Registry;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.core.registries.Registries;
import net.minecraft.resources.Identifier;
import net.minecraft.resources.ResourceKey;
import net.neoforged.neoforge.registries.DeferredHolder;
import net.neoforged.neoforge.registries.DeferredRegister;
import net.fabricmc.fabric.api.event.registry.FabricRegistryBuilder;

@SuppressWarnings({"rawtypes", "unchecked"})
public final class Probe {
    static DeferredHolder reg(DeferredRegister r, String name) { return r.register(name, () -> "value"); }

    public static String registers() {
        DeferredRegister r1 = DeferredRegister.create(Registries.ITEM, "demo");
        DeferredRegister r2 = DeferredRegister.create(Registries.ITEM, "demo");
        DeferredHolder x = reg(r1, "x");
        DeferredHolder y = reg(r2, "y");
        return "" + (r1 == r2) + r1.equals(r2) + r1.getEntries().size() + r2.getEntries().size()
            + (r1.getEntries().iterator().next() == x) + (r2.getEntries().iterator().next() == y)
            + r1.getEntries().contains(y);
    }

    public static String registerClass() {
        Object r = DeferredRegister.create(Registries.ITEM, "demo");
        return r.getClass().getName() + (r instanceof DeferredRegister);
    }

    public static String spelledRegister() {
        try { return "cast" + ((DeferredRegister) (Object) "deferred-register/registry/minecraft:item/demo").getEntries(); }
        catch (ClassCastException e) { return "CCE"; }
    }

    public static String entries() {
        DeferredRegister r = DeferredRegister.create(Registries.ITEM, "demo");
        Collection before = r.getEntries();
        DeferredHolder a = reg(r, "x");
        Collection e = r.getEntries();
        DeferredHolder b = reg(r, "y");
        String s = "" + (e instanceof List) + (e instanceof Set) + (e == before) + (e == r.getEntries())
            + e.size() + before.size() + e.contains(a) + e.contains(b);
        Iterator it = e.iterator();
        s += "|" + (it.next() == a) + (it.next() == b) + it.hasNext();
        try { e.add(a); s += "|added"; } catch (UnsupportedOperationException x) { s += "|UOE"; }
        try { e.remove(a); s += "removed"; } catch (UnsupportedOperationException x) { s += "UOE"; }
        try { e.clear(); s += "cleared"; } catch (UnsupportedOperationException x) { s += "UOE"; }
        return s + "|" + e.size() + "|" + e;
    }

    public static String entriesIteratorRemove() {
        DeferredRegister r = DeferredRegister.create(Registries.ITEM, "demo");
        reg(r, "x");
        try { Iterator i = r.getEntries().iterator(); i.next(); i.remove(); return "removed" + r.getEntries().size(); }
        catch (UnsupportedOperationException x) { return "UOE"; }
    }

    public static String entriesEmpty() {
        Collection e = DeferredRegister.create(Registries.BLOCK, "demo").getEntries();
        return "" + e.isEmpty() + e.size() + e + e.iterator().hasNext() + e.equals(Set.of()) + e.equals(List.of());
    }

    public static String entriesClass() {
        return DeferredRegister.create(Registries.ITEM, "demo").getEntries().getClass().getName();
    }

    public static String vanillaIterates() {
        return "" + BuiltInRegistries.ITEM.iterator().hasNext() + BuiltInRegistries.BLOCK.iterator().hasNext();
    }

    static ResourceKey key(String path) {
        return ResourceKey.createRegistryKey(Identifier.fromNamespaceAndPath("demo", path));
    }

    /** A registry the jar built and registered: it holds only what the jar registers. */
    public static String builtWalk() {
        Registry r = FabricRegistryBuilder.create(key("things")).buildAndRegister();
        Registry.register(r, Identifier.fromNamespaceAndPath("demo", "first"), "first");
        Registry.register(r, Identifier.fromNamespaceAndPath("demo", "second"), "second");
        List<Object> seen = new ArrayList<>();
        for (Object value : r) seen.add(value);
        return seen.toString();
    }

    /** Java: `true`, the jar's own object. */
    public static String fromMine() {
        MappedRegistry mine = new MappedRegistry(key("mine"), null);
        return "" + (FabricRegistryBuilder.from(mine).buildAndRegister() == mine);
    }
}
"##;

fn fixture(test: &str) -> javac_fixture::Fixture {
    let mut sources = STUBS.to_vec();
    sources.push(("demo/Probe.java", PROBE));
    javac_fixture::Fixture::build(test, &sources)
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

/// Each `DeferredRegister.create` is a new register (`new DeferredRegister`,
/// whose `equals` is `Object`'s) holding only what was registered through it,
/// of the class `create` made; a `String` spelled like a register's old text
/// is no register. Two `create` calls for one registry and namespace were one
/// `String` sharing one entry list (#182 P1).
#[test]
fn each_create_is_a_register_of_its_own() {
    let fixture = fixture("registry-answers-registers");
    for method in ["registers", "registerClass", "spelledRegister"] {
        let (text, unknown) = run(&fixture, method);
        assert_eq!(text, expected(method), "{method}: {unknown:?}");
        assert!(unknown.is_empty(), "{method}: {unknown:?}");
    }
}

/// `getEntries()` is NeoForge's `entriesView`: one `UnmodifiableSet` per
/// register, made before any `register` and live after it, whose mutators
/// (and its iterator's `remove`) throw, in registration order, printed and
/// compared as a set. It was a new mutable `List` per call (#182 P2).
#[test]
fn get_entries_is_the_registers_unmodifiable_live_set() {
    let fixture = fixture("registry-answers-entries");
    for method in ["entries", "entriesEmpty", "entriesClass"] {
        let (text, unknown) = run(&fixture, method);
        assert_eq!(text, expected(method), "{method}: {unknown:?}");
        assert!(unknown.is_empty(), "{method}: {unknown:?}");
    }
}

/// The view's iterator is `UnmodifiableCollection`'s, whose `remove` throws
/// `UnsupportedOperationException`. A host iterator does not know the view
/// it walks is unmodifiable, so `remove` on it is counted, not answered.
#[test]
fn removing_through_the_views_iterator_is_counted() {
    let fixture = fixture("registry-answers-iterator-remove");
    let (_, unknown) = run(&fixture, "entriesIteratorRemove");
    assert_eq!(
        unknown,
        vec![("java/util/Iterator#remove()V".to_owned(), 1)],
        "{unknown:?}"
    );
}

/// Walking a vanilla registry yields the game's entries, which the host does
/// not hold: the walk is counted, never answered as an empty registry (#182
/// P3; Java 25 answers `truetrue`).
#[test]
fn walking_a_vanilla_registry_is_counted() {
    let fixture = fixture("registry-answers-vanilla");
    let (text, unknown) = run(&fixture, "vanillaIterates");
    assert_ne!(text, expected("vanillaIterates"));
    let walks: usize = unknown
        .iter()
        .filter(|(call, _)| call.ends_with("#iterator()Ljava/util/Iterator;"))
        .map(|(_, count)| count)
        .sum();
    assert_eq!(walks, 2, "{unknown:?}");
}

/// A registry the jar built and registered holds only what the jar
/// registers into it, so its walk is exact and nothing is counted.
#[test]
fn a_registry_the_jar_built_walks_exactly() {
    let fixture = fixture("registry-answers-built");
    let (text, unknown) = run(&fixture, "builtWalk");
    assert_eq!(text, "[first, second]");
    assert!(unknown.is_empty(), "{unknown:?}");
}

/// `from(mine).buildAndRegister()` is `mine` in Java. The host answers the
/// `Registry` handle for the registry's id, which later registrations name,
/// so the call is counted: `== mine` is no longer silently `false` (#182 P4).
#[test]
fn from_a_jar_built_registry_counts_build_and_register() {
    let fixture = fixture("registry-answers-from");
    let (_, unknown) = run(&fixture, "fromMine");
    assert!(
        unknown.iter().any(|(call, _)| call
            == "net/fabricmc/fabric/api/event/registry/FabricRegistryBuilder#buildAndRegister()Lnet/minecraft/core/WritableRegistry;"),
        "{unknown:?}"
    );
}

//! Holders before NeoForge binds them, Fabric registry builders, and Strings
//! spelled like the registry family's old text (#177's review: B1, B2, M1).
//!
//! B1: `FabricRegistryBuilder` was `fabric-registry-builder/<digit><id>`
//! text, so a jar String spelled so built a canonical registry (even
//! `BuiltInRegistries.ITEM`) from text. A builder is a handle now, and the
//! String's cast throws. B2: a holder answered as bound before its
//! `RegisterEvent`; NeoForge throws its unbound-value NPE, and every way of
//! asking (`DeferredHolder`, `Holder`, `Supplier`) answers alike. M1:
//! `Holder.isBound()` on what is no holder is counted, and old-marker Strings
//! cast to `Registry`/`Holder`/`DeferredHolder` throw. The stubs exist only
//! for `javac`: only `demo/` ships.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::StdHost;

/// `java` (Temurin 25.0.5) running these methods, compiled with `javac
/// --release 21` against the real 26.3 `server-game.jar` and its server
/// libraries (after `SharedConstants.tryDetectVersion(); Bootstrap.bootStrap()`),
/// NeoForge 26.3.0.101's universal jar and its libraries, and
/// fabric-registry-sync-v0 8.0.4 (fabric-api 0.165.0). The `c*` lines are
/// the #177 reviewer's probe (rv177, fabric-registry-sync 8.0.5, whose
/// builder bytecode is the same), the rest this fix's; pasted verbatim, the
/// same on two runs.
const EXPECTED: &str = "\
c10=falsefalse\n\
c11=NPE:Trying to access unbound value: ResourceKey[minecraft:item / demo:x]\n\
c12=NPE:Trying to access unbound value: ResourceKey[minecraft:item / demo:x]\n\
c13=NPE:Trying to access unbound value: ResourceKey[minecraft:item / demo:x]\n\
c63=CCE\n\
c66=CCE\n\
c67=CCE\n\
c70=CCE\n\
c71=CCE\n\
c72=CCE\n\
c74=CCE\n\
unboundViews=Optional.empty|false|falsefalse\n\
vanillaEntry=true\n\
customRegistry=false|ISE:Registry not present for DeferredHolder{ResourceKey[demo:things / demo:x]}: demo:things\n\
builders=truetruetruetrue|net.fabricmc.fabric.api.event.registry.FabricRegistryBuilder\n\
twoBuilders=falsefalse\n\
";

const STUBS: &[(&str, &str)] = &[
    (
        "net/minecraft/resources/Identifier.java",
        "package net.minecraft.resources;\n\
         public final class Identifier { public static Identifier parse(String id) { return null; } }\n",
    ),
    (
        "net/minecraft/resources/ResourceKey.java",
        "package net.minecraft.resources;\n\
         public class ResourceKey<T> {\n\
         public static <T> ResourceKey<T> createRegistryKey(Identifier id) { return null; }\n\
         public Identifier identifier() { return null; }\n}\n",
    ),
    (
        "net/minecraft/core/registries/Registries.java",
        "package net.minecraft.core.registries;\n\
         public final class Registries { public static final net.minecraft.resources.ResourceKey<?> ITEM = null; }\n",
    ),
    (
        "net/minecraft/core/Registry.java",
        "package net.minecraft.core;\n\
         public interface Registry<T> extends Iterable<T> { net.minecraft.resources.ResourceKey<?> key(); }\n",
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
        "net/minecraft/core/MappedRegistry.java",
        "package net.minecraft.core;\n\
         public abstract class MappedRegistry<T> implements WritableRegistry<T> {}\n",
    ),
    (
        "net/minecraft/core/DefaultedMappedRegistry.java",
        "package net.minecraft.core;\n\
         public abstract class DefaultedMappedRegistry<T> extends MappedRegistry<T> implements DefaultedRegistry<T> {}\n",
    ),
    (
        "net/minecraft/core/Holder.java",
        "package net.minecraft.core;\n\
         public interface Holder<T> { T value(); boolean isBound(); }\n",
    ),
    (
        "net/neoforged/neoforge/registries/DeferredRegister.java",
        "package net.neoforged.neoforge.registries;\n\
         public class DeferredRegister<T> {\n\
         public static <T> DeferredRegister<T> create(net.minecraft.resources.ResourceKey<?> registry, String namespace) { return null; }\n\
         public <I extends T> DeferredHolder<T, I> register(String name, java.util.function.Supplier<? extends I> value) { return null; }\n}\n",
    ),
    (
        "net/neoforged/neoforge/registries/DeferredHolder.java",
        "package net.neoforged.neoforge.registries;\n\
         public class DeferredHolder<R, T extends R> implements net.minecraft.core.Holder<R>, java.util.function.Supplier<T> {\n\
         public R value() { return null; }\n\
         public T get() { return null; }\n\
         public boolean isBound() { return false; }\n\
         public java.util.Optional<T> asOptional() { return null; }\n\
         public net.minecraft.resources.Identifier getId() { return null; }\n}\n",
    ),
    (
        "net/fabricmc/fabric/api/event/registry/RegistryAttribute.java",
        "package net.fabricmc.fabric.api.event.registry;\n\
         public enum RegistryAttribute { PERSISTED, MODDED, SYNCED }\n",
    ),
    (
        "net/fabricmc/fabric/api/event/registry/FabricRegistryBuilder.java",
        "package net.fabricmc.fabric.api.event.registry;\n\
         public final class FabricRegistryBuilder<T, R extends net.minecraft.core.WritableRegistry<T>> {\n\
         public static <T> FabricRegistryBuilder<T, net.minecraft.core.MappedRegistry<T>> create(net.minecraft.resources.ResourceKey<?> key) { return null; }\n\
         public FabricRegistryBuilder<T, R> attribute(RegistryAttribute attribute) { return this; }\n\
         public R buildAndRegister() { return null; }\n}\n",
    ),
];

/// The probes the oracle ran (their `main` left out).
const PROBE: &str = r##"package demo;

import java.util.function.Supplier;
import net.minecraft.core.DefaultedMappedRegistry;
import net.minecraft.core.DefaultedRegistry;
import net.minecraft.core.Holder;
import net.minecraft.core.MappedRegistry;
import net.minecraft.core.Registry;
import net.minecraft.core.registries.Registries;
import net.minecraft.resources.Identifier;
import net.minecraft.resources.ResourceKey;
import net.neoforged.neoforge.registries.DeferredHolder;
import net.neoforged.neoforge.registries.DeferredRegister;
import net.fabricmc.fabric.api.event.registry.FabricRegistryBuilder;
import net.fabricmc.fabric.api.event.registry.RegistryAttribute;

@SuppressWarnings({"rawtypes", "unchecked"})
public final class Probe {
    static Identifier id(String t) { return Identifier.parse(t); }
    static DeferredHolder reg(DeferredRegister r, String name) { return r.register(name, () -> "value"); }
    static String exc(Throwable e) {
        if (e instanceof NullPointerException) return "NPE:" + e.getMessage();
        if (e instanceof IllegalStateException) return "ISE:" + e.getMessage();
        if (e instanceof ClassCastException) return "CCE";
        return "EXC";
    }

    public static String c10() {
        DeferredHolder a = reg(DeferredRegister.create(Registries.ITEM, "demo"), "x");
        Holder h = a;
        return "" + a.isBound() + h.isBound();
    }
    public static String c11() {
        DeferredHolder a = reg(DeferredRegister.create(Registries.ITEM, "demo"), "x");
        try { Object v = a.get(); return "got:" + (v == a.get()); } catch (RuntimeException e) { return exc(e); }
    }
    public static String c12() {
        Holder h = reg(DeferredRegister.create(Registries.ITEM, "demo"), "x");
        try { Object v = h.value(); return "got:" + (v != null); } catch (RuntimeException e) { return exc(e); }
    }
    public static String c13() {
        DeferredHolder a = reg(DeferredRegister.create(Registries.ITEM, "demo"), "x");
        Supplier s = a;
        try { return "got:" + (s.get() != null); } catch (RuntimeException e) { return exc(e); }
    }
    public static String c63() {
        try { Registry r = (Registry) (Object) "registry/minecraft:item"; return "cast:" + r.key(); } catch (ClassCastException e) { return "CCE"; }
    }
    public static String c66() {
        try { Holder h = (Holder) (Object) "holder/deferred-register/registry/minecraft:item/demo/x"; return "cast:" + h.isBound(); }
        catch (ClassCastException e) { return "CCE"; }
    }
    public static String c67() {
        try { DeferredHolder h = (DeferredHolder) (Object) "holder/deferred-register/registry/minecraft:item/demo/x"; return "cast:" + h.get(); }
        catch (ClassCastException e) { return "CCE"; }
        catch (RuntimeException e) { return exc(e); }
    }
    public static String c70() {
        try { Object r = ((FabricRegistryBuilder) (Object) "fabric-registry-builder/1minecraft:item").buildAndRegister();
              return "cast:" + (r == (Object) net.minecraft.core.registries.BuiltInRegistries.ITEM) + (r instanceof DefaultedRegistry); }
        catch (ClassCastException e) { return "CCE"; }
    }
    public static String c71() {
        try { Registry r = (Registry) ((FabricRegistryBuilder) (Object) "fabric-registry-builder/2Not An Id!").buildAndRegister();
              return "cast:" + (r instanceof DefaultedRegistry) + "|" + r.key().identifier(); }
        catch (ClassCastException e) { return "CCE"; }
    }
    public static String c72() {
        try { Registry r = (Registry) ((FabricRegistryBuilder) (Object) "fabric-registry-builder/1demo:x").buildAndRegister();
              Registry s = (Registry) ((FabricRegistryBuilder) (Object) "fabric-registry-builder/2demo:x").buildAndRegister();
              return "cast:" + (r == s) + (r instanceof MappedRegistry) + (s instanceof DefaultedMappedRegistry); }
        catch (ClassCastException e) { return "CCE"; }
    }
    public static String c74() {
        Object o = "holder/deferred-register/registry/minecraft:item/demo/x";
        try { return "cast:" + ((DeferredHolder) o).getId(); } catch (ClassCastException e) { return "CCE"; }
    }

    /** asOptional and isBound through every view, before RegisterEvent. */
    public static String unboundViews() {
        DeferredHolder a = reg(DeferredRegister.create(Registries.ITEM, "demo"), "x");
        Holder h = a;
        return a.asOptional() + "|" + a.asOptional().isPresent() + "|" + a.isBound() + h.isBound();
    }

    /** An entry vanilla already has is bound during construction. */
    public static String vanillaEntry() {
        DeferredHolder a = reg(DeferredRegister.create(Registries.ITEM, "minecraft"), "stone");
        return "" + a.isBound();
    }

    /** A holder of a registry that does not exist yet. */
    public static String customRegistry() {
        DeferredHolder a = reg(DeferredRegister.create(ResourceKey.createRegistryKey(id("demo:things")), "demo"), "x");
        String s = a.isBound() + "|";
        try { return s + a.get(); } catch (RuntimeException e) { return s + exc(e); }
    }

    /** A builder is a FabricRegistryBuilder; the same builder is itself; two builders are two. */
    public static String builders() {
        ResourceKey k = ResourceKey.createRegistryKey(id("demo:built"));
        FabricRegistryBuilder b = FabricRegistryBuilder.create(k);
        Object o = b;
        return (o instanceof FabricRegistryBuilder) + "" + (b == b) + b.equals(b) + (b.attribute(RegistryAttribute.SYNCED) == b)
            + "|" + o.getClass().getName();
    }

    public static String twoBuilders() {
        ResourceKey k = ResourceKey.createRegistryKey(id("demo:built"));
        Object a = FabricRegistryBuilder.create(k);
        Object b = FabricRegistryBuilder.create(k);
        return "" + (a == b) + a.equals(b);
    }
}
"##;

fn fixture(test: &str) -> javac_fixture::Fixture {
    let mut sources = STUBS.to_vec();
    sources.push((
        "net/minecraft/core/registries/BuiltInRegistries.java",
        "package net.minecraft.core.registries;\n\
         public final class BuiltInRegistries { public static final net.minecraft.core.DefaultedRegistry<?> ITEM = null; }\n",
    ));
    sources.push(("demo/Probe.java", PROBE));
    javac_fixture::Fixture::build(test, &sources)
}

fn expected(method: &str) -> &'static str {
    EXPECTED
        .lines()
        .find_map(|line| line.strip_prefix(method)?.strip_prefix('='))
        .unwrap_or_else(|| panic!("no expected line for {method}"))
}

/// Run `method` on a fresh host (mod construction: no `RegisterEvent` has
/// fired): its text and the unknown calls it counted.
fn run(fixture: &javac_fixture::Fixture, method: &str) -> (String, Vec<(String, usize)>) {
    let mut host = fixture.jar.seed_host(StdHost::new());
    let text = fixture.text_on(&mut host, method);
    (text, host.unknown_host_calls())
}

/// B1 and M1: Strings spelled like a registry, a holder or a builder once
/// were are Strings, so their casts to `Registry`, `Holder`,
/// `DeferredHolder` and `FabricRegistryBuilder` throw Java's
/// `ClassCastException`. No canonical registry is ever built from them.
#[test]
fn old_marker_strings_do_not_cast_to_the_family() {
    let fixture = fixture("host-registries-retired");
    for method in ["c63", "c66", "c67", "c70", "c71", "c72", "c74"] {
        let (text, unknown) = run(&fixture, method);
        assert_eq!(text, expected(method), "{method}: {unknown:?}");
        assert!(unknown.is_empty(), "{method}: {unknown:?}");
    }
}

/// B1: a builder the runtime made is a `FabricRegistryBuilder` (its class
/// exact: the class is final), a fluent setter answers the same object, and
/// two builders for one key are two objects: identity there is counted, never
/// answered from equal keys.
#[test]
fn registry_builders_are_handles() {
    let fixture = fixture("host-registries-builders");
    let (text, unknown) = run(&fixture, "builders");
    assert_eq!(text, expected("builders"), "{unknown:?}");
    // Fabric's `RegistryAttribute` constants are not modelled: the read is
    // counted (the setter answers the builder whatever its argument).
    assert!(
        unknown.iter().all(|(call, _)| call
            .starts_with("net/fabricmc/fabric/api/event/registry/RegistryAttribute#SYNCED")),
        "{unknown:?}"
    );
    let (_, unknown) = run(&fixture, "twoBuilders");
    for counted in ["#if_acmp", "#equals(Ljava/lang/Object;)Z"] {
        assert!(
            unknown.iter().any(|(call, _)| call.contains(counted)),
            "{counted}: {unknown:?}"
        );
    }
}

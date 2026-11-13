//! Game ids are typed handles (`Value::Host`, #156 P4): `Identifier`,
//! `ResourceKey`, `TagKey` and `BlockItemId` (`host_ids.rs`).
//!
//! They used to be text a jar `String` could spell (`identifier/a:x`,
//! `tag-key/…`, `resource-key/…`, `block-item-id/…`). The stubs exist only
//! for `javac`: only `demo/` ships, so every call reaches the host's model as
//! a real mod's does.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::StdHost;

/// `java` (Temurin 25.0.5) running this `Probe`, compiled with `javac
/// --release 21` against the real 26.3 `server-game.jar` and its libraries
/// plus NeoForge 26.3.0.101's universal jar and its libraries (`main`, left
/// out here, ran `Bootstrap.bootStrap()` first: a `DeferredHolder` needs it),
/// printed these lines, the same on two runs; pasted verbatim. `keyHash` and
/// `tagHash` are identity hashes (they changed when the probe changed): no
/// run can reproduce them, so the interpreter counts them (tests below).
const EXPECTED: &str = "\
spelled=falsefalsefalsefalsefalsefalsefalsefalseidentifier/a:x,falsefalsefalsefalsefalsefalsefalsefalseresource-key/registry/minecraft:item/ridentifier/a:x,falsefalsefalsefalsefalsefalsefalsefalsetag-key/registry/minecraft:item|a:t,falsefalsefalsefalsefalsefalsefalsefalseblock-item-id/ridentifier/a:b,ridentifier/a:i,falsefalsefalsefalsefalsefalsefalsefalsehid:a:x,falsefalsefalsefalsefalsefalsefalsefalsehrk:minecraft:item|a:x,falsefalsefalsefalsefalsefalsefalsefalsea:x,\n\
spelledStrings=Stringtrue911338280,Stringtrue793652700,Stringtrue1216354522,Stringtrue95135,3iht|identifier/a:xa:xhid:a:x\n\
slashStrings=falsefalse,falsefalse,falsefalse,falsefalse,5is|identifier/a:xa:xtag-key/registry/minecraft:item|a:t\n\
identifiers=a:x|3127|truefalsefalsetrue|-11|truetrue|net.minecraft.resources.Identifier|a:x3127true|[a:x, b:x]true1\n\
hashed={minecraft:stone=0, minecraft:silk_touch=4, demo:machine=1, a:x=2, z:q=3, b:long/path/here=5}|[minecraft:stone, minecraft:silk_touch, demo:machine, a:x, z:q, b:long/path/here]|2truefalse|[b:long/path/here, demo:machine, z:q, minecraft:silk_touch, minecraft:stone, a:x]\n\
keys=ResourceKey[minecraft:item / a:x]|a:x|minecraft:item|truefalse|truefalsefalse|net.minecraft.resources.ResourceKeytrue|ResourceKey[minecraft:dimension / minecraft:overworld]|[ResourceKey[minecraft:item / a:x], ResourceKey[minecraft:dimension / minecraft:overworld]]true|true\n\
tags=TagKey[minecraft:item / a:t]|a:t|truefalse|truefalsefalsefalse|net.minecraft.tags.TagKeytrue|true\n\
blockItems=BlockItemId[block=ResourceKey[minecraft:block / minecraft:lamp], item=ResourceKey[minecraft:item / minecraft:lamp]]|ResourceKey[minecraft:block / a:b]|ResourceKey[minecraft:item / a:i]|truefalsefalse|true|net.minecraft.references.BlockItemIdtrue|BlockItemId[block=ResourceKey[minecraft:block / minecraft:b], item=ResourceKey[minecraft:item / minecraft:i]]\n\
types=mc:stonetruefalsetruefalsetrue,id:a:xtruefalsetruefalsefalse,key:a:kfalsetruefalsefalsefalse,tag:a:tfalsefalsefalsetruefalse,bii:minecraft:lampfalsefalsefalsetruefalse,str:a:xfalsefalsetruefalsefalse,truetruefalsea:xtrue\n\
sorts=[a:a, b:a, a:b, c:b, c:b/c, minecraft:stone, z:stone]|[a:a, b:a, a:b, c:b, c:b/c, minecraft:stone, z:stone]|[a:a, b:a, a:b, c:b, c:b/c, minecraft:stone, z:stone]|{a:a=3, b:a=0, a:b=1, c:b=6, c:b/c=5, minecraft:stone=2, z:stone=4}|a:az:stone3true|[a:a, b:a, a:b, c:b, c:b/c, minecraft:stone, z:stone]true|[z:stone, minecraft:stone, c:b/c, c:b, a:b, b:a, a:a]|z:stonea:a\n\
keySort=CCE\n\
collections=4,truetruetruetrue4|truetrue[b:y]|onetrue{a:x=one}|true5true1|{a:x=ResourceKey[minecraft:block / a:k], b:k=TagKey[minecraft:item / a:t]}TagKey[minecraft:item / a:t]true|a:x=3|101057,3127,[a:x]\n\
holders=demo:thing|ResourceKey[minecraft:item / demo:thing]|truetruefalsefalse|demo:other/path_1.x-y|other/path_1.x-y|IdentifierException:Non [a-z0-9_.-] character in namespace of identifier: x:y:z|IdentifierException:Non [a-z0-9/._-] character in path of location: x:y:z|IdentifierException:Non [a-z0-9_.-] character in namespace of identifier: Bad Mod:ok|IdentifierException:Non [a-z0-9/._-] character in path of location: ok:UPPER|case|IdentifierException:Non [a-z0-9_.-] character in namespace of identifier: x/y:z|IdentifierException:Non [a-z0-9/._-] character in path of location: x:y,z#w?\n\
holderSeparators=trueIdentifierException:Non [a-z0-9/._-] character in path of location: a:b:c,truea:b/c,trueIdentifierException:Non [a-z0-9/._-] character in path of location: a:b|c,trueIdentifierException:Non [a-z0-9/._-] character in path of location: a:b,c,trueIdentifierException:Non [a-z0-9/._-] character in path of location: a:b#c,trueIdentifierException:Non [a-z0-9/._-] character in path of location: a:b?c,trueIdentifierException:Non [a-z0-9/._-] character in path of location: a:b\\\\c,trueIdentifierException:Non [a-z0-9/._-] character in path of location: a:b c,\n\
tagRegistries=falsefalsefalse|falsetruetrue|trueResourceKey[minecraft:item / minecraft:block]|TagKey[minecraft:block / a:t]|a:t|truetruefalse\n\
exceptions=E:net.minecraft.IdentifierException:Non [a-z0-9_.-] character in namespace of identifier: A:btrue|T:IdentifierException|R:Non [a-z0-9/._-] character in path of location: a:B|IdentifierException:Non [a-z0-9/._-] character in path of location: a:q\\\"\\\\\\t\\u00E9\\uD83D\\uDE00\u{7f}\\u0001/|IdentifierException:Non [a-z0-9_.-] character in namespace of identifier: \\u00C9:x\n\
nulls=NullPointerException:Cannot invoke \"String.indexOf(int)\" because \"identifier\" is null|NullPointerException:Cannot invoke \"String.indexOf(int)\" because \"identifier\" is null|NullPointerException:Cannot invoke \"String.equals(Object)\" because \"namespace\" is null|NullPointerException:Cannot invoke \"String.length()\" because \"path\" is null|NullPointerException:Cannot invoke \"String.length()\" because \"path\" is null|NullPointerException:Cannot invoke \"String.equals(Object)\" because \"namespace\" is null\n\
identity=truetruetruetruetrue|326\n\
identifierIdentity=false\n\
identifierIdentityMap=2\n\
blockItemIdentity=false\n\
keyHash=362479845\n\
tagHash=1497761092\n\
";

const IDENTIFIER_STUB: &str = r#"package net.minecraft.resources;

public final class Identifier implements Comparable<Identifier> {
    public static Identifier fromNamespaceAndPath(String n, String p) { return null; }
    public static Identifier parse(String id) { return null; }
    public static Identifier tryParse(String id) { return null; }
    public static Identifier withDefaultNamespace(String path) { return null; }
    public static Identifier tryBuild(String n, String p) { return null; }
    public String getNamespace() { return null; }
    public String getPath() { return null; }
    public int compareTo(Identifier o) { return 0; }
}
"#;

const RESOURCE_KEY_STUB: &str = r#"package net.minecraft.resources;

public class ResourceKey<T> {
    public static <T> ResourceKey<T> create(ResourceKey<?> registry, Identifier id) { return null; }
    public Identifier identifier() { return null; }
    public Identifier registry() { return null; }
    public boolean isFor(ResourceKey<?> registry) { return false; }
}
"#;

const REGISTRIES_STUB: &str = r#"package net.minecraft.core.registries;

import net.minecraft.resources.ResourceKey;

public final class Registries {
    public static final ResourceKey<?> ITEM = null;
    public static final ResourceKey<?> BLOCK = null;
    public static final ResourceKey<?> DIMENSION = null;
}
"#;

const TAG_KEY_STUB: &str = r#"package net.minecraft.tags;

import net.minecraft.resources.Identifier;
import net.minecraft.resources.ResourceKey;

public final class TagKey<T> {
    public static <T> TagKey<T> create(ResourceKey<?> registry, Identifier id) { return null; }
    public Identifier location() { return null; }
    public ResourceKey<?> registry() { return null; }
    public boolean isFor(ResourceKey<?> registry) { return false; }
}
"#;

const BLOCK_ITEM_ID_STUB: &str = r#"package net.minecraft.references;

import net.minecraft.resources.Identifier;
import net.minecraft.resources.ResourceKey;

public final class BlockItemId {
    public static BlockItemId create(Identifier block, Identifier item) { return null; }
    public static BlockItemId create(String block, String item) { return null; }
    public static BlockItemId create(String name) { return null; }
    public ResourceKey<?> block() { return null; }
    public ResourceKey<?> item() { return null; }
}
"#;

const HOLDER_STUB: &str = r#"package net.minecraft.core;

public interface Holder<T> {
    java.util.Optional<net.minecraft.resources.ResourceKey<T>> unwrapKey();
}
"#;

const DEFERRED_REGISTER_STUB: &str = r#"package net.neoforged.neoforge.registries;

import java.util.function.Supplier;
import net.minecraft.resources.ResourceKey;

public class DeferredRegister<T> {
    public static <T> DeferredRegister<T> create(ResourceKey<?> registry, String namespace) { return null; }
    public <I extends T> DeferredHolder<T, I> register(String name, Supplier<? extends I> value) { return null; }
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
import net.minecraft.core.registries.Registries;
import net.minecraft.references.BlockItemId;
import net.minecraft.resources.Identifier;
import net.minecraft.resources.ResourceKey;
import net.minecraft.tags.TagKey;
import net.neoforged.neoforge.registries.DeferredHolder;
import net.neoforged.neoforge.registries.DeferredRegister;

public final class Probe {
    static Identifier id(String text) { return Identifier.parse(text); }

    /** Strings spelled like the old markers (and a handle's text form) stay Strings. */
    public static String spelled() {
        Object[] texts = {"identifier/a:x", "resource-key/registry/minecraft:item/ridentifier/a:x",
            "tag-key/registry/minecraft:item|a:t", "block-item-id/ridentifier/a:b,ridentifier/a:i",
            "hid:a:x", "hrk:minecraft:item|a:x", "a:x"};
        Identifier x = id("a:x");
        ResourceKey<?> key = ResourceKey.create(Registries.ITEM, x);
        TagKey<?> tag = TagKey.create(Registries.ITEM, id("a:t"));
        StringBuilder out = new StringBuilder();
        for (Object t : texts) {
            out.append(t instanceof Identifier).append(t instanceof ResourceKey).append(t instanceof TagKey)
                .append(t instanceof BlockItemId).append(t.equals(x)).append(x.equals(t)).append(key.equals(t))
                .append(tag.equals(t)).append(t).append(',');
        }
        return out.toString();
    }

    /** Strings spelled like a handle's text form are Strings: class, hash, map keys, concat. */
    public static String spelledStrings() {
        Object[] texts = {"hid:a:x", "hrk:minecraft:item|a:x", "htk:minecraft:item|a:t", "a:x"};
        Identifier x = id("a:x");
        StringBuilder out = new StringBuilder();
        for (Object t : texts) {
            out.append(t.getClass().getSimpleName()).append(t instanceof String).append(t.hashCode()).append(',');
        }
        Map<Object, String> m = new HashMap<>();
        m.put(x, "i");
        m.put("hid:a:x", "h");
        m.put("a:x", "t");
        return out.append(m.size()).append(m.get(id("a:x"))).append(m.get("hid:a:x")).append(m.get("a:x"))
            .append('|').append("" + "identifier/a:x" + x + "hid:a:x").toString();
    }

    /** The old markers themselves: equals, map keys, concat (their class, hash and instanceof wait for #156 P17). */
    public static String slashStrings() {
        Object[] texts = {"identifier/a:x", "resource-key/registry/minecraft:item/ridentifier/a:x",
            "tag-key/registry/minecraft:item|a:t", "block-item-id/ridentifier/a:b,ridentifier/a:i"};
        Identifier x = id("a:x");
        StringBuilder out = new StringBuilder();
        Map<Object, String> m = new HashMap<>();
        m.put(x, "i");
        for (Object t : texts) {
            out.append(t.equals(x)).append(x.equals(t)).append(',');
            m.put(t, "s");
        }
        return out.append(m.size()).append(m.get(id("a:x"))).append(m.get("identifier/a:x")).append('|')
            .append("" + texts[0] + x + texts[2]).toString();
    }

    /** Identifier: text, hash, equals, order, class. */
    public static String identifiers() {
        Identifier a = id("a:x");
        Identifier b = Identifier.fromNamespaceAndPath("a", "x");
        Identifier c = id("b:x");
        Object o = a;
        return a + "|" + a.hashCode() + "|" + a.equals(b) + a.equals(c) + a.equals("a:x") + o.equals(b) + "|"
            + a.compareTo(c) + c.compareTo(a) + "|" + (o instanceof Identifier) + (o instanceof Comparable)
            + "|" + o.getClass().getName() + "|" + String.valueOf(o) + Objects.hashCode(o) + Objects.equals(a, b)
            + "|" + List.of(a, c) + List.of(a).contains(b) + List.of(a, c).indexOf(c);
    }

    /** Identifiers as hash keys iterate in Java's order. */
    public static String hashed() {
        Map<Identifier, Integer> m = new HashMap<>();
        Set<Object> s = new HashSet<>();
        String[] ids = {"minecraft:stone", "demo:machine", "a:x", "z:q", "minecraft:silk_touch", "b:long/path/here"};
        for (int i = 0; i < ids.length; i++) {
            m.put(id(ids[i]), i);
            s.add(id(ids[i]));
        }
        return m + "|" + s + "|" + m.get(id("a:x")) + s.contains(Identifier.fromNamespaceAndPath("z", "q"))
            + m.containsKey(id("q:z")) + "|" + new TreeSet<>(m.keySet());
    }

    /** ResourceKey: text, parts, class. */
    public static String keys() {
        ResourceKey<?> k = ResourceKey.create(Registries.ITEM, id("a:x"));
        ResourceKey<?> dim = ResourceKey.create(Registries.DIMENSION, id("overworld"));
        Object o = k;
        return k + "|" + k.identifier() + "|" + k.registry() + "|" + k.isFor(Registries.ITEM) + k.isFor(Registries.BLOCK)
            + "|" + k.equals(ResourceKey.create(Registries.ITEM, id("a:x"))) + k.equals(ResourceKey.create(Registries.BLOCK, id("a:x")))
            + k.equals(id("a:x")) + "|" + o.getClass().getName() + (o instanceof ResourceKey) + "|" + dim + "|"
            + List.of(k, dim) + List.of(k).contains(ResourceKey.create(Registries.ITEM, id("a:x")))
            + "|" + (((ResourceKey<?>) (Object) Registries.BLOCK) != null);
    }

    /** TagKey: text, parts, equals, class. */
    public static String tags() {
        TagKey<?> t = TagKey.create(Registries.ITEM, id("a:t"));
        Object o = t;
        return t + "|" + t.location() + "|" + t.isFor(Registries.ITEM) + t.isFor(Registries.BLOCK) + "|"
            + t.equals(TagKey.create(Registries.ITEM, id("a:t"))) + t.equals(TagKey.create(Registries.BLOCK, id("a:t")))
            + t.equals(TagKey.create(Registries.ITEM, id("a:u"))) + t.equals("a:t") + "|" + o.getClass().getName()
            + (o instanceof Record) + "|" + List.of(t).contains(TagKey.create(Registries.ITEM, id("a:t")));
    }

    /** BlockItemId: text, keys, equals, class. */
    public static String blockItems() {
        BlockItemId named = BlockItemId.create("lamp");
        BlockItemId pair = BlockItemId.create(id("a:b"), id("a:i"));
        BlockItemId strings = BlockItemId.create("b", "i");
        Object o = pair;
        return named + "|" + pair.block() + "|" + pair.item() + "|" + pair.equals(BlockItemId.create(id("a:b"), id("a:i")))
            + pair.equals(named) + pair.equals(strings) + "|" + (pair.block() == ResourceKey.create(Registries.BLOCK, id("a:b")))
            + "|" + o.getClass().getName() + (o instanceof Record) + "|" + strings;
    }

    static String kind(Object o) {
        return switch (o) {
            case Identifier i when i.getNamespace().equals("minecraft") -> "mc:" + i.getPath();
            case Identifier i -> "id:" + i;
            case ResourceKey<?> k -> "key:" + k.identifier();
            case TagKey<?> t -> "tag:" + t.location();
            case BlockItemId b -> "bii:" + b.item().identifier();
            case String s -> "str:" + s;
            default -> "other";
        };
    }

    /** Type switches, patterns, Class.isInstance/isAssignableFrom/cast on the handles. */
    public static String types() {
        Object[] values = {id("minecraft:stone"), id("a:x"), ResourceKey.create(Registries.ITEM, id("a:k")),
            TagKey.create(Registries.ITEM, id("a:t")), BlockItemId.create("lamp"), "a:x"};
        StringBuilder out = new StringBuilder();
        for (Object o : values) {
            out.append(kind(o)).append(Identifier.class.isInstance(o)).append(ResourceKey.class.isInstance(o))
                .append(Comparable.class.isInstance(o)).append(Record.class.isInstance(o))
                .append(o instanceof Identifier i && i.getPath().startsWith("s")).append(',');
        }
        Object o = id("a:x");
        return out.append(Comparable.class.isAssignableFrom(o.getClass()))
            .append(Identifier.class.isAssignableFrom(o.getClass())).append(Record.class.isAssignableFrom(o.getClass()))
            .append(Identifier.class.cast(o)).append(o.getClass() == Identifier.class).toString();
    }

    /** Identifiers in natural order: path first, then namespace (Identifier.compareTo). */
    public static String sorts() {
        List<Identifier> ids = new ArrayList<>();
        for (String text : new String[] {"b:a", "a:b", "minecraft:stone", "a:a", "z:stone", "c:b/c", "c:b"}) {
            ids.add(id(text));
        }
        List<Identifier> sorted = new ArrayList<>(ids);
        Collections.sort(sorted);
        List<Identifier> listSort = new ArrayList<>(ids);
        listSort.sort(null);
        TreeMap<Identifier, Integer> tree = new TreeMap<>();
        for (int i = 0; i < ids.size(); i++) {
            tree.put(ids.get(i), i);
        }
        TreeSet<Object> set = new TreeSet<>(ids);
        return sorted + "|" + listSort + "|" + ids.stream().sorted().toList() + "|" + tree + "|" + tree.firstKey()
            + tree.lastKey() + tree.get(id("a:a")) + tree.containsKey(id("c:b")) + "|" + set + set.contains(id("z:stone"))
            + "|" + ids.stream().sorted(Comparator.reverseOrder()).toList()
            + "|" + ids.stream().max(Comparator.naturalOrder()).get() + ids.stream().min(Comparator.naturalOrder()).get();
    }

    /** ResourceKey is not Comparable: sorting two throws ClassCastException. */
    public static String keySort() {
        List<Object> keys = new ArrayList<>(List.of(ResourceKey.create(Registries.ITEM, id("a:b")),
            ResourceKey.create(Registries.ITEM, id("a:a"))));
        try {
            keys.sort(null);
            return "sorted " + keys;
        } catch (ClassCastException e) {
            return "CCE";
        }
    }

    /** Handles held by host collections: found, removed and printed by their own equals/toString. */
    public static String collections() {
        List<Object> list = new ArrayList<>(List.of(id("a:x"), ResourceKey.create(Registries.ITEM, id("a:k")),
            TagKey.create(Registries.ITEM, id("a:t")), BlockItemId.create("lamp"), id("b:y")));
        Set<Identifier> set = new HashSet<>(List.of(id("a:x"), id("b:y")));
        Map<Identifier, String> map = new HashMap<>(Map.of(id("a:x"), "one"));
        Map<Object, Object> linked = new LinkedHashMap<>();
        linked.put(id("a:x"), ResourceKey.create(Registries.BLOCK, id("a:k")));
        linked.put(id("b:k"), TagKey.create(Registries.ITEM, id("a:t")));
        return list.indexOf(id("b:y")) + "," + list.contains(ResourceKey.create(Registries.ITEM, id("a:k")))
            + list.contains(TagKey.create(Registries.ITEM, id("a:t"))) + list.contains(BlockItemId.create("lamp"))
            + list.remove(id("a:x")) + list.size() + "|" + set.contains(id("b:y")) + set.remove(id("a:x")) + set
            + "|" + map.get(id("a:x")) + map.containsKey(Identifier.fromNamespaceAndPath("a", "x")) + map
            + "|" + Set.of(id("a:x"), id("b:y")).contains(id("a:x")) + Map.of(id("q:r"), 5).get(id("q:r"))
            + List.copyOf(list).equals(list) + List.of(id("a:x"), id("a:x")).lastIndexOf(id("a:x"))
            + "|" + linked + linked.get(id("b:k")) + linked.containsValue(ResourceKey.create(Registries.BLOCK, id("a:k"))) + "|" + Map.entry(id("a:x"), 3)
            + "|" + List.of(id("a:x"), id("b:y")).hashCode() + "," + Set.of(id("a:x")).hashCode() + "," + Arrays.asList(id("a:x")).toString();
    }

    static String thrown(java.util.function.Supplier<Object> body) {
        try {
            return String.valueOf(body.get());
        } catch (RuntimeException e) {
            return e.getClass().getSimpleName() + ":" + e.getMessage();
        }
    }

    @SuppressWarnings({"rawtypes", "unchecked"})
    static DeferredHolder holder(String namespace, String name) {
        DeferredRegister register = DeferredRegister.create(Registries.ITEM, namespace);
        return register.register(name, () -> "value");
    }

    @SuppressWarnings({"rawtypes", "unchecked"})
    static Object key(DeferredHolder holder) {
        return ((net.minecraft.core.Holder) holder).unwrapKey().get();
    }

    /** NeoForge holders: register builds the id with Identifier.fromNamespaceAndPath, so bad text throws. */
    @SuppressWarnings("rawtypes")
    public static String holders() {
        DeferredHolder a = holder("demo", "thing");
        DeferredHolder b = holder("demo", "thing");
        DeferredHolder c = holder("demo", "other/path_1.x-y");
        return a.getId() + "|" + key(a) + "|" + a.getId().equals(b.getId()) + (key(a) == key(b))
            + a.getId().equals(c.getId()) + (key(a) == key(c)) + "|" + c.getId() + "|" + c.getId().getPath()
            + "|" + thrown(() -> holder("x:y", "z").getId()) + "|" + thrown(() -> holder("x", "y:z").getId())
            + "|" + thrown(() -> holder("Bad Mod", "ok")) + "|" + thrown(() -> holder("ok", "UPPER|case"))
            + "|" + thrown(() -> holder("x/y", "z")) + "|" + thrown(() -> holder("x", "y,z#w?"));
    }

    /** Names carrying every separator never make an id, so never alias a real holder. */
    public static String holderSeparators() {
        StringBuilder out = new StringBuilder();
        String[] parts = {":", "/", "|", ",", "#", "?", "\\", " "};
        for (String part : parts) {
            out.append(thrown(() -> holder("a" + part + "b", "c").getId()).startsWith("IdentifierException"))
                .append(thrown(() -> holder("a", "b" + part + "c").getId())).append(',');
        }
        return out.toString();
    }

    /** A TagKey's registry is compared by identity: a key in another registry that shares the id is another tag. */
    @SuppressWarnings({"rawtypes", "unchecked"})
    public static String tagRegistries() {
        ResourceKey notRoot = ResourceKey.create(Registries.ITEM, id("minecraft:block"));
        TagKey<?> odd = TagKey.create(notRoot, id("a:t"));
        TagKey<?> real = TagKey.create(Registries.BLOCK, id("a:t"));
        return "" + (odd == real) + odd.equals(real) + real.equals(odd) + "|" + odd.isFor(Registries.BLOCK) + odd.isFor(notRoot)
            + real.isFor(Registries.BLOCK) + "|" + (odd.registry() == notRoot) + odd.registry() + "|" + odd + "|" + odd.location()
            + "|" + (odd == TagKey.create((ResourceKey) ResourceKey.create(Registries.ITEM, id("minecraft:block")), id("a:t")))
            + odd.equals(TagKey.create(notRoot, id("a:t"))) + List.of(odd).contains(real);
    }

    /** IdentifierException is a RuntimeException: every catch that covers one catches it. */
    public static String exceptions() {
        StringBuilder out = new StringBuilder();
        try {
            Identifier.parse("A:b");
        } catch (Exception e) {
            out.append("E:").append(e.getClass().getName()).append(':').append(e.getMessage()).append(e instanceof RuntimeException);
        }
        try {
            BlockItemId.create("a:b");
        } catch (Throwable t) {
            out.append("|T:").append(t.getClass().getSimpleName());
        }
        try {
            Identifier.fromNamespaceAndPath("a", "B");
        } catch (RuntimeException e) {
            out.append("|R:").append(e.getMessage());
        }
        // The message is `StringEscapeUtils.escapeJava`'d.
        return out.append("|").append(thrown(() -> Identifier.fromNamespaceAndPath("a", "q\"\\\t\u00e9\ud83d\ude00\u007f\u0001/")))
            .append("|").append(thrown(() -> Identifier.parse("\u00c9:x"))).toString();
    }

    /** null arguments throw where the real factories dereference them. */
    public static String nulls() {
        return thrown(() -> Identifier.parse(null)) + "|" + thrown(() -> Identifier.tryParse(null)) + "|"
            + thrown(() -> Identifier.fromNamespaceAndPath(null, "x")) + "|" + thrown(() -> Identifier.fromNamespaceAndPath("a", null))
            + "|" + thrown(() -> Identifier.withDefaultNamespace(null)) + "|" + thrown(() -> Identifier.tryBuild(null, "x"));
    }

    /** Interned kinds are one object per value; the same handle is one object. */
    public static String identity() {
        Identifier a = id("a:x");
        Identifier same = a;
        ResourceKey<?> k1 = ResourceKey.create(Registries.ITEM, id("a:x"));
        ResourceKey<?> k2 = ResourceKey.create(Registries.ITEM, id("a:x"));
        TagKey<?> t1 = TagKey.create(Registries.ITEM, id("a:t"));
        TagKey<?> t2 = TagKey.create(Registries.ITEM, id("a:t"));
        Map<Object, Integer> im = new IdentityHashMap<>();
        im.put(k1, 1);
        im.put(k2, 2);
        im.put(t1, 3);
        im.put(t2, 4);
        im.put(a, 5);
        im.put(same, 6);
        return "" + (a == same) + (k1 == k2) + (t1 == t2)
            + (k1 != ResourceKey.create(Registries.BLOCK, id("a:x"))) + (a != id("b:x")) + "|" + im.size() + im.get(k2) + im.get(a);
    }

    /** Two parses of one id: equal, but two objects (Identifier is not interned). */
    public static String identifierIdentity() {
        return "" + (id("a:x") == id("a:x"));
    }

    public static String identifierIdentityMap() {
        Map<Object, Integer> im = new IdentityHashMap<>();
        im.put(id("a:x"), 1);
        im.put(id("a:x"), 2);
        return "" + im.size();
    }

    /** Two equal BlockItemIds: a record, not interned. */
    public static String blockItemIdentity() {
        return "" + (BlockItemId.create("lamp") == BlockItemId.create("lamp"));
    }

    /** ResourceKey and TagKey hash by identity (no hashCode of their own over an identity hash). */
    public static String keyHash() {
        return "" + ResourceKey.create(Registries.ITEM, id("a:x")).hashCode();
    }

    public static String tagHash() {
        return "" + TagKey.create(Registries.ITEM, id("a:t")).hashCode();
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
            ("net/minecraft/tags/TagKey.java", TAG_KEY_STUB),
            (
                "net/minecraft/references/BlockItemId.java",
                BLOCK_ITEM_ID_STUB,
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

/// The ids answer as 26.3 does on Java 25, with nothing counted: text,
/// `hashCode`, `equals`, order, class; as hash keys (Java's iteration order),
/// in natural order (`Collections.sort`, `List.sort(null)`, `sorted()`,
/// `TreeMap`/`TreeSet`), inside host collections, through type switches,
/// patterns and `Class.isInstance`/`isAssignableFrom`; identity of the
/// interned keys; strings spelled like their handles' text are Strings;
/// NeoForge holders whose namespace or name is no id throw `IdentifierException`
/// (so names carrying separators never alias another holder); a `TagKey`
/// compares its registry key's whole identity; `IdentifierException` is
/// caught as a `RuntimeException`; and `null` throws Java 25's NPE.
#[test]
fn the_ids_answer_as_java_25_and_no_string_spells_one() {
    let fixture = fixture("host-ids");
    for method in [
        "spelledStrings",
        "slashStrings",
        "identifiers",
        "hashed",
        "keys",
        "tags",
        "blockItems",
        "types",
        "sorts",
        "collections",
        "holders",
        "holderSeparators",
        "tagRegistries",
        "exceptions",
        "nulls",
        "identity",
    ] {
        let (text, unknown) = run(&fixture, method);
        assert_eq!(text, expected(method), "{method}: {unknown:?}");
        assert_eq!(unknown, Vec::<(String, usize)>::new(), "{method}");
    }
}

/// A string spelled like an old marker is never the id: no `equals`, no
/// `instanceof` says so. `"identifier/a:x" instanceof Identifier` is still
/// counted rather than answered, because `word/rest` text is classified by
/// `is_plain_string` until #156 P17; nothing else is counted.
#[test]
fn a_string_spelled_like_an_old_marker_is_not_an_id() {
    let fixture = fixture("host-ids-spelled");
    let (text, unknown) = run(&fixture, "spelled");
    assert_eq!(text, expected("spelled"));
    assert!(
        unknown
            .iter()
            .all(|(call, _)| call.ends_with("#<instanceof>(Ljava/lang/Object;)Z")),
        "{unknown:?}"
    );
}

/// Two `Identifier.parse("a:x")` are equal but two objects (Java 25: `==` is
/// `false`, an `IdentityHashMap` holds both), and two equal `BlockItemId`s
/// are two records. The kinds are not canonical, so identity on two
/// separately made handles is counted, never answered from their equal keys.
#[test]
fn identity_of_two_equal_identifiers_is_counted() {
    let fixture = fixture("host-ids-identity");
    for (method, call) in [
        ("identifierIdentity", "java/lang/Object#if_acmp("),
        ("blockItemIdentity", "java/lang/Object#if_acmp("),
        ("identifierIdentityMap", "java/util/Map#put("),
    ] {
        let (_, unknown) = run(&fixture, method);
        assert!(
            unknown.iter().any(|(counted, _)| counted.starts_with(call)),
            "{method}: {unknown:?}"
        );
    }
}

/// What only a JVM knows is counted: `ResourceKey.hashCode` is
/// `Object.hashCode` and `TagKey`'s record hash mixes in its registry key's
/// (identity hashes, a different number per JVM), and `ResourceKey` is not
/// `Comparable` (Java 25 throws `ClassCastException` sorting two).
#[test]
fn identity_hashes_and_a_key_sort_are_counted() {
    let fixture = fixture("host-ids-counted");
    for (method, call) in [
        ("keyHash", "java/lang/Object#hashCode()I"),
        ("tagHash", "java/lang/Object#hashCode()I"),
        (
            "keySort",
            "java/lang/Comparable#compareTo(Ljava/lang/Object;)I",
        ),
    ] {
        let (_, unknown) = run(&fixture, method);
        assert!(
            unknown.iter().any(|(counted, _)| counted == call),
            "{method}: {unknown:?}"
        );
    }
}

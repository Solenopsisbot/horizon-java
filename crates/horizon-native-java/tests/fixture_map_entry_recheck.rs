//! The re-check of #221 (#156 P14e, review 5905494234): a library map's stored entry is one
//! object across readings (B4), two equal `entrySet()` views are equal inside other collections
//! (B5), an `IdentityHashMap` entry compares another entry's key and value by `==` (B1), and
//! `remove(key)` after a walk detaches only that key's node (B3).
//!
//! The stubs exist only for `javac`: the host models Guava's `ImmutableMap` and Gson's
//! `JsonObject` itself.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::StdHost;

/// OpenJDK 25.0.5 running this `Probe` (its `main`, left out here, prints each method), compiled
/// against the real guava-33.6.0-jre and gson-2.14.0 jars of the 26.3 libraries; the same on two
/// runs; pasted verbatim.
const EXPECTED: &str = "\
storedEntries=truetrue1|truetrue2|true2\n\
wrappedEntries=falsetrue\n\
viewsInCollections=true111true1true|java.lang.IllegalArgumentException:duplicate element: [a=1, b=2]|false\n\
identityEntries=truefalsetruefalsetruefalsetruefalsetruefalse\n\
removeAfterWalk=[a=7, b=2, c=3]|falsetrue|2b=8{a=7, b=5, c=3}\n\
";

const IMMUTABLE_MAP_STUB: &str = "package com.google.common.collect;\n\
public abstract class ImmutableMap<K, V> implements java.util.Map<K, V> {\n\
public static <K, V> ImmutableMap<K, V> of(K k, V v) { throw new AssertionError(); }\n\
public static <K, V> ImmutableMap<K, V> of(K k, V v, K k2, V v2) { throw new AssertionError(); }\n}\n";

const JSON_ELEMENT_STUB: &str = "package com.google.gson;\npublic abstract class JsonElement {}\n";

const JSON_OBJECT_STUB: &str = "package com.google.gson;\n\
public final class JsonObject extends JsonElement {\n\
public void addProperty(String p, Number v) { throw new AssertionError(); }\n\
public java.util.Set<java.util.Map.Entry<String, JsonElement>> entrySet() { throw new AssertionError(); }\n}\n";

const PROBE: &str = r##"package demo;

import com.google.common.collect.ImmutableMap;
import com.google.gson.JsonObject;
import java.util.*;
import java.util.function.Supplier;

@SuppressWarnings({"rawtypes", "unchecked"})
public final class Probe {
    record P(int x) {}

    static final class JE implements Map.Entry<Object, Object> {
        final Object k, v;
        JE(Object k, Object v) { this.k = k; this.v = v; }
        public Object getKey() { return k; }
        public Object getValue() { return v; }
        public Object setValue(Object x) { throw new UnsupportedOperationException(); }
        public boolean equals(Object o) { return o instanceof Map.Entry<?, ?> e && Objects.equals(k, e.getKey()) && Objects.equals(v, e.getValue()); }
        public int hashCode() { return Objects.hashCode(k) ^ Objects.hashCode(v); }
    }

    static String t(Supplier<Object> s) {
        try { return String.valueOf(s.get()); }
        catch (RuntimeException e) { return e.getClass().getName() + ":" + e.getMessage(); }
    }
    static Map<String, Integer> fill(Map<String, Integer> m) { m.put("a", 1); m.put("b", 2); return m; }
    static Map.Entry first(Map m) { return (Map.Entry) m.entrySet().iterator().next(); }
    static int distinct(Iterable<?> a, Iterable<?> b) {
        Map<Object, Boolean> ids = new IdentityHashMap<>();
        for (Object e : a) ids.put(e, true);
        for (Object e : b) ids.put(e, true);
        return ids.size();
    }

    // B4: a stored library entry is one object across readings.
    public static String storedEntries() {
        Map<String, Integer> sg = Collections.singletonMap("a", 1);
        Object x = null, y = null;
        for (var e : sg.entrySet()) x = e;
        for (var e : sg.entrySet()) y = e;
        Map<String, Integer> g1 = ImmutableMap.of("a", 1);
        Map<String, Integer> g2 = ImmutableMap.of("a", 1, "b", 2);
        JsonObject o = new JsonObject();
        o.addProperty("a", 1);
        o.addProperty("b", 2);
        return "" + (first(sg) == first(sg)) + (x == y) + distinct(sg.entrySet(), sg.entrySet())
            + "|" + (first(g1) == first(g1)) + (first(g2) == first(g2)) + distinct(g2.entrySet(), g2.entrySet())
            + "|" + (o.entrySet().iterator().next() == o.entrySet().iterator().next()) + distinct(o.entrySet(), o.entrySet());
    }

    // B4: a map whose readings this model does not know (Java 25's `Properties` hands out a
    // new `ConcurrentHashMap.MapEntry` per `next()`; `false`): counted.
    public static String unknownReadings() {
        Properties p = new Properties();
        p.put("a", "1");
        return "" + (first(p) == first(p));
    }

    // B4: `unmodifiableMap` wraps a new `UnmodifiableEntry` per `next()`.
    public static String wrappedEntries() {
        Map<String, Integer> u = Collections.unmodifiableMap(fill(new HashMap<>()));
        return "" + (first(u) == first(u)) + first(u).equals(first(u));
    }

    // B5: two equal `entrySet()` views inside collections.
    public static String viewsInCollections() {
        Map<String, Integer> m = fill(new HashMap<>()), n = fill(new HashMap<>());
        Set<Object> holding = new HashSet<>();
        holding.add(m.entrySet());
        Map<Object, Integer> keyed = new HashMap<>();
        keyed.put(m.entrySet(), 1);
        return "" + List.of(m.entrySet()).contains(n.entrySet())
            + new HashSet<Object>(List.of(m.entrySet(), n.entrySet())).size()
            + new HashSet<Object>(List.of(fill(new LinkedHashMap<>()).entrySet(), fill(new LinkedHashMap<>()).entrySet())).size()
            + new HashSet<Object>(List.of(fill(new TreeMap<>()).entrySet(), fill(new TreeMap<>()).entrySet())).size()
            + holding.contains(n.entrySet()) + keyed.get(n.entrySet()) + keyed.containsKey(n.entrySet())
            + "|" + t(() -> Set.of(m.entrySet(), n.entrySet()))
            + "|" + List.of(m.entrySet()).contains(fill(new HashMap<>()).keySet());
    }

    // B1: an `IdentityHashMap` entry compares the other entry's key and value by `==`.
    public static String identityEntries() {
        P p = new P(1), q = new P(2);
        IdentityHashMap<Object, Object> m = new IdentityHashMap<>();
        m.put(p, q);
        var e = first(m);
        IdentityHashMap<Object, Object> nk = new IdentityHashMap<>();
        nk.put(null, q);
        IdentityHashMap<Object, Object> nv = new IdentityHashMap<>();
        nv.put(p, null);
        return "" + e.equals(new AbstractMap.SimpleEntry<>(p, q)) + e.equals(new AbstractMap.SimpleEntry<>(p, new P(2)))
            + e.equals(new AbstractMap.SimpleImmutableEntry<>(p, q)) + e.equals(new AbstractMap.SimpleImmutableEntry<>(new P(1), q))
            + e.equals(new JE(p, q)) + e.equals(new JE(new P(1), q))
            + first(nk).equals(new AbstractMap.SimpleEntry<>(null, q)) + first(nk).equals(new AbstractMap.SimpleEntry<>(null, new P(2)))
            + first(nv).equals(new AbstractMap.SimpleEntry<>(p, null)) + first(nv).equals(new AbstractMap.SimpleEntry<>(new P(1), null));
    }

    // B3: `remove(key)` after a walk detaches only that key's node.
    public static String removeAfterWalk() {
        Map<String, Integer> m = fill(new HashMap<>());
        m.put("c", 3);
        List<Map.Entry<String, Integer>> held = new ArrayList<>(m.entrySet());
        m.remove("b");
        m.remove("zz");
        m.remove("c", 9);
        m.put("a", 7);
        m.put("b", 5);
        Map.Entry<String, Integer> b2 = null;
        for (var e : m.entrySet()) if (e.getKey().equals("b")) b2 = e;
        return held + "|" + (held.get(1) == b2) + (held.get(0) == first(m)) + "|" + held.get(1).setValue(8) + held.get(1) + m;
    }
}
"##;

fn fixture() -> javac_fixture::Fixture {
    javac_fixture::Fixture::build(
        "map-entry-recheck",
        &[
            (
                "com/google/common/collect/ImmutableMap.java",
                IMMUTABLE_MAP_STUB,
            ),
            ("com/google/gson/JsonElement.java", JSON_ELEMENT_STUB),
            ("com/google/gson/JsonObject.java", JSON_OBJECT_STUB),
            ("demo/Probe.java", PROBE),
        ],
    )
}

fn expected(method: &str) -> &'static str {
    EXPECTED
        .lines()
        .find_map(|line| line.strip_prefix(method)?.strip_prefix('='))
        .unwrap_or_else(|| panic!("no expected line for {method}"))
}

fn run(fixture: &javac_fixture::Fixture, method: &str) -> (String, Vec<(String, usize)>) {
    let mut host = fixture.jar.seed_host(StdHost::new());
    let text = fixture.text_on(&mut host, method);
    (text, host.unknown_host_calls())
}

/// Every case answers as Java 25 does, with nothing counted.
#[test]
fn entries_and_entry_views_answer_as_java_25() {
    let fixture = fixture();
    for method in [
        "storedEntries",
        "wrappedEntries",
        "viewsInCollections",
        "identityEntries",
        "removeAfterWalk",
    ] {
        let (text, unknown) = run(&fixture, method);
        assert_eq!(text, expected(method), "{method}: {unknown:?}");
        assert!(unknown.is_empty(), "{method}: {unknown:?}");
    }
}

/// Two readings of one mapping of a map whose entry class this model does not know are one key
/// with two origins: `==` between them is counted, never answered (#221 review B4).
#[test]
fn readings_of_an_unknown_maps_entry_are_counted() {
    let fixture = fixture();
    let (_, unknown) = run(&fixture, "unknownReadings");
    assert!(
        unknown.iter().any(|(call, _)| call.contains("if_acmp")),
        "{unknown:?}"
    );
}

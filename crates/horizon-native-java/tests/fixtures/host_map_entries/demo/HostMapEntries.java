package demo;

import java.util.*;
import java.util.function.Supplier;

/** The host's map entries (#156 P14e): each class's own `Map.Entry` rules, as Java 25 runs them. */
public final class HostMapEntries {
  static String attempt(Supplier<Object> s) {
    try {
      return String.valueOf(s.get());
    } catch (RuntimeException e) {
      return e.getClass().getName() + ": " + e.getMessage();
    }
  }

  static int identities(Object... objects) {
    Map<Object, Boolean> seen = new IdentityHashMap<>();
    for (Object o : objects) seen.put(o, true);
    return seen.size();
  }

  static Map<String, Integer> filled(Map<String, Integer> m) {
    m.put("a", 1);
    m.put("b", 2);
    return m;
  }

  /** A String spelled like the old marker is a String. */
  public static String spelled() {
    Object s = "map-entry/ra,rb";
    Map<Object, Integer> m = new HashMap<>();
    m.put(s, 1);
    return s.getClass().getName() + (s instanceof Map.Entry) + s.equals(Map.entry("a", "b"))
        + Map.entry("a", "b").equals(s) + s.hashCode() + "|" + s + "|" + m.get("map-entry/ra,rb")
        + ((String) s).length() + "|" + new HashSet<>(List.of(s, Map.entry("a", "b"))).size();
  }

  /** A String spelled like an entry handle's key is a String. */
  public static String spelledKey() {
    Object s = "java/util/KeyValueHolder#entry1";
    Object t = "java/util/HashMap$Node#node2";
    return s.getClass().getName() + (s instanceof Map.Entry) + s.equals(Map.entry("a", "b")) + s.hashCode() + "|"
        + t.getClass().getName() + (t instanceof Map.Entry) + t + "|" + (s + "" + t).length();
  }

  /** `Map.entry`: an immutable `KeyValueHolder`. */
  public static String holder() {
    Map.Entry<String, String> e = Map.entry("a", "b");
    return e + "|" + e.getClass().getName() + "|" + e.hashCode() + "," + ("a".hashCode() ^ "b".hashCode()) + "|"
        + e.equals(new AbstractMap.SimpleEntry<>("a", "b")) + new AbstractMap.SimpleEntry<>("a", "b").equals(e)
        + e.equals(Map.entry("a", "c")) + e.equals("a=b") + "|" + attempt(() -> e.setValue("c")) + "|"
        + attempt(() -> Map.entry(null, "b")) + "|" + attempt(() -> Map.entry("a", null)) + "|"
        + (e instanceof Map.Entry) + e.getKey() + e.getValue();
  }

  /** Two `Map.entry` calls are two objects. */
  public static String holderIdentity() {
    Map.Entry<String, String> a = Map.entry("a", "b");
    Map.Entry<String, String> b = Map.entry("a", "b");
    return "" + (a == b) + (a == a) + identities(a, b, a) + a.equals(b) + new HashSet<>(List.of(a)).contains(b)
        + "|" + Map.Entry.copyOf(a).getClass().getName() + (Map.Entry.copyOf(a) == a);
  }

  /** A `HashMap` node: write-through `setValue`, `k=v`, `hash(k) ^ hash(v)`. */
  public static String hashNode() {
    Map<String, Integer> m = filled(new HashMap<>());
    Map.Entry<String, Integer> e = m.entrySet().iterator().next();
    String cls = e.getClass().getName();
    Integer old = e.setValue(10);
    return cls + "|" + e + "|" + old + "|" + m + "|" + e.hashCode() + "|" + e.equals(Map.entry("a", 10))
        + Map.entry("a", 10).equals(e) + m.entrySet().contains(Map.entry("a", 10)) + "|"
        + Map.Entry.copyOf(e).getClass().getName();
  }

  /** The same node through two iterators is one object. */
  public static String nodeIdentity() {
    StringBuilder out = new StringBuilder();
    for (Map<String, Integer> m : List.<Map<String, Integer>>of(filled(new HashMap<>()), filled(new LinkedHashMap<>()), filled(new TreeMap<>()))) {
      Object a = m.entrySet().iterator().next();
      Object b = m.entrySet().iterator().next();
      out.append(a == b).append(identities(a, b)).append(',');
    }
    return out.toString();
  }

  /** A live node sees a later `put` of its key. */
  public static String livePut() {
    Map<String, Integer> m = filled(new HashMap<>());
    Map.Entry<String, Integer> e = m.entrySet().iterator().next();
    m.put("a", 5);
    m.put("c", 3);
    return e.getValue() + "|" + e + "|" + e.setValue(6) + "|" + m;
  }

  /** #212: a node whose key was removed keeps its value; `setValue` updates it, not the map. */
  public static String detached() {
    Map<String, Integer> m = filled(new HashMap<>());
    Map.Entry<String, Integer> e = m.entrySet().iterator().next();
    m.remove("a");
    Integer old = e.setValue(2);
    return old + "," + e.getValue() + "," + m + "," + e + "," + e.hashCode();
  }

  /** A removed key put again is a new node. */
  public static String reput() {
    Map<String, Integer> m = filled(new HashMap<>());
    Map.Entry<String, Integer> e = m.entrySet().iterator().next();
    m.remove("a");
    m.put("a", 7);
    Map.Entry<String, Integer> now = m.entrySet().iterator().next();
    return e.getValue() + "," + (e == now) + "," + now + "," + m + "," + e.equals(now);
  }

  /** `LinkedHashMap` nodes in insertion order, written through. */
  public static String linked() {
    Map<String, Integer> m = new LinkedHashMap<>();
    m.put("z", 1);
    m.put("y", 2);
    StringBuilder out = new StringBuilder();
    for (Map.Entry<String, Integer> e : m.entrySet()) {
      out.append(e.getClass().getName()).append(e).append(e.setValue(e.getValue() * 10)).append(';');
    }
    return out + "|" + m;
  }

  /** `TreeMap` nodes write through; `firstEntry()` is an immutable snapshot, new each call. */
  public static String tree() {
    TreeMap<String, Integer> m = new TreeMap<>(filled(new HashMap<>()));
    Map.Entry<String, Integer> node = m.entrySet().iterator().next();
    Map.Entry<String, Integer> first = m.firstEntry();
    String cls = node.getClass().getName() + "," + first.getClass().getName();
    node.setValue(9);
    return cls + "|" + m + "|" + first + "|" + attempt(() -> first.setValue(3)) + "|" + (m.firstEntry() == m.firstEntry())
        + m.firstEntry().equals(node) + node.equals(m.firstEntry()) + "|" + m.ceilingEntry("b") + m.higherEntry("b")
        + "|" + m.pollFirstEntry() + m;
  }

  /** `Map.Entry.comparingByKey/Value` sorting a list of entries. */
  public static String sorting() {
    Map<String, Integer> m = new HashMap<>();
    m.put("x", 3);
    m.put("y", 1);
    m.put("z", 2);
    List<Map.Entry<String, Integer>> byValue = new ArrayList<>(m.entrySet());
    byValue.sort(Map.Entry.comparingByValue());
    List<Map.Entry<String, Integer>> byKey = new ArrayList<>(m.entrySet());
    byKey.sort(Map.Entry.comparingByKey());
    return byValue + "|" + byKey + "|" + (byValue.get(0) == byKey.get(1));
  }

  /** Entries inside host collections. */
  public static String collections() {
    Map<String, Integer> m = filled(new HashMap<>());
    Map.Entry<String, Integer> e = m.entrySet().iterator().next();
    Set<Object> set = new HashSet<>(List.of(e, Map.entry("a", 1), Map.entry("b", 2)));
    Map<Object, String> keyed = new HashMap<>();
    keyed.put(Map.entry("a", 1), "x");
    return set.size() + "|" + List.of(e, Map.entry("q", 0)) + "|" + keyed.get(e) + "|" + Map.of(Map.entry("k", "v"), 1)
        + "|" + new ArrayList<>(m.entrySet()).indexOf(Map.entry("b", 2));
  }

  /** `null` keys and values. */
  public static String nulls() {
    Map<String, Integer> m = new HashMap<>();
    m.put(null, null);
    Map.Entry<String, Integer> e = m.entrySet().iterator().next();
    return e + "|" + e.hashCode() + "|" + e.equals(new AbstractMap.SimpleEntry<>(null, null)) + "|" + e.getKey() + e.getValue();
  }

  /** Pattern matching and `Class` checks. */
  public static String types() {
    Object o = Map.entry("a", "b");
    String kind = switch (o) {
      case Map.Entry<?, ?> en -> "entry:" + en.getKey();
      default -> "other";
    };
    Object n = filled(new HashMap<>()).entrySet().iterator().next();
    return kind + "|" + Map.Entry.class.isInstance(o) + Map.Entry.class.isInstance(n) + (n instanceof Comparable)
        + "|" + Map.Entry.class.isAssignableFrom(o.getClass());
  }

  /** `TreeMap.deleteEntry` of a node with two children copies its successor into it. */
  public static String treeDelete() {
    TreeMap<String, Integer> m = new TreeMap<>();
    for (String k : List.of("d", "b", "f", "a", "c", "e", "g")) m.put(k, k.charAt(0) - 'a');
    List<Map.Entry<String, Integer>> es = new ArrayList<>(m.entrySet());
    m.remove("d");
    Map.Entry<String, Integer> nowE = null;
    for (Map.Entry<String, Integer> e : m.entrySet()) {
      if (e.getKey().equals("e")) nowE = e;
    }
    return es + "|" + m + "|" + es.get(4).setValue(40) + m.get("e") + "|" + (nowE == es.get(3))
        + (nowE == es.get(4)) + "|" + es.get(3).setValue(30) + m;
  }

  /** A key removed through `keySet()`: the node is detached. */
  public static String viewRemove() {
    Map<String, Integer> m = filled(new HashMap<>());
    Map.Entry<String, Integer> e = m.entrySet().iterator().next();
    m.keySet().remove("a");
    return e.getValue() + "," + e.setValue(5) + "," + m;
  }

  record Colliding(int v) {
    @Override
    public int hashCode() {
      return 0;
    }
  }

  /** A bin of colliding keys becomes a tree: its nodes are replaced. */
  public static String treeified() {
    Map<Colliding, Integer> m = new HashMap<>();
    for (int i = 0; i < 70; i++) m.put(new Colliding(i), i);
    Map.Entry<Colliding, Integer> e = m.entrySet().iterator().next();
    m.put(new Colliding(0), 100);
    return e.getKey().v() + "," + e.getValue();
  }

  enum Side {
    LEFT,
    RIGHT
  }

  static <K> String entryRules(Map<K, Integer> m, K first, K other) {
    Map.Entry<K, Integer> e = m.entrySet().iterator().next();
    Map.Entry<K, Integer> again = m.entrySet().iterator().next();
    String cls = e.getClass().getName();
    m.put(first, 5);
    String live = "" + e.getValue();
    Integer old = e.setValue(7);
    return cls + "," + (e == again) + e.equals(again) + "," + live + "," + old + "," + m.get(first) + "," + e + ","
        + attempt(() -> e.setValue(null)) + "," + m.get(first);
  }

  /** `EnumMap`: a new entry per `next()`, reading and writing the map. */
  public static String enumMapEntries() {
    Map<Side, Integer> m = new EnumMap<>(Side.class);
    m.put(Side.LEFT, 1);
    m.put(Side.RIGHT, 2);
    return entryRules(m, Side.LEFT, Side.RIGHT);
  }

  /** `ConcurrentHashMap`: a new `MapEntry` per `next()`, its value a snapshot, `setValue` a `put`. */
  public static String concurrentEntries() {
    Map<String, Integer> m = new java.util.concurrent.ConcurrentHashMap<>();
    m.put("a", 1);
    return entryRules(m, "a", "b");
  }

  /** `Hashtable`: its nodes, no `null`. */
  public static String hashtableEntries() {
    Map<String, Integer> m = new Hashtable<>();
    m.put("a", 1);
    return entryRules(m, "a", "b");
  }

  /** `WeakHashMap`: its nodes. */
  public static String weakEntries() {
    Map<String, Integer> m = new WeakHashMap<>();
    m.put("a", 1);
    return entryRules(m, "a", "b");
  }

  /** `IdentityHashMap`: a new entry per `next()`, reading and writing the map. */
  public static String identityEntries() {
    Map<String, Integer> m = new IdentityHashMap<>();
    m.put("a", 1);
    return entryRules(m, "a", "b");
  }

  /** `ConcurrentSkipListMap`: immutable snapshots. */
  public static String skipListEntries() {
    Map<String, Integer> m = new java.util.concurrent.ConcurrentSkipListMap<>();
    m.put("a", 1);
    Map.Entry<String, Integer> e = m.entrySet().iterator().next();
    m.put("a", 5);
    return e.getClass().getName() + "," + e + "," + attempt(() -> e.setValue(7)) + "," + (e == m.entrySet().iterator().next());
  }
}

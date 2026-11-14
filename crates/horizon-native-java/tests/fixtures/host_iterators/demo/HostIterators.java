package demo;

import java.util.*;
import java.util.function.Supplier;
import java.util.stream.Collector;
import java.util.stream.Collectors;
import java.util.stream.Stream;

/** Iterators, enumerations, list iterators and collectors (#156 P14f), as Java 25 runs them. */
public final class HostIterators {
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

  /** Strings spelled like the old markers are Strings. */
  public static String spelled() {
    StringBuilder out = new StringBuilder();
    for (Object s : new Object[] {"iterator/0", "list-iterator/1", "resource-enumeration/0", "collector/list", "collector/joining/,"}) {
      Map<Object, Integer> m = new HashMap<>();
      m.put(s, 1);
      out.append(s.getClass().getName()).append(s instanceof Iterator).append(s instanceof Enumeration)
          .append(s instanceof Collector).append(s.equals(List.of(1).iterator())).append(s.hashCode())
          .append(m.get(s)).append(((String) s).length()).append(',');
    }
    return out.toString();
  }

  /** Two `iterator()` calls are two iterators with their own cursors. */
  public static String twoIterators() {
    List<Integer> l = new ArrayList<>(List.of(1, 2, 3));
    Iterator<Integer> a = l.iterator();
    Iterator<Integer> b = l.iterator();
    String same = "" + (a == b) + a.equals(b) + a.equals(a) + identities(a, b, a) + List.of(a).contains(b) + List.of(a).contains(a);
    a.next();
    a.next();
    return same + "|" + a.next() + b.next() + a.hasNext() + b.hasNext() + "|" + (a instanceof Iterator);
  }

  /** An iterator's class is its collection's (`ArrayList$Itr`, no `ListIterator`). */
  public static String iteratorTypes() {
    Object it = new ArrayList<>(List.of(1)).iterator();
    return "" + (it instanceof ListIterator) + it.getClass().getName();
  }

  /** Fail-fast: a structural change after `iterator()` throws at `next()`; `hasNext` reads the size. */
  public static String failFast() {
    List<Integer> l = new ArrayList<>(List.of(1, 2));
    Iterator<Integer> it = l.iterator();
    it.next();
    l.add(3);
    String first = it.hasNext() + attempt(it::next);
    Set<String> s = new HashSet<>(Set.of("x", "y"));
    Iterator<String> si = s.iterator();
    s.add("z");
    String second = attempt(si::next);
    List<Integer> m = new ArrayList<>(List.of(1, 2, 3));
    Iterator<Integer> mi = m.iterator();
    mi.next();
    mi.remove();
    return first + "|" + second + "|" + mi.next() + m + attempt(() -> {
      mi.remove();
      mi.remove();
      return "no";
    });
  }

  /** `Collections.emptyIterator()`/`emptyEnumeration()` are shared singletons. */
  public static String empties() {
    Iterator<Object> a = Collections.emptyIterator();
    Iterator<Object> b = Collections.emptyIterator();
    Enumeration<Object> e = Collections.emptyEnumeration();
    return "" + (a == b) + a.hasNext() + attempt(a::next) + "|" + (e == Collections.emptyEnumeration()) + e.hasMoreElements()
        + attempt(e::nextElement) + "|" + identities(a, b, e);
  }

  /** `Collections.enumeration(c)` walks `c.iterator()`, so it fails fast too. */
  public static String enumerations() {
    List<String> l = new ArrayList<>(List.of("a", "b"));
    Enumeration<String> e = Collections.enumeration(l);
    Enumeration<String> f = Collections.enumeration(l);
    String first = "" + (e == f) + e.hasMoreElements() + e.nextElement() + f.nextElement();
    l.add("c");
    return first + "|" + attempt(e::nextElement) + "|" + identities(e, f, e);
  }

  /** `Vector.elements()` reads the vector live, without fail-fast. */
  public static String vectorElements() {
    Vector<String> v = new Vector<>(List.of("a"));
    Enumeration<String> e = v.elements();
    String first = e.nextElement() + e.hasMoreElements();
    v.add("b");
    return first + e.hasMoreElements() + e.nextElement() + "|" + attempt(e::nextElement) + "|" + (v.elements() == v.elements());
  }

  /** `ClassLoader.getResources` of a missing name: a new empty enumeration per call. */
  public static String resources() throws Exception {
    ClassLoader loader = HostIterators.class.getClassLoader();
    Enumeration<java.net.URL> a = loader.getResources("no/such/resource.txt");
    Enumeration<java.net.URL> b = loader.getResources("no/such/resource.txt");
    return "" + a.hasMoreElements() + (a == b) + identities(a, b);
  }

  /** Two `listIterator()` calls are two list iterators with their own cursors. */
  public static String listIterators() {
    List<String> l = new ArrayList<>(List.of("a", "b", "c"));
    ListIterator<String> a = l.listIterator();
    ListIterator<String> b = l.listIterator(1);
    String same = "" + (a == b) + a.equals(b) + identities(a, b, a);
    a.next();
    a.set("A");
    a.add("x");
    return same + "|" + a.nextIndex() + b.nextIndex() + l + "|" + attempt(b::next) + "|" + (a instanceof ListIterator);
  }

  /** `Collectors.toList()` is a new collector each call; collecting works either way. */
  public static String collectors() {
    Collector<Object, ?, List<Object>> a = Collectors.toList();
    Collector<Object, ?, List<Object>> b = Collectors.toList();
    Collector<CharSequence, ?, String> j = Collectors.joining(",;", "[", "]");
    return "" + (a == b) + identities(a, b, a) + (a instanceof Collector) + "|" + Stream.of(1, 2).collect(a)
        + Stream.of(3).collect(a) + "|" + Stream.of("x", "y").collect(j) + Stream.of("z").collect(j) + "|"
        + Stream.of("p", "q").collect(Collectors.toSet()).size() + "|"
        + Stream.of("k1", "k2").collect(Collectors.toMap(k -> k, String::length)) + "|"
        + Stream.of("a", "b").collect(Collectors.joining()) + Stream.of("a", "b").collect(Collectors.joining("-"));
  }
}

package demo;

import com.google.common.collect.ImmutableList;
import com.google.common.collect.ImmutableSet;
import com.google.common.collect.Lists;
import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.Collection;
import java.util.Collections;
import java.util.HashMap;
import java.util.HashSet;
import java.util.IdentityHashMap;
import java.util.LinkedList;
import java.util.List;
import java.util.Map;
import java.util.PriorityQueue;
import java.util.RandomAccess;
import java.util.Set;
import java.util.stream.Collectors;
import java.util.stream.Stream;

/**
 * Collections the host makes are objects of their own (#156 P14c): Java's class for each factory, `==` tells two
 * apart (the JDK's shared empties are one object each), a write to one never shows in another, and a `String`
 * spelled like the host's old marker text is a `String`.
 */
public final class Probe {
    static String name(Object o) {
        return o.getClass().getName().replace("java.util.", "").replace("com.google.common.collect.", "g.");
    }

    /** `((List<?>) s).size()`: `C` for Java's `ClassCastException`. */
    static String cast(Object s) {
        try {
            return "" + ((List<?>) s).size();
        } catch (ClassCastException e) {
            return "C";
        }
    }

    static String b(boolean v) {
        return v ? "T" : "F";
    }

    /** `getClass()` of each JDK factory's answer. */
    public static String jdkClasses() {
        Object[] made = {
            List.of(), List.of(1), List.of(1, 2), List.of(1, 2, 3),
            List.copyOf(new ArrayList<>(List.of(1))), List.copyOf(new ArrayList<>(List.of(1, 2, 3))),
            Set.of(), Set.of(1), Set.of(1, 2), Set.of(1, 2, 3), Set.copyOf(new ArrayList<>(List.of(1, 1))),
            Map.of(), Map.of(1, 2), Map.of(1, 2, 3, 4), Map.copyOf(new HashMap<>(Map.of(1, 2))),
            Collections.emptyList(), Collections.emptySet(), Collections.emptyMap(),
            Collections.singletonList(1), Collections.singleton(1), Collections.singletonMap(1, 2),
            Collections.unmodifiableList(new ArrayList<>()), Collections.unmodifiableList(new LinkedList<>()),
            Collections.unmodifiableSet(new HashSet<>()), Collections.unmodifiableMap(new HashMap<>()),
            Stream.of().toList(), Stream.of(1).toList(), Stream.of(1, 2, 3).toList(),
        };
        StringBuilder out = new StringBuilder();
        for (Object o : made) {
            out.append(name(o)).append(',');
        }
        return out.toString();
    }

    /** `getClass()` of Guava's immutable factories. */
    public static String guavaClasses() {
        Object[] made = {
            ImmutableList.of(), ImmutableList.of(1), ImmutableList.of(1, 2),
            ImmutableList.copyOf(new ArrayList<>(List.of(1))), ImmutableList.copyOf(new ArrayList<>(List.of(1, 2))),
            ImmutableSet.copyOf(new ArrayList<>()), ImmutableSet.copyOf(new ArrayList<>(List.of(1, 1))),
            ImmutableSet.copyOf(new ArrayList<>(List.of(1, 2))),
        };
        StringBuilder out = new StringBuilder();
        for (Object o : made) {
            out.append(name(o)).append(',');
        }
        return out.toString();
    }

    /** The answers still behave: reads, `instanceof`, mutators refused, a view reading through. */
    public static String behaves() {
        List<Integer> source = new ArrayList<>(List.of(1));
        List<Integer> view = Collections.unmodifiableList(source);
        source.add(2);
        Object of = List.of(1, 2);
        String refused;
        try {
            List.of(1).add(2);
            refused = "added";
        } catch (UnsupportedOperationException e) {
            refused = "UOE";
        }
        return view + "|" + view.size() + "|" + of + "|" + ((List<?>) of).get(1) + b(((List<?>) of).contains(2))
            + "|" + of.hashCode() + "|" + b(of instanceof List) + b(of instanceof RandomAccess)
            + b(of instanceof Collection) + b(of instanceof Set) + b(Collections.emptyMap() instanceof Map) + "|"
            + refused + "|" + b(of.equals(new ArrayList<>(List.of(1, 2)))) + b(Set.of(1, 2).equals(Set.of(2, 1)));
    }

    /** Two host-made collections are two objects; the JDK's shared empties are one each. */
    public static String identity() {
        List<Integer> source = new ArrayList<>(List.of(1, 2, 3));
        Object a = List.of(1);
        Object b = List.of(1);
        IdentityHashMap<Object, Integer> ids = new IdentityHashMap<>();
        ids.put(a, 1);
        ids.put(b, 2);
        ids.put(a, 3);
        HashMap<Object, Integer> equal = new HashMap<>();
        equal.put(a, 1);
        equal.put(b, 2);
        Object view = Collections.unmodifiableList(source);
        return b(a == b) + b(a.equals(b)) + b(a == a) + b(view == Collections.unmodifiableList(source))
            + b(List.of() == List.of()) + b(Set.of() == Set.of()) + b(Map.of() == Map.of())
            + b(Collections.emptyList() == Collections.emptyList()) + b(Collections.emptySet() == Collections.emptySet())
            + b(Collections.emptyMap() == Collections.emptyMap()) + b((Object) List.of() == Collections.emptyList())
            + b(List.of() == Stream.of().toList()) + "|" + ids.size() + ids.get(a) + "|" + equal.size() + equal.get(a)
            + "|" + b(source.reversed() == source.reversed()) + b(source.reversed().equals(source.reversed()))
            + name(source.reversed()) + source.reversed();
    }

    /** `equals` by the receiver's real class: `AbstractList`/`AbstractSet`/`AbstractMap`'s contracts, `==` otherwise. */
    public static String equalsContracts() {
        Object al = new ArrayList<>(List.of(1, 2));
        Object of = List.of(1);
        Object deque = new ArrayDeque<>(List.of(1));
        Object queue = new PriorityQueue<>(List.of(1));
        Object linked = new LinkedList<>(List.of(1));
        HashMap<Integer, Integer> hm = new HashMap<>(Map.of(1, 2));
        Object values = hm.values();
        Object keys = hm.keySet();
        Object entries = hm.entrySet();
        List<Integer> typed = new ArrayList<>(List.of(1, 2));
        Collection<Integer> typedDeque = new ArrayDeque<>(List.of(1));
        return b(al.equals(Set.of(1, 2))) + b(of.equals(Set.of(1))) + b(deque.equals(new ArrayDeque<>(List.of(1))))
            + b(queue.equals(new PriorityQueue<>(List.of(1)))) + b(values.equals(new ArrayList<>(List.of(2))))
            + b(linked.equals(new ArrayDeque<>(List.of(1)))) + b(typed.equals(Set.of(1, 2)))
            + b(typedDeque.equals(new ArrayDeque<>(List.of(1))))
            + b(deque.equals(deque)) + "|" + b(al.equals(List.of(1, 2))) + b(of.equals(new LinkedList<>(List.of(1))))
            + b(keys.equals(Set.of(1))) + b(entries.equals(Map.of(1, 2).entrySet()))
            + b(((Object) Set.of(1, 2)).equals(new HashSet<>(List.of(2, 1)))) + b(((Object) Map.of(1, 2)).equals(hm))
            + b(((Object) Collections.emptyList()).equals(new ArrayList<>())) + b(al.equals("[1, 2]"))
            + b(((Object) Set.of()).equals(List.of()));
    }

    /** The JDK's shared empties are one object however reached. */
    public static String sharedEmpties() {
        Object toList = Stream.of().toList();
        return b(toList == Stream.of().toList()) + b(toList == List.of())
            + b(Stream.of().collect(Collectors.toUnmodifiableList()) == List.of())
            + b(Stream.of().collect(Collectors.toUnmodifiableSet()) == Set.of())
            + b(List.copyOf(new ArrayList<>()) == List.of()) + b(Set.copyOf(new HashSet<>()) == Set.of())
            + b(Map.copyOf(new HashMap<>()) == Map.of()) + b(Map.ofEntries() == Map.of());
    }

    /** A copy Java hands back unchanged is the same object. */
    public static String copies() {
        List<Integer> l12 = List.of(1, 2);
        List<Integer> ln = List.of(1, 2, 3);
        List<Integer> streamed = Stream.of(1, 2).toList();
        Set<Integer> s = Set.of(1);
        Map<Integer, Integer> m = Map.of(1, 2);
        List<Integer> u = Collections.unmodifiableList(new ArrayList<>(List.of(1)));
        List<Integer> ul = Collections.unmodifiableList(new LinkedList<>(List.of(1)));
        Set<Integer> us = Collections.unmodifiableSet(new HashSet<>(Set.of(1)));
        Map<Integer, Integer> um = Collections.unmodifiableMap(new HashMap<>(Map.of(1, 2)));
        ImmutableList<Integer> gl = ImmutableList.of(1, 2);
        ImmutableSet<Integer> gs = ImmutableSet.copyOf(new ArrayList<>(List.of(1, 2)));
        return b(List.copyOf(l12) == l12) + b(List.copyOf(ln) == ln) + b(List.copyOf(streamed) == streamed)
            + b(Set.copyOf(s) == s) + b(Map.copyOf(m) == m) + b(Collections.unmodifiableList(u) == u)
            + b(Collections.unmodifiableList(ul) == ul) + b(Collections.unmodifiableSet(us) == us)
            + b(Collections.unmodifiableMap(um) == um) + b(Collections.unmodifiableList(l12) == l12)
            + b(ImmutableList.copyOf(gl) == gl) + b(ImmutableSet.copyOf(gs) == gs) + b(List.copyOf(u) == u);
    }

    /** `getSimpleName`/`getCanonicalName` of the nested classes the host's collections are. */
    public static String names() {
        Object[] made = {
            List.of(1), List.of(1, 2, 3), Set.of(), Set.of(1), Map.of(1, 2), Map.of(), Collections.emptyList(),
            Collections.singletonMap(1, 2), Collections.unmodifiableList(new ArrayList<>()),
            new ArrayList<>(List.of(1)).reversed(), ImmutableList.of(1),
        };
        StringBuilder out = new StringBuilder();
        for (Object o : made) {
            out.append(o.getClass().getSimpleName()).append('/').append(o.getClass().getCanonicalName()).append(',');
        }
        return out.toString();
    }

    /** A write to one host-made list never shows in another. */
    public static String writes() {
        List<Integer> one = Lists.newArrayList(List.<Integer>of());
        List<Integer> two = Lists.newArrayList(List.<Integer>of());
        one.add(1);
        two.add(2);
        two.add(3);
        return one + "|" + two + "|" + b(one == two) + b(one.equals(two));
    }

    /** A `String` spelled like the host's old collection text is a `String`. */
    public static String strings() {
        List<Integer> made = Lists.newArrayList(List.of(1));
        Object frozen = List.of(1, 2, 3);
        StringBuilder out = new StringBuilder();
        for (int n = 0; n < 3; n++) {
            for (String text : new String[] {
                "java.util.ArrayList#neutral", "java.util.ImmutableCollections$ListN#immutable",
                "hcoll:java.util.ArrayList#neutral", "hcoll:java.util.ImmutableCollections$ListN#immutable",
            }) {
                Object s = text.replace('.', '/') + n;
                HashMap<Object, Integer> keys = new HashMap<>();
                keys.put(s, 1);
                keys.put(made, 2);
                keys.put(frozen, 3);
                out.append(name(s)).append(b(s instanceof List)).append(b(s instanceof String))
                    .append(b(s.equals(made))).append(b(s.equals(frozen))).append(b(made.equals(s)))
                    .append(b(s.hashCode() == ((String) s).hashCode())).append(keys.size()).append(keys.get(s))
                    .append(String.valueOf(s).length()).append(cast(s)).append(';');
            }
        }
        return out.toString();
    }

    public static void main(String[] args) {
        System.out.println("jdkClasses=" + jdkClasses());
        System.out.println("guavaClasses=" + guavaClasses());
        System.out.println("behaves=" + behaves());
        System.out.println("identity=" + identity());
        System.out.println("equalsContracts=" + equalsContracts());
        System.out.println("sharedEmpties=" + sharedEmpties());
        System.out.println("copies=" + copies());
        System.out.println("names=" + names());
        System.out.println("writes=" + writes());
        System.out.println("strings=" + strings());
    }
}

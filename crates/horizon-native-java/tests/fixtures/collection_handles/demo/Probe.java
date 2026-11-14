package demo;

import com.google.common.collect.ImmutableList;
import com.google.common.collect.ImmutableSet;
import com.google.common.collect.Lists;
import java.util.ArrayList;
import java.util.Collection;
import java.util.Collections;
import java.util.HashMap;
import java.util.HashSet;
import java.util.IdentityHashMap;
import java.util.LinkedList;
import java.util.List;
import java.util.Map;
import java.util.RandomAccess;
import java.util.Set;
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
            for (String text : new String[] {"java.util.ArrayList#neutral", "java.util.ImmutableCollections$ListN#immutable"}) {
                Object s = text.replace('.', '/') + n;
                HashMap<Object, Integer> keys = new HashMap<>();
                keys.put(s, 1);
                keys.put(made, 2);
                keys.put(frozen, 3);
                out.append(name(s)).append(b(s instanceof List)).append(b(s instanceof String))
                    .append(b(s.equals(made))).append(b(s.equals(frozen))).append(b(made.equals(s)))
                    .append(b(s.hashCode() == ((String) s).hashCode())).append(keys.size()).append(keys.get(s))
                    .append(String.valueOf(s).length()).append(';');
            }
        }
        return out.toString();
    }

    public static void main(String[] args) {
        System.out.println("jdkClasses=" + jdkClasses());
        System.out.println("guavaClasses=" + guavaClasses());
        System.out.println("behaves=" + behaves());
        System.out.println("identity=" + identity());
        System.out.println("writes=" + writes());
        System.out.println("strings=" + strings());
    }
}

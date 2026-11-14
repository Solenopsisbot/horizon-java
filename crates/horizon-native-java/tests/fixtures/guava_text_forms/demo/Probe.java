package demo;

import com.google.common.collect.ImmutableList;
import com.google.common.collect.ImmutableMap;
import com.google.common.collect.ImmutableSet;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.stream.Stream;

/**
 * Guava's immutable collections print, compare and hash as their `AbstractCollection`/`AbstractMap`
 * contracts say (#223). `main` prints what each probe answers on the real JVM with Guava 33.6.0;
 * Horizon runs the probes alone.
 */
public final class Probe {
    record P(int x) {}

    /** `toString`: `[e1, e2]` with each element's own text, by every route to it. */
    public static String listText() {
        ImmutableList<String> r = Stream.of("b", "a").collect(ImmutableList.toImmutableList());
        ImmutableList<String> one = ImmutableList.of("solo");
        ImmutableList<String> two = ImmutableList.of("x", "y");
        return r.getClass().getName() + r + "|" + r.toString() + "|" + String.valueOf(one) + one.getClass().getName()
            + "|" + two + "|" + ImmutableList.of() + "|" + ImmutableList.copyOf(List.of(3, 4));
    }

    /** Nested collections and elements with their own `toString`. */
    public static String nested() {
        return "" + ImmutableList.of(List.of(1, 2), ImmutableList.of("z")) + "|" + List.of(ImmutableList.of("q")) + "|"
            + ImmutableSet.of(ImmutableList.of(1));
    }

    /** A jar object's own `toString` inside a collection: Guava's as the JDK's (counted here). */
    public static String jarElements() {
        return "" + ImmutableList.of(new P(1)) + "|" + List.of(new P(2));
    }

    /** Sets and maps: `[..]` and `{k=v}`. */
    public static String setAndMapText() {
        ImmutableSet<String> s = Stream.of("p", "q", "p").collect(ImmutableSet.toImmutableSet());
        return s + "|" + ImmutableSet.of("m", "n") + "|" + ImmutableSet.of() + "|" + ImmutableMap.of("k", "v", "k2", 2)
            + "|" + ImmutableMap.of() + "|" + ImmutableMap.of("only", List.of(1));
    }

    /** `equals`/`hashCode` follow `List`/`Set`/`Map`'s contracts against the JDK's. */
    public static String jdkEquality() {
        return "" + ImmutableList.of(1).equals(List.of(1)) + List.of(1).equals(ImmutableList.of(1))
            + (ImmutableList.of(1, 2).hashCode() == List.of(1, 2).hashCode()) + ImmutableList.of(1, 2).equals(List.of(2, 1))
            + "|" + ImmutableSet.of(1, 2).equals(Set.of(2, 1)) + Set.of(2, 1).equals(ImmutableSet.of(1, 2))
            + (ImmutableSet.of(1, 2).hashCode() == Set.of(1, 2).hashCode()) + "|"
            + ImmutableMap.of("a", 1).equals(Map.of("a", 1)) + Map.of("a", 1).equals(ImmutableMap.of("a", 1))
            + (ImmutableMap.of("a", 1).hashCode() == Map.of("a", 1).hashCode()) + "|" + ImmutableList.of(1, 2).hashCode()
            + "," + ImmutableSet.of("a").hashCode() + "," + ImmutableMap.of("a", 1).hashCode();
    }

    public static void main(String[] args) {
        System.out.println("listText=" + listText());
        System.out.println("nested=" + nested());
        System.out.println("jarElements=" + jarElements());
        System.out.println("setAndMapText=" + setAndMapText());
        System.out.println("jdkEquality=" + jdkEquality());
    }
}

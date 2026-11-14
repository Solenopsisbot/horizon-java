package demo;

import com.google.common.collect.ImmutableList;
import com.google.common.collect.ImmutableSet;
import java.util.Collections;
import java.util.IdentityHashMap;
import java.util.Iterator;
import java.util.Map;
import java.util.stream.Collector;
import java.util.stream.Collectors;
import java.util.stream.Stream;

/**
 * Objects the JDK and Guava share, which the host must not split (#222's review): every
 * `Collections` empty collection's iterator is `Collections.emptyIterator()`'s one object, and
 * Guava's `toImmutableList()`/`toImmutableSet()` are its `TO_IMMUTABLE_LIST`/`TO_IMMUTABLE_SET`
 * constants, while `Collectors.toList()` is a new collector each call. `main` prints what each
 * probe answers on the real JVM; Horizon runs the probes alone.
 */
public final class Probe {
    private static String t(boolean b) {
        return b ? "T" : "F";
    }

    /** The empty collections' iterators are the one empty iterator. */
    public static String emptyIterators() {
        Iterator<Object> shared = Collections.emptyIterator();
        Map<Object, Object> empty = Collections.emptyMap();
        return t(shared == Collections.emptyList().iterator())
            + t(shared == Collections.emptySet().iterator())
            + t(shared == Collections.emptyList().iterator())
            + t(Collections.emptyList().iterator() == Collections.emptyList().iterator())
            + t(Collections.emptySet().iterator() == Collections.emptySet().iterator())
            + t((Object) shared == empty.entrySet().iterator())
            + t(shared == empty.keySet().iterator())
            + t(shared == empty.values().iterator())
            + "|" + Collections.emptyList().iterator().hasNext();
    }

    /** Other empty collections make a new iterator per call. */
    public static String freshIterators() {
        return t(java.util.List.of().iterator() == java.util.List.of().iterator())
            + t(new java.util.ArrayList<>().iterator() == new java.util.ArrayList<>().iterator())
            + t((Object) Collections.emptyIterator() == java.util.List.of().iterator());
    }

    /** Guava's two collectors are shared constants; the JDK's are new each call. */
    public static String collectors() {
        Collector<Object, ?, ImmutableList<Object>> a = ImmutableList.toImmutableList();
        Collector<Object, ?, ImmutableList<Object>> b = ImmutableList.toImmutableList();
        Collector<Object, ?, ImmutableSet<Object>> s = ImmutableSet.toImmutableSet();
        Map<Object, Integer> ids = new IdentityHashMap<>();
        ids.put(a, 1);
        ids.put(b, 2);
        ids.put(s, 3);
        ids.put(ImmutableSet.toImmutableSet(), 4);
        return t(a == b) + t(s == ImmutableSet.toImmutableSet()) + t((Object) a == s)
            + t(Collectors.toList() == Collectors.toList()) + ids.size();
    }

    /** `toImmutableSet()` keeps each element once, in encounter order. */
    public static String immutableSet() {
        ImmutableSet<String> words = Stream.of("b", "a", "b").collect(ImmutableSet.toImmutableSet());
        ImmutableSet<Integer> numbers = Stream.of(3, 1, 3, 2, 1).collect(ImmutableSet.toImmutableSet());
        StringBuilder out = new StringBuilder().append(words.size()).append(':');
        for (String word : words) {
            out.append(word);
        }
        out.append('|').append(numbers.size()).append(':');
        for (Integer number : numbers) {
            out.append(number);
        }
        return out.append('|').append(words.contains("b")).toString();
    }

    public static void main(String[] args) {
        System.out.println("emptyIterators=" + emptyIterators());
        System.out.println("freshIterators=" + freshIterators());
        System.out.println("collectors=" + collectors());
        System.out.println("immutableSet=" + immutableSet());
    }
}

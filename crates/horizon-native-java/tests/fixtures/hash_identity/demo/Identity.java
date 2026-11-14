package demo;

import java.lang.invoke.MethodHandle;
import java.lang.invoke.MethodHandles;
import java.lang.invoke.MethodType;
import java.nio.file.Path;
import java.util.HashMap;
import java.util.HashSet;
import java.util.IdentityHashMap;
import java.util.LinkedHashSet;
import java.util.Map;
import java.util.Set;

/**
 * One object put into a hash collection twice is one key (#156 P14b). A `MethodHandle` keeps `Object.equals` and
 * an identity hash, so only the stored object itself proves the second `add` finds the first: `p.key == key` in
 * `HashMap.getNode`, `equals` in `Set.of`'s probe. A `Path` hashes by value. Two handles made separately are two
 * objects to Java; Horizon cannot prove that and counts them.
 */
public final class Identity {
    static void target() {}

    static MethodHandle handle() {
        try {
            return MethodHandles.lookup().findStatic(Identity.class, "target", MethodType.methodType(void.class));
        } catch (ReflectiveOperationException e) {
            throw new IllegalStateException(e);
        }
    }

    static String b(boolean v) {
        return v ? "T" : "F";
    }

    static String setOf(Object x, Object y) {
        try {
            return "" + Set.of(x, y).size();
        } catch (IllegalArgumentException e) {
            return "I";
        }
    }

    static String setOf(Object x, Object y, Object z) {
        try {
            return "" + Set.of(x, y, z).size();
        } catch (IllegalArgumentException e) {
            return "I";
        }
    }

    static String mapOf(Object x, Object y) {
        try {
            return "" + Map.of(x, 1, y, 2).size();
        } catch (IllegalArgumentException e) {
            return "I";
        }
    }

    static String mapOf(Object x, Object y, Object z) {
        try {
            return "" + Map.of(x, 1, y, 2, z, 3).size();
        } catch (IllegalArgumentException e) {
            return "I";
        }
    }

    /** `HashSet`, `LinkedHashSet` and `HashMap`: add, put, `computeIfAbsent`, contains, get and remove. */
    public static String tables() {
        MethodHandle h = handle();
        Path p = Path.of("/a");
        Set<Object> set = new HashSet<>();
        String out = b(set.add(h)) + b(set.add(h)) + set.size() + b(set.contains(h));
        // A `Path` in a set of its own: beside an identity-hashed key, whose bin
        // no run can name, its lookup would call `Path.equals` on an unknown node.
        Set<Object> paths = new HashSet<>();
        out += b(paths.add(p)) + b(paths.add(p)) + paths.size();
        Set<Object> linked = new LinkedHashSet<>();
        linked.add(h);
        linked.add(h);
        out += linked.size();
        Map<Object, Integer> map = new HashMap<>();
        map.put(h, 1);
        out += map.put(h, 2) + "/" + map.get(h) + "/" + map.computeIfAbsent(h, k -> 9) + "/" + map.size();
        out += b(set.remove(h)) + set.size() + b(set.contains(h));
        return out;
    }

    /** `Set.of`/`Map.of` (`Set12`/`SetN`, `Map1`/`MapN`): a key given twice throws, and lookups find it. */
    public static String probes() {
        MethodHandle h = handle();
        Path p = Path.of("/a");
        String out = setOf(h, h) + setOf(h, p, h) + mapOf(h, h) + mapOf(h, p, h) + setOf(h, p) + setOf(h, p, "x");
        Set<Object> three = Set.of(h, p, "x");
        Map<Object, Integer> map = Map.of(p, 7, h, 8, "x", 9);
        return out + b(three.contains(h)) + b(Set.of(p, h).contains(h)) + map.get(h) + b(map.containsKey(h));
    }

    /** `IdentityHashMap`: `==` on the stored object. */
    public static String identity() {
        MethodHandle h = handle();
        IdentityHashMap<Object, Integer> map = new IdentityHashMap<>();
        map.put(h, 1);
        Integer old = map.put(h, 2);
        return old + "/" + map.get(h) + "/" + map.size() + b(map.containsKey(h)) + map.remove(h) + map.size();
    }

    /** Two handles made separately: two objects to Java, which Horizon counts rather than answering. */
    public static String separate() {
        MethodHandle h = handle();
        MethodHandle g = handle();
        Set<Object> set = new HashSet<>();
        set.add(h);
        boolean added = set.add(g);
        return b(h == g) + b(added) + set.size() + setOf(h, g);
    }

    public static void main(String[] args) {
        System.out.println("tables=" + tables());
        System.out.println("probes=" + probes());
        System.out.println("identity=" + identity());
        System.out.println("separate=" + separate());
    }
}

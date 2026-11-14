package demo;

import java.util.Collection;
import java.util.HashMap;
import java.util.LinkedHashMap;
import java.util.Map;
import java.util.Set;
import java.util.TreeMap;
import java.util.TreeSet;

/**
 * Collections the host used to answer as text literals are objects (#156 P14d1): a map's views are the same object on
 * every call and follow the map, two maps' views are two objects, and `System.getProperties()`/`getenv()` are one
 * object each, `getenv()`'s unmodifiable.
 */
public final class Probe {
    static String b(boolean v) {
        return v ? "T" : "F";
    }

    static String views(Map<String, Integer> m) {
        m.put("a", 1);
        Set<String> k = m.keySet();
        Collection<Integer> v = m.values();
        Set<Map.Entry<String, Integer>> e = m.entrySet();
        m.put("b", 2);
        Map<String, Integer> other = new HashMap<>(m);
        return b(k == m.keySet()) + b(v == m.values()) + b(e == m.entrySet()) + b((Object) k == other.keySet())
            + b(k.equals(other.keySet())) + "|" + k.size() + m.keySet().size() + v.size() + e.size() + "|"
            + new TreeSet<>(k);
    }

    /** A `HashMap`'s, a `LinkedHashMap`'s and a `TreeMap`'s views. */
    public static String mapViews() {
        return views(new HashMap<>()) + ";" + views(new LinkedHashMap<>()) + ";" + views(new TreeMap<>());
    }

    /** Two empty maps' views are two objects, and each stays as empty as its map (main shared one `set`). */
    public static String emptyViews() {
        Map<String, Integer> a = new HashMap<>();
        Map<String, Integer> b = new HashMap<>();
        Set<String> keys = a.keySet();
        Set<Map.Entry<String, Integer>> entries = a.entrySet();
        b.put("x", 1);
        int seen = 0;
        for (Map.Entry<String, Integer> e : entries) {
            seen += e.getValue();
        }
        return b((Object) keys == b.keySet()) + b((Object) entries == b.entrySet()) + b(keys.isEmpty())
            + seen + b.keySet().size() + b.entrySet().size();
    }

    /** `System.getProperties()` and `System.getenv()`: one object each, the environment's unmodifiable. */
    public static String system() {
        Map<String, String> env = System.getenv();
        String put;
        try {
            env.put("horizon-probe", "x");
            put = "put";
        } catch (UnsupportedOperationException e) {
            put = "UOE";
        }
        return b(System.getProperties() == System.getProperties()) + b(env == System.getenv())
            + b(env.equals(System.getenv())) + "|" + env.getClass().getName() + "|" + put;
    }

    public static void main(String[] args) {
        System.out.println("mapViews=" + mapViews());
        System.out.println("emptyViews=" + emptyViews());
        System.out.println("system=" + system());
    }
}

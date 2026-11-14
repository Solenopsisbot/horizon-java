package demo;

import java.nio.file.Path;
import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.Collections;
import java.util.IdentityHashMap;
import java.util.Iterator;
import java.util.LinkedHashMap;
import java.util.LinkedList;
import java.util.List;
import java.util.Map;
import java.util.PriorityQueue;
import java.util.TreeMap;
import java.util.TreeSet;

/**
 * One object stored in a collection and read back is that object: `==` on every read path a host collection
 * answers (#156 P14a). `Path` objects have no canonical form, so only the stored object proves identity.
 */
public final class Reads {
    static String same(Object a, Object b) {
        return a == b ? "T" : "F";
    }

    /** Lists, deques, queues and sorted sets, and the views and copies read from them. */
    public static String sequences() {
        Path p = Path.of("/a");
        Path q = Path.of("/b");
        List<Path> list = new ArrayList<>(List.of(q));
        list.add(p);
        LinkedList<Path> linked = new LinkedList<>();
        linked.add(p);
        ArrayDeque<Path> deque = new ArrayDeque<>();
        deque.add(p);
        PriorityQueue<Path> queue = new PriorityQueue<>();
        queue.add(p);
        TreeSet<Path> sorted = new TreeSet<>(List.of(q));
        sorted.add(p);
        Iterator<Path> it = list.iterator();
        it.next();
        return same(list.get(1), p) + same(list.getLast(), p) + same(it.next(), p) + same(linked.getFirst(), p)
            + same(deque.peekFirst(), p) + same(deque.pollLast(), p) + same(queue.peek(), p)
            + same(sorted.first(), p) + same(Collections.unmodifiableList(list).get(1), p)
            + same(list.subList(1, 2).get(0), p) + same(new ArrayList<>(list).get(1), p) + same(list.toArray()[1], p)
            + same(list.reversed().get(0), p) + same(list.stream().skip(1).toList().get(0), p)
            + list.indexOf(p) + (list.contains(p) ? "c" : "-");
    }

    /**
     * Maps: values and keys read back by lookup. (A `values()`/`keySet()` view's copy and an entry are inline text
     * until #156 P14c-P14e.)
     */
    public static String maps() {
        Path p = Path.of("/a");
        Map<String, Path> linked = new LinkedHashMap<>();
        linked.put("k", p);
        TreeMap<Path, String> tree = new TreeMap<>();
        tree.put(p, "v");
        TreeMap<String, Path> values = new TreeMap<>();
        values.put("k", p);
        IdentityHashMap<Object, Integer> ids = new IdentityHashMap<>();
        ids.put(linked.get("k"), 1);
        return same(linked.get("k"), p) + same(linked.getOrDefault("k", null), p) + same(tree.firstKey(), p)
            + same(values.get("k"), p) + ids.containsKey(p);
    }

    public static void main(String[] args) {
        System.out.println("sequences=" + sequences());
        System.out.println("maps=" + maps());
    }
}

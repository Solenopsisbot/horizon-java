package demo;

import com.google.common.base.Joiner;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Iterator;
import java.util.List;

/**
 * Guava's `Joiner.join(Iterator)` walks the iterator through its own `hasNext`/`next` (#225): it
 * consumes it, fails fast as it does, and stops at a `null` part. `main` prints what each probe
 * answers on the real JVM with Guava 33.6.0; Horizon runs the probes alone.
 */
public final class Probe {
    static String attempt(java.util.function.Supplier<Object> s) {
        try {
            return String.valueOf(s.get());
        } catch (RuntimeException e) {
            return e.getClass().getName() + ": " + e.getMessage();
        }
    }

    /** The iterator is consumed. */
    public static String consumes() {
        Iterator<Integer> it = new ArrayList<>(List.of(1, 2, 3)).iterator();
        it.next();
        String joined = Joiner.on(",").join(it);
        return joined + "|" + it.hasNext() + "|" + Joiner.on("-").join(List.of("a", "b"));
    }

    /** A list changed after `iterator()` fails fast inside `join`. */
    public static String failsFast() {
        List<Integer> l = new ArrayList<>(List.of(1, 2));
        Iterator<Integer> it = l.iterator();
        l.add(3);
        return attempt(() -> Joiner.on(",").join(it));
    }

    /** A jar's own iterator runs its own code. */
    public static String jarIterator() {
        Iterator<String> it = new Iterator<>() {
            int n = 0;

            public boolean hasNext() {
                return n < 3;
            }

            public String next() {
                return "p" + n++;
            }
        };
        return Joiner.on("+").join(it) + "|" + it.hasNext();
    }

    /** A `null` part throws after being drawn; the rest stays in the iterator. */
    public static String nullPart() {
        Iterator<String> it = new ArrayList<>(Arrays.asList("a", null, "c")).iterator();
        return attempt(() -> Joiner.on(",").join(it)) + "|" + it.hasNext() + it.next();
    }

    public static void main(String[] args) {
        System.out.println("consumes=" + consumes());
        System.out.println("failsFast=" + failsFast());
        System.out.println("jarIterator=" + jarIterator());
        System.out.println("nullPart=" + nullPart());
    }
}

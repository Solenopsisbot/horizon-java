package demo;

import java.util.ArrayList;
import java.util.Collections;
import java.util.HashMap;
import java.util.IdentityHashMap;
import java.util.List;
import java.util.Map;
import java.util.function.Supplier;

/**
 * A jar that allocates an object with fields on every callback, far past 65,536, and keeps a few objects alive
 * through every kind of reference (#197): a static, a `HashMap`, an `Object[]`, a lambda's capture, an unmodifiable
 * view of a jar list, a `List.of`, an `IdentityHashMap` key, and a chain reached only through fields.
 */
public final class Probe {
    static final class Cell {
        int v;
        String name;
        Cell next;

        Cell(int v, String name) {
            this.v = v;
            this.name = name;
        }

        String show() {
            return v + name;
        }
    }

    static Cell held;
    static final Map<String, Cell> MAP = new HashMap<>();
    static final Object[] ARRAY = new Object[2];
    static Supplier<Cell> captured;
    static List<Cell> view;
    static List<Cell> frozen;
    static final IdentityHashMap<Object, String> IDS = new IdentityHashMap<>();
    static int ticks;
    static int checksum;

    /** Builds the kept objects. */
    public static String setup() {
        Cell shared = new Cell(7, "shared");
        shared.next = new Cell(8, "second");
        shared.next.next = new Cell(9, "third");
        held = shared;
        MAP.put("k", shared);
        ARRAY[0] = shared;
        Cell local = new Cell(11, "captured");
        captured = () -> local;
        List<Cell> list = new ArrayList<>();
        list.add(new Cell(12, "viewed"));
        view = Collections.unmodifiableList(list);
        frozen = List.of(new Cell(13, "frozen"));
        IDS.put(shared, "id");
        return "ok";
    }

    /** One callback: a few objects with fields, all dropped. */
    public static String tick() {
        Cell garbage = new Cell(++ticks, "garbage");
        garbage.next = new Cell(ticks * 2, "more");
        checksum += garbage.next.v - garbage.v;
        return "t";
    }

    /** The kept objects: their fields, and that every reference to `shared` is the same object. */
    public static String check() {
        Cell shared = MAP.get("k");
        return ticks + "|" + checksum + "|" + shared.v + shared.name + shared.next.v + shared.next.name
            + shared.next.next.v + shared.next.next.name + "|" + (held == shared) + (ARRAY[0] == shared)
            + IDS.containsKey(shared) + IDS.get(held) + "|" + captured.get().v + captured.get().name
            + (captured.get() == captured.get()) + "|" + view.get(0).v + view.get(0).name + "|" + frozen.get(0).v
            + frozen.get(0).name;
    }

    static void garbage(int n) {
        for (int i = 0; i < n; i++) {
            Cell g = new Cell(i, "g");
            g.next = new Cell(-i, "h");
        }
    }

    /** An object the caller gets back after 3,000 garbage objects (#226 review). */
    public static Object makeReturned() {
        garbage(3000);
        return new Cell(40, "returned");
    }

    /** An object the caller keeps and passes back after many callbacks. */
    public static Object makeHeld() {
        return new Cell(41, "held");
    }

    /** A jar `ArrayList` the caller gets back. */
    public static Object makeReturnedList() {
        garbage(3000);
        List<Cell> list = new ArrayList<>();
        list.add(new Cell(42, "retlist"));
        return list;
    }

    /** A bound method reference the caller gets back. */
    public static Object makeReturnedLambda() {
        Cell c = new Cell(43, "retlambda");
        garbage(3000);
        return (Supplier<String>) c::show;
    }

    public static String read(Object o) {
        return ((Cell) o).show();
    }

    @SuppressWarnings("unchecked")
    public static String readList(Object o) {
        return ((List<Cell>) o).get(0).show();
    }

    @SuppressWarnings("unchecked")
    public static String readSupplier(Object o) {
        return ((Supplier<String>) o).get();
    }

    public static void main(String[] args) {
        setup();
        Object returned = makeReturned();
        System.out.println("returned=" + read(returned));
        Object held = makeHeld();
        Object list = makeReturnedList();
        Object lambda = makeReturnedLambda();
        for (int i = 0; i < 100_000; i++) {
            tick();
        }
        System.out.println("held=" + read(held) + "|" + readList(list) + "|" + readSupplier(lambda));
        System.out.println("check=" + check());
    }
}

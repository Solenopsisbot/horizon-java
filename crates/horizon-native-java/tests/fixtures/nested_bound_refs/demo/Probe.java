package demo;

import java.util.ArrayList;
import java.util.Iterator;
import java.util.List;
import java.util.ListIterator;
import java.util.concurrent.Callable;
import java.util.function.Supplier;

/**
 * A bound method reference called through another bound reference throws what the inner call throws, at any depth
 * (#224), exactly as the lambda form and the direct call do.
 */
public final class Probe {
    /** What `s.get()` throws (its class name), or `=` and its answer. */
    static String cls(Supplier<?> s) {
        try {
            return "=" + s.get();
        } catch (RuntimeException e) {
            return e.getClass().getName();
        }
    }

    /** What `c.call()` throws (its class name and message), or `=` and its answer. */
    static String call(Callable<?> c) {
        try {
            return "=" + c.call();
        } catch (Exception e) {
            return e.getClass().getName() + ":" + e.getMessage();
        }
    }

    static String fail() throws Exception {
        throw new Exception("checked");
    }

    String name = "probe";

    String failing() throws java.io.IOException {
        throw new java.io.IOException("io " + name);
    }

    /** `it::next` past the end, through `n::get`; the lambda form and the direct call. */
    public static String iterator() {
        Iterator<String> it = List.of("x").iterator();
        it.next();
        Supplier<String> n = it::next;
        return cls(n::get) + "|" + cls(() -> n.get()) + "|" + cls(n);
    }

    /** A list iterator's `li::previous` at the start, the same ways. */
    public static String listIterator() {
        ListIterator<String> li = new ArrayList<>(List.of("x")).listIterator();
        Supplier<String> p = li::previous;
        return cls(p::get) + "|" + cls(() -> p.get()) + "|" + cls(p);
    }

    /** A bound reference to a method that throws a checked exception, through `Callable`. */
    public static String checked() throws Exception {
        Probe probe = new Probe();
        Callable<String> c = probe::failing;
        Callable<String> outer = c::call;
        Callable<String> stat = Probe::fail;
        Callable<String> statOuter = stat::call;
        return call(outer) + "|" + call(() -> c.call()) + "|" + call(c) + "|" + call(statOuter) + "|" + call(stat);
    }

    /** Two and three levels of bound references over a throwing call, and over a normal one. */
    public static String nested() {
        Iterator<String> it = List.of("x").iterator();
        it.next();
        Supplier<String> one = it::next;
        Supplier<String> two = one::get;
        Supplier<String> three = two::get;
        Iterator<String> live = List.of("y", "z").iterator();
        Supplier<String> ok = live::next;
        Supplier<String> okTwo = ok::get;
        Supplier<String> okThree = okTwo::get;
        return cls(two) + "|" + cls(three::get) + "|" + cls(() -> three.get()) + "|" + cls(okThree::get) + "|"
            + cls(okThree);
    }

    public static void main(String[] args) throws Exception {
        System.out.println("iterator=" + iterator());
        System.out.println("listIterator=" + listIterator());
        System.out.println("checked=" + checked());
        System.out.println("nested=" + nested());
    }
}

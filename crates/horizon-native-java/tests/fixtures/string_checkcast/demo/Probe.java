package demo;

/**
 * Casts of a `String` (#167): `java.lang.String` is final and implements exactly `Serializable`,
 * `Comparable`, `CharSequence`, `Constable` and `ConstantDesc`, so a cast to any other type
 * throws `ClassCastException`. Each probe casts a computed `String` (no constant the compiler
 * could fold) and answers `ok` or the exception's message, then `|` and `instanceof`. `main`
 * prints every probe on the real JVM; Horizon runs the probes alone.
 */
public final class Probe {
    interface Iface {}

    static class Klass {}

    private static Object text() {
        StringBuilder text = new StringBuilder("hel");
        return text.append("lo").toString();
    }

    private static String cast(Runnable cast, boolean instance) {
        try {
            cast.run();
            return "ok|" + instance;
        } catch (ClassCastException e) {
            return "CCE " + e.getMessage() + "|" + instance;
        }
    }

    public static String runnable() {
        Object s = text();
        return cast(() -> { Runnable c = (Runnable) s; }, s instanceof Runnable);
    }

    public static String list() {
        Object s = text();
        return cast(() -> { java.util.List<?> c = (java.util.List<?>) s; }, s instanceof java.util.List);
    }

    public static String supplier() {
        Object s = text();
        return cast(() -> { java.util.function.Supplier<?> c = (java.util.function.Supplier<?>) s; }, s instanceof java.util.function.Supplier);
    }

    public static String charSequence() {
        Object s = text();
        return cast(() -> { CharSequence c = (CharSequence) s; }, s instanceof CharSequence);
    }

    public static String comparable() {
        Object s = text();
        return cast(() -> { Comparable<?> c = (Comparable<?>) s; }, s instanceof Comparable);
    }

    public static String serializable() {
        Object s = text();
        return cast(() -> { java.io.Serializable c = (java.io.Serializable) s; }, s instanceof java.io.Serializable);
    }

    public static String constable() {
        Object s = text();
        return cast(() -> { java.lang.constant.Constable c = (java.lang.constant.Constable) s; }, s instanceof java.lang.constant.Constable);
    }

    public static String constantDesc() {
        Object s = text();
        return cast(() -> { java.lang.constant.ConstantDesc c = (java.lang.constant.ConstantDesc) s; }, s instanceof java.lang.constant.ConstantDesc);
    }

    public static String integer() {
        Object s = text();
        return cast(() -> { Integer c = (Integer) s; }, s instanceof Integer);
    }

    public static String objectArray() {
        Object s = text();
        return cast(() -> { Object[] c = (Object[]) s; }, s instanceof Object[]);
    }

    public static String stringArray() {
        Object s = text();
        return cast(() -> { String[] c = (String[]) s; }, s instanceof String[]);
    }

    public static String intArray() {
        Object s = text();
        return cast(() -> { int[] c = (int[]) s; }, s instanceof int[]);
    }

    public static String intMatrix() {
        Object s = text();
        return cast(() -> { int[][] c = (int[][]) s; }, s instanceof int[][]);
    }

    public static String jarInterface() {
        Object s = text();
        return cast(() -> { Iface c = (Iface) s; }, s instanceof Iface);
    }

    public static String jarClass() {
        Object s = text();
        return cast(() -> { Klass c = (Klass) s; }, s instanceof Klass);
    }

    public static String jarArray() {
        Object s = text();
        return cast(() -> { Iface[] c = (Iface[]) s; }, s instanceof Iface[]);
    }

    public static String desktop() {
        Object s = text();
        return cast(() -> { java.awt.event.ActionListener c = (java.awt.event.ActionListener) s; }, s instanceof java.awt.event.ActionListener);
    }

    public static String logging() {
        Object s = text();
        return cast(() -> { java.util.logging.Filter c = (java.util.logging.Filter) s; }, s instanceof java.util.logging.Filter);
    }

    public static String sql() {
        Object s = text();
        return cast(() -> { java.sql.Wrapper c = (java.sql.Wrapper) s; }, s instanceof java.sql.Wrapper);
    }

    public static String gameClass() {
        Object s = text();
        return cast(() -> { net.minecraft.world.item.Item c = (net.minecraft.world.item.Item) s; },
            s instanceof net.minecraft.world.item.Item);
    }

    /** Text shaped like a class name: the host's stand-in for objects it does not model. */
    public static String classNameText() {
        StringBuilder name = new StringBuilder("net/minecraft/core/");
        Object s = name.append("Registry").toString();
        return cast(() -> { Iface c = (Iface) s; }, s instanceof Iface);
    }

    public static void main(String[] args) {
        System.out.println("runnable=" + runnable());
        System.out.println("list=" + list());
        System.out.println("supplier=" + supplier());
        System.out.println("charSequence=" + charSequence());
        System.out.println("comparable=" + comparable());
        System.out.println("serializable=" + serializable());
        System.out.println("constable=" + constable());
        System.out.println("constantDesc=" + constantDesc());
        System.out.println("integer=" + integer());
        System.out.println("objectArray=" + objectArray());
        System.out.println("stringArray=" + stringArray());
        System.out.println("intArray=" + intArray());
        System.out.println("intMatrix=" + intMatrix());
        System.out.println("jarInterface=" + jarInterface());
        System.out.println("jarClass=" + jarClass());
        System.out.println("jarArray=" + jarArray());
        System.out.println("desktop=" + desktop());
        System.out.println("logging=" + logging());
        System.out.println("sql=" + sql());
        System.out.println("gameClass=" + gameClass());
        System.out.println("classNameText=" + classNameText());
    }
}

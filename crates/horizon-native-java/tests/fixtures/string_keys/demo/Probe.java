package demo;

import java.lang.invoke.MethodHandle;
import java.lang.invoke.MethodHandles;
import java.lang.invoke.MethodType;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.function.Consumer;
import java.util.function.Function;

/**
 * A `String` spelling an object's key is that `String`, whatever the key's package (#156 P12b2, #198 review F1):
 * jar classes in packages named like host markers (`map`, `list`, `set`, `stream`, ...) included. And
 * `Class.cast` of a `String` to a class it is not throws Java's `ClassCastException` (F2).
 */
@SuppressWarnings({"rawtypes", "unchecked"})
public final class Probe {
    interface Call {
        Object run() throws Throwable;
    }

    static String b(boolean v) {
        return v ? "T" : "F";
    }

    /** `ok` or the exception's simple class name and message. */
    static String failure(Call call) {
        try {
            Object result = call.run();
            return result == null ? "ok" : "=" + result;
        } catch (Throwable e) {
            return e.getClass().getSimpleName() + (e instanceof ClassCastException ? "" : ":" + e.getMessage());
        }
    }

    static String cast(Class<?> c, Object o) {
        try {
            c.cast(o);
            return "ok;";
        } catch (ClassCastException e) {
            return e.getMessage() + ";";
        }
    }

    public static class Foo {
        public int value = 7;

        public String hello() {
            return "hi";
        }
    }

    /** `Class.cast` of a `String` (F2), and `Method.invoke` on one (F4). */
    public static String casts() {
        ArrayList<Object> list = new ArrayList<>();
        Foo foo = new Foo();
        StringBuilder out = new StringBuilder();
        out.append(cast(ArrayList.class, "hello")).append(cast(ArrayList.class, "java/util/ArrayList#0"))
            .append(cast(list.getClass(), "hello")).append(cast(StringBuilder.class, "java/lang/StringBuilder#0"))
            .append(cast(HashMap.class, "x")).append(cast(RuntimeException.class, "x"))
            .append(cast(Runnable.class, "x")).append(cast(List.class, "x")).append(cast(Foo.class, "demo/Probe$Foo#0"))
            .append(cast(CharSequence.class, "x")).append(cast(Comparable.class, "x")).append(cast(Object.class, "x"))
            .append(cast(String.class, "x")).append(cast(ArrayList.class, list)).append(cast(Foo.class, foo));
        for (Object s : new Object[] {"demo/Probe$Foo#0", "plain"}) {
            out.append(failure(() -> Foo.class.getMethod("hello").invoke(s))).append(';');
        }
        return out.append(foo.value).toString();
    }

    public static String mapExact() {
        map.Thing t = new map.Thing();
        t.value = 42;
        StringBuilder out = new StringBuilder();
        for (int n = 0; n < 3; n++) {
            Object s = "map/Thing#" + n;
            String label = switch (s) {
                case map.Thing x -> "thing";
                case String x -> "string";
                default -> "other";
            };
            out.append(b(s instanceof map.Thing)).append(b(map.Thing.class.isInstance(s))).append(s.getClass().getName())
                .append('|').append(failure(() -> (map.Thing) s)).append('|').append(failure(() -> map.Thing.class.cast(s)))
                .append('|').append(String.valueOf(s)).append('|').append(label).append('|')
                .append(b(s.equals(t))).append(b(t.equals(s))).append(';');
        }
        return out.append(t.value).toString();
    }

    public static String listExact() {
        list.Thing t = new list.Thing();
        t.value = 42;
        StringBuilder out = new StringBuilder();
        for (int n = 0; n < 3; n++) {
            Object s = "list/Thing#" + n;
            String label = switch (s) {
                case list.Thing x -> "thing";
                case String x -> "string";
                default -> "other";
            };
            out.append(b(s instanceof list.Thing)).append(b(list.Thing.class.isInstance(s))).append(s.getClass().getName())
                .append('|').append(failure(() -> (list.Thing) s)).append('|').append(failure(() -> list.Thing.class.cast(s)))
                .append('|').append(String.valueOf(s)).append('|').append(label).append('|')
                .append(b(s.equals(t))).append(b(t.equals(s))).append(';');
        }
        return out.append(t.value).toString();
    }

    public static String setExact() {
        set.Thing t = new set.Thing();
        t.value = 42;
        StringBuilder out = new StringBuilder();
        for (int n = 0; n < 3; n++) {
            Object s = "set/Thing#" + n;
            String label = switch (s) {
                case set.Thing x -> "thing";
                case String x -> "string";
                default -> "other";
            };
            out.append(b(s instanceof set.Thing)).append(b(set.Thing.class.isInstance(s))).append(s.getClass().getName())
                .append('|').append(failure(() -> (set.Thing) s)).append('|').append(failure(() -> set.Thing.class.cast(s)))
                .append('|').append(String.valueOf(s)).append('|').append(label).append('|')
                .append(b(s.equals(t))).append(b(t.equals(s))).append(';');
        }
        return out.append(t.value).toString();
    }

    public static String streamExact() {
        stream.Thing t = new stream.Thing();
        t.value = 42;
        StringBuilder out = new StringBuilder();
        for (int n = 0; n < 3; n++) {
            Object s = "stream/Thing#" + n;
            String label = switch (s) {
                case stream.Thing x -> "thing";
                case String x -> "string";
                default -> "other";
            };
            out.append(b(s instanceof stream.Thing)).append(b(stream.Thing.class.isInstance(s))).append(s.getClass().getName())
                .append('|').append(failure(() -> (stream.Thing) s)).append('|').append(failure(() -> stream.Thing.class.cast(s)))
                .append('|').append(String.valueOf(s)).append('|').append(label).append('|')
                .append(b(s.equals(t))).append(b(t.equals(s))).append(';');
        }
        return out.append(t.value).toString();
    }

    public static String optionalExact() {
        optional.Thing t = new optional.Thing();
        t.value = 42;
        StringBuilder out = new StringBuilder();
        for (int n = 0; n < 3; n++) {
            Object s = "optional/Thing#" + n;
            String label = switch (s) {
                case optional.Thing x -> "thing";
                case String x -> "string";
                default -> "other";
            };
            out.append(b(s instanceof optional.Thing)).append(b(optional.Thing.class.isInstance(s))).append(s.getClass().getName())
                .append('|').append(failure(() -> (optional.Thing) s)).append('|').append(failure(() -> optional.Thing.class.cast(s)))
                .append('|').append(String.valueOf(s)).append('|').append(label).append('|')
                .append(b(s.equals(t))).append(b(t.equals(s))).append(';');
        }
        return out.append(t.value).toString();
    }

    public static String typeExact() {
        type.Thing t = new type.Thing();
        t.value = 42;
        StringBuilder out = new StringBuilder();
        for (int n = 0; n < 3; n++) {
            Object s = "type/Thing#" + n;
            String label = switch (s) {
                case type.Thing x -> "thing";
                case String x -> "string";
                default -> "other";
            };
            out.append(b(s instanceof type.Thing)).append(b(type.Thing.class.isInstance(s))).append(s.getClass().getName())
                .append('|').append(failure(() -> (type.Thing) s)).append('|').append(failure(() -> type.Thing.class.cast(s)))
                .append('|').append(String.valueOf(s)).append('|').append(label).append('|')
                .append(b(s.equals(t))).append(b(t.equals(s))).append(';');
        }
        return out.append(t.value).toString();
    }

    public static String componentExact() {
        component.Thing t = new component.Thing();
        t.value = 42;
        StringBuilder out = new StringBuilder();
        for (int n = 0; n < 3; n++) {
            Object s = "component/Thing#" + n;
            String label = switch (s) {
                case component.Thing x -> "thing";
                case String x -> "string";
                default -> "other";
            };
            out.append(b(s instanceof component.Thing)).append(b(component.Thing.class.isInstance(s))).append(s.getClass().getName())
                .append('|').append(failure(() -> (component.Thing) s)).append('|').append(failure(() -> component.Thing.class.cast(s)))
                .append('|').append(String.valueOf(s)).append('|').append(label).append('|')
                .append(b(s.equals(t))).append(b(t.equals(s))).append(';');
        }
        return out.append(t.value).toString();
    }

    public static String uuidExact() {
        uuid.Thing t = new uuid.Thing();
        t.value = 42;
        StringBuilder out = new StringBuilder();
        for (int n = 0; n < 3; n++) {
            Object s = "uuid/Thing#" + n;
            String label = switch (s) {
                case uuid.Thing x -> "thing";
                case String x -> "string";
                default -> "other";
            };
            out.append(b(s instanceof uuid.Thing)).append(b(uuid.Thing.class.isInstance(s))).append(s.getClass().getName())
                .append('|').append(failure(() -> (uuid.Thing) s)).append('|').append(failure(() -> uuid.Thing.class.cast(s)))
                .append('|').append(String.valueOf(s)).append('|').append(label).append('|')
                .append(b(s.equals(t))).append(b(t.equals(s))).append(';');
        }
        return out.append(t.value).toString();
    }

    public static String codecExact() {
        codec.Thing t = new codec.Thing();
        t.value = 42;
        StringBuilder out = new StringBuilder();
        for (int n = 0; n < 3; n++) {
            Object s = "codec/Thing#" + n;
            String label = switch (s) {
                case codec.Thing x -> "thing";
                case String x -> "string";
                default -> "other";
            };
            out.append(b(s instanceof codec.Thing)).append(b(codec.Thing.class.isInstance(s))).append(s.getClass().getName())
                .append('|').append(failure(() -> (codec.Thing) s)).append('|').append(failure(() -> codec.Thing.class.cast(s)))
                .append('|').append(String.valueOf(s)).append('|').append(label).append('|')
                .append(b(s.equals(t))).append(b(t.equals(s))).append(';');
        }
        return out.append(t.value).toString();
    }

    public static String iteratorExact() {
        iterator.Thing t = new iterator.Thing();
        t.value = 42;
        StringBuilder out = new StringBuilder();
        for (int n = 0; n < 3; n++) {
            Object s = "iterator/Thing#" + n;
            String label = switch (s) {
                case iterator.Thing x -> "thing";
                case String x -> "string";
                default -> "other";
            };
            out.append(b(s instanceof iterator.Thing)).append(b(iterator.Thing.class.isInstance(s))).append(s.getClass().getName())
                .append('|').append(failure(() -> (iterator.Thing) s)).append('|').append(failure(() -> iterator.Thing.class.cast(s)))
                .append('|').append(String.valueOf(s)).append('|').append(label).append('|')
                .append(b(s.equals(t))).append(b(t.equals(s))).append(';');
        }
        return out.append(t.value).toString();
    }

    public static String durationExact() {
        duration.Thing t = new duration.Thing();
        t.value = 42;
        StringBuilder out = new StringBuilder();
        for (int n = 0; n < 3; n++) {
            Object s = "duration/Thing#" + n;
            String label = switch (s) {
                case duration.Thing x -> "thing";
                case String x -> "string";
                default -> "other";
            };
            out.append(b(s instanceof duration.Thing)).append(b(duration.Thing.class.isInstance(s))).append(s.getClass().getName())
                .append('|').append(failure(() -> (duration.Thing) s)).append('|').append(failure(() -> duration.Thing.class.cast(s)))
                .append('|').append(String.valueOf(s)).append('|').append(label).append('|')
                .append(b(s.equals(t))).append(b(t.equals(s))).append(';');
        }
        return out.append(t.value).toString();
    }

    public static String localeExact() {
        locale.Thing t = new locale.Thing();
        t.value = 42;
        StringBuilder out = new StringBuilder();
        for (int n = 0; n < 3; n++) {
            Object s = "locale/Thing#" + n;
            String label = switch (s) {
                case locale.Thing x -> "thing";
                case String x -> "string";
                default -> "other";
            };
            out.append(b(s instanceof locale.Thing)).append(b(locale.Thing.class.isInstance(s))).append(s.getClass().getName())
                .append('|').append(failure(() -> (locale.Thing) s)).append('|').append(failure(() -> locale.Thing.class.cast(s)))
                .append('|').append(String.valueOf(s)).append('|').append(label).append('|')
                .append(b(s.equals(t))).append(b(t.equals(s))).append(';');
        }
        return out.append(t.value).toString();
    }

    public static String list_renderExact() {
        list.render.Thing t = new list.render.Thing();
        t.value = 42;
        StringBuilder out = new StringBuilder();
        for (int n = 0; n < 3; n++) {
            Object s = "list/render/Thing#" + n;
            String label = switch (s) {
                case list.render.Thing x -> "thing";
                case String x -> "string";
                default -> "other";
            };
            out.append(b(s instanceof list.render.Thing)).append(b(list.render.Thing.class.isInstance(s))).append(s.getClass().getName())
                .append('|').append(failure(() -> (list.render.Thing) s)).append('|').append(failure(() -> list.render.Thing.class.cast(s)))
                .append('|').append(String.valueOf(s)).append('|').append(label).append('|')
                .append(b(s.equals(t))).append(b(t.equals(s))).append(';');
        }
        return out.append(t.value).toString();
    }

    public static String set_rowsExact() {
        set.rows.Thing t = new set.rows.Thing();
        t.value = 42;
        StringBuilder out = new StringBuilder();
        for (int n = 0; n < 3; n++) {
            Object s = "set/rows/Thing#" + n;
            String label = switch (s) {
                case set.rows.Thing x -> "thing";
                case String x -> "string";
                default -> "other";
            };
            out.append(b(s instanceof set.rows.Thing)).append(b(set.rows.Thing.class.isInstance(s))).append(s.getClass().getName())
                .append('|').append(failure(() -> (set.rows.Thing) s)).append('|').append(failure(() -> set.rows.Thing.class.cast(s)))
                .append('|').append(String.valueOf(s)).append('|').append(label).append('|')
                .append(b(s.equals(t))).append(b(t.equals(s))).append(';');
        }
        return out.append(t.value).toString();
    }

    public static String mapCalls() throws Throwable {
        map.Thing t = new map.Thing();
        t.value = 42;
        Function<map.Thing, Integer> f = map.Thing::get;
        Consumer<map.Thing> set = x -> x.value = 99;
        MethodHandle mh = MethodHandles.lookup().findVirtual(map.Thing.class, "get", MethodType.methodType(int.class));
        StringBuilder out = new StringBuilder();
        for (int n = 0; n < 3; n++) {
            Object s = "map/Thing#" + n;
            out.append(failure(() -> ((Function) f).apply(s))).append(failure(() -> (int) mh.invoke(s)))
                .append(failure(() -> { ((Consumer) set).accept(s); return null; })).append(';');
        }
        return out.append('|').append(t.value).toString();
    }

    public static String listCalls() throws Throwable {
        list.Thing t = new list.Thing();
        t.value = 42;
        Function<list.Thing, Integer> f = list.Thing::get;
        Consumer<list.Thing> set = x -> x.value = 99;
        MethodHandle mh = MethodHandles.lookup().findVirtual(list.Thing.class, "get", MethodType.methodType(int.class));
        StringBuilder out = new StringBuilder();
        for (int n = 0; n < 3; n++) {
            Object s = "list/Thing#" + n;
            out.append(failure(() -> ((Function) f).apply(s))).append(failure(() -> (int) mh.invoke(s)))
                .append(failure(() -> { ((Consumer) set).accept(s); return null; })).append(';');
        }
        return out.append('|').append(t.value).toString();
    }

    public static String setCalls() throws Throwable {
        set.Thing t = new set.Thing();
        t.value = 42;
        Function<set.Thing, Integer> f = set.Thing::get;
        Consumer<set.Thing> set = x -> x.value = 99;
        MethodHandle mh = MethodHandles.lookup().findVirtual(set.Thing.class, "get", MethodType.methodType(int.class));
        StringBuilder out = new StringBuilder();
        for (int n = 0; n < 3; n++) {
            Object s = "set/Thing#" + n;
            out.append(failure(() -> ((Function) f).apply(s))).append(failure(() -> (int) mh.invoke(s)))
                .append(failure(() -> { ((Consumer) set).accept(s); return null; })).append(';');
        }
        return out.append('|').append(t.value).toString();
    }

    public static String streamCalls() throws Throwable {
        stream.Thing t = new stream.Thing();
        t.value = 42;
        Function<stream.Thing, Integer> f = stream.Thing::get;
        Consumer<stream.Thing> set = x -> x.value = 99;
        MethodHandle mh = MethodHandles.lookup().findVirtual(stream.Thing.class, "get", MethodType.methodType(int.class));
        StringBuilder out = new StringBuilder();
        for (int n = 0; n < 3; n++) {
            Object s = "stream/Thing#" + n;
            out.append(failure(() -> ((Function) f).apply(s))).append(failure(() -> (int) mh.invoke(s)))
                .append(failure(() -> { ((Consumer) set).accept(s); return null; })).append(';');
        }
        return out.append('|').append(t.value).toString();
    }

    public static String optionalCalls() throws Throwable {
        optional.Thing t = new optional.Thing();
        t.value = 42;
        Function<optional.Thing, Integer> f = optional.Thing::get;
        Consumer<optional.Thing> set = x -> x.value = 99;
        MethodHandle mh = MethodHandles.lookup().findVirtual(optional.Thing.class, "get", MethodType.methodType(int.class));
        StringBuilder out = new StringBuilder();
        for (int n = 0; n < 3; n++) {
            Object s = "optional/Thing#" + n;
            out.append(failure(() -> ((Function) f).apply(s))).append(failure(() -> (int) mh.invoke(s)))
                .append(failure(() -> { ((Consumer) set).accept(s); return null; })).append(';');
        }
        return out.append('|').append(t.value).toString();
    }

    public static String typeCalls() throws Throwable {
        type.Thing t = new type.Thing();
        t.value = 42;
        Function<type.Thing, Integer> f = type.Thing::get;
        Consumer<type.Thing> set = x -> x.value = 99;
        MethodHandle mh = MethodHandles.lookup().findVirtual(type.Thing.class, "get", MethodType.methodType(int.class));
        StringBuilder out = new StringBuilder();
        for (int n = 0; n < 3; n++) {
            Object s = "type/Thing#" + n;
            out.append(failure(() -> ((Function) f).apply(s))).append(failure(() -> (int) mh.invoke(s)))
                .append(failure(() -> { ((Consumer) set).accept(s); return null; })).append(';');
        }
        return out.append('|').append(t.value).toString();
    }

    public static String componentCalls() throws Throwable {
        component.Thing t = new component.Thing();
        t.value = 42;
        Function<component.Thing, Integer> f = component.Thing::get;
        Consumer<component.Thing> set = x -> x.value = 99;
        MethodHandle mh = MethodHandles.lookup().findVirtual(component.Thing.class, "get", MethodType.methodType(int.class));
        StringBuilder out = new StringBuilder();
        for (int n = 0; n < 3; n++) {
            Object s = "component/Thing#" + n;
            out.append(failure(() -> ((Function) f).apply(s))).append(failure(() -> (int) mh.invoke(s)))
                .append(failure(() -> { ((Consumer) set).accept(s); return null; })).append(';');
        }
        return out.append('|').append(t.value).toString();
    }

    public static String uuidCalls() throws Throwable {
        uuid.Thing t = new uuid.Thing();
        t.value = 42;
        Function<uuid.Thing, Integer> f = uuid.Thing::get;
        Consumer<uuid.Thing> set = x -> x.value = 99;
        MethodHandle mh = MethodHandles.lookup().findVirtual(uuid.Thing.class, "get", MethodType.methodType(int.class));
        StringBuilder out = new StringBuilder();
        for (int n = 0; n < 3; n++) {
            Object s = "uuid/Thing#" + n;
            out.append(failure(() -> ((Function) f).apply(s))).append(failure(() -> (int) mh.invoke(s)))
                .append(failure(() -> { ((Consumer) set).accept(s); return null; })).append(';');
        }
        return out.append('|').append(t.value).toString();
    }

    public static String codecCalls() throws Throwable {
        codec.Thing t = new codec.Thing();
        t.value = 42;
        Function<codec.Thing, Integer> f = codec.Thing::get;
        Consumer<codec.Thing> set = x -> x.value = 99;
        MethodHandle mh = MethodHandles.lookup().findVirtual(codec.Thing.class, "get", MethodType.methodType(int.class));
        StringBuilder out = new StringBuilder();
        for (int n = 0; n < 3; n++) {
            Object s = "codec/Thing#" + n;
            out.append(failure(() -> ((Function) f).apply(s))).append(failure(() -> (int) mh.invoke(s)))
                .append(failure(() -> { ((Consumer) set).accept(s); return null; })).append(';');
        }
        return out.append('|').append(t.value).toString();
    }

    public static String iteratorCalls() throws Throwable {
        iterator.Thing t = new iterator.Thing();
        t.value = 42;
        Function<iterator.Thing, Integer> f = iterator.Thing::get;
        Consumer<iterator.Thing> set = x -> x.value = 99;
        MethodHandle mh = MethodHandles.lookup().findVirtual(iterator.Thing.class, "get", MethodType.methodType(int.class));
        StringBuilder out = new StringBuilder();
        for (int n = 0; n < 3; n++) {
            Object s = "iterator/Thing#" + n;
            out.append(failure(() -> ((Function) f).apply(s))).append(failure(() -> (int) mh.invoke(s)))
                .append(failure(() -> { ((Consumer) set).accept(s); return null; })).append(';');
        }
        return out.append('|').append(t.value).toString();
    }

    public static String durationCalls() throws Throwable {
        duration.Thing t = new duration.Thing();
        t.value = 42;
        Function<duration.Thing, Integer> f = duration.Thing::get;
        Consumer<duration.Thing> set = x -> x.value = 99;
        MethodHandle mh = MethodHandles.lookup().findVirtual(duration.Thing.class, "get", MethodType.methodType(int.class));
        StringBuilder out = new StringBuilder();
        for (int n = 0; n < 3; n++) {
            Object s = "duration/Thing#" + n;
            out.append(failure(() -> ((Function) f).apply(s))).append(failure(() -> (int) mh.invoke(s)))
                .append(failure(() -> { ((Consumer) set).accept(s); return null; })).append(';');
        }
        return out.append('|').append(t.value).toString();
    }

    public static String localeCalls() throws Throwable {
        locale.Thing t = new locale.Thing();
        t.value = 42;
        Function<locale.Thing, Integer> f = locale.Thing::get;
        Consumer<locale.Thing> set = x -> x.value = 99;
        MethodHandle mh = MethodHandles.lookup().findVirtual(locale.Thing.class, "get", MethodType.methodType(int.class));
        StringBuilder out = new StringBuilder();
        for (int n = 0; n < 3; n++) {
            Object s = "locale/Thing#" + n;
            out.append(failure(() -> ((Function) f).apply(s))).append(failure(() -> (int) mh.invoke(s)))
                .append(failure(() -> { ((Consumer) set).accept(s); return null; })).append(';');
        }
        return out.append('|').append(t.value).toString();
    }

    public static String list_renderCalls() throws Throwable {
        list.render.Thing t = new list.render.Thing();
        t.value = 42;
        Function<list.render.Thing, Integer> f = list.render.Thing::get;
        Consumer<list.render.Thing> set = x -> x.value = 99;
        MethodHandle mh = MethodHandles.lookup().findVirtual(list.render.Thing.class, "get", MethodType.methodType(int.class));
        StringBuilder out = new StringBuilder();
        for (int n = 0; n < 3; n++) {
            Object s = "list/render/Thing#" + n;
            out.append(failure(() -> ((Function) f).apply(s))).append(failure(() -> (int) mh.invoke(s)))
                .append(failure(() -> { ((Consumer) set).accept(s); return null; })).append(';');
        }
        return out.append('|').append(t.value).toString();
    }

    public static String set_rowsCalls() throws Throwable {
        set.rows.Thing t = new set.rows.Thing();
        t.value = 42;
        Function<set.rows.Thing, Integer> f = set.rows.Thing::get;
        Consumer<set.rows.Thing> set = x -> x.value = 99;
        MethodHandle mh = MethodHandles.lookup().findVirtual(set.rows.Thing.class, "get", MethodType.methodType(int.class));
        StringBuilder out = new StringBuilder();
        for (int n = 0; n < 3; n++) {
            Object s = "set/rows/Thing#" + n;
            out.append(failure(() -> ((Function) f).apply(s))).append(failure(() -> (int) mh.invoke(s)))
                .append(failure(() -> { ((Consumer) set).accept(s); return null; })).append(';');
        }
        return out.append('|').append(t.value).toString();
    }

    public static void main(String[] args) throws Throwable {
        System.out.println("casts=" + casts());
        System.out.println("mapExact=" + mapExact());
        System.out.println("mapCalls=" + mapCalls());
        System.out.println("listExact=" + listExact());
        System.out.println("listCalls=" + listCalls());
        System.out.println("setExact=" + setExact());
        System.out.println("setCalls=" + setCalls());
        System.out.println("streamExact=" + streamExact());
        System.out.println("streamCalls=" + streamCalls());
        System.out.println("optionalExact=" + optionalExact());
        System.out.println("optionalCalls=" + optionalCalls());
        System.out.println("typeExact=" + typeExact());
        System.out.println("typeCalls=" + typeCalls());
        System.out.println("componentExact=" + componentExact());
        System.out.println("componentCalls=" + componentCalls());
        System.out.println("uuidExact=" + uuidExact());
        System.out.println("uuidCalls=" + uuidCalls());
        System.out.println("codecExact=" + codecExact());
        System.out.println("codecCalls=" + codecCalls());
        System.out.println("iteratorExact=" + iteratorExact());
        System.out.println("iteratorCalls=" + iteratorCalls());
        System.out.println("durationExact=" + durationExact());
        System.out.println("durationCalls=" + durationCalls());
        System.out.println("localeExact=" + localeExact());
        System.out.println("localeCalls=" + localeCalls());
        System.out.println("list_renderExact=" + list_renderExact());
        System.out.println("list_renderCalls=" + list_renderCalls());
        System.out.println("set_rowsExact=" + set_rowsExact());
        System.out.println("set_rowsCalls=" + set_rowsCalls());
    }
}

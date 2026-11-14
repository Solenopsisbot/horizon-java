package demo;

/** Horizon's entry to {@link Types}' probes (the fixture runner calls `demo.Probe`). */
public final class Probe {
    public static String texts() throws Exception {
        return Types.texts();
    }

    public static String structure() throws Exception {
        return Types.structure();
    }

    public static String equality() throws Exception {
        return Types.equality();
    }

    public static String annotated() throws Exception {
        return Types.annotated();
    }

    public static String identity() throws Exception {
        return Types.identity();
    }

    public static String spelled() throws Exception {
        return Types.spelled();
    }

    public static String forged() {
        return Types.forged();
    }
}

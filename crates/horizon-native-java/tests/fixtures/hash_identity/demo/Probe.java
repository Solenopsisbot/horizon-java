package demo;

/** Horizon's entry to {@link Identity}'s probes (the fixture runner calls `demo.Probe`). */
public final class Probe {
    public static String tables() {
        return Identity.tables();
    }

    public static String probes() {
        return Identity.probes();
    }

    public static String identity() {
        return Identity.identity();
    }

    public static String separate() {
        return Identity.separate();
    }
}

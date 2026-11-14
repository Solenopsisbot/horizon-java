package demo;

/** Horizon's entry to {@link Reads}' probes (the fixture runner calls `demo.Probe`). */
public final class Probe {
    public static String sequences() {
        return Reads.sequences();
    }

    public static String maps() {
        return Reads.maps();
    }
}

package log;

public final class Probe {
  public static void mark(String text) { System.out.println("mark=" + text); }

  /** The object a callback was handed, for the test to compare. */
  public static void see(Object object) { System.out.println("see=" + System.identityHashCode(object)); }
}

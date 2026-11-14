package demo;

/**
 * Jar classes extending {@code java.lang.Thread} (#194): construction through each {@code super(..)}
 * a mod uses, {@code Thread}'s getters and setters, an overridden {@code run()} and an inherited one
 * that runs its target, and {@code start()}.
 */
public class Probe {
  static int targetRuns;

  /** A worker that overrides {@code run()}. */
  public static class Worker extends Thread {
    public int runs;

    public Worker() {}

    public Worker(String name) {
      super(name);
    }

    @Override
    public void run() {
      runs++;
    }
  }

  /** A subclass that keeps {@code Thread.run()}: it runs its target. */
  public static class Carrier extends Thread {
    public Carrier(Runnable target) {
      super(target);
    }

    public Carrier(Runnable target, String name) {
      super(target, name);
    }
  }

  /** Construction, the getters and setters, and {@code run()}, as Java 25 answers them. */
  public static String subclasses() {
    Worker named = new Worker("loader");
    StringBuilder out = new StringBuilder();
    out.append(named.getName()).append(',');
    named.setName("renamed");
    out.append(named.getName()).append(',');
    out.append(named.isDaemon());
    named.setDaemon(true);
    out.append(named.isDaemon()).append(',');
    named.setPriority(7);
    out.append(named.getPriority()).append(',');
    try {
      named.setPriority(11);
      out.append("no");
    } catch (IllegalArgumentException expected) {
      out.append("iae");
    }
    out.append(',');
    named.run();
    Thread typed = named;
    typed.run();
    out.append(named.runs).append(',');
    Carrier carrier = new Carrier(() -> targetRuns++, "carrier");
    carrier.run();
    new Carrier(() -> targetRuns += 10).run();
    out.append(targetRuns).append(',').append(carrier.getName()).append(',');
    Thread plain = new Thread(() -> targetRuns += 100, "plain");
    plain.run();
    new Thread().run();
    out.append(targetRuns).append(',').append(plain.getName()).append(',');
    out.append(named instanceof Thread).append(named instanceof Runnable).append(',');
    out.append(new Worker().isDaemon());
    try {
      named.setName(null);
      out.append(",no");
    } catch (NullPointerException expected) {
      out.append(",npe");
    }
    return out.toString();
  }

  /** {@code start()}: a second one throws; the first runs the body on a new thread. */
  public static String starts() {
    Worker worker = new Worker("watchdog");
    worker.setDaemon(true);
    worker.start();
    String twice;
    try {
      worker.start();
      twice = "no";
    } catch (IllegalThreadStateException expected) {
      twice = "itse";
    }
    new Carrier(() -> {}).start();
    return twice;
  }

  /** An unnamed thread's {@code Thread-<n>} and an unset priority: the JVM's, not known here. */
  public static String unknowns() {
    Worker unnamed = new Worker();
    return unnamed.getName() + "," + unnamed.getPriority();
  }
}

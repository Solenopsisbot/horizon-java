package demo;

/** The jar's `main` entrypoint, and a later plain call that reads the statics. */
public final class Probe {
  public void onInitialize() {
    Counter.N += 10;
    log.Probe.mark("init:" + Counter.N + ":" + Counter.BOX.n);
  }

  public static String read() {
    return Counter.N + ":" + Counter.SEEN + ":" + Counter.BOX.n;
  }
}

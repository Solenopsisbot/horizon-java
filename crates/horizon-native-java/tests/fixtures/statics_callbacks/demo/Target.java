package demo;

public final class Target {
  public static void twice() { log.Probe.mark("target:" + Counter.N); }
  public static void other() { log.Probe.mark("other:" + Counter.N); }
}

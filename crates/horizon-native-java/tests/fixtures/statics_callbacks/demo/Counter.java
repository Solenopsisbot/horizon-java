package demo;

import java.util.ArrayList;
import java.util.List;

/** Static state the callbacks share: a counter, a list and an object. */
public final class Counter {
  public static int N;
  public static final List<String> SEEN = new ArrayList<>();
  public static final Box BOX = new Box();

  static { log.Probe.mark("clinit"); }

  public static final class Box { public int n; }
}

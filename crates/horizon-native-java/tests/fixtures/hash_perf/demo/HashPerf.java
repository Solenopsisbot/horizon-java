package demo;

import java.util.*;

/** Hash collection loops for the cost of a member call at any size (#203). */
public final class HashPerf {
  /** `merge` into a map of `keys` String keys, `ops` times; answers a checksum. */
  public static int merges(int ops, int keys) {
    String[] names = new String[keys];
    for (int i = 0; i < keys; i++) names[i] = "key" + i;
    Map<String, Integer> m = new HashMap<>();
    for (int i = 0; i < ops; i++) m.merge(names[i % keys], 1, Integer::sum);
    return m.size() * 31 + m.get(names[0]);
  }

  /** `put` then `get` on a map of `keys` Integer keys, `ops` times. */
  public static int putGets(int ops, int keys) {
    Map<Integer, Integer> m = new HashMap<>();
    int s = 0;
    for (int i = 0; i < ops; i++) {
      m.put(i % keys, i);
      s += m.get(i % keys);
    }
    return s + m.size();
  }

  /** `add` and `contains` on a set of `keys` Integers, `ops` times. */
  public static int setOps(int ops, int keys) {
    Set<Integer> h = new LinkedHashSet<>();
    int s = 0;
    for (int i = 0; i < ops; i++) {
      h.add(i % keys);
      if (h.contains(i % 7)) s++;
    }
    return s + h.size();
  }
}

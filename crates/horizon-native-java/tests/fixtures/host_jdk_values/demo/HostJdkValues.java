package demo;

import java.net.URI;
import java.time.Duration;
import java.util.*;

/** UUID, Duration and URI as host objects (#156 P1a). */
public final class HostJdkValues {
  static final String U = "550e8400-e29b-41d4-a716-446655440000";
  static final String[] OLD = {"uuid/" + U, "duration/5", "uri/http://example.com/a"};

  static int identities(Object... objects) {
    Map<Object, Boolean> seen = new IdentityHashMap<>();
    for (Object o : objects) seen.put(o, true);
    return seen.size();
  }

  /** Strings spelled like the old markers are Strings, equal to no value. */
  public static String spelled() {
    Object[] real = {UUID.fromString(U), Duration.ofNanos(5), URI.create("http://example.com/a")};
    StringBuilder out = new StringBuilder();
    for (int i = 0; i < OLD.length; i++) {
      Object t = OLD[i];
      out.append(t.equals(real[i])).append(real[i].equals(t)).append("" + t).append(((String) t).length()).append(',');
    }
    return out.append(List.of(real).indexOf(OLD[0])).toString();
  }

  /** `word/rest` text is not yet told from a marker (#156 P17): counted. */
  public static String spelledTypes() {
    StringBuilder out = new StringBuilder();
    for (Object t : OLD) out.append(t instanceof UUID).append(t instanceof Duration).append(t instanceof URI).append(',');
    return out.toString();
  }

  /** Equal by value, Java's hash and text, the signed compareTo, hash and tree collections. */
  public static String uuids() {
    UUID a = UUID.fromString(U);
    UUID b = UUID.fromString(U.toUpperCase());
    UUID high = UUID.fromString("80000000-0000-0000-0000-000000000000");
    UUID low = UUID.fromString("00000000-0000-0000-0000-000000000001");
    Object o = a;
    return "" + a.equals(b) + b + "|" + a.hashCode() + "|" + high.compareTo(low) + low.compareTo(high) + a.compareTo(b)
        + "|" + (o instanceof Comparable) + (o instanceof java.io.Serializable) + o.getClass().getName() + "|"
        + new HashSet<>(List.of(a)).contains(b) + new HashMap<>(Map.of(a, 7)).get(b) + new TreeSet<>(List.of(high, low, a))
        + "|" + identities(a, a);
  }

  /** ISO-8601 text, value equality, Java's hash, `compareTo` as nanos difference, ZERO. */
  public static String durations() {
    Duration a = Duration.ofSeconds(90);
    Duration b = Duration.ofMillis(90_000);
    return a + "|" + Duration.ofMillis(1500) + "|" + Duration.ofSeconds(-1, 500_000_000) + "|" + Duration.ofDays(2)
        + "|" + a.equals(b) + a.hashCode() + "," + Duration.ofSeconds(3, 7).hashCode() + "|"
        + Duration.ofSeconds(90, 5).compareTo(Duration.ofSeconds(90, 1)) + "," + a.compareTo(Duration.ofSeconds(91)) + "|"
        + (Duration.ofSeconds(0) == Duration.ZERO) + (Duration.ofMillis(5).minus(Duration.ofMillis(5)) == Duration.ZERO)
        + "|" + new HashSet<>(List.of(a)).contains(b) + new TreeSet<>(List.of(Duration.ofSeconds(5), a, Duration.ZERO));
  }

  /** The text, value equality of one text, the accessors. */
  public static String uris() {
    URI a = URI.create("http://example.com/a?q=1");
    URI b = URI.create("http://example.com/a?q=1");
    return a + "|" + a.equals(b) + a.getScheme() + a.getHost() + "|" + new HashSet<>(List.of(a)).size()
        + "|" + (a instanceof Comparable) + a.getClass().getName();
  }

  /** Two objects made separately: identity is unknown (counted). */
  public static String identity() {
    return "" + (UUID.fromString(U) == UUID.fromString(U)) + (Duration.ofSeconds(9) == Duration.ofSeconds(9));
  }

  /** Two URI texts with equal components (counted). */
  public static String uriCase() {
    return "" + URI.create("HTTP://Example.com/a").equals(URI.create("http://example.com/a"));
  }

  /** A URI's hash and order are its components' (counted). */
  public static String uriHash() {
    return "" + URI.create("http://example.com/").hashCode() + URI.create("a:b").compareTo(URI.create("a:c"));
  }
}

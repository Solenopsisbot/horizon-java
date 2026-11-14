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

  /** `new UUID(msb, lsb)`: a JDK object whose methods read its two fields. */
  public static String constructed() {
    UUID n = new UUID(1L, 2L);
    UUID m = new UUID(1L, 2L);
    UUID p = UUID.fromString("00000000-0000-0001-0000-000000000002");
    UUID hi = new UUID(0x8000000000000000L, 1L);
    return n + "|" + n.hashCode() + "," + hi.hashCode() + "|" + n.equals(p) + p.equals(n) + n.equals(m) + n.equals("x")
        + n.equals(null) + "|" + n.compareTo(p) + "," + p.compareTo(hi) + "," + hi.compareTo(n) + "," + n.compareTo(m) + "|"
        + n.version() + n.variant() + "|" + n.getMostSignificantBits() + "," + n.getLeastSignificantBits() + "|"
        + (n == n) + (n == m) + "|" + n.getClass().getName() + (n instanceof Comparable) + "|" + String.valueOf(hi)
        + "|" + Objects.equals(p, m);
  }

  /** The review's case: an explicit `toString` of a constructed UUID. */
  public static String nuToString() {
    return new UUID(1L, 2L).toString();
  }

  /** `new URI(text)`: a JDK object no model reads (counted, never its key). */
  public static String newUri() throws Exception {
    return new URI("http://a/b").toString();
  }

  public static String newUriConcat() throws Exception {
    return "" + new URI("http://a/b");
  }

  /** A constructed UUID in hash and tree collections with a parsed one. */
  public static String constructedSets() {
    UUID n = new UUID(1L, 2L);
    UUID p = UUID.fromString("00000000-0000-0001-0000-000000000002");
    return new HashSet<>(List.of(n, p)).size() + "|" + new TreeSet<>(List.of(n, p, new UUID(0L, 5L))) + "|"
        + List.of(p).contains(n) + List.of(n).indexOf(p);
  }

  /** A parsed UUID's fields; `fromString`'s legacy form. */
  public static String uuidFields() {
    UUID p = UUID.fromString(U);
    UUID r = UUID.randomUUID();
    return p.version() + "," + p.variant() + "," + p.getMostSignificantBits() + "," + p.getLeastSignificantBits()
        + "|" + r.version() + r.variant() + "|" + UUID.fromString("1-2-3-4-5") + "|"
        + UUID.fromString("ffffffff-ffff-1fff-cfff-ffffffffffff").variant() + UUID.fromString("00000000-0000-0000-0000-000000000000").variant()
        + UUID.fromString("00000000-0000-0000-e000-000000000000").variant() + UUID.fromString("00000000-0000-0000-c000-000000000000").variant();
  }

  static String attempt(java.util.function.Supplier<Object> s) {
    try {
      return String.valueOf(s.get());
    } catch (RuntimeException e) {
      return e.getClass().getSimpleName() + ":" + e.getMessage();
    }
  }

  /** Duration's range: `long` seconds; overflow throws. Conversions as Java rounds them. */
  public static String durationEdges() {
    return attempt(() -> Duration.ofDays(Long.MAX_VALUE)) + "|"
        + attempt(() -> Duration.ofSeconds(Long.MAX_VALUE).multipliedBy(2)) + "|"
        + attempt(() -> Duration.ofSeconds(Long.MAX_VALUE).plus(Duration.ofSeconds(1))) + "|"
        + attempt(() -> Duration.ofSeconds(Long.MIN_VALUE).minus(Duration.ofNanos(1))) + "|"
        + attempt(() -> Duration.ofSeconds(Long.MIN_VALUE).negated()) + "|"
        + attempt(() -> Duration.ofSeconds(Long.MIN_VALUE).abs()) + "|"
        + attempt(() -> Duration.ofSeconds(Long.MAX_VALUE).toNanos()) + "|"
        + attempt(() -> Duration.ofSeconds(Long.MAX_VALUE).toMillis()) + "|"
        + attempt(() -> Duration.ofSeconds(1).dividedBy(0)) + "|"
        + Duration.ofMillis(-500).toSeconds() + "," + Duration.ofMillis(-500).toMillis() + ","
        + Duration.ofSeconds(-59, -500_000_000).toMinutes() + "," + Duration.ofSeconds(-60, -500_000_000).toMinutes() + ","
        + Duration.ofNanos(-1).toNanos() + "," + Duration.ofSeconds(-3600, -1).toHours() + "," + Duration.ofSeconds(-86400, -1).toDays() + ","
        + Duration.ofNanos(-1_500_000).toMillis() + "," + Duration.ofSeconds(Long.MIN_VALUE).getSeconds() + "|"
        + Duration.ofSeconds(7).dividedBy(-2) + "," + Duration.ofNanos(-7).dividedBy(2) + "|"
        + attempt(() -> Duration.ofSeconds(Long.MIN_VALUE).dividedBy(-1));
  }

  /** A URI's components (RFC 2396 as java.net.URI parses them). */
  public static String uriParts() {
    URI a = URI.create("http://user@Example.com:8080/a/b?q=1#f");
    URI m = URI.create("mailto:a@b.com");
    URI rel = URI.create("a/b");
    URI bare = URI.create("http://h");
    return a.getScheme() + "," + a.getHost() + "," + a.getAuthority() + "," + a.getPath() + "," + a.getQuery() + ","
        + a.getFragment() + "," + a.getUserInfo() + "," + a.getPort() + "|"
        + m.getScheme() + "," + m.getHost() + "," + m.getPath() + "," + m.getAuthority() + "," + m.isOpaque() + m.isAbsolute() + "|"
        + rel.getScheme() + "," + rel.getHost() + "," + rel.getPath() + "," + rel.isAbsolute() + "|"
        + bare.getPath().isEmpty() + "," + bare.getHost() + "," + bare.getPort();
  }

  /** Text java.net.URI refuses, decoded and ASCII forms (counted). */
  public static String uriBad() {
    return attempt(() -> URI.create("not a uri"));
  }

  public static String uriEscaped() {
    return URI.create("http://h/a%20b").getPath() + URI.create("http://h/\u00e9").toASCIIString();
  }

  public static String uriOddHost() {
    return "" + URI.create("http://a_b/x").getHost();
  }
}

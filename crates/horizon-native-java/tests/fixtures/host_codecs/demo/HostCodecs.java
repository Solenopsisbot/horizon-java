package demo;

import com.mojang.datafixers.kinds.App;
import com.mojang.datafixers.util.Pair;
import com.mojang.serialization.Codec;
import net.minecraft.network.codec.ByteBufCodecs;
import java.util.*;

/** DataFixerUpper's Pair and the game's codecs as host objects (#156 P5c). */
public final class HostCodecs {
  static final String[] OLD = {
    "dfu-pair/i:1,a",
    "nf-codec/0",
    "nf-codec/compound-tag",
    "com/mojang/serialization/Codec#codec",
    "codec/opaque",
  };

  static int identities(Object... objects) {
    Map<Object, Boolean> seen = new IdentityHashMap<>();
    for (Object o : objects) seen.put(o, true);
    return seen.size();
  }

  /** Strings spelled like the old markers are Strings, equal to no pair. */
  public static String spelled() {
    Object p = Pair.of(1, "a");
    StringBuilder out = new StringBuilder();
    for (Object t : OLD) {
      out.append(t.equals(p)).append(p.equals(t)).append("" + t).append(((String) t).length()).append(',');
    }
    return out.append(List.of(OLD).indexOf(OLD[0])).append(List.of(p).indexOf(OLD[0])).toString();
  }

  /** `word/rest` text is not yet told from a marker (#156 P17): these checks are counted. */
  public static String spelledTypes() {
    StringBuilder out = new StringBuilder();
    for (Object t : OLD) {
      out.append(t instanceof String).append(t instanceof Pair).append(t instanceof Codec).append(',');
    }
    return out.toString();
  }

  /** A pair hands back its values; equal by them, a new object each time; text and hash. */
  public static String pairs() {
    Pair<Integer, String> p = Pair.of(1, "a");
    Pair<Integer, String> q = Pair.of(1, "a");
    Pair<String, Integer> s = p.swap();
    Object o = p;
    return "" + p.getFirst() + p.getSecond() + s.getFirst() + s.getSecond() + "|" + (p == q) + p.equals(q)
        + p.equals(s) + s.swap().equals(p) + p.equals(Pair.of(1, "b")) + p.equals(null) + identities(p, p, q) + "|"
        + p + "|" + Pair.of(null, null) + "|" + p.hashCode() + "," + Pair.of(null, null).hashCode() + "|"
        + (o instanceof Pair) + (o instanceof App) + (o instanceof Integer) + o.getClass().getName()
        + o.getClass().getSuperclass().getName() + "|" + new HashSet<>(List.of(p)).contains(q)
        + new HashMap<>(Map.of(p, 7)).get(q) + List.of(s, q).indexOf(p);
  }

  /** Nested pairs compare and print through their values. */
  public static String nested() {
    Pair<Pair<Integer, String>, List<Integer>> n = Pair.of(Pair.of(2, "b"), List.of(3, 4));
    return "" + n + "|" + n.hashCode() + "|" + n.equals(Pair.of(Pair.of(2, "b"), List.of(3, 4)));
  }

  /** A pair of an identity-hashed object: its hash and text are counted. */
  public static String identityHeld() {
    return "" + Pair.of(new Object(), 1).hashCode();
  }

  /** A library's codec constants are one object each (#199 F2). */
  public static String codecConstants() {
    Object s = Codec.STRING;
    Set<Object> set = new HashSet<>();
    return "" + (s == Codec.STRING) + (s == Codec.INT) + (ByteBufCodecs.INT == ByteBufCodecs.INT)
        + (ByteBufCodecs.INT == ByteBufCodecs.VAR_INT) + s.equals(Codec.STRING) + s.equals(Codec.INT)
        + identities(s, Codec.STRING, Codec.INT) + List.of(s).contains(Codec.STRING)
        + new ArrayList<>(List.of(Codec.INT, s)).indexOf(Codec.STRING) + set.add(Codec.BOOL) + set.add(Codec.BOOL)
        + set.size() + set.contains(Codec.BOOL);
  }

  /** `Set.of` refuses one constant twice, printing it (DataFixerUpper's `toString`). */
  public static String codecDuplicate() {
    try {
      return "" + Set.of(Codec.STRING, Codec.STRING).size();
    } catch (IllegalArgumentException x) {
      return x.getMessage();
    }
  }

  public static String codecText() {
    return Codec.STRING + "|" + Codec.INT + "|" + Codec.BOOL;
  }

  /** A pair is never equal to an object that is not a pair (#199 F4). */
  public static String pairVsJdk() {
    Pair<Integer, String> p = Pair.of(1, "a");
    return "" + p.equals(new Object()) + p.equals(new ArrayList<>()) + p.equals(new AbstractMap.SimpleEntry<>(1, "a"))
        + p.equals(Map.entry(1, "a")) + p.equals(List.of(1, "a"));
  }

  /** An unmodelled codec constant: counted (Java: `truetrue`). */
  public static String opaqueCodec() {
    Object c = Codec.STRING;
    return "" + (c == Codec.STRING) + (c instanceof Codec);
  }
}

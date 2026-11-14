package demo;

import com.mojang.brigadier.Message;
import java.util.*;
import net.minecraft.ChatFormatting;
import net.minecraft.network.chat.Component;
import net.minecraft.network.chat.FormattedText;
import net.minecraft.network.chat.MutableComponent;

/**
 * Chat components as host objects (#156 P5a): made by the jar through the real API in both runs
 * (Minecraft 26.3's server jar on the oracle, the runtime's model here).
 */
public final class HostComponents {
  /** The old marker text, and the handle's key and text forms. */
  static final String[] OLD = {
    "component/literal/hi",
    "component/translatable/demo.key",
    "component/json/{\"text\":\"hi\"}",
  };
  static final String[] ENCODED = {"{\"text\":\"hi\"}", "hcomp:{\"text\":\"hi\"}"};

  static int identities(Object... objects) {
    Map<Object, Boolean> seen = new IdentityHashMap<>();
    for (Object o : objects) seen.put(o, true);
    return seen.size();
  }

  /** Strings spelled like the old markers or the handle's text forms equal no component. */
  public static String spelled() {
    Object c = Component.literal("hi");
    StringBuilder out = new StringBuilder();
    List<String> all = new ArrayList<>(List.of(OLD));
    all.addAll(List.of(ENCODED));
    for (Object t : all) {
      out.append(t.equals(c)).append(c.equals(t)).append(t.equals(t.toString())).append("" + t)
          .append(((String) t).length()).append(',');
    }
    return out.append(all.indexOf(OLD[0])).append(List.of(c).indexOf(OLD[0])).toString();
  }

  /** The handle's text forms are plain text: whole String behaviour, and no component. */
  public static String encoded() {
    StringBuilder out = new StringBuilder();
    Map<Object, Integer> m = new HashMap<>();
    for (Object t : ENCODED) {
      out.append(t instanceof String).append(t.getClass().getName()).append(t.hashCode())
          .append(t instanceof Component).append(t instanceof MutableComponent).append(',');
      m.put(t, t.toString().length());
    }
    return out.append(m.get(ENCODED[0])).toString();
  }

  /** `word/rest` text is not yet told from a marker (#156 P17): these checks are counted. */
  public static String spelledTypes() {
    StringBuilder out = new StringBuilder();
    for (Object t : OLD) {
      out.append(t instanceof Component).append(t instanceof MutableComponent).append(',');
    }
    return out.toString();
  }

  /** Class and supertypes, the Class surface, identity, equality between bare components. */
  public static String real() {
    MutableComponent a = Component.literal("hi");
    MutableComponent b = Component.literal("hi");
    MutableComponent k = Component.translatable("demo.key");
    Object o = a;
    return "" + (o instanceof MutableComponent) + (o instanceof Component) + (o instanceof Message)
        + (o instanceof FormattedText) + (o instanceof Integer) + (o instanceof HostComponents) + "|"
        + o.getClass().getName() + "|" + o.getClass().getSuperclass().getName() + o.getClass().isInterface()
        + "|" + o.getClass().getSimpleName() + "|" + (a == b) + a.equals(b) + b.equals(a) + a.equals(a)
        + a.equals(Component.literal("ho")) + a.equals(k) + k.equals(Component.translatable("demo.key"))
        + k.equals(Component.translatable("other")) + identities(a, a, b) + "|" + List.of(a).indexOf(b)
        + List.of(a, k).indexOf(Component.translatable("demo.key")) + List.of(a).contains(k) + "|"
        + Component.class.isAssignableFrom(o.getClass()) + MutableComponent.class.isInstance(k)
        + Message.class.isInstance(o);
  }

  /** `toString` and `getString` of the bare forms. */
  public static String text() {
    MutableComponent a = Component.literal("hi");
    MutableComponent k = Component.translatable("demo.key");
    return a + "|" + k + "|" + a.getString() + "|" + k.getString() + "|" + String.valueOf((Object) a);
  }

  /** `copy` is a new object with the same contents. */
  public static String copy() {
    MutableComponent a = Component.literal("hi");
    MutableComponent c = a.copy();
    return "" + (c == a) + c.equals(a) + a.equals(c) + identities(a, c) + c;
  }

  /** `append` adds a sibling to the receiver and returns it: one object, its text the whole. */
  public static String appended() {
    MutableComponent a = Component.literal("hi");
    MutableComponent b = a.append("!").append(Component.literal("?"));
    return "" + (a == b) + a.getString() + identities(a, b);
  }

  /** `withStyle`/`withColor` change the receiver's style and return it. */
  public static String styled() {
    MutableComponent a = Component.literal("hi");
    return "" + (a.withStyle(ChatFormatting.RED) == a) + (a.withStyle(ChatFormatting.BOLD, ChatFormatting.ITALIC) == a)
        + (a.withColor(0x123456) == a) + a.getString();
  }

  /** A sibling changed after it was appended shows in its parent; `copy` shares its siblings. */
  public static String appendInPlace() {
    MutableComponent a = Component.literal("hi");
    MutableComponent b = a.append("!");
    MutableComponent s = Component.literal("s");
    a.append(s);
    s.append("?");
    MutableComponent c = a.copy();
    a.append("x");
    s.append("~");
    return "" + (a == b) + a.getString() + identities(a, b) + "|" + c.getString() + (c == a) + "|"
        + Component.translatable("k", s).getString();
  }

  /** A changed component no longer equals what it was made as; a copy equals its source. */
  public static String changedEquals() {
    MutableComponent a = Component.literal("hi");
    MutableComponent before = Component.literal("hi");
    boolean was = a.equals(before);
    a.append("!");
    MutableComponent c = a.copy();
    return "" + was + a.equals(before) + c.equals(a) + List.of(before).contains(a);
  }

  /** Two styled components: equal contents and style. */
  public static String styledEquals() {
    MutableComponent a = Component.literal("hi").withStyle(ChatFormatting.RED);
    return "" + a.equals(Component.literal("hi")) + a.equals(Component.literal("hi").withStyle(ChatFormatting.RED));
  }

  /** A key changed after it went into a hash collection keeps the hash it had then (#193 B1). */
  public static String hashSetMutatedKey() {
    Set<Component> s = new HashSet<>();
    MutableComponent c = Component.literal("c");
    s.add(c);
    c.append("x");
    return "" + s.contains(c) + s.contains(Component.literal("c")) + s.size() + s.remove(c) + s.size();
  }

  public static String hashMapMutatedKey() {
    Map<Component, Integer> m = new HashMap<>();
    MutableComponent c = Component.literal("c");
    m.put(c, 1);
    c.append("x");
    return "" + m.get(c) + m.containsKey(c) + m.size() + m.get(Component.literal("c"));
  }

  public static String hashSetBecomesEqual() {
    Set<Component> s = new HashSet<>();
    MutableComponent x = Component.literal("x");
    MutableComponent y = Component.literal("x").append("1");
    s.add(x);
    s.add(y);
    x.append("1");
    return "" + s.size() + s.contains(x) + s.contains(y);
  }

  public static String hashSetEqualDistinct() {
    Set<Component> s = new HashSet<>();
    s.add(Component.literal("a"));
    s.add(Component.literal("a"));
    s.add(Component.literal("a").append("b"));
    s.add(Component.literal("a").append("b"));
    return "" + s.size();
  }

  /** Strings spelled like a handle's key are Strings (#193 B2). */
  public static String forgedKey() {
    Object s = "net/minecraft/network/chat/MutableComponent#0:{\"text\":\"x\"}";
    Object h = "hcomp:net/minecraft/network/chat/MutableComponent#0:{\"text\":\"x\"}";
    MutableComponent real = Component.literal("x");
    return "" + (s instanceof Component) + (h instanceof Component) + real.equals(s) + s.equals(real)
        + real.append((String) s).getString().length();
  }

  public static String forgedNoColon() {
    Object s = "net/minecraft/network/chat/MutableComponent#0";
    Object t = "net/minecraft/network/chat/MutableComponent#0:x";
    return "" + (s instanceof Component) + (t instanceof Component);
  }

  /** `PlainTextContents.EMPTY` prints `empty`. */
  public static String emptyText() {
    return Component.literal("").toString() + "|" + Component.empty().toString() + "|"
        + Component.empty().getString().length();
  }

  /** `a.append(a)` succeeds; reading `a` then overflows the stack. */
  public static String selfAppendIdentity() {
    MutableComponent a = Component.literal("a");
    return "" + (a.append(a) == a);
  }

  public static String selfAppendRead() {
    MutableComponent a = Component.literal("a");
    a.append(a);
    try {
      return a.getString();
    } catch (StackOverflowError e) {
      return "SOE";
    }
  }

  /** Children grown after they were appended: the parent's rendered text (#193 B3). */
  public static String chainBound() {
    MutableComponent c = Component.literal("c");
    MutableComponent[] s = new MutableComponent[200];
    for (int i = 0; i < 200; i++) {
      s[i] = Component.literal("s");
      c.append(s[i]);
    }
    String b = "y".repeat(60000);
    for (int i = 0; i < 200; i++) s[i].append(b);
    return "" + c.getString().length();
  }

  public static String fanout() {
    MutableComponent leaf = Component.literal("x");
    MutableComponent top = Component.literal("");
    for (int i = 0; i < 4000; i++) top.append(leaf);
    leaf.append("y".repeat(40000));
    return "" + top.getString().length();
  }

  /** A translation's text is the server language's, with its arguments. */
  public static String knownKey() {
    return Component.translatable("gui.done").getString();
  }

  public static String translatableArgLive() {
    MutableComponent x = Component.literal("x");
    MutableComponent t = Component.translatable("rv.unknown.key %s", x);
    x.append("!");
    return t.getString();
  }

  /** A type switch over the class and its interfaces: a Mixin could add an interface. */
  public static String typeSwitch() {
    Object o = Component.literal("hi");
    return switch (o) {
      case Runnable r -> "R";
      case MutableComponent m -> "M";
      default -> "d";
    };
  }

  /** Hash-based collections need `hashCode` (contents, style, siblings): counted. */
  public static String hashed() {
    Set<Object> set = new HashSet<>(List.of(Component.literal("hi")));
    return "" + set.contains(Component.literal("hi")) + Set.of(Component.literal("a")).size();
  }

  /** Inside host collections: a list prints its elements; an immutable map looks up by hash. */
  public static String listed() {
    return List.of(Component.literal("hi"), Component.translatable("demo.key")).toString();
  }

  public static String mapped() {
    return "" + Map.of(Component.literal("hi"), 1).get(Component.literal("hi"));
  }

  /** `Set.of` refuses two equal components (their hashes are equal too): `Set12`, then `SetN`. */
  public static String duplicate() {
    try {
      return "" + Set.of(Component.literal("a"), Component.literal("a")).size();
    } catch (IllegalArgumentException x) {
      return x.getMessage();
    }
  }

  public static String duplicateN() {
    try {
      return "" + Set.of(Component.literal("a"), Component.translatable("b"), Component.literal("a")).size();
    } catch (IllegalArgumentException x) {
      return x.getMessage();
    }
  }

  public static String hash() {
    return "" + Component.literal("hi").hashCode();
  }

  /** No natural order. */
  public static String sorted() {
    try {
      new TreeSet<Object>().add(Component.literal("hi"));
      return "added";
    } catch (ClassCastException x) {
      return "CCE";
    }
  }
}

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

  /** What `append`/`withStyle` answer: their result's text and equality, counted (departure D1). */
  public static String appended() {
    MutableComponent a = Component.literal("hi").append("!").append(Component.literal("?"));
    return a.getString() + "," + a.equals(Component.literal("hi!?"));
  }

  public static String styled() {
    MutableComponent a = Component.literal("hi").withStyle(ChatFormatting.RED);
    return a.getString() + "," + a.equals(Component.literal("hi"));
  }

  /** `append` changes the receiver and returns it (departure D1: counted). */
  public static String appendInPlace() {
    MutableComponent a = Component.literal("hi");
    MutableComponent b = a.append("!");
    return "" + (a == b) + a.getString() + identities(a, b);
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

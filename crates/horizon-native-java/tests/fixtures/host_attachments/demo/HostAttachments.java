package demo;

import java.util.*;
import net.minecraft.network.RegistryFriendlyByteBuf;
import net.neoforged.neoforge.attachment.AttachmentSyncHandler;
import net.neoforged.neoforge.attachment.AttachmentType;
import net.neoforged.neoforge.attachment.IAttachmentHolder;

/**
 * NeoForge's AttachmentType and its builder as host objects (#156 P8c): built by the jar through
 * the real API in both runs (NeoForge 26.3.0.101 on the oracle, the runtime's model here).
 */
public final class HostAttachments {
  static final String[] OLD = {
    "nf-attachment/net/neoforged/neoforge/attachment/AttachmentType/0",
    "nf-attachment/net/neoforged/neoforge/attachment/AttachmentType$Builder/0",
  };
  static final String[] ENCODED = {"hnatt:0", "hnattb:0"};

  /** A sync handler: the builder only records it. */
  static final class Sync implements AttachmentSyncHandler<Integer> {
    public void write(RegistryFriendlyByteBuf buf, Integer value, boolean initial) {}

    public Integer read(IAttachmentHolder holder, RegistryFriendlyByteBuf buf, Integer previous) {
      return previous;
    }
  }

  static int identities(Object... objects) {
    Map<Object, Boolean> seen = new IdentityHashMap<>();
    for (Object o : objects) seen.put(o, true);
    return seen.size();
  }

  /** Strings spelled like the old markers or the handles' text forms are Strings, equal to no type. */
  public static String spelled() {
    Object type = AttachmentType.builder(() -> 0).build();
    StringBuilder out = new StringBuilder();
    List<String> all = new ArrayList<>(List.of(OLD));
    all.addAll(List.of(ENCODED));
    for (Object t : all) {
      out.append(t.equals(type)).append(type.equals(t)).append(t.equals(t.toString())).append("" + t)
          .append(((String) t).length()).append(',');
    }
    return out.append(all.indexOf(OLD[0])).toString();
  }

  /** The handles' text forms are plain text: whole String behaviour, and no attachment type. */
  public static String encoded() {
    StringBuilder out = new StringBuilder();
    Map<Object, Integer> m = new HashMap<>();
    for (Object t : ENCODED) {
      out.append(t instanceof String).append(t.getClass().getName()).append(t.hashCode())
          .append(t instanceof AttachmentType).append(t instanceof AttachmentType.Builder).append(',');
      m.put(t, t.toString().length());
    }
    return out.append(m.get("hnatt:0")).toString();
  }

  /** `word/rest` text waits for #156 P17's is_plain_string: these checks are counted. */
  public static String spelledTypes() {
    StringBuilder out = new StringBuilder();
    for (Object t : OLD) {
      out.append(t instanceof AttachmentType).append(t instanceof AttachmentType.Builder).append(',');
    }
    return out.toString();
  }

  /** Two types and a builder: class, supertypes, identity, the generic equality paths. */
  public static String real() {
    AttachmentType.Builder<Integer> b = AttachmentType.builder(() -> 0);
    AttachmentType<Integer> t = b.build();
    AttachmentType<Integer> u = AttachmentType.builder(() -> 0).build();
    Object o = t;
    Object ob = b;
    return "" + (o instanceof AttachmentType) + (ob instanceof AttachmentType.Builder)
        + (o instanceof AttachmentType.Builder) + (ob instanceof AttachmentType) + (o instanceof Integer)
        + (o instanceof HostAttachments) + "|" + o.getClass().getName() + "|" + ob.getClass().getName() + "|"
        + o.getClass().getSuperclass().getName() + ob.getClass().getSuperclass().getName() + o.getClass().isInterface()
        + "|" + o.getClass().getSimpleName() + "|" + (t == u) + t.equals(u) + t.equals(t) + identities(t, t, u)
        + "|" + new HashSet<>(List.of(t)).contains(u) + new HashSet<>(List.of(t)).contains(t) + List.of(t).indexOf(t)
        + List.of(t, u).indexOf(u) + "|" + AttachmentType.class.isAssignableFrom(o.getClass())
        + AttachmentType.class.isInstance(ob);
  }

  /** A builder's methods answer the builder itself. */
  public static String chain() {
    AttachmentType.Builder<Integer> b = AttachmentType.builder(() -> 0);
    AttachmentType.Builder<Integer> c = b.sync(new Sync());
    return "" + (b == c) + c.equals(b) + identities(b, c) + (c.build() != b.build());
  }

  /** A type switch: an interface a Mixin could add needs the class; the type itself is exact. */
  public static String typeSwitch() {
    Object o = AttachmentType.builder(() -> 0).build();
    return switch (o) {
      case Runnable r -> "R";
      case AttachmentType<?> a -> "A";
      default -> "d";
    };
  }

  /** Object's text and hash (an identity hash); a nested class's simple name; no natural order. */
  public static String text() {
    Object o = AttachmentType.builder(() -> 0).build();
    return o + "," + o.hashCode();
  }

  public static String builderSimpleName() {
    return AttachmentType.builder(() -> 0).getClass().getSimpleName();
  }

  public static String interfaces() {
    return "" + AttachmentType.builder(() -> 0).build().getClass().getInterfaces().length;
  }

  public static String sorted() {
    try {
      new TreeSet<Object>().add(AttachmentType.builder(() -> 0).build());
      return "added";
    } catch (ClassCastException x) {
      return "CCE";
    }
  }
}

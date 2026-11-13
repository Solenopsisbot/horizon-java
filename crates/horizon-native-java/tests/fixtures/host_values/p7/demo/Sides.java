package demo;

import java.util.*;
import org.spongepowered.asm.mixin.MixinEnvironment.Side;

/** MixinEnvironment.Side constants: enum constants with bodies, compared by ordinal. */
public final class Sides {
  public static String sides() {
    Side c = Side.CLIENT;
    Side s = Side.SERVER;
    Object o = c;
    Map<Object, Integer> identity = new IdentityHashMap<>();
    identity.put(c, 1);
    identity.put(Side.CLIENT, 2);
    return c.name() + c.ordinal() + c + "|" + c.compareTo(s) + "," + s.compareTo(Side.UNKNOWN) + "," + c.compareTo(Side.CLIENT)
        + "|" + (c == Side.CLIENT) + c.equals(Side.CLIENT) + c.equals(s) + (o instanceof Enum) + (o instanceof Comparable)
        + (o instanceof Side) + (o instanceof String) + "|" + new TreeSet<>(List.of(s, Side.UNKNOWN, c))
        + List.of(c, s).contains(Side.SERVER) + List.of(c).indexOf(s) + identity.size() + "|" + String.valueOf(o) + ("" + s);
  }
}

package demo;

import net.neoforged.fml.config.ConfigTracker;

/** A jar's own copy of a loader class (#124): one {@code Class}, the copy's. */
public final class Copy {
  public static String copy() throws Exception {
    Class<?> literal = ConfigTracker.class;
    return ""
        + (literal == ConfigTracker.INSTANCE.getClass())
        + (literal == Class.forName("net.neoforged.fml.config.ConfigTracker"))
        + (literal.getSuperclass() == Object.class)
        + ","
        + literal.getName()
        + ","
        + literal.getMethod("count").getName()
        + ","
        + literal.getDeclaredFields().length
        + ","
        + literal;
  }
}

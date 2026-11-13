package demo;

import net.neoforged.bus.api.Event;

/** The Class surface of a bundled copy of an owned class name answers from the jar's class file. */
public final class Bundled {
  public static String superOf() {
    return Event.class.getSuperclass().getName();
  }

  public static String walk() {
    StringBuilder out = new StringBuilder();
    for (Class<?> c = Sub.class; c != null; c = c.getSuperclass()) out.append(c.getName()).append('>');
    return out.toString();
  }

  public static String method() {
    try {
      return Event.class.getMethod("mine").getName();
    } catch (NoSuchMethodException x) {
      return "NSME";
    }
  }

  public static String assignable() {
    return "" + Marker.class.isAssignableFrom(Sub.class) + Base.class.isAssignableFrom(Event.class)
        + Event.class.isInterface() + (new Sub() instanceof Marker);
  }

  public static void main(String[] args) {
    System.out.println("superOf=" + superOf());
    System.out.println("walk=" + walk());
    System.out.println("method=" + method());
    System.out.println("assignable=" + assignable());
  }
}

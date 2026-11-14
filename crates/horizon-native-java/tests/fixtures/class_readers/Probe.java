package demo;

import java.lang.annotation.Retention;
import java.lang.annotation.RetentionPolicy;
import java.lang.invoke.MethodHandle;
import java.lang.invoke.MethodHandles;
import java.lang.invoke.MethodType;
import java.lang.reflect.Array;
import java.lang.reflect.Method;

/**
 * The readers of a {@code Class} value (#156 P13a): every call below takes a class and answers
 * from its name. P13a routes them through one accessor; P13b makes the classes handles, and these
 * answers must not change.
 */
public class Probe {
  @Retention(RetentionPolicy.RUNTIME)
  public @interface Tag {}

  public enum Color { RED, GREEN }

  @Tag
  public static class Tagged {
    public static String twice(String text) {
      return text + text;
    }

    @Tag
    public void marked() {}
  }

  /** The {@code Class} members that read the name. */
  public static String members() {
    return String.class.getName() + "," + Tagged.class.getSimpleName() + ","
        + Tagged.class.getPackageName() + "," + int[].class.getName() + ","
        + int.class.isPrimitive() + String.class.isPrimitive() + "," + int[].class.isArray()
        + String.class.isArray() + "," + Object.class.isAssignableFrom(String.class);
  }

  /** APIs that take a {@code Class}. */
  public static String arguments() throws Throwable {
    Object array = Array.newInstance(String.class, 2);
    Method marked = Tagged.class.getMethod("marked");
    MethodType type = MethodType.methodType(String.class, String.class);
    MethodHandle twice = MethodHandles.lookup().findStatic(Tagged.class, "twice", type);
    return Enum.valueOf(Color.class, "GREEN") + "," + Array.getLength(array) + ","
        + Tagged.class.isAnnotationPresent(Tag.class) + marked.isAnnotationPresent(Tag.class) + ","
        + (Tagged.class.getAnnotation(Tag.class) != null) + "," + type + ","
        + (String) twice.invokeExact("ab") + "," + type.changeReturnType(int.class);
  }
}

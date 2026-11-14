package demo;

import java.io.Serializable;
import java.lang.annotation.Retention;
import java.lang.annotation.RetentionPolicy;
import java.lang.invoke.MethodType;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.HashSet;
import java.util.IdentityHashMap;
import java.util.List;
import java.util.Map;

/**
 * #215's review: every {@code Class} a jar can obtain is one object (annotation values, {@code
 * annotationType()}, {@code MethodType}, {@code StackWalker}), so hash tables and lists find it by
 * an {@code X.class} literal; and the members arrays and primitives answer as Java 25 does.
 */
public class Review {
  @Retention(RetentionPolicy.RUNTIME)
  @interface Ann {
    Class<?> value();

    Class<?>[] more() default {int.class, String[].class};
  }

  @Ann(String.class)
  static class Holder {}

  public static class Foo {}

  public interface Shape {}

  public static class Bar extends Foo implements Shape {}

  static Class<?> annValue() {
    return Holder.class.getAnnotation(Ann.class).value();
  }

  static Class<?> annType() {
    return Holder.class.getAnnotation(Ann.class).annotationType();
  }

  static Class<?> mtReturn() {
    return MethodType.methodType(String.class, int.class).returnType();
  }

  static Class<?> mtParameter() {
    return MethodType.methodType(String.class, int.class).parameterType(0);
  }

  static Class<?> caller() {
    return StackWalker.getInstance(StackWalker.Option.RETAIN_CLASS_REFERENCE).getCallerClass();
  }

  static Map<Class<?>, String> keyed() {
    HashMap<Class<?>, String> map = new HashMap<>();
    map.put(String.class, "s");
    map.put(Ann.class, "a");
    map.put(int.class, "i");
    map.put(Review.class, "r");
    return map;
  }

  /** The review's lookups: a Class from each producer finds the literal's entry. */
  public static String lookups() {
    Map<Class<?>, String> byAnn = new HashMap<>();
    byAnn.put(annValue(), "v");
    Map<Class<?>, String> both = new HashMap<>();
    both.put(annValue(), "x");
    both.put(String.class, "y");
    HashSet<Class<?>> set = new HashSet<>();
    set.add(String.class);
    set.add(mtReturn());
    Map<Class<?>, Integer> merged = new HashMap<>();
    merged.merge(String.class, 1, Integer::sum);
    merged.merge(annValue(), 1, Integer::sum);
    IdentityHashMap<Class<?>, String> identity = new IdentityHashMap<>();
    identity.put(String.class, "s");
    Class<?>[] more = Holder.class.getAnnotation(Ann.class).more();
    return keyed().get(annValue())
        + keyed().get(annType())
        + keyed().get(mtReturn())
        + keyed().get(mtParameter())
        + keyed().get(caller())
        + keyed().get(more[0])
        + ","
        + keyed().containsKey(annValue())
        + new HashSet<>(List.of(String.class)).contains(annValue())
        + List.of(String.class).contains(annValue())
        + ","
        + new ArrayList<>(List.of(String.class)).indexOf(mtReturn())
        + byAnn.get(String.class)
        + both.size()
        + set.size()
        + merged.values()
        + identity.get(annValue())
        + ","
        + (annValue() == String.class)
        + (annType() == Ann.class)
        + (mtReturn() == String.class)
        + (more[0] == int.class)
        + (more[1] == String[].class)
        + (caller() == Review.class);
  }

  /**
   * #215's re-check: a plain subclass has no Signature attribute, so its
   * generic superclass and interfaces are its raw classes, the literals'
   * own objects (a map keyed by {@code Foo.class} finds them).
   */
  public static String genericSuper() {
    Map<Object, String> keyed = new HashMap<>();
    keyed.put(Foo.class, "f");
    keyed.put(Shape.class, "s");
    java.lang.reflect.Type sup = Bar.class.getGenericSuperclass();
    java.lang.reflect.Type itf = Bar.class.getGenericInterfaces()[0];
    return keyed.get(sup) + keyed.get(itf) + "," + (sup == Foo.class) + (itf == Shape.class);
  }

  static String thrown(Runnable run) {
    try {
      run.run();
      return "none";
    } catch (RuntimeException error) {
      return error.getClass().getSimpleName() + ":" + error.getMessage();
    }
  }

  static String forName(String name) {
    try {
      return String.valueOf(Class.forName(name));
    } catch (ClassNotFoundException error) {
      return "CNFE:" + error.getMessage();
    }
  }

  /** Arrays, primitives and interfaces: Java 25's answers. */
  public static String members() throws Exception {
    Class<?>[] arrayInterfaces = int[].class.getInterfaces();
    Object anonymous = new Object() {};
    return ""
        + (Shape.class.getSuperclass() == null)
        + (Ann.class.getSuperclass() == null)
        + (arrayInterfaces.length == 2
            && arrayInterfaces[0] == Cloneable.class
            && arrayInterfaces[1] == Serializable.class)
        + ","
        + Object[].class.isAssignableFrom(String[].class)
        + Serializable.class.isAssignableFrom(Foo[].class)
        + Shape[].class.isAssignableFrom(Bar[].class)
        + Object[].class.isAssignableFrom(int[][].class)
        + Object.class.isAssignableFrom(int.class)
        + Object[].class.isAssignableFrom(int[].class)
        + int[].class.isAssignableFrom(long[].class)
        + Bar[].class.isAssignableFrom(Foo[].class)
        + ","
        + thrown(() -> int.class.cast(1))
        + "|"
        + int.class.cast(null)
        + ","
        + Foo.class.getCanonicalName()
        + "|"
        + int[].class.getCanonicalName()
        + "|"
        + Foo[][].class.getCanonicalName()
        + "|"
        + anonymous.getClass().getCanonicalName()
        + "|"
        + Map.Entry.class.getCanonicalName()
        + "|"
        + Map.Entry.class.getSimpleName()
        + ","
        + (Class.forName("[I") == int[].class)
        + (Class.forName("[Ljava.lang.String;") == String[].class)
        + (Class.forName("[[Ldemo.Review$Foo;") == Foo[][].class)
        + "|"
        + forName("java/lang/String")
        + "|"
        + forName("[V")
        + "|"
        + forName("[Ljava.lang.String");
  }
}

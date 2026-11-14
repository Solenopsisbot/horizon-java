package demo;

import java.lang.reflect.Method;
import java.lang.reflect.Type;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.HashSet;
import java.util.IdentityHashMap;
import java.util.List;
import java.util.Map;
import java.util.Objects;
import java.util.Set;

/**
 * A {@code String} spelled like a class is that {@code String}, never the {@code Class} (#156
 * P13c): every check below counts the texts that behave as the {@code String} they are. On a JVM
 * that is all of them.
 */
public class Probe {
  public static class Foo {}

  interface Check {
    Object run() throws Exception;
  }

  static boolean throwsCce(Check check) {
    try {
      check.run();
      return false;
    } catch (ClassCastException expected) {
      return true;
    } catch (Exception other) {
      return false;
    }
  }

  static int javaHash(String text) {
    int hash = 0;
    for (int i = 0; i < text.length(); i++) {
      hash = 31 * hash + text.charAt(i);
    }
    return hash;
  }

  static int label(Object value) {
    return switch (value) {
      case Class<?> klass -> 1;
      case String text -> 2;
      default -> 3;
    };
  }

  static int typeLabel(Object value) {
    return switch (value) {
      case Type type -> 1;
      default -> 2;
    };
  }

  /** The names each class has: internal, binary, a primitive's, an array's both ways. */
  static final String[] SPELLINGS = {
    "java/lang/String",
    "java.lang.String",
    "demo/Probe$Foo",
    "demo.Probe$Foo",
    "java/util/List",
    "int",
    "void",
    "[I",
    "[Ljava/lang/String;",
    "[Ljava.lang.String;",
    "class java.lang.String",
    "java/lang/Class",
  };

  /** The classes those spellings name. */
  static final Class<?>[] CLASSES = {
    String.class,
    String.class,
    Foo.class,
    Foo.class,
    List.class,
    int.class,
    void.class,
    int[].class,
    String[].class,
    String[].class,
    String.class,
    Class.class,
  };

  public static String forged() throws Exception {
    String[] checks = {
      "instanceof", "cast", "isInstance", "getClass", "equals", "hashCode", "text", "collections",
      "identity", "switch", "reflection", "charSequence",
    };
    int[] strings = new int[checks.length];
    Method length = String.class.getMethod("length");
    Method getName = Class.class.getMethod("getName");
    for (int k = 0; k < SPELLINGS.length; k++) {
      String spelled = SPELLINGS[k];
      Class<?> klass = CLASSES[k];
      // A fresh String with that text, typed `Object` as a forger would pass it.
      Object s = new StringBuilder(spelled).toString();
      Set<Object> set = new HashSet<>(List.of(klass, Foo.class));
      Map<Object, String> map = new HashMap<>();
      map.put(klass, "class");
      Map<Object, String> identity = new IdentityHashMap<>();
      identity.put(klass, "class");
      List<Object> list = new ArrayList<>(List.of(klass));
      boolean[] ok = {
        s instanceof String && !(s instanceof Class) && !(s instanceof Type),
        throwsCce(() -> (Class<?>) s) && throwsCce(() -> (Type) s),
        !Class.class.isInstance(s) && !Type.class.isInstance(s) && String.class.isInstance(s),
        s.getClass() == String.class && s.getClass().getName().equals("java.lang.String"),
        !s.equals(klass)
            && !klass.equals(s)
            && !Objects.equals(s, klass)
            && !Objects.equals(klass, s)
            && s.equals(spelled),
        s.hashCode() == javaHash(spelled) && Objects.hashCode(s) == javaHash(spelled),
        String.valueOf(s).equals(spelled)
            && ("" + s).equals(spelled)
            && new StringBuilder().append(s).toString().equals(spelled)
            && String.format("%s", s).equals(spelled),
        !set.contains(s)
            && map.get(s) == null
            && !map.containsKey(s)
            && identity.get(s) == null
            && !list.contains(s)
            && list.indexOf(s) == -1
            && !List.of(klass).contains(s)
            && !Set.of(klass).contains(s)
            && Map.of(klass, "c").get(s) == null,
        s != (Object) klass,
        label(s) == 2 && typeLabel(s) == 2,
        (int) length.invoke(s) == spelled.length() && throwsIae(() -> getName.invoke(s)),
        ((CharSequence) s).length() == spelled.length(),
      };
      for (int i = 0; i < ok.length; i++) {
        if (ok[i]) {
          strings[i]++;
        }
      }
    }
    StringBuilder out = new StringBuilder();
    for (int i = 0; i < checks.length; i++) {
      out.append(i == 0 ? "" : ",").append(checks[i]).append('=').append(strings[i]);
    }
    return out.toString();
  }

  static boolean throwsIae(Check check) {
    try {
      check.run();
      return false;
    } catch (IllegalArgumentException expected) {
      return true;
    } catch (Exception other) {
      return false;
    }
  }

  /** {@code Class.forName} resolves a binary name: the one way text names a class. */
  public static String forName() throws Exception {
    return ""
        + (Class.forName("java.lang.String") == String.class)
        + (Class.forName("demo.Probe$Foo") == Foo.class)
        + (Class.forName("[I") == int[].class)
        + (Class.forName(new StringBuilder("java.").append("util.List").toString()) == List.class);
  }
}

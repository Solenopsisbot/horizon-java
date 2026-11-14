package demo;

import com.google.common.reflect.TypeToken;
import com.google.gson.FormattingStyle;
import com.google.gson.GsonBuilder;
import java.lang.reflect.ParameterizedType;
import java.lang.reflect.Type;
import java.util.*;

/**
 * Gson's FormattingStyle and Guava's TypeToken as host objects (#156 P5b): made through the real
 * API in both runs (gson 2.14.0 and guava 33.6.0 on the oracle, the runtime's model here).
 */
public final class HostLibraryValues {
  static final String[] OLD = {
    "formatting-style/0a/2020/1",
    "gson-style/newline=0a;indent=2020;space=1",
    "typetoken/java/lang/String",
  };

  static int identities(Object... objects) {
    Map<Object, Boolean> seen = new IdentityHashMap<>();
    for (Object o : objects) seen.put(o, true);
    return seen.size();
  }

  /** Strings spelled like the old markers are Strings, equal to no style or token. */
  public static String spelled() {
    Object style = FormattingStyle.PRETTY;
    Object token = TypeToken.of(String.class);
    StringBuilder out = new StringBuilder();
    for (Object t : OLD) {
      out.append(t.equals(style)).append(style.equals(t)).append(t.equals(token)).append(token.equals(t))
          .append("" + t).append(((String) t).length()).append(',');
    }
    return out.append(List.of(OLD).indexOf(OLD[2])).append(List.of(token).indexOf(OLD[2])).toString();
  }

  /** `word/rest` text is not yet told from a marker (#156 P17): these checks are counted. */
  public static String spelledTypes() {
    StringBuilder out = new StringBuilder();
    for (Object t : OLD) {
      out.append(t instanceof String).append(t.hashCode()).append(t instanceof FormattingStyle)
          .append(t instanceof TypeToken).append(',');
    }
    return out.toString();
  }

  /** The constants are one object each; every `with*` makes a new one; getters and output. */
  public static String styles() {
    FormattingStyle p = FormattingStyle.PRETTY;
    FormattingStyle four = p.withIndent("    ");
    FormattingStyle same = p.withIndent("  ");
    Object o = four;
    return "" + (p == FormattingStyle.PRETTY) + p.equals(FormattingStyle.PRETTY) + (same == p) + same.equals(p)
        + (four == p.withIndent("    ")) + identities(p, FormattingStyle.PRETTY, same, four) + "|"
        + four.getIndent().length() + four.getNewline().length() + four.usesSpaceAfterSeparators()
        + FormattingStyle.COMPACT.getIndent().isEmpty() + "|" + (o instanceof FormattingStyle) + (o instanceof Integer)
        + o.getClass().getName() + o.getClass().getSuperclass().getName() + "|"
        + List.of(p, four).indexOf(four) + new HashSet<>(List.of(p)).contains(same) + "|"
        + new GsonBuilder().setFormattingStyle(four).create().toJson(List.of(1, 2));
  }

  /** `TypeToken.of`: a new token each time, equal by type; class, getters, text. */
  public static String tokens() {
    TypeToken<String> a = TypeToken.of(String.class);
    TypeToken<String> b = TypeToken.of(String.class);
    TypeToken<Integer> i = TypeToken.of(Integer.class);
    Object o = a;
    return "" + (a == b) + a.equals(b) + a.equals(i) + a.equals("java.lang.String") + identities(a, a, b) + "|"
        + (a.getType() == String.class) + a.getRawType().getName() + "|" + a + "|" + String.valueOf(i) + "|"
        + (o instanceof TypeToken) + (o instanceof Integer) + o.getClass().getName() + "|"
        + o.getClass().getSuperclass().getName() + "|" + List.of(a, i).indexOf(TypeToken.of(Integer.class))
        + new HashSet<>(List.of(a)).contains(b);
  }

  /** Identity hashes and `Object.toString` text: counted. */
  public static String styleText() {
    return FormattingStyle.PRETTY + "," + FormattingStyle.PRETTY.hashCode();
  }

  public static String tokenHash() {
    return "" + TypeToken.of(String.class).hashCode();
  }

  /** An interface a library class could gain is counted; Java's case is `F`. */
  public static String typeSwitch() {
    Object o = FormattingStyle.COMPACT;
    return switch (o) {
      case Runnable r -> "R";
      case FormattingStyle f -> "F";
      default -> "d";
    };
  }

  // Gson's TypeToken and its canonical types (#156 P5b).
  static class Outer<T> {
    class Inner<U> {}
  }

  static List<String> ls;
  static Map<String, List<Integer>> nested;
  static Outer<String>.Inner<Integer> member;

  static Type field(String name) {
    try {
      return HostLibraryValues.class.getDeclaredField(name).getGenericType();
    } catch (NoSuchFieldException x) {
      throw new IllegalStateException(x);
    }
  }

  /** A subclass token: Gson's own canonical copy of the JDK node, equal both ways, not `==`. */
  public static String gsonSubclass() {
    com.google.gson.reflect.TypeToken<List<String>> t = new com.google.gson.reflect.TypeToken<List<String>>() {};
    Type ty = t.getType();
    Type jdk = field("ls");
    return ty + "|" + (ty == t.getType()) + ty.equals(jdk) + jdk.equals(ty) + (ty == jdk) + "|"
        + ty.getClass().getName() + "|" + (ty instanceof ParameterizedType) + (((ParameterizedType) ty).getRawType() == List.class)
        + "|" + t.getRawType().getName() + "|" + t + "|"
        + t.equals(new com.google.gson.reflect.TypeToken<List<String>>() {})
        + t.equals(com.google.gson.reflect.TypeToken.get(String.class)) + t.equals("x");
  }

  /** `get`: a class stays itself, an array class becomes Gson's GenericArrayTypeImpl. */
  public static String gsonGet() {
    com.google.gson.reflect.TypeToken<String> s = com.google.gson.reflect.TypeToken.get(String.class);
    com.google.gson.reflect.TypeToken<String[]> a = com.google.gson.reflect.TypeToken.get(String[].class);
    return "" + (s.getType() == String.class) + (s.getRawType() == String.class) + s + "|" + a.getType().getClass().getName()
        + "|" + a + "|" + a.getRawType().getName() + "|" + com.google.gson.reflect.TypeToken.get(int[].class) + "|"
        + com.google.gson.reflect.TypeToken.getArray(String.class).equals(a) + a.equals(com.google.gson.reflect.TypeToken.get(String[].class))
        + com.google.gson.reflect.TypeToken.get(field("ls")).equals(new com.google.gson.reflect.TypeToken<List<String>>() {});
  }

  /** Nested arguments are Gson objects too, the same one on every read. */
  public static String gsonNested() {
    Type ty = new com.google.gson.reflect.TypeToken<Map<String, List<Integer>>>() {}.getType();
    ParameterizedType p = (ParameterizedType) ty;
    Type arg = p.getActualTypeArguments()[1];
    Type jdkArg = ((ParameterizedType) field("nested")).getActualTypeArguments()[1];
    return ty + "|" + arg.getClass().getName() + "|" + (arg == p.getActualTypeArguments()[1]) + arg.equals(jdkArg)
        + jdkArg.equals(arg) + "|" + (p.getActualTypeArguments()[0] == String.class) + p.getOwnerType() + "|"
        + ty.equals(field("nested"));
  }

  /** Wildcards print as Gson prints them. */
  public static String gsonWildcards() {
    return new com.google.gson.reflect.TypeToken<List<? extends Number>>() {}.getType() + "|"
        + new com.google.gson.reflect.TypeToken<List<?>>() {}.getType() + "|"
        + new com.google.gson.reflect.TypeToken<List<? super Integer>>() {}.getType();
  }

  /** A member type prints without its owner's arguments (the JDK prints them). */
  public static String gsonMember() {
    Type ty = new com.google.gson.reflect.TypeToken<Outer<String>.Inner<Integer>>() {}.getType();
    return ty + "|" + field("member") + "|" + ty.equals(field("member")) + field("member").equals(ty);
  }

  /** Hashes are classes' identity hashes: counted. */
  public static String gsonHash() {
    return "" + new com.google.gson.reflect.TypeToken<List<String>>() {}.hashCode();
  }
}

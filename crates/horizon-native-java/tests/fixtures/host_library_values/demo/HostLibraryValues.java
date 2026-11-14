package demo;

import com.google.common.reflect.TypeToken;
import com.google.gson.FormattingStyle;
import com.google.gson.GsonBuilder;
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
    Map<Object, Integer> m = new HashMap<>();
    for (Object t : OLD) {
      out.append(t.equals(style)).append(style.equals(t)).append(t.equals(token)).append(token.equals(t))
          .append(t instanceof String).append(t.hashCode()).append("" + t).append(',');
      m.put(t, ((String) t).length());
    }
    return out.append(m.get(OLD[2])).toString();
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
}

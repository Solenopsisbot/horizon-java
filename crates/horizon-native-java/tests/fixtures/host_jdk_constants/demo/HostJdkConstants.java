package demo;

import java.nio.charset.CodingErrorAction;
import java.util.*;
import java.util.concurrent.TimeUnit;
import java.util.regex.Pattern;

/** JDK constants and small values as host objects (#156 P1b). */
public final class HostJdkConstants {
  static final String[] OLD = {
    "time-unit/SECONDS", "coding-error-action/REPORT", "unicode-block/BASIC_LATIN", "locale/US", "regex-pattern/a+"
  };

  static Object[] real() {
    return new Object[] {TimeUnit.SECONDS, CodingErrorAction.REPORT, Character.UnicodeBlock.BASIC_LATIN, Locale.US,
      Pattern.compile("a+")};
  }

  /** Strings spelled like the old markers are Strings: equal to no value, printed as text. */
  public static String spelled() {
    Object[] real = real();
    StringBuilder out = new StringBuilder();
    for (int i = 0; i < OLD.length; i++) {
      Object t = OLD[i];
      out.append(t.equals(real[i])).append(real[i].equals(t)).append("" + t).append(((String) t).length())
          .append(',');
    }
    return out.append(List.of(real).indexOf(OLD[0])).toString();
  }

  /** The constants: one object each, their text, an enum's order. */
  public static String constants() {
    Object unit = TimeUnit.SECONDS;
    return "" + (TimeUnit.SECONDS == TimeUnit.SECONDS) + TimeUnit.SECONDS + TimeUnit.SECONDS.compareTo(TimeUnit.DAYS)
        + TimeUnit.HOURS.compareTo(TimeUnit.SECONDS) + TimeUnit.SECONDS.equals(TimeUnit.MINUTES) + TimeUnit.SECONDS.toMillis(5)
        + "|" + (CodingErrorAction.REPORT == CodingErrorAction.REPORT) + CodingErrorAction.REPORT
        + CodingErrorAction.IGNORE.equals(CodingErrorAction.REPORT) + "|"
        + (Character.UnicodeBlock.of('a') == Character.UnicodeBlock.BASIC_LATIN) + Character.UnicodeBlock.of('a')
        + "|" + (unit instanceof TimeUnit) + unit.getClass().getName() + "|"
        + new HashSet<>(List.of(TimeUnit.SECONDS, TimeUnit.SECONDS, TimeUnit.DAYS)).size()
        + List.of(TimeUnit.DAYS, TimeUnit.SECONDS).indexOf(TimeUnit.SECONDS);
  }

  /** Locale constants: text, tag, BaseLocale's hash, equality. */
  public static String locales() {
    return Locale.US + "|" + Locale.US.toLanguageTag() + "|" + Locale.US.hashCode() + "," + Locale.ENGLISH.hashCode()
        + "," + Locale.ROOT.hashCode() + "|" + Locale.ROOT + "|" + Locale.ENGLISH + "|" + Locale.US.equals(Locale.US)
        + Locale.US.equals(Locale.ENGLISH) + Locale.ROOT.equals(Locale.US) + "|" + Locale.US.getCountry()
        + Locale.ENGLISH.getLanguage() + "|" + new HashSet<>(List.of(Locale.US)).contains(Locale.US)
        + Locale.US.getClass().getName();
  }

  /** A compiled pattern: its text, one object per compile. */
  public static String patterns() {
    Pattern p = Pattern.compile("a+");
    Pattern q = Pattern.compile("a+");
    return p.pattern() + "|" + p + "|" + (p == p) + (p == q) + p.equals(q) + p.equals(p) + "|" + p.getClass().getName()
        + "|" + new HashSet<>(List.of(p)).contains(q);
  }

  /** Java 25's locale cache: `Locale.of`/`forLanguageTag` answer the constants' objects. */
  public static String localeIdentity() {
    return "" + (Locale.US == Locale.US) + (Locale.ROOT == Locale.ROOT) + (Locale.of("en", "US") == Locale.US)
        + (Locale.of("EN", "us") == Locale.US) + (Locale.of("en") == Locale.ENGLISH) + (Locale.of("") == Locale.ROOT)
        + (Locale.forLanguageTag("en-US") == Locale.US) + (Locale.forLanguageTag("und") == Locale.ROOT)
        + (Locale.forLanguageTag("") == Locale.ROOT) + (Locale.forLanguageTag("zh-cn") == Locale.CHINA)
        + (Locale.CHINA == Locale.PRC) + (Locale.UK == Locale.US) + (Locale.of("fr", "CA") == Locale.CANADA_FRENCH)
        + "|" + Locale.of("iw") + Locale.of("ji") + Locale.of("in") + "|" + Locale.forLanguageTag("fr-ca") + "|"
        + Locale.TAIWAN + Locale.GERMANY.hashCode() + "|" + "TITLE".toLowerCase(Locale.FRANCE)
        + "|" + Locale.of("en", "001").toLanguageTag();
  }

  /** Two `Locale.of` of a key no constant holds (counted: the cache holds it softly). */
  public static String cachedIdentity() {
    return "" + (Locale.of("fr", "BE") == Locale.of("fr", "BE"));
  }

  /** Java tailors Turkish case mapping (counted). */
  public static String turkish() {
    return "TITLE".toLowerCase(Locale.of("tr", "TR"));
  }

  /** An identity hash (counted). */
  public static String unitHash() {
    return "" + TimeUnit.SECONDS.hashCode();
  }
}

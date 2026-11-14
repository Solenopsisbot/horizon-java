package demo;

import java.util.*;

/** JDK objects a jar builds with `new`: Object's members as each class (re)defines them (#217). */
public final class JdkObjects {
  public static String object_equals() {
    return "" + new Object().equals(new Object());
  }

  public static String object_same() {
    Object a = new Object();
    return "" + a.equals(a) + (a == a);
  }

  public static String object_other() {
    return "" + new Object().equals("x") + new Object().equals(null);
  }

  public static String object_hash() {
    return "" + (new Object().hashCode() == new Object().hashCode());
  }

  public static String object_text() {
    return new Object().toString().replaceAll("@[0-9a-f]+$", "@");
  }

  public static String object_concat() {
    return ("" + new Object()).replaceAll("@[0-9a-f]+$", "@");
  }

  public static String object_contains() {
    return "" + List.of(new Object()).contains(new Object());
  }

  public static String object_objects() {
    return "" + Objects.equals(new Object(), new Object());
  }

  public static String random_equals() {
    return "" + new java.util.Random(1).equals(new java.util.Random(1));
  }

  public static String random_same() {
    java.util.Random a = new java.util.Random(1);
    return "" + a.equals(a) + (a == a);
  }

  public static String random_other() {
    return "" + new java.util.Random(1).equals("x") + new java.util.Random(1).equals(null);
  }

  public static String random_hash() {
    return "" + (new java.util.Random(1).hashCode() == new java.util.Random(1).hashCode());
  }

  public static String random_text() {
    return new java.util.Random(1).toString().replaceAll("@[0-9a-f]+$", "@");
  }

  public static String random_concat() {
    return ("" + new java.util.Random(1)).replaceAll("@[0-9a-f]+$", "@");
  }

  public static String random_contains() {
    return "" + List.of(new java.util.Random(1)).contains(new java.util.Random(1));
  }

  public static String random_objects() {
    return "" + Objects.equals(new java.util.Random(1), new java.util.Random(1));
  }

  public static String crc_equals() {
    return "" + new java.util.zip.CRC32().equals(new java.util.zip.CRC32());
  }

  public static String crc_same() {
    java.util.zip.CRC32 a = new java.util.zip.CRC32();
    return "" + a.equals(a) + (a == a);
  }

  public static String crc_other() {
    return "" + new java.util.zip.CRC32().equals("x") + new java.util.zip.CRC32().equals(null);
  }

  public static String crc_hash() {
    return "" + (new java.util.zip.CRC32().hashCode() == new java.util.zip.CRC32().hashCode());
  }

  public static String crc_text() {
    return new java.util.zip.CRC32().toString().replaceAll("@[0-9a-f]+$", "@");
  }

  public static String crc_concat() {
    return ("" + new java.util.zip.CRC32()).replaceAll("@[0-9a-f]+$", "@");
  }

  public static String crc_contains() {
    return "" + List.of(new java.util.zip.CRC32()).contains(new java.util.zip.CRC32());
  }

  public static String crc_objects() {
    return "" + Objects.equals(new java.util.zip.CRC32(), new java.util.zip.CRC32());
  }

  public static String adler_equals() {
    return "" + new java.util.zip.Adler32().equals(new java.util.zip.Adler32());
  }

  public static String adler_same() {
    java.util.zip.Adler32 a = new java.util.zip.Adler32();
    return "" + a.equals(a) + (a == a);
  }

  public static String adler_other() {
    return "" + new java.util.zip.Adler32().equals("x") + new java.util.zip.Adler32().equals(null);
  }

  public static String adler_hash() {
    return "" + (new java.util.zip.Adler32().hashCode() == new java.util.zip.Adler32().hashCode());
  }

  public static String adler_text() {
    return new java.util.zip.Adler32().toString().replaceAll("@[0-9a-f]+$", "@");
  }

  public static String adler_concat() {
    return ("" + new java.util.zip.Adler32()).replaceAll("@[0-9a-f]+$", "@");
  }

  public static String adler_contains() {
    return "" + List.of(new java.util.zip.Adler32()).contains(new java.util.zip.Adler32());
  }

  public static String adler_objects() {
    return "" + Objects.equals(new java.util.zip.Adler32(), new java.util.zip.Adler32());
  }

  public static String builder_equals() {
    return "" + new StringBuilder("ab").equals(new StringBuilder("ab"));
  }

  public static String builder_same() {
    StringBuilder a = new StringBuilder("ab");
    return "" + a.equals(a) + (a == a);
  }

  public static String builder_other() {
    return "" + new StringBuilder("ab").equals("x") + new StringBuilder("ab").equals(null);
  }

  public static String builder_hash() {
    return "" + (new StringBuilder("ab").hashCode() == new StringBuilder("ab").hashCode());
  }

  public static String builder_text() {
    return new StringBuilder("ab").toString().replaceAll("@[0-9a-f]+$", "@");
  }

  public static String builder_concat() {
    return ("" + new StringBuilder("ab")).replaceAll("@[0-9a-f]+$", "@");
  }

  public static String builder_contains() {
    return "" + List.of(new StringBuilder("ab")).contains(new StringBuilder("ab"));
  }

  public static String builder_objects() {
    return "" + Objects.equals(new StringBuilder("ab"), new StringBuilder("ab"));
  }

  public static String builder_compare() {
    return "" + Integer.signum(new StringBuilder("ab").compareTo(new StringBuilder("ab")));
  }

  public static String atomic_equals() {
    return "" + new java.util.concurrent.atomic.AtomicInteger(5).equals(new java.util.concurrent.atomic.AtomicInteger(5));
  }

  public static String atomic_same() {
    java.util.concurrent.atomic.AtomicInteger a = new java.util.concurrent.atomic.AtomicInteger(5);
    return "" + a.equals(a) + (a == a);
  }

  public static String atomic_other() {
    return "" + new java.util.concurrent.atomic.AtomicInteger(5).equals("x") + new java.util.concurrent.atomic.AtomicInteger(5).equals(null);
  }

  public static String atomic_hash() {
    return "" + (new java.util.concurrent.atomic.AtomicInteger(5).hashCode() == new java.util.concurrent.atomic.AtomicInteger(5).hashCode());
  }

  public static String atomic_text() {
    return new java.util.concurrent.atomic.AtomicInteger(5).toString().replaceAll("@[0-9a-f]+$", "@");
  }

  public static String atomic_concat() {
    return ("" + new java.util.concurrent.atomic.AtomicInteger(5)).replaceAll("@[0-9a-f]+$", "@");
  }

  public static String atomic_contains() {
    return "" + List.of(new java.util.concurrent.atomic.AtomicInteger(5)).contains(new java.util.concurrent.atomic.AtomicInteger(5));
  }

  public static String atomic_objects() {
    return "" + Objects.equals(new java.util.concurrent.atomic.AtomicInteger(5), new java.util.concurrent.atomic.AtomicInteger(5));
  }

  public static String lock_equals() {
    return "" + new java.util.concurrent.locks.ReentrantLock().equals(new java.util.concurrent.locks.ReentrantLock());
  }

  public static String lock_same() {
    java.util.concurrent.locks.ReentrantLock a = new java.util.concurrent.locks.ReentrantLock();
    return "" + a.equals(a) + (a == a);
  }

  public static String lock_other() {
    return "" + new java.util.concurrent.locks.ReentrantLock().equals("x") + new java.util.concurrent.locks.ReentrantLock().equals(null);
  }

  public static String lock_hash() {
    return "" + (new java.util.concurrent.locks.ReentrantLock().hashCode() == new java.util.concurrent.locks.ReentrantLock().hashCode());
  }

  public static String lock_text() {
    return new java.util.concurrent.locks.ReentrantLock().toString().replaceAll("@[0-9a-f]+$", "@");
  }

  public static String lock_concat() {
    return ("" + new java.util.concurrent.locks.ReentrantLock()).replaceAll("@[0-9a-f]+$", "@");
  }

  public static String lock_contains() {
    return "" + List.of(new java.util.concurrent.locks.ReentrantLock()).contains(new java.util.concurrent.locks.ReentrantLock());
  }

  public static String lock_objects() {
    return "" + Objects.equals(new java.util.concurrent.locks.ReentrantLock(), new java.util.concurrent.locks.ReentrantLock());
  }

  public static String joiner_equals() {
    return "" + new java.util.StringJoiner(",").equals(new java.util.StringJoiner(","));
  }

  public static String joiner_same() {
    java.util.StringJoiner a = new java.util.StringJoiner(",");
    return "" + a.equals(a) + (a == a);
  }

  public static String joiner_other() {
    return "" + new java.util.StringJoiner(",").equals("x") + new java.util.StringJoiner(",").equals(null);
  }

  public static String joiner_hash() {
    return "" + (new java.util.StringJoiner(",").hashCode() == new java.util.StringJoiner(",").hashCode());
  }

  public static String joiner_text() {
    return new java.util.StringJoiner(",").toString().replaceAll("@[0-9a-f]+$", "@");
  }

  public static String joiner_concat() {
    return ("" + new java.util.StringJoiner(",")).replaceAll("@[0-9a-f]+$", "@");
  }

  public static String joiner_contains() {
    return "" + List.of(new java.util.StringJoiner(",")).contains(new java.util.StringJoiner(","));
  }

  public static String joiner_objects() {
    return "" + Objects.equals(new java.util.StringJoiner(","), new java.util.StringJoiner(","));
  }

  public static String zipEntry_equals() {
    return "" + new java.util.zip.ZipEntry("a").equals(new java.util.zip.ZipEntry("a"));
  }

  public static String zipEntry_same() {
    java.util.zip.ZipEntry a = new java.util.zip.ZipEntry("a");
    return "" + a.equals(a) + (a == a);
  }

  public static String zipEntry_other() {
    return "" + new java.util.zip.ZipEntry("a").equals("x") + new java.util.zip.ZipEntry("a").equals(null);
  }

  public static String zipEntry_hash() {
    return "" + (new java.util.zip.ZipEntry("a").hashCode() == new java.util.zip.ZipEntry("a").hashCode());
  }

  public static String zipEntry_text() {
    return new java.util.zip.ZipEntry("a").toString().replaceAll("@[0-9a-f]+$", "@");
  }

  public static String zipEntry_concat() {
    return ("" + new java.util.zip.ZipEntry("a")).replaceAll("@[0-9a-f]+$", "@");
  }

  public static String zipEntry_contains() {
    return "" + List.of(new java.util.zip.ZipEntry("a")).contains(new java.util.zip.ZipEntry("a"));
  }

  public static String zipEntry_objects() {
    return "" + Objects.equals(new java.util.zip.ZipEntry("a"), new java.util.zip.ZipEntry("a"));
  }

  public static String decimal_equals() {
    return "" + new java.math.BigDecimal("1.0").equals(new java.math.BigDecimal("1.0"));
  }

  public static String decimal_same() {
    java.math.BigDecimal a = new java.math.BigDecimal("1.0");
    return "" + a.equals(a) + (a == a);
  }

  public static String decimal_other() {
    return "" + new java.math.BigDecimal("1.0").equals("x") + new java.math.BigDecimal("1.0").equals(null);
  }

  public static String decimal_hash() {
    return "" + (new java.math.BigDecimal("1.0").hashCode() == new java.math.BigDecimal("1.0").hashCode());
  }

  public static String decimal_text() {
    return new java.math.BigDecimal("1.0").toString().replaceAll("@[0-9a-f]+$", "@");
  }

  public static String decimal_concat() {
    return ("" + new java.math.BigDecimal("1.0")).replaceAll("@[0-9a-f]+$", "@");
  }

  public static String decimal_contains() {
    return "" + List.of(new java.math.BigDecimal("1.0")).contains(new java.math.BigDecimal("1.0"));
  }

  public static String decimal_objects() {
    return "" + Objects.equals(new java.math.BigDecimal("1.0"), new java.math.BigDecimal("1.0"));
  }

  public static String decimal_compare() {
    return "" + Integer.signum(new java.math.BigDecimal("1.0").compareTo(new java.math.BigDecimal("1.0")));
  }

  public static String integer_equals() {
    return "" + new java.math.BigInteger("12").equals(new java.math.BigInteger("12"));
  }

  public static String integer_same() {
    java.math.BigInteger a = new java.math.BigInteger("12");
    return "" + a.equals(a) + (a == a);
  }

  public static String integer_other() {
    return "" + new java.math.BigInteger("12").equals("x") + new java.math.BigInteger("12").equals(null);
  }

  public static String integer_hash() {
    return "" + (new java.math.BigInteger("12").hashCode() == new java.math.BigInteger("12").hashCode());
  }

  public static String integer_text() {
    return new java.math.BigInteger("12").toString().replaceAll("@[0-9a-f]+$", "@");
  }

  public static String integer_concat() {
    return ("" + new java.math.BigInteger("12")).replaceAll("@[0-9a-f]+$", "@");
  }

  public static String integer_contains() {
    return "" + List.of(new java.math.BigInteger("12")).contains(new java.math.BigInteger("12"));
  }

  public static String integer_objects() {
    return "" + Objects.equals(new java.math.BigInteger("12"), new java.math.BigInteger("12"));
  }

  public static String integer_compare() {
    return "" + Integer.signum(new java.math.BigInteger("12").compareTo(new java.math.BigInteger("12")));
  }

  public static String date_equals() {
    return "" + new java.util.Date(0L).equals(new java.util.Date(0L));
  }

  public static String date_same() {
    java.util.Date a = new java.util.Date(0L);
    return "" + a.equals(a) + (a == a);
  }

  public static String date_other() {
    return "" + new java.util.Date(0L).equals("x") + new java.util.Date(0L).equals(null);
  }

  public static String date_hash() {
    return "" + (new java.util.Date(0L).hashCode() == new java.util.Date(0L).hashCode());
  }

  public static String date_text() {
    return new java.util.Date(0L).toString().replaceAll("@[0-9a-f]+$", "@");
  }

  public static String date_concat() {
    return ("" + new java.util.Date(0L)).replaceAll("@[0-9a-f]+$", "@");
  }

  public static String date_contains() {
    return "" + List.of(new java.util.Date(0L)).contains(new java.util.Date(0L));
  }

  public static String date_objects() {
    return "" + Objects.equals(new java.util.Date(0L), new java.util.Date(0L));
  }

  public static String date_compare() {
    return "" + Integer.signum(new java.util.Date(0L).compareTo(new java.util.Date(0L)));
  }

  public static String bits_equals() {
    return "" + new java.util.BitSet().equals(new java.util.BitSet());
  }

  public static String bits_same() {
    java.util.BitSet a = new java.util.BitSet();
    return "" + a.equals(a) + (a == a);
  }

  public static String bits_other() {
    return "" + new java.util.BitSet().equals("x") + new java.util.BitSet().equals(null);
  }

  public static String bits_hash() {
    return "" + (new java.util.BitSet().hashCode() == new java.util.BitSet().hashCode());
  }

  public static String bits_text() {
    return new java.util.BitSet().toString().replaceAll("@[0-9a-f]+$", "@");
  }

  public static String bits_concat() {
    return ("" + new java.util.BitSet()).replaceAll("@[0-9a-f]+$", "@");
  }

  public static String bits_contains() {
    return "" + List.of(new java.util.BitSet()).contains(new java.util.BitSet());
  }

  public static String bits_objects() {
    return "" + Objects.equals(new java.util.BitSet(), new java.util.BitSet());
  }

  public static String format_equals() {
    return "" + new java.text.DecimalFormat("0.0").equals(new java.text.DecimalFormat("0.0"));
  }

  public static String format_same() {
    java.text.DecimalFormat a = new java.text.DecimalFormat("0.0");
    return "" + a.equals(a) + (a == a);
  }

  public static String format_other() {
    return "" + new java.text.DecimalFormat("0.0").equals("x") + new java.text.DecimalFormat("0.0").equals(null);
  }

  public static String format_hash() {
    return "" + (new java.text.DecimalFormat("0.0").hashCode() == new java.text.DecimalFormat("0.0").hashCode());
  }

  public static String format_text() {
    return new java.text.DecimalFormat("0.0").toString().replaceAll("@[0-9a-f]+$", "@");
  }

  public static String format_concat() {
    return ("" + new java.text.DecimalFormat("0.0")).replaceAll("@[0-9a-f]+$", "@");
  }

  public static String format_contains() {
    return "" + List.of(new java.text.DecimalFormat("0.0")).contains(new java.text.DecimalFormat("0.0"));
  }

  public static String format_objects() {
    return "" + Objects.equals(new java.text.DecimalFormat("0.0"), new java.text.DecimalFormat("0.0"));
  }

  public static String address_equals() {
    return "" + java.net.InetSocketAddress.createUnresolved("h", 1).equals(java.net.InetSocketAddress.createUnresolved("h", 1));
  }

  public static String address_same() {
    java.net.InetSocketAddress a = java.net.InetSocketAddress.createUnresolved("h", 1);
    return "" + a.equals(a) + (a == a);
  }

  public static String address_other() {
    return "" + java.net.InetSocketAddress.createUnresolved("h", 1).equals("x") + java.net.InetSocketAddress.createUnresolved("h", 1).equals(null);
  }

  public static String address_hash() {
    return "" + (java.net.InetSocketAddress.createUnresolved("h", 1).hashCode() == java.net.InetSocketAddress.createUnresolved("h", 1).hashCode());
  }

  public static String address_text() {
    return java.net.InetSocketAddress.createUnresolved("h", 1).toString().replaceAll("@[0-9a-f]+$", "@");
  }

  public static String address_concat() {
    return ("" + java.net.InetSocketAddress.createUnresolved("h", 1)).replaceAll("@[0-9a-f]+$", "@");
  }

  public static String address_contains() {
    return "" + List.of(java.net.InetSocketAddress.createUnresolved("h", 1)).contains(java.net.InetSocketAddress.createUnresolved("h", 1));
  }

  public static String address_objects() {
    return "" + Objects.equals(java.net.InetSocketAddress.createUnresolved("h", 1), java.net.InetSocketAddress.createUnresolved("h", 1));
  }

  public static String entry_equals() {
    return "" + new java.util.AbstractMap.SimpleEntry<>("k", "v").equals(new java.util.AbstractMap.SimpleEntry<>("k", "v"));
  }

  public static String entry_same() {
    java.util.AbstractMap.SimpleEntry<String, String> a = new java.util.AbstractMap.SimpleEntry<>("k", "v");
    return "" + a.equals(a) + (a == a);
  }

  public static String entry_other() {
    return "" + new java.util.AbstractMap.SimpleEntry<>("k", "v").equals("x") + new java.util.AbstractMap.SimpleEntry<>("k", "v").equals(null);
  }

  public static String entry_hash() {
    return "" + (new java.util.AbstractMap.SimpleEntry<>("k", "v").hashCode() == new java.util.AbstractMap.SimpleEntry<>("k", "v").hashCode());
  }

  public static String entry_text() {
    return new java.util.AbstractMap.SimpleEntry<>("k", "v").toString().replaceAll("@[0-9a-f]+$", "@");
  }

  public static String entry_concat() {
    return ("" + new java.util.AbstractMap.SimpleEntry<>("k", "v")).replaceAll("@[0-9a-f]+$", "@");
  }

  public static String entry_contains() {
    return "" + List.of(new java.util.AbstractMap.SimpleEntry<>("k", "v")).contains(new java.util.AbstractMap.SimpleEntry<>("k", "v"));
  }

  public static String entry_objects() {
    return "" + Objects.equals(new java.util.AbstractMap.SimpleEntry<>("k", "v"), new java.util.AbstractMap.SimpleEntry<>("k", "v"));
  }

  public static String adder_equals() {
    return "" + new java.util.concurrent.atomic.DoubleAdder().equals(new java.util.concurrent.atomic.DoubleAdder());
  }

  public static String adder_same() {
    java.util.concurrent.atomic.DoubleAdder a = new java.util.concurrent.atomic.DoubleAdder();
    return "" + a.equals(a) + (a == a);
  }

  public static String adder_other() {
    return "" + new java.util.concurrent.atomic.DoubleAdder().equals("x") + new java.util.concurrent.atomic.DoubleAdder().equals(null);
  }

  public static String adder_hash() {
    return "" + (new java.util.concurrent.atomic.DoubleAdder().hashCode() == new java.util.concurrent.atomic.DoubleAdder().hashCode());
  }

  public static String adder_text() {
    return new java.util.concurrent.atomic.DoubleAdder().toString().replaceAll("@[0-9a-f]+$", "@");
  }

  public static String adder_concat() {
    return ("" + new java.util.concurrent.atomic.DoubleAdder()).replaceAll("@[0-9a-f]+$", "@");
  }

  public static String adder_contains() {
    return "" + List.of(new java.util.concurrent.atomic.DoubleAdder()).contains(new java.util.concurrent.atomic.DoubleAdder());
  }

  public static String adder_objects() {
    return "" + Objects.equals(new java.util.concurrent.atomic.DoubleAdder(), new java.util.concurrent.atomic.DoubleAdder());
  }

  public static String exception_equals() {
    return "" + new Exception("m").equals(new Exception("m"));
  }

  public static String exception_same() {
    Exception a = new Exception("m");
    return "" + a.equals(a) + (a == a);
  }

  public static String exception_other() {
    return "" + new Exception("m").equals("x") + new Exception("m").equals(null);
  }

  public static String exception_hash() {
    return "" + (new Exception("m").hashCode() == new Exception("m").hashCode());
  }

  public static String exception_text() {
    return new Exception("m").toString().replaceAll("@[0-9a-f]+$", "@");
  }

  public static String exception_concat() {
    return ("" + new Exception("m")).replaceAll("@[0-9a-f]+$", "@");
  }

  public static String exception_contains() {
    return "" + List.of(new Exception("m")).contains(new Exception("m"));
  }

  public static String exception_objects() {
    return "" + Objects.equals(new Exception("m"), new Exception("m"));
  }

  public static String sorted_random() {
    return attempt(() -> new TreeSet<Object>(List.of(new java.util.Random(1), new java.util.Random(1))).size());
  }

  public static String sorted_decimal() {
    return attempt(() -> new TreeSet<Object>(List.of(new java.math.BigDecimal("1.0"), new java.math.BigDecimal("1.00"))).size());
  }

  public static String sub_random_equals() {
    return "" + new MyRandom().equals(new MyRandom()) + Objects.equals(new MyRandom(), new MyRandom());
  }

  public static String sub_decimal_equals() {
    return "" + new MyDecimal().equals(new MyDecimal());
  }

  public static String sub_decimal_objects() {
    return "" + Objects.equals(new MyDecimal(), new MyDecimal());
  }

  public static String as_object_equals() {
    Object a = new java.math.BigDecimal("1.0");
    return "" + a.equals(new java.math.BigDecimal("1.0"));
  }

  public static String set_objects() {
    return "" + Objects.equals(new HashSet<>(List.of(1)), new HashSet<>(List.of(1)));
  }

  public static String set_equals() {
    return "" + new HashSet<>(List.of(1)).equals(new HashSet<>(List.of(1)));
  }

  static String attempt(java.util.function.Supplier<Object> s) {
    try {
      return String.valueOf(s.get());
    } catch (RuntimeException e) {
      return e.getClass().getName() + ": " + e.getMessage();
    }
  }

  /** A jar class that keeps `Random`'s (`Object`'s) `equals`. */
  static final class MyRandom extends java.util.Random {}

  /** A jar class that keeps `BigDecimal`'s `equals`. */
  static final class MyDecimal extends java.math.BigDecimal {
    MyDecimal() {
      super("1.0");
    }
  }
}

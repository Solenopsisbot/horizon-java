package demo;

import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.util.ArrayList;
import java.util.Collections;
import java.util.HashMap;
import java.util.HashSet;
import java.util.List;
import java.util.Objects;
import java.util.Optional;
import java.util.TreeSet;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.LinkedBlockingQueue;
import java.util.concurrent.ThreadPoolExecutor;
import java.util.concurrent.TimeUnit;
import java.util.function.Supplier;

/**
 * Objects a jar allocates are typed handles keyed {@code class#n} (#156 P12b): a {@code String}
 * spelling one's key is that string, never the object. Every check below counts the texts that
 * behave as the {@code String} they are; on a JVM that is all of them.
 */
public class Probe {
  public static class Foo {
    public int value;

    public Foo(int value) {
      this.value = value;
    }

    public String name() {
      return "foo";
    }

    @Override
    public String toString() {
      return "Foo(" + value + ")";
    }
  }

  /** Its own {@code equals}, {@code hashCode} and natural order. */
  public static final class Bar implements Comparable<Bar> {
    final int id;

    Bar(int id) {
      this.id = id;
    }

    @Override
    public boolean equals(Object other) {
      return other instanceof Bar bar && bar.id == id;
    }

    @Override
    public int hashCode() {
      return id * 31;
    }

    @Override
    public String toString() {
      return "Bar" + id;
    }

    @Override
    public int compareTo(Bar other) {
      return Integer.compare(other.id, id);
    }
  }

  /** A static an object is kept in, read back reflectively. */
  public static Object keptFoo;

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

  static int javaHash(String text) {
    int hash = 0;
    for (int i = 0; i < text.length(); i++) {
      hash = 31 * hash + text.charAt(i);
    }
    return hash;
  }

  static int label(Object value) {
    return switch (value) {
      case Foo foo -> 1;
      case ExecutorService service -> 2;
      case List<?> list -> 3;
      case String text -> 4;
      default -> 5;
    };
  }

  /** Every text {@code <class>#<n>} for the classes allocated here and n below 64. */
  public static String forged() throws Exception {
    Foo foo = new Foo(7);
    ThreadPoolExecutor pool =
        new ThreadPoolExecutor(1, 1, 0L, TimeUnit.SECONDS, new LinkedBlockingQueue<>());
    ArrayList<Object> list = new ArrayList<>();
    list.add(foo);
    StringBuilder built = new StringBuilder("built");
    HashMap<Object, String> map = new HashMap<>();
    map.put(foo, "foo");
    map.put(pool, "pool");
    map.put(list, "list");
    Field value = Foo.class.getField("value");
    Method name = Foo.class.getMethod("name");
    String[] classes = {
      "demo/Probe$Foo",
      "java/util/concurrent/ThreadPoolExecutor",
      "java/util/ArrayList",
      "java/lang/StringBuilder",
      "java/util/concurrent/LinkedBlockingQueue",
      "java/util/HashMap",
    };
    String[] checks = {
      "instanceof", "cast", "getClass", "equals", "hashCode", "text", "collections", "identity",
      "reflection", "switch", "charSequence",
    };
    int[] strings = new int[checks.length];
    for (String owner : classes) {
      for (int n = 0; n < 64; n++) {
        String text = owner + "#" + n;
        // A fresh string with that text, typed `Object` as a forger would pass it.
        Object s = new StringBuilder(owner).append('#').append(n).toString();
        boolean[] ok = {
          s instanceof String
              && !(s instanceof Foo)
              && !(s instanceof ExecutorService)
              && !(s instanceof List)
              && !(s instanceof StringBuilder),
          throwsCce(() -> (Foo) s)
              && throwsCce(() -> (ExecutorService) s)
              && throwsCce(() -> (List<?>) s),
          s.getClass() == String.class && s.getClass().getName().equals("java.lang.String"),
          s.equals(text)
              && !s.equals(foo)
              && !foo.equals(s)
              && !list.equals(s)
              && !Objects.equals(s, pool)
              && !built.equals(s),
          s.hashCode() == javaHash(text) && Objects.hashCode(s) == javaHash(text),
          String.valueOf(s).equals(text)
              && ("" + s).equals(text)
              && new StringBuilder().append(s).toString().equals(text)
              && Objects.toString(s).equals(text)
              && s.toString().equals(text)
              && String.format("%s", s).equals(text),
          !list.contains(s)
              && list.indexOf(s) == -1
              && map.get(s) == null
              && !map.containsKey(s)
              && !new HashSet<Object>(List.of(foo, pool)).contains(s),
          s != (Object) foo && s != (Object) pool && s != (Object) list && s != (Object) built,
          throwsIae(() -> value.get(s))
              && throwsIae(() -> name.invoke(s))
              && !ExecutorService.class.isInstance(s),
          label(s) == 4,
          ((CharSequence) s).length() == text.length()
              && new StringBuilder().append((CharSequence) s).toString().equals(text),
        };
        for (int i = 0; i < ok.length; i++) {
          if (ok[i]) {
            strings[i]++;
          }
        }
      }
    }
    pool.shutdown();
    StringBuilder out = new StringBuilder();
    for (int i = 0; i < checks.length; i++) {
      out.append(i == 0 ? "" : ",").append(checks[i]).append('=').append(strings[i]);
    }
    return out.toString();
  }

  /** One object keeps one identity through every path a value takes. */
  public static String identity() throws Exception {
    Foo foo = new Foo(1);
    Foo other = new Foo(1);
    ThreadPoolExecutor pool =
        new ThreadPoolExecutor(1, 1, 0L, TimeUnit.SECONDS, new LinkedBlockingQueue<>());
    List<Object> list = new ArrayList<>();
    list.add(foo);
    HashMap<String, Object> map = new HashMap<>();
    map.put("k", foo);
    Object[] array = {foo};
    Supplier<Foo> captured = () -> foo;
    List<Object> synced = Collections.synchronizedList(list);
    List<Object> copied = new ArrayList<>(List.of(foo, pool));
    Field field = Probe.class.getField("keptFoo");
    keptFoo = foo;
    ((ExecutorService) pool).shutdown();
    return ""
        + (list.get(0) == foo)
        + (map.get("k") == foo)
        + (array[0] == foo)
        + (captured.get() == foo)
        + (Optional.of(foo).get() == foo)
        + (synced.get(0) == foo)
        + (copied.get(0) == foo)
        + (copied.get(1) == pool)
        + (field.get(null) == foo)
        + "|"
        + (foo == other)
        + foo.equals(other)
        + list.contains(other)
        + list.contains(foo);
  }

  /** The class's own methods, through the interpreter; `Object`'s identity hash is counted. */
  public static String ownMethods() {
    Bar a = new Bar(2);
    Bar b = new Bar(2);
    Bar c = new Bar(5);
    HashSet<Object> set = new HashSet<>(List.of(a, c));
    TreeSet<Bar> sorted = new TreeSet<>(List.of(a, c, new Bar(3)));
    List<Bar> listed = new ArrayList<>(List.of(a, c, new Bar(3)));
    Collections.sort(listed);
    Foo foo = new Foo(9);
    return a.equals(b)
        + "," + (a == b)
        + "," + Objects.equals(a, b)
        + "," + a.hashCode()
        + "," + Objects.hash(a, c)
        + "," + set.contains(b)
        + "," + joined(sorted)
        + "," + joined(listed)
        + "," + a.compareTo(c)
        + "," + ("" + foo)
        + "," + String.valueOf(a)
        + "," + (foo.hashCode() == foo.hashCode());
  }

  /** The elements' own `toString`, run by the jar (a host collection's text cannot run it). */
  static String joined(Iterable<Bar> bars) {
    StringBuilder out = new StringBuilder();
    for (Bar bar : bars) {
      out.append(bar).append(' ');
    }
    return out.toString();
  }

  /** `getClass()` of a jar object and of a JDK object a jar `new`ed. */
  public static String classSurface() {
    Object foo = new Foo(1);
    Object pool =
        new ThreadPoolExecutor(1, 1, 0L, TimeUnit.SECONDS, new LinkedBlockingQueue<>());
    Object list = new ArrayList<String>();
    ((ThreadPoolExecutor) pool).shutdown();
    return foo.getClass().getName()
        + "," + foo.getClass().getSimpleName()
        + "," + foo.getClass().getSuperclass().getName()
        + "," + (foo.getClass() == Foo.class)
        + "," + pool.getClass().getName()
        + "," + (pool.getClass() == ThreadPoolExecutor.class)
        + "," + list.getClass().getName()
        + "," + (foo instanceof Foo)
        + (pool instanceof ThreadPoolExecutor)
        + (list instanceof List);
  }
}

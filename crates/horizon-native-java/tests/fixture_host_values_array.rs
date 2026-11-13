//! Host objects are typed handles (`Value::Host`, #156 P11): arrays.
//!
//! An array used to be marker text (`array/<class>/<method>/<n>`, and one
//! shared `array/empty`), the same `Value::Ref` a Java `String` is. Every
//! expected string is what this class printed on Java 25 (OpenJDK 25.0.5,
//! `javac --release 21`, two runs alike). Each method is either answered
//! exactly as Java 25 does, uncounted ([`EXACT`]), or counted with exactly
//! the calls [`COUNTED`] names: an array's own `toString`/`hashCode` are its
//! identity hash, which Java leaves to the VM, and the `Arrays` content
//! helpers are not modelled yet. A counted method that starts matching Java
//! moves to [`EXACT`].

use horizon_native_java::{NativeJar, StdHost, Value};
use std::{fs, io::Write, path::PathBuf, process::Command};
use zip::write::SimpleFileOptions;

const SOURCE: &str = r#"
package demo;

import java.util.*;
import java.util.stream.*;

public final class ArrayValues {
  /** Strings spelled like the arrays' old markers, or a handle's text form, stay Strings. */
  public static String spelled() {
    Object[] texts = {"array/empty", "array/demo/ArrayValues/spelled/0", "array/java/lang/String/args/0", "harr:[I#0",
        "harr:[Ljava/lang/String;#3", "[I#0", "[Ljava/lang/String;#2", "[[I#7", "[I#empty", "color-collection#0"};
    StringBuilder out = new StringBuilder();
    for (Object t : texts) {
      out.append(t instanceof String).append(t instanceof CharSequence).append(t.getClass().getName())
          .append(t instanceof Object[]).append(t instanceof int[]).append(t instanceof Cloneable)
          .append(t.equals(new int[0])).append(Objects.equals(t, String.valueOf(t)))
          .append(t.equals(t.toString())).append(t.toString().length()).append(t.hashCode())
          .append(sw(t)).append(castInts(t).startsWith("class java.lang.String cannot be cast to class [I ("))
          .append(int[].class.isInstance(t)).append(',');
    }
    int[] real = {5};
    List<Object> list = new ArrayList<>(List.of("harr:[I#0", real));
    Map<Object, String> m = new HashMap<>();
    m.put("harr:[I#0", "h");
    m.put(real, "r");
    out.append(list.get(0)).append(list.get(0) instanceof String).append(list.get(1) instanceof String)
        .append(list.get(1) instanceof int[]).append(list.get(1) == real).append(List.of("harr:[I#0", real).get(1) == real)
        .append('|').append(m.size()).append(m.get("harr:[I#0")).append(m.get(real))
        .append('|').append("" + "array/empty" + "harr:[I#0").append('|').append(castInts("harr:[I#0"))
        .append('|').append(castObjects("harr:[Ljava/lang/String;#3"))
        .append('|').append(Arrays.hashCode(new Object[] {"harr:[I#0", "x"}));
    return out.toString();
  }

  /** A type switch over arrays and strings. */
  static String sw(Object o) {
    return switch (o) {
      case String s -> "string:" + s.length();
      case int[] a -> "ints:" + a.length;
      case Object[] a -> "objects:" + a.length;
      default -> "other";
    };
  }

  static String castInts(Object o) {
    try { return "ok" + ((int[]) o).length; } catch (ClassCastException e) { return e.getMessage(); }
  }
  static String castStrings(Object o) {
    try { return "ok" + ((String[]) o).length; } catch (ClassCastException e) { return e.getMessage(); }
  }
  static String castObjects(Object o) {
    try { return "ok" + ((Object[]) o).length; } catch (ClassCastException e) { return e.getMessage(); }
  }
  static String castString(Object o) {
    try { return "ok" + ((String) o).length(); } catch (ClassCastException e) { return e.getMessage(); }
  }

  /** The real arrays: their classes, casts, equality, clones and lengths. */
  public static String real() {
    int[] a = {1, 2, 3};
    String[] s = {"x", null, "z"};
    long[] l = {1L, -2L};
    int[][] grid = {{1, 2}, {3}};
    Object[] mixed = {a, s, "q", 4};
    Object o = s;
    Object ints = a;
    return a.getClass().getName() + s.getClass().getName() + grid.getClass().getName() + l.getClass().getName()
        + mixed.getClass().getName() + "|" + a.getClass().getComponentType() + "," + s.getClass().getComponentType().getName()
        + "|" + (o instanceof Object[]) + (o instanceof String[]) + (o instanceof Cloneable) + (o instanceof java.io.Serializable)
        + (ints instanceof Object[]) + (ints instanceof int[]) + (ints instanceof long[]) + (grid instanceof Object[])
        + (o instanceof CharSequence[]) + (o instanceof Integer[]) + (ints instanceof Comparable)
        + "|" + castStrings(new Object[0]) + "|" + castObjects(s) + "|" + castInts(l) + "|" + castString(a) + "|" + castObjects(grid)
        + "|" + castStrings(mixed) + "|" + castObjects(ints)
        + "|" + a.equals(a) + a.equals(a.clone()) + Objects.equals(s, s) + Objects.equals(s, s.clone())
        + "|" + a.clone()[2] + a.clone().getClass().getName() + a.length + s.length + grid[1].length + Arrays.asList(s) + Arrays.asList(s).size()
        + "|" + Arrays.hashCode(s) + Arrays.hashCode((Object[]) null);
  }

  /** The content-based Arrays helpers, one call each. */
  static final int[] A = {1, 2, 3};
  static final String[] S = {"x", null, "z"};
  static final double[] D = {1.5, -0.0, Double.NaN};
  static final int[][] GRID = {{1, 2}, {3}};
  public static String helperToStringInts() { return Arrays.toString(A); }
  public static String helperToStringStrings() { return Arrays.toString(S); }
  public static String helperToStringDoubles() { return Arrays.toString(D); }
  public static String helperDeepToString() { return Arrays.deepToString(GRID); }
  public static String helperHashCodeInts() { return "" + Arrays.hashCode(A); }
  public static String helperHashCodeDoubles() { return "" + Arrays.hashCode(D); }
  public static String helperDeepHashCode() { return "" + Arrays.deepHashCode(GRID); }
  public static String helperEqualsInts() { return "" + Arrays.equals(A, new int[] {1, 2, 3}); }
  public static String helperEqualsStrings() { return "" + Arrays.equals(S, new String[] {"x", null, "z"}); }
  public static String helperEqualsDoubles() { return "" + Arrays.equals(D, new double[] {1.5, 0.0, Double.NaN}); }
  public static String helperDeepEquals() { return "" + Arrays.deepEquals(GRID, new int[][] {{1, 2}, {3}}); }

  /** Sorting arrays of arrays with a comparator, finding them in lists; type switches and Class.isInstance. */
  public static String sorted() {
    int[] a = {1, 2, 3};
    int[] b = {4};
    List<int[]> list = new ArrayList<>(List.of(a, b));
    list.sort(Comparator.comparingInt(x -> x.length));
    List<int[]> sorted = List.of(a, b).stream().sorted(Comparator.comparingInt(x -> -x.length)).toList();
    return "" + (list.get(0) == b) + (list.get(1) == a) + (sorted.get(0) == a) + list.indexOf(a) + List.of(a, b).contains(b)
        + List.of(a).indexOf(b) + sw(a) + sw(new String[2]) + sw(new int[0][]) + sw(new long[1])
        + int[].class.isInstance(a) + Object[].class.isInstance(new String[0]) + Object[].class.isInstance(a);
  }

  /** Natural ordering of arrays: an array is no Comparable. */
  @SuppressWarnings({"unchecked", "rawtypes"})
  public static String naturalOrder() {
    List<Object> list = new ArrayList<>(List.of(new int[] {2}, new int[] {1}));
    try { Collections.sort((List) list); return "sorted"; } catch (ClassCastException e) { return e.getMessage(); }
  }

  /** A store into an array checks the value's class. */
  interface Store { void run() throws Exception; }
  static class Base {}
  static class Sub extends Base {}
  static String store(Store r) {
    try { r.run(); return "ok"; } catch (Exception e) { return e.getClass().getSimpleName() + ":" + e.getMessage(); }
  }
  public static String stores() {
    String[] big = {"p", "q", "r"};
    return store(() -> { Object[] x = new String[1]; x[0] = 1; })
        + "|" + store(() -> { Object[] x = new Integer[1]; x[0] = "s"; })
        + "|" + store(() -> { Object[] x = new int[1][]; x[0] = new long[1]; })
        + "|" + store(() -> { Object[] x = new Sub[1]; x[0] = new Base(); })
        + "|" + store(() -> { Object[] x = new Base[1]; x[0] = new Sub(); })
        + "|" + store(() -> { Object[] x = new CharSequence[1]; x[0] = "s"; x[0] = 1; })
        + "|" + store(() -> { Object[] x = new String[1][]; x[0] = new Object[0]; })
        + "|" + store(() -> { Object[] x = new Object[1][]; x[0] = new String[0]; x[0] = new int[0]; })
        + "|" + store(() -> { Object[] x = new Cloneable[1]; x[0] = new int[0]; x[0] = "s"; })
        + "|" + store(() -> { Object[] x = new String[2]; x[0] = null; x[1] = "t"; })
        + "|" + store(() -> { Object unused = new ArrayList<Object>(List.of(1)).toArray(new String[0]); })
        + "|" + store(() -> { new ArrayList<Object>(List.of("a", 1)).toArray(big); }) + big[0] + big[1] + big[2]
        + "|" + store(() -> { Object unused = new ArrayList<Object>(List.of(1)).toArray(String[]::new); })
        + "|" + store(() -> { Object unused = Arrays.copyOf(new Object[] {1}, 1, String[].class); })
        + "|" + store(() -> { Object unused = Arrays.copyOf(new Integer[] {1}, 1, String[].class); })
        + "|" + store(() -> { Object unused = Arrays.copyOfRange(new Object[] {"a", 1}, 0, 2, String[].class); })
        + "|" + store(() -> { Object unused = Arrays.copyOf(new int[][] {{1}}, 1, long[][].class); })
        + "|" + store(() -> { Object unused = Arrays.copyOf(new CharSequence[] {"a"}, 1, Integer[].class); })
        + "|" + store(() -> { Object unused = Arrays.copyOf(new Number[] {1.5}, 1, Integer[].class); })
        + "|" + store(() -> { Object unused = Arrays.copyOf(new String[] {"a"}, 1, CharSequence[].class); })
        + "|" + Arrays.copyOf(new Object[] {"a"}, 1, String[].class)[0]
        + new ArrayList<Object>(List.of("b")).toArray(new String[0])[0];
  }

  /** Lambdas, method references and enum constants into arrays of a jar interface and enum. */
  interface Hint { void set(int x); static Hint of(Runnable r) { return x -> r.run(); } }
  enum Mode { A, B }
  static final Hint HINT = Hint.of(() -> {});
  public static String storeLambdas() {
    java.util.function.IntConsumer c = Integer::valueOf;
    Hint[] hints = {HINT, x -> {}, c::accept, new Hint() { public void set(int x) {} }};
    Mode[] modes = {Mode.B, Mode.A};
    Object[] objects = hints;
    objects[1] = HINT;
    return "" + hints.length + (hints[1] == HINT) + modes[0] + Mode.values().length;
  }

  /** A cast of an unmodelled call's array to a narrower array class: counted, its descriptor an array's. */
  public static String castUnmodelled() {
    Object signers = String.class.getSigners();
    String[] strings = (String[]) signers;
    return "" + (strings == null);
  }

  /** A store of a value whose class the runtime does not know (the current thread) into an array of a jar class. */
  public static String storeUnknown() {
    Object thread = Thread.currentThread();
    return store(() -> { Object[] x = new Base[1]; x[0] = thread; });
  }

  /** Class objects (still text in the runtime) into Class[] and Type[]. */
  public static String storeClasses() {
    Class<?>[] classes = {int.class, String.class, ArrayValues.class};
    java.lang.reflect.Type[] types = {long.class};
    Object[] objects = classes;
    objects[1] = boolean.class;
    return classes[0].getName() + classes[1].getName() + types.length;
  }

  /** F2: stores the runtime can decide from the value's class, into final and JDK component types. */
  static String st(Object[] array, Object value) {
    try { array[0] = value; return "ok"; } catch (ArrayStoreException e) { return "ASE:" + e.getMessage(); }
  }
  public static String storesDecided() {
    return st(new String[1], new Base()) + "|" + st(new Integer[1], Mode.A) + "|" + st(new String[1], new Object())
        + "|" + st(new String[1], new StringBuilder()) + "|" + st(new Integer[1], new ArrayList<>())
        + "|" + st(new Integer[1], new HashMap<>()) + "|" + st(new String[1], java.nio.file.StandardOpenOption.READ)
        + "|" + st(new Comparable[1], new Object()) + "|" + st(new Runnable[1], new Base())
        + "|" + st(new java.io.Serializable[1], new Base()) + "|" + st(new Iterable[1], new Base())
        + "|" + st(new Boolean[1], new Sub()) + "|" + st(new Class[1], "x") + "|" + st(new Class[1], int.class)
        + "|" + st(new CharSequence[1], new StringBuilder()) + "|" + st(new String[1], "x")
        + "|" + st(new Mode[1], Mode.B) + "|" + st(new Base[1], new Sub())
        + "|" + store(() -> { Object unused = Arrays.copyOf(new Object[] {new Object()}, 1, String[].class); });
  }

  /** F2: a lambda's class is a hidden class: a misfit, but its name is unknown. */
  public static String storeLambdaIntoStrings() {
    Runnable r = () -> {};
    return st(new String[1], r).startsWith("ASE:demo.ArrayValues$$Lambda") + "";
  }

  /** F2: a host collection's exact class (List12) is not kept: a misfit, its name unknown. */
  public static String storeListIntoIntegers() {
    return st(new Integer[1], List.of(1));
  }

  /** F3: count() of a SIZED pipeline does not traverse: no sort, no peek, no map. */
  public static String sizedCount() {
    List<Object> log = new ArrayList<>();
    int[] a = new int[0];
    int[] b = new int[0];
    return List.of(a, b).stream().sorted().count() + "," + Stream.of(a, b).sorted().map(x -> 1).count()
        + "," + Stream.of(a, b).sorted().limit(5).count() + "," + Stream.of(a, b).sorted().skip(1).count()
        + "," + Stream.of(1, 2, 3).peek(log::add).count() + log.size()
        + "," + Stream.of(1, 2).peek(log::add).map(x -> x).count() + log.size()
        + "," + Arrays.stream(new Object[] {a, b}).sorted().peek(log::add).count() + log.size()
        + "," + IntStream.range(0, 5).sorted().count() + "," + Stream.of(a, b).sorted().sorted().count();
  }

  /** F3: a pipeline that is not SIZED (filter) traverses: peek runs, the sort compares. */
  public static String traversedCount() {
    List<Object> log = new ArrayList<>();
    String peeked = Stream.of(1, 2, 3).filter(x -> true).peek(log::add).count() + "" + log.size()
        + Stream.of(4, 5).peek(log::add).toList() + log.size();
    String sorted;
    try { sorted = "" + Stream.of(new int[0], new int[0]).sorted().toList().size(); } catch (ClassCastException e) { sorted = e.getMessage(); }
    String filtered;
    try { filtered = "" + Stream.of(new int[0], new int[0]).filter(x -> true).sorted().count(); } catch (ClassCastException e) { filtered = e.getMessage(); }
    return peeked + "|" + sorted + "|" + filtered + "|" + Stream.of(3, 1, 2).sorted().toList() + Stream.concat(Stream.of(1), Stream.of(2)).peek(log::add).count() + log.size();
  }

  /** Sorting an array and a String: String.compareTo's bridge casts the array. */
  @SuppressWarnings({"unchecked", "rawtypes"})
  public static String naturalOrderMixed() {
    List<Object> list = new ArrayList<>(List.of(new int[0], "s"));
    try { Collections.sort((List) list); return "sorted"; } catch (ClassCastException e) { return e.getMessage(); }
  }

  /** Direct System.arraycopy: bounds first, then each element against the destination. */
  public static String arraycopies() {
    String[] d = {"p", "q", "r"};
    String[] e = new String[2];
    return store(() -> System.arraycopy(new Integer[] {1}, 0, new String[1], 0, 5))
        + "|" + store(() -> System.arraycopy(new Integer[] {null}, 0, new String[1], 0, 1))
        + "|" + store(() -> System.arraycopy(new Integer[] {1}, 0, new String[1], 0, 1))
        + "|" + store(() -> System.arraycopy(new CharSequence[] {"a"}, 0, new Integer[1], 0, 1))
        + "|" + store(() -> System.arraycopy(new Object[] {"a", 1, "c"}, 0, d, 0, 3)) + d[0] + d[1] + d[2]
        + "|" + store(() -> System.arraycopy(new Object[] {"a"}, 0, new String[1], 0, 5))
        + "|" + store(() -> System.arraycopy(new Object[] {"a", "b"}, 0, e, 0, 2)) + e[1]
        + "|" + store(() -> System.arraycopy(new Object[] {new int[0]}, 0, new long[1][], 0, 1))
        + "|" + store(() -> System.arraycopy(new int[1][], 0, new long[1][], 0, 1))
        + "|" + store(() -> System.arraycopy(new Base[] {new Sub(), new Base()}, 0, new Sub[2], 0, 2));
  }

  /** Another collection's toArray store failure (its message and partial writes are its own). */
  public static String storeOtherCollection() {
    return store(() -> { Object unused = new HashSet<Object>(List.of(1)).toArray(new String[0]); });
  }

  /** Arrays held by collections stay the same object; an equal copy is another. */
  public static String held() {
    int[] a = {7};
    int[] copy = a.clone();
    List<Object> list = new ArrayList<>();
    list.add(a);
    Map<Object, String> m = new HashMap<>();
    m.put(a, "a");
    Object[] box = {a};
    ((int[]) list.get(0))[0] = 9;
    return "" + list.contains(a) + list.contains(copy) + list.indexOf(copy) + (list.get(0) == a) + (box[0] == a)
        + m.get(a) + m.get(copy) + m.containsKey(a) + a[0] + copy[0] + ((int[]) box[0])[0];
  }

  /** Separate allocations are different objects, never counted. */
  public static String separate() {
    int[] a = new int[0];
    int[] b = new int[0];
    String[] c = new String[] {"x"};
    Map<Object, Integer> m = new IdentityHashMap<>();
    m.put(a, 1);
    m.put(b, 2);
    m.put(a, 3);
    return "" + (a == b) + (a != b) + ((Object) c == (Object) c.clone()) + m.size() + m.get(a) + m.get(b);
  }

  /** The same array flowing through two locals is one object. */
  public static String sameHandle() {
    int[] a = {1};
    int[] b = a;
    Object c = a;
    String[] s = new String[2];
    Object[] t = s;
    return "" + (a == b) + (c == a) + (s == t) + (a != b);
  }

  /** Two answers of a call the runtime does not model (Java: both null). */
  public static String twoUnmodelled() {
    return "" + (String.class.getSigners() == String.class.getSigners());
  }

  /** Each getStackTrace call on the current thread is a new array. */
  public static String twoTraces() {
    return "" + (Thread.currentThread().getStackTrace() == Thread.currentThread().getStackTrace());
  }

  /** An array's own toString and hashCode are identity based: one call each. */
  public static String arrayToString() { int[] a = {1}; String t = a.toString(); return "" + (t != null && t.startsWith("[I@")); }
  public static String arrayConcat() { String[] s = {"x"}; return "" + ("" + s).startsWith("[Ljava.lang.String;@"); }
  public static String arrayHashCode() { int[] a = {1}; return "" + (a.hashCode() == a.hashCode()); }
  public static String arrayIdentityHash() { int[] a = {1}; return "" + (System.identityHashCode(a) == System.identityHashCode(a)); }
}
"#;

fn javac() -> PathBuf {
    std::env::var_os("JAVA_HOME")
        .map(|home| PathBuf::from(home).join("bin/javac"))
        .unwrap_or_else(|| "javac".into())
}

/// Compile the fixture into a jar; each test passes its own tag so parallel
/// tests never share a build root.
fn jar(tag: &str) -> NativeJar {
    let root = std::env::temp_dir().join(format!(
        "horizon-native-java-array-values-{tag}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    let source = root.join("src/demo/ArrayValues.java");
    fs::create_dir_all(source.parent().unwrap()).unwrap();
    fs::create_dir_all(root.join("classes")).unwrap();
    fs::write(&source, SOURCE).unwrap();
    let status = Command::new(javac())
        .args(["--release", "21", "-nowarn", "-encoding", "UTF-8", "-d"])
        .arg(root.join("classes"))
        .arg(&source)
        .status()
        .unwrap();
    assert!(status.success(), "javac failed");
    let jar_path = root.join("array-values.jar");
    let mut zip = zip::ZipWriter::new(fs::File::create(&jar_path).unwrap());
    for class in fs::read_dir(root.join("classes/demo")).unwrap() {
        let class = class.unwrap().path();
        let name = format!("demo/{}", class.file_name().unwrap().to_string_lossy());
        zip.start_file(name, SimpleFileOptions::default()).unwrap();
        zip.write_all(&fs::read(&class).unwrap()).unwrap();
    }
    zip.finish().unwrap();
    let jar = NativeJar::from_path(&jar_path).unwrap();
    let _ = fs::remove_dir_all(&root);
    jar
}

/// Run one method on a fresh host: its answer and the unknown calls it made.
fn run(jar: &NativeJar, name: &str) -> (Value, Vec<(String, usize)>) {
    let mut host = StdHost::new();
    let value = jar
        .invoke_method_with_args(
            "demo.ArrayValues",
            name,
            "()Ljava/lang/String;",
            None,
            &[],
            &mut host,
        )
        .unwrap_or_else(|error| panic!("{name} failed: {error}"));
    (value, host.unknown_host_calls())
}

/// The methods answered exactly as Java 25 does, with nothing counted.
const EXACT: &[(&str, &str)] = &[
    // Strings spelled like the old markers (`array/...`), the handle text form (`harr:...`) and the new keys (`[I#0`, `color-collection#0`) are Strings: `instanceof String`, `getClass`, `equals`, `hashCode`, a type switch, a cast to `int[]`, `int[].class.isInstance`, list elements and `HashMap` keys.
    (
        "spelled",
        "truetruejava.lang.Stringfalsefalsefalsefalsetruetrue11-319826409string:11truefalse,truetruejava.lang.Stringfalsefalsefalsefalsetruetrue321513487838string:32truefalse,truetruejava.lang.Stringfalsefalsefalsefalsetruetrue29553930770string:29truefalse,truetruejava.lang.Stringfalsefalsefalsefalsetruetrue9-326518244string:9truefalse,truetruejava.lang.Stringfalsefalsefalsefalsetruetrue26-1057508844string:26truefalse,truetruejava.lang.Stringfalsefalsefalsefalsetruetrue42782267string:4truefalse,truetruejava.lang.Stringfalsefalsefalsefalsetruetrue21759994836string:21truefalse,truetruejava.lang.Stringfalsefalsefalsefalsetruetrue586822685string:5truefalse,truetruejava.lang.Stringfalsefalsefalsefalsetruetrue81143864280string:8truefalse,truetruejava.lang.Stringfalsefalsefalsefalsetruetrue1817748405string:18truefalse,harr:[I#0truefalsetruetruetrue|2hr|array/emptyharr:[I#0|class java.lang.String cannot be cast to class [I (java.lang.String and [I are in module java.base of loader 'bootstrap')|class java.lang.String cannot be cast to class [Ljava.lang.Object; (java.lang.String and [Ljava.lang.Object; are in module java.base of loader 'bootstrap')|-1532129891",
    ),
    // Array classes, component types, JLS array `instanceof`, casts with Java's exact `ClassCastException` text, `equals`, clones, lengths, `Arrays.asList`, `Arrays.hashCode(Object[])`.
    (
        "real",
        "[I[Ljava.lang.String;[[I[J[Ljava.lang.Object;|int,java.lang.String|truetruetruetruefalsetruefalsetruetruefalsefalse|class [Ljava.lang.Object; cannot be cast to class [Ljava.lang.String; ([Ljava.lang.Object; and [Ljava.lang.String; are in module java.base of loader 'bootstrap')|ok3|class [J cannot be cast to class [I ([J and [I are in module java.base of loader 'bootstrap')|class [I cannot be cast to class java.lang.String ([I and java.lang.String are in module java.base of loader 'bootstrap')|ok2|class [Ljava.lang.Object; cannot be cast to class [Ljava.lang.String; ([Ljava.lang.Object; and [Ljava.lang.String; are in module java.base of loader 'bootstrap')|class [I cannot be cast to class [Ljava.lang.Object; ([I and [Ljava.lang.Object; are in module java.base of loader 'bootstrap')|truefalsetruefalse|3[I331[x, null, z]3|1452330",
    ),
    // `List.sort`/`sorted()` with a comparator, `indexOf`/`contains` in host lists, type switches and `Class.isInstance` on arrays.
    (
        "sorted",
        "truetruetrue1true-1ints:3objects:2objects:0othertruetruefalse",
    ),
    // An array is no `Comparable`: natural ordering throws Java's `ClassCastException`.
    (
        "naturalOrder",
        "class [I cannot be cast to class java.lang.Comparable ([I and java.lang.Comparable are in module java.base of loader 'bootstrap')",
    ),
    // `aastore` into a typed array, `ArrayList.toArray(T[])` (with its partial copy) and `Arrays.copyOf(.., newType)` throw Java's `ArrayStoreException` texts.
    (
        "stores",
        "ArrayStoreException:java.lang.Integer|ArrayStoreException:java.lang.String|ArrayStoreException:[J|ArrayStoreException:demo.ArrayValues$Base|ok|ArrayStoreException:java.lang.Integer|ArrayStoreException:[Ljava.lang.Object;|ArrayStoreException:[I|ArrayStoreException:java.lang.String|ok|ArrayStoreException:arraycopy: element type mismatch: can not cast one of the elements of java.lang.Object[] to the type of the destination array, java.lang.String|ArrayStoreException:arraycopy: element type mismatch: can not cast one of the elements of java.lang.Object[] to the type of the destination array, java.lang.Stringaqr|ArrayStoreException:arraycopy: element type mismatch: can not cast one of the elements of java.lang.Object[] to the type of the destination array, java.lang.String|ArrayStoreException:arraycopy: element type mismatch: can not cast one of the elements of java.lang.Object[] to the type of the destination array, java.lang.String|ArrayStoreException:arraycopy: type mismatch: can not copy java.lang.Integer[] into java.lang.String[]|ArrayStoreException:arraycopy: element type mismatch: can not cast one of the elements of java.lang.Object[] to the type of the destination array, java.lang.String|ArrayStoreException:arraycopy: type mismatch: can not copy [I[] into [J[]|ArrayStoreException:arraycopy: type mismatch: can not copy java.lang.CharSequence[] into java.lang.Integer[]|ArrayStoreException:arraycopy: element type mismatch: can not cast one of the elements of java.lang.Number[] to the type of the destination array, java.lang.Integer|ok|ab",
    ),
    // Lambdas, method references, anonymous classes and enum constants stored into arrays of a jar interface and enum.
    ("storeLambdas", "4trueB2"),
    // `Class` objects (text until P13) into `Class[]` and `Type[]` are no `ArrayStoreException`.
    ("storeClasses", "intboolean1"),
    // An array held by a list, a map and another array stays that one object; its clone is equal in content only.
    ("held", "truefalse-1truetrueanulltrue979"),
    // Separate allocations are different objects, also as `IdentityHashMap` keys.
    ("separate", "falsetruefalse232"),
    // One array through two locals: provably one object.
    ("sameHandle", "truetruetruefalse"),
    // Two `getStackTrace()` arrays of the current thread: two objects.
    ("twoTraces", "false"),
    // Stores decided from the value's class: jar objects and enums, JDK objects a jar built, a host enum constant, into final classes (`String[]`, `Integer[]`, `Boolean[]`, `Class[]`) and JDK interfaces a jar class whose chain ends at `Object` is not; and stores that fit.
    (
        "storesDecided",
        "ASE:demo.ArrayValues$Base|ASE:demo.ArrayValues$Mode|ASE:java.lang.Object|ASE:java.lang.StringBuilder|ASE:java.util.ArrayList|ASE:java.util.HashMap|ASE:java.nio.file.StandardOpenOption|ASE:java.lang.Object|ASE:demo.ArrayValues$Base|ASE:demo.ArrayValues$Base|ASE:demo.ArrayValues$Base|ASE:demo.ArrayValues$Sub|ASE:java.lang.String|ok|ok|ok|ok|ok|ArrayStoreException:arraycopy: element type mismatch: can not cast one of the elements of java.lang.Object[] to the type of the destination array, java.lang.String",
    ),
    // `count()` of a SIZED pipeline does not traverse (JDK 25 `ReduceOps`): no sort compares two arrays, no `peek` or `map` runs, `limit`/`skip` adjust the size.
    ("sizedCount", "2,2,2,1,30,20,20,5,2"),
    // A pipeline that is not SIZED (`filter`), or a terminal that traverses (`toList`), runs `peek` and sorts (Java's `ClassCastException` for arrays).
    (
        "traversedCount",
        "33[4, 5]5|class [I cannot be cast to class java.lang.Comparable ([I and java.lang.Comparable are in module java.base of loader 'bootstrap')|class [I cannot be cast to class java.lang.Comparable ([I and java.lang.Comparable are in module java.base of loader 'bootstrap')|[1, 2, 3]25",
    ),
    // `"s".compareTo(int[])`: `String.compareTo`'s bridge casts the array.
    (
        "naturalOrderMixed",
        "class [I cannot be cast to class java.lang.String ([I and java.lang.String are in module java.base of loader 'bootstrap')",
    ),
    // Direct `System.arraycopy`: bounds first, then each element against the destination, copying up to the first misfit; `null` fits.
    (
        "arraycopies",
        "ArrayIndexOutOfBoundsException:arraycopy: last source index 5 out of bounds for object array[1]|ok|ArrayStoreException:arraycopy: type mismatch: can not copy java.lang.Integer[] into java.lang.String[]|ArrayStoreException:arraycopy: type mismatch: can not copy java.lang.CharSequence[] into java.lang.Integer[]|ArrayStoreException:arraycopy: element type mismatch: can not cast one of the elements of java.lang.Object[] to the type of the destination array, java.lang.Stringaqr|ArrayIndexOutOfBoundsException:arraycopy: last source index 5 out of bounds for object array[1]|okb|ArrayStoreException:arraycopy: element type mismatch: can not cast one of the elements of java.lang.Object[] to the type of the destination array, [J|ok|ArrayStoreException:arraycopy: element type mismatch: can not cast one of the elements of demo.ArrayValues$Base[] to the type of the destination array, demo.ArrayValues$Sub",
    ),
];

/// The methods whose answer the runtime does not have: Java 25's answer, and
/// exactly the calls counted instead (the neutral value they answer is not
/// checked: it is no Java answer).
#[allow(clippy::type_complexity)]
const COUNTED: &[(&str, &str, &[(&str, usize)])] = &[
    // Unmodelled on main and here (content based, no identity involved).
    (
        "helperToStringInts",
        "[1, 2, 3]",
        &[("java/util/Arrays#toString([I)Ljava/lang/String;", 1)],
    ),
    // Unmodelled on main and here (content based, no identity involved).
    (
        "helperToStringStrings",
        "[x, null, z]",
        &[(
            "java/util/Arrays#toString([Ljava/lang/Object;)Ljava/lang/String;",
            1,
        )],
    ),
    // Unmodelled on main and here (content based, no identity involved).
    (
        "helperToStringDoubles",
        "[1.5, -0.0, NaN]",
        &[("java/util/Arrays#toString([D)Ljava/lang/String;", 1)],
    ),
    // Unmodelled on main and here (content based, no identity involved).
    (
        "helperDeepToString",
        "[[1, 2], [3]]",
        &[(
            "java/util/Arrays#deepToString([Ljava/lang/Object;)Ljava/lang/String;",
            1,
        )],
    ),
    // Unmodelled on main and here (content based, no identity involved).
    (
        "helperHashCodeInts",
        "30817",
        &[("java/util/Arrays#hashCode([I)I", 1)],
    ),
    // Unmodelled on main and here (content based, no identity involved).
    (
        "helperHashCodeDoubles",
        "569406559",
        &[("java/util/Arrays#hashCode([D)I", 1)],
    ),
    // Unmodelled on main and here (content based, no identity involved).
    (
        "helperDeepHashCode",
        "31809",
        &[("java/util/Arrays#deepHashCode([Ljava/lang/Object;)I", 1)],
    ),
    // Unmodelled on main and here (content based, no identity involved).
    (
        "helperEqualsInts",
        "true",
        &[("java/util/Arrays#equals([I[I)Z", 1)],
    ),
    // Unmodelled on main and here (content based, no identity involved).
    (
        "helperEqualsStrings",
        "true",
        &[(
            "java/util/Arrays#equals([Ljava/lang/Object;[Ljava/lang/Object;)Z",
            1,
        )],
    ),
    // Unmodelled on main and here (content based, no identity involved).
    (
        "helperEqualsDoubles",
        "false",
        &[("java/util/Arrays#equals([D[D)Z", 1)],
    ),
    // Unmodelled on main and here (content based, no identity involved).
    (
        "helperDeepEquals",
        "true",
        &[(
            "java/util/Arrays#deepEquals([Ljava/lang/Object;[Ljava/lang/Object;)Z",
            1,
        )],
    ),
    // An unmodelled call's array is the shared empty array of its declared type: a narrowing cast is counted, its descriptor an array's (`[Ljava/lang/String;`).
    (
        "castUnmodelled",
        "true",
        &[
            (
                "[Ljava/lang/String;#<checkcast>(Ljava/lang/Object;)[Ljava/lang/String;",
                1,
            ),
            ("java/lang/Class#getSigners()[Ljava/lang/Object;", 1),
        ],
    ),
    // A store of a value whose class the runtime does not know (the current thread) into an array of a jar class: counted, then stored, as `checkcast` counts the same cast.
    (
        "storeUnknown",
        "ArrayStoreException:java.lang.Thread",
        &[("[Ldemo/ArrayValues$Base;#<aastore>(Ljava/lang/Object;)V", 1)],
    ),
    // A `HashSet`'s `toArray(T[])` store failure: its message and partial writes are its own class's, so the call is counted.
    (
        "storeOtherCollection",
        "ArrayStoreException:java.lang.Integer",
        &[(
            "java/util/HashSet#toArray([Ljava/lang/Object;)[Ljava/lang/Object;",
            1,
        )],
    ),
    // Two answers of an unmodelled call are shared empty arrays: identity between them is counted, never answered from equal keys.
    (
        "twoUnmodelled",
        "true",
        &[
            ("java/lang/Class#getSigners()[Ljava/lang/Object;", 2),
            (
                "java/lang/Object#if_acmp(Ljava/lang/Object;Ljava/lang/Object;)Z",
                1,
            ),
        ],
    ),
    // `toString()` is `[I@<identity hash>`: no value the runtime has.
    (
        "arrayToString",
        "true",
        &[("java/lang/Object#toString()Ljava/lang/String;", 1)],
    ),
    // String concatenation of an array prints its `toString()`.
    (
        "arrayConcat",
        "true",
        &[("java/lang/Object#toString()Ljava/lang/String;", 1)],
    ),
    // `hashCode()` is the identity hash.
    (
        "arrayHashCode",
        "true",
        &[("java/lang/Object#hashCode()I", 2)],
    ),
    // `System.identityHashCode`.
    (
        "arrayIdentityHash",
        "true",
        &[("java/lang/System#identityHashCode(Ljava/lang/Object;)I", 2)],
    ),
    // A lambda into a `String[]`: a misfit, but its hidden class's name is unknown.
    (
        "storeLambdaIntoStrings",
        "true",
        &[("[Ljava/lang/String;#<aastore>(Ljava/lang/Object;)V", 1)],
    ),
    // `List.of(1)` into an `Integer[]`: a misfit, but the runtime does not keep its class (`List12`).
    (
        "storeListIntoIntegers",
        "ASE:java.util.ImmutableCollections$List12",
        &[("[Ljava/lang/Integer;#<aastore>(Ljava/lang/Object;)V", 1)],
    ),
];

#[test]
fn no_string_spells_an_array_and_the_arrays_answer_as_java_25() {
    let jar = jar("exact");
    let mut wrong = Vec::new();
    for (name, expected) in EXACT {
        let (value, unknown) = run(&jar, name);
        if value != Value::Ref(Some((*expected).into())) || !unknown.is_empty() {
            wrong.push(format!("{name}: {value:?} {unknown:?}"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn what_the_runtime_cannot_answer_is_counted_by_name() {
    let jar = jar("counted");
    let mut wrong = Vec::new();
    for (name, java, calls) in COUNTED {
        let (value, unknown) = run(&jar, name);
        let expected = calls
            .iter()
            .map(|(call, count)| ((*call).to_owned(), *count))
            .collect::<Vec<_>>();
        if unknown != expected {
            wrong.push(format!("{name} (Java 25: {java}): {value:?} {unknown:?}"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

//! Host objects are typed handles (`Value::Host`, #156 P11): arrays.
//!
//! An array used to be marker text (`array/<class>/<method>/<n>`, and one
//! shared `array/empty`), the same `Value::Ref` a Java `String` is. Every
//! expected string is what this class printed on Java 25 (OpenJDK 25.0.5,
//! `javac --release 21`, two runs alike). An array's own `toString` and
//! `hashCode` are identity based (`[I@1b6d3586`): Java defines no value for
//! them, so the fixture checks only what Java does define about them.

use horizon_native_java::{NativeJar, StdHost, Value};
use std::{fs, io::Write, path::PathBuf, process::Command};
use zip::write::SimpleFileOptions;

const SOURCE: &str = r#"
package demo;

import java.util.*;

public final class ArrayValues {
  /** Strings spelled like the arrays' old markers, or a handle's text form, stay Strings. */
  public static String spelled() {
    Object[] texts = {"array/empty", "array/demo/ArrayValues/spelled/0", "array/java/lang/String/args/0", "harr:[I#0", "harr:[Ljava/lang/String;#3"};
    StringBuilder out = new StringBuilder();
    for (Object t : texts) {
      out.append(t instanceof Object[]).append(t instanceof int[]).append(t instanceof Cloneable)
          .append(t.equals(new int[0])).append(Objects.equals(t, String.valueOf(t)))
          .append(t.equals(t.toString())).append(t.toString().length())
          // `word/rest` text is still classified by is_plain_string (#156 P17).
          .append(t.toString().contains("/") ? "" : String.valueOf(t.hashCode())).append(',');
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

  /** The content-based Arrays helpers. */
  public static String helpers() {
    int[] a = {1, 2, 3};
    String[] s = {"x", null, "z"};
    long[] l = {1L, -2L};
    double[] d = {1.5, -0.0, Double.NaN};
    char[] c = {'h', 'i'};
    boolean[] z = {true, false};
    int[][] grid = {{1, 2}, {3}};
    Object[] mixed = {a, s, "q", 4};
    return Arrays.toString(a) + Arrays.toString(s) + Arrays.toString(l) + Arrays.toString(d) + Arrays.toString(c)
        + Arrays.toString(z) + Arrays.deepToString(grid) + Arrays.deepToString(mixed) + Arrays.toString((int[]) null)
        + "|" + Arrays.hashCode(a) + "," + Arrays.hashCode(l) + "," + Arrays.hashCode(d)
        + "," + Arrays.hashCode(c) + "," + Arrays.hashCode(z) + "," + Arrays.deepHashCode(grid) + "," + Arrays.hashCode((int[]) null)
        + "|" + Arrays.equals(a, new int[] {1, 2, 3}) + Arrays.equals(a, new int[] {1, 2}) + Arrays.equals(s, new String[] {"x", null, "z"})
        + Arrays.equals(d, new double[] {1.5, 0.0, Double.NaN}) + Arrays.equals(d, new double[] {1.5, -0.0, Double.NaN})
        + Arrays.deepEquals(grid, new int[][] {{1, 2}, {3}}) + Arrays.equals(grid, new int[][] {{1, 2}, {3}});
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

  /** An array's own toString and hashCode are identity-based. */
  public static String identityText() {
    int[] a = {1};
    String[] s = {"x"};
    String text = a.toString();
    String strings = String.valueOf((Object) s);
    return (text != null && text.startsWith("[I@")) + "," + (strings != null && strings.startsWith("[Ljava.lang.String;@")) + ","
        + (a.hashCode() == System.identityHashCode(a)) + "," + (a.hashCode() == a.hashCode()) + "," + (text != null && text.equals("" + a));
  }
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

const IF_ACMP: &str = "java/lang/Object#if_acmp(Ljava/lang/Object;Ljava/lang/Object;)Z";

/// No string is an array, and the real arrays answer as Java 25 does: their
/// classes and casts, the content-based `Arrays` helpers, the collections
/// that hold them, and identity between separate arrays and of one array
/// flowing through two locals. None of it is counted.
#[test]
fn no_string_spells_an_array_and_the_arrays_answer_as_java_25() {
    let jar = jar("exact");
    let mut wrong = Vec::new();
    for (name, expected) in [
        (
            "spelled",
            "falsefalsefalsefalsetruetrue11,falsefalsefalsefalsetruetrue32,falsefalsefalsefalsetruetrue29,falsefalsefalsefalsetruetrue9-326518244,falsefalsefalsefalsetruetrue26,harr:[I#0truefalsetruetruetrue|2hr|array/emptyharr:[I#0|class java.lang.String cannot be cast to class [I (java.lang.String and [I are in module java.base of loader 'bootstrap')|class java.lang.String cannot be cast to class [Ljava.lang.Object; (java.lang.String and [Ljava.lang.Object; are in module java.base of loader 'bootstrap')|-1532129891",
        ),
        (
            "real",
            "[I[Ljava.lang.String;[[I[J[Ljava.lang.Object;|int,java.lang.String|truetruetruetruefalsetruefalsetruetruefalsefalse|class [Ljava.lang.Object; cannot be cast to class [Ljava.lang.String; ([Ljava.lang.Object; and [Ljava.lang.String; are in module java.base of loader 'bootstrap')|ok3|class [J cannot be cast to class [I ([J and [I are in module java.base of loader 'bootstrap')|class [I cannot be cast to class java.lang.String ([I and java.lang.String are in module java.base of loader 'bootstrap')|ok2|class [Ljava.lang.Object; cannot be cast to class [Ljava.lang.String; ([Ljava.lang.Object; and [Ljava.lang.String; are in module java.base of loader 'bootstrap')|class [I cannot be cast to class [Ljava.lang.Object; ([I and [Ljava.lang.Object; are in module java.base of loader 'bootstrap')|truefalsetruefalse|3[I331[x, null, z]3|1452330",
        ),
        // Two `getStackTrace()` arrays of the current thread: two objects.
        ("twoTraces", "false"),
        // An array held by a list, a map and another array stays that one
        // object; its clone is equal in content only.
        ("held", "truefalse-1truetrueanulltrue979"),
        // Separate allocations are different objects, also as
        // `IdentityHashMap` keys.
        ("separate", "falsetruefalse232"),
        // One array through two locals: provably one object.
        ("sameHandle", "truetruetruefalse"),
    ] {
        let (value, unknown) = run(&jar, name);
        if value != Value::Ref(Some(expected.into())) || !unknown.is_empty() {
            wrong.push(format!("{name}: {value:?} {unknown:?}"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// Two answers of a call the runtime does not model (Java 25: both `null`,
/// so `true`). Each is the runtime's shared empty array, a new handle per
/// call: one key standing for whatever each call returned, so identity
/// cannot tell one object from two. The comparison is counted, never
/// answered from the equal keys.
#[test]
fn identity_of_two_shared_empty_arrays_is_counted() {
    let jar = jar("empties");
    let (value, unknown) = run(&jar, "twoUnmodelled");
    assert!(
        unknown.iter().any(|(call, _)| call == IF_ACMP),
        "{value:?} {unknown:?}"
    );
}

/// `Arrays.toString`/`hashCode`/`equals`/`deep*` of primitive and nested
/// arrays read the elements, so no identity is involved, but the runtime
/// does not model them (main counts every one): each is answered as Java 25
/// does or counted, never answered differently.
#[test]
fn arrays_helpers_answer_as_java_25_or_are_counted() {
    let jar = jar("helpers");
    let (value, unknown) = run(&jar, "helpers");
    assert!(
        value == Value::Ref(Some("[1, 2, 3][x, null, z][1, -2][1.5, -0.0, NaN][h, i][true, false][[1, 2], [3]][[1, 2, 3], [x, null, z], q, 4]null|30817,993,569406559,4290,40359,31809,0|truefalsetruefalsetruetruefalse".into())) || !unknown.is_empty(),
        "{value:?} {unknown:?}"
    );
}

/// An array's `toString()` (`[I@1b6d3586`) and `hashCode()` name its identity
/// hash, which Java leaves to the VM. The runtime has no such number and
/// never invents one: what Java does define about them (the `[I@` prefix,
/// `hashCode() == System.identityHashCode(a)`) is answered as Java 25 does,
/// or the call is counted.
#[test]
fn identity_text_of_an_array_is_never_invented() {
    let jar = jar("identity-text");
    let (value, unknown) = run(&jar, "identityText");
    assert!(
        value == Value::Ref(Some("true,true,true,true,true".into())) || !unknown.is_empty(),
        "{value:?} {unknown:?}"
    );
}

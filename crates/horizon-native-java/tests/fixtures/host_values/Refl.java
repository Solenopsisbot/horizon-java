package demo;

import java.lang.reflect.*;
import java.util.*;

/** Reflection objects as host handles (#156 P10): each method is one case. */
public class Refl {
  public static final int COUNT = 9;
  private transient volatile String[] names;
  protected static long[][] grid;
  int plain;
  enum Mode { ON }

  public Refl() {}
  protected Refl(int a, String... rest) throws java.io.IOException, IllegalStateException {}
  public static int target(int x) throws java.io.IOException { return x * 2; }
  static synchronized String join(int a, String[] b, List<String> c) { return a + b[0] + c; }
  private final void hidden() {}

  public interface Api {
    int LIMIT = 3;
    String name();
    default int size() { return 1; }
    static void util() {}
  }

  public static class Super {
    public int sf = 7;
    public static final String TAG = "t";
    public String foo() { return "super-foo"; }
  }

  public static class Base { public Object get() { return "b"; } }

  public static class Cov extends Base { public String get() { return "c"; } }

  public static class Cmp implements Comparable<Cmp> { public int compareTo(Cmp o) { return 0; } }

  public static String fmt(String format, Object... args) { return String.format(format, args); }

  // #169 review: return-type ranking, virtual dispatch, getMethods/getFields.
  public interface ISeq { CharSequence seq(); }
  public interface IStr { String seq(); }
  public abstract static class SeqStr implements ISeq, IStr {}
  public abstract static class StrSeq implements IStr, ISeq {}
  public interface ISb { StringBuilder seq(); }
  public abstract static class SeqSb implements ISeq, ISb {}
  public interface IArrO { Object[] arr(); }
  public interface IArrS { String[] arr(); }
  public abstract static class ArrBoth implements IArrO, IArrS {}
  public interface IHeld { Super get(); }
  public interface IHeldSub { Sub get(); }
  public abstract static class HeldBoth implements IHeld, IHeldSub {}

  public static class Leaf extends Sub { public String foo() { return "leaf-foo"; } public String name() { return "leaf"; } }
  public abstract static class Abs implements Api {}
  public static class AbsLeaf extends Abs { public String name() { return "absleaf"; } public int size() { return 7; } }
  public static class Over { public String f(long x) { return "long"; } }
  public static class OverSub extends Over { public String f(long x) { return "sub-long"; } }
  public interface I1 { default String d() { return "i1"; } }
  public interface I2 extends I1 { default String d() { return "i2"; } }
  public static class Dia implements I1, I2 {}
  public static class PrivBase { private String p() { return "base"; } }
  public static class PrivSub extends PrivBase { private String p() { return "sub"; } }
  public static class StaticBase { public static String s() { return "base-static"; } }
  public static class StaticSub extends StaticBase { public static String s() { return "sub-static"; } }

  public interface Consts { int K = 1; String S = "s"; }
  public static class FBase implements Consts { public int a; public static int sa; protected int hidden; }
  public static class FSub extends FBase { public int b; }
  public interface GI { default String g() { return "gi"; } }
  public interface GJ extends GI { default String g() { return "gj"; } }
  public static class GBase implements GJ {}
  public static class GSub extends GBase implements GI {}

  public static class Sub extends Super implements Api {
    public String name() { return "sub"; }
  }

  public static class Inner {
    public Inner(Refl[] all) {}
  }

  static Method target() throws Exception { return Refl.class.getMethod("target", int.class); }
  static Field count() throws Exception { return Refl.class.getDeclaredField("COUNT"); }
  static Constructor<?> ctor() throws Exception { return Refl.class.getDeclaredConstructor(int.class, String[].class); }

  static String castMethod(Object o) {
    try { return "ok" + ((Method) o).getName(); } catch (ClassCastException e) { return e.getMessage(); }
  }

  /** Strings spelled like the old markers (and the handles' encodings) stay Strings. */
  public static String spelled() throws Exception {
    Object[] texts = {
      "reflect-method/demo/Refl#target(I)I", "reflect-field/demo/Refl#COUNT:I:25", "reflect-field/demo/Refl#COUNT:I:25#3",
      "reflect-constructor/demo/Refl#()V", "hrmth:demo/Refl#target(I)I:9", "hrfld:demo/Refl#COUNT:I:25#3", "hrctr:demo/Refl#()V:1"
    };
    Method m = target();
    Field f = count();
    Constructor<?> k = Refl.class.getConstructor();
    StringBuilder out = new StringBuilder();
    Map<Object, String> map = new HashMap<>();
    for (Object t : texts) {
      out.append(t instanceof Method).append(t instanceof Field).append(t instanceof Constructor)
          .append(t instanceof Member).append(t.equals(m)).append(m.equals(t)).append(f.equals(t)).append(k.equals(t))
          .append(t.equals(t.toString())).append('|').append("" + t).append(',');
      // A `word/rest` string (the old markers) is still not a String to
      // is_plain_string until #156 P17, and any text with a `#` is an object
      // to getClass until P12; the handles' encodings are Strings here.
      if (t.toString().startsWith("h")) {
        out.append(t instanceof String).append(t.hashCode()).append(',');
        map.put(t, "s");
      }
    }
    map.put(m, "m");
    map.put(f, "f");
    return out.append(map.size()).append(map.get(texts[4])).append(map.get(m)).append(map.get(f))
        .append('|').append(castMethod(texts[4])).append('|').append(castMethod(m)).toString();
  }

  /** Java 25's toString of every modifier shape. */
  public static String text() throws Exception {
    Class<?> c = Refl.class;
    Object[] members = {
      target(), c.getDeclaredMethod("join", int.class, String[].class, List.class), c.getDeclaredMethod("hidden"),
      Api.class.getMethod("name"), Api.class.getMethod("size"), Api.class.getMethod("util"),
      count(), c.getDeclaredField("names"), c.getDeclaredField("grid"), c.getDeclaredField("plain"), Mode.class.getField("ON"),
      c.getConstructor(), ctor(), Inner.class.getConstructor(Refl[].class)
    };
    StringBuilder out = new StringBuilder();
    for (Object member : members) {
      out.append(member).append(" #").append(member.hashCode()).append('\n');
    }
    return out.append(String.valueOf(target())).append('|').append("" + count()).toString();
  }

  /** Copies are equal, share a hash, and are each their class. */
  public static String real() throws Exception {
    Method m = target();
    Field f = count();
    Constructor<?> k = ctor();
    Object o = m;
    return "" + m.equals(target()) + f.equals(count()) + k.equals(ctor()) + m.equals(f) + f.equals(m)
        + k.equals(Refl.class.getConstructor()) + m.equals(Refl.class.getDeclaredMethod("hidden"))
        + "|" + (m.hashCode() == target().hashCode()) + (f.hashCode() == count().hashCode())
        + "|" + m.getClass().getName() + "," + f.getClass().getName() + "," + k.getClass().getName()
        + "|" + (o instanceof Executable) + (o instanceof Member) + (o instanceof AccessibleObject)
        + (o instanceof GenericDeclaration) + (o instanceof AnnotatedElement) + (((Object) f) instanceof GenericDeclaration)
        + (((Object) f) instanceof Executable) + (((Object) k) instanceof Executable) + (o instanceof Comparable)
        + "|" + m.invoke(null, 21) + f.get(null) + m.getName() + f.getName() + k.getParameterCount()
        + Refl.class.getConstructor().newInstance().plain;
  }

  /** Collections find a copy by Field/Method equals. */
  public static String collections() throws Exception {
    Field f1 = count(), f2 = count();
    Method m1 = target(), m2 = target();
    List<Object> list = new ArrayList<>(List.of(f1, m1));
    Set<Object> set = new HashSet<>(List.of(f1, m1));
    set.add(f2);
    set.add(m2);
    Map<Object, Integer> map = new HashMap<>();
    map.put(f1, 1);
    map.put(m1, 2);
    return "" + list.contains(f2) + list.indexOf(m2) + set.size() + map.get(f2) + map.get(m2)
        + List.of(f1).contains(f2) + Optional.of(m1).equals(Optional.of(m2)) + list.equals(List.of(f2, m2))
        + "|" + (list.get(0) == f1) + (list.get(0) != f2);
  }

  /** The same Field, Method or Constructor flowing to both sides is one object. */
  public static String sameHandle() throws Exception {
    Method m = target();
    Method n = m;
    Field f = count();
    Object g = f;
    Constructor<?> k = ctor();
    Constructor<?> l = k;
    List<Field> fields = new ArrayList<>(List.of(f));
    fields.get(0).setAccessible(true);
    return "" + (m == n) + (f == g) + (k == l) + (fields.get(0) == f) + f.isAccessible() + count().isAccessible();
  }

  /** Two lookups of one field are two Field copies: never ==. */
  public static String fieldCopies() throws Exception {
    Field a = count(), b = count();
    Map<Object, Integer> identity = new IdentityHashMap<>();
    identity.put(a, 1);
    identity.put(b, 2);
    return "" + (a == b) + (a != b) + identity.size();
  }

  /** Two lookups of one method: equal, not == (counted). */
  public static String methodCopies() throws Exception {
    return "" + (target() == target());
  }

  /** Two lookups of one constructor in an IdentityHashMap: two keys (counted). */
  public static String constructorCopies() throws Exception {
    Map<Object, Integer> identity = new IdentityHashMap<>();
    identity.put(ctor(), 1);
    identity.put(ctor(), 2);
    return "" + identity.size();
  }

  /**
   * Fields paired with a sort key through a stream keep their copy: setAccessible, then get
   * (Shulker Box Tooltip's config tree pairs each field with an ordinal the same way).
   */
  public static String fieldStream() throws Exception {
    Refl holder = new Refl();
    holder.names = new String[] {"x"};
    holder.plain = 4;
    return java.util.Arrays.stream(Refl.class.getDeclaredFields())
        .filter(f -> !Modifier.isStatic(f.getModifiers()))
        .map(f -> Map.entry(-f.getName().length(), f))
        .sorted(Comparator.comparingInt(Map.Entry::getKey))
        .map(Map.Entry::getValue)
        .map(f -> { f.setAccessible(true); try { return f.getName() + "=" + f.get(holder); } catch (Exception e) { return e.toString(); } })
        .map(s -> s.startsWith("names=") ? "names=array" : s)
        .collect(java.util.stream.Collectors.joining(","));
  }

  /** A HashMap keyed by Method, copied and read back (DazzleConf's default-method map). */
  public static String methodMap() throws Exception {
    Map<Method, String> map = new HashMap<>();
    for (Method m : new Method[] {Api.class.getMethod("name"), Api.class.getMethod("size")}) {
      map.put(m, m.getName());
    }
    Map<Method, String> copy = new HashMap<>(map);
    int seen = 0;
    for (Map.Entry<Method, String> entry : copy.entrySet()) {
      Objects.requireNonNull(entry.getKey(), "key");
      seen++;
    }
    Map<Method, String> fixed = Collections.unmodifiableMap(copy);
    return seen + "," + fixed.size() + "," + fixed.get(Api.class.getMethod("size")) + "," + fixed.containsKey(target());
  }

  static String lookupField(Class<?> c, String name, boolean declared) {
    try {
      Field f = declared ? c.getDeclaredField(name) : c.getField(name);
      return f + "=" + f.getModifiers();
    } catch (NoSuchFieldException e) {
      return "NSFE:" + e.getMessage();
    }
  }

  static String lookupMethod(String name, Class<?>... parameters) {
    try {
      Method m = Refl.class.getDeclaredMethod(name, parameters);
      return m + "=" + m.invoke(null, 21);
    } catch (Exception e) {
      return e.getClass().getSimpleName() + ":" + e.getMessage();
    }
  }

  /** A lookup name spelled like a key, a copy or a marker is just a name nobody declares. */
  public static String forgedNames() {
    StringBuilder out = new StringBuilder();
    for (String name : new String[] {"COUNT:I:25", "COUNT#4", "COUNT:I:25#0", "reflect-field/demo/Refl#COUNT:I:25", "plain,i0", "COUNT,i0#0", ""}) {
      out.append(lookupField(Refl.class, name, true)).append(';').append(lookupField(Refl.class, name, false)).append(';');
    }
    return out.append(lookupMethod("target(I)I:9")).append(';').append(lookupMethod("target(I)I", int.class))
        .append(';').append(lookupMethod("target", int.class)).append(';').append(lookupMethod("target", long.class)).toString();
  }

  /** No spelled name is ever the same object as a real Field copy, or shares its override. */
  public static String forgedCopies() throws Exception {
    Field real = count();
    Field names = Refl.class.getDeclaredField("names");
    names.setAccessible(true);
    for (int i = 0; i < 200; i++) {
      for (String name : new String[] {"COUNT:I:25#" + i, "COUNT#" + i, "names:[Ljava/lang/String;:130#" + i}) {
        try {
          Field g = Refl.class.getDeclaredField(name);
          if (g == real || g.equals(real) || g.isAccessible()) return "forged " + name;
        } catch (NoSuchFieldException e) {
        }
      }
    }
    return "none" + names.isAccessible() + real.isAccessible();
  }

  /** getField follows superclasses and superinterfaces: the declaring class's own field. */
  public static String inheritedFields() throws Exception {
    Field sf = Sub.class.getField("sf");
    Field tag = Sub.class.getField("TAG");
    Field limit = Sub.class.getField("LIMIT");
    return sf.getDeclaringClass().getName() + "," + sf.getModifiers() + "," + sf.getType() + "," + sf + "," + sf.get(new Sub())
        + "," + sf.equals(Super.class.getField("sf")) + (sf.hashCode() == Super.class.getField("sf").hashCode())
        + "|" + tag + "=" + tag.get(null) + "|" + limit + "=" + limit.get(null) + "," + limit.getDeclaringClass().getName()
        + "|" + lookupField(Sub.class, "nope", false) + "|" + lookupField(Sub.class, "sf", true);
  }

  /** A JDK field's facts are not in any class file the run holds: counted. */
  public static String jdkFieldModifiers() throws Exception {
    return "" + Integer.class.getField("MAX_VALUE").getModifiers();
  }

  public static String jdkFieldDeclaring() throws Exception {
    return Integer.class.getField("MAX_VALUE").getDeclaringClass().getName();
  }

  static String getMethodOf(Class<?> c, String name, Class<?>... parameters) {
    try {
      Method m = c.getMethod(name, parameters);
      return m.getDeclaringClass().getName() + ":" + m;
    } catch (NoSuchMethodException e) {
      return "NSME:" + e.getMessage();
    }
  }

  /** getMethod: public members only, inherited from superclasses, superinterfaces and Object. */
  public static String getMethodRules() throws Exception {
    Method foo = Sub.class.getMethod("foo");
    return getMethodOf(Refl.class, "hidden") + "|" + getMethodOf(Sub.class, "foo") + "|" + foo.equals(Super.class.getMethod("foo"))
        + (foo.hashCode() == Super.class.getMethod("foo").hashCode()) + foo.invoke(new Sub())
        + "|" + getMethodOf(Sub.class, "size") + "=" + Sub.class.getMethod("size").invoke(new Sub())
        + "|" + Sub.class.getMethod("hashCode").getDeclaringClass().getName() + "," + Sub.class.getMethod("wait", long.class, int.class).getDeclaringClass().getName()
        + "|" + getMethodOf(Sub.class, "nope") + "|" + getMethodOf(Api.class, "hashCode") + "|" + getMethodOf(Sub.class, "util")
        + "|" + getMethodOf(Api.class, "util") + "|" + getMethodOf(Refl.class, "target", long.class)
        + "|" + Cov.class.getMethod("get").getReturnType().getName() + "," + Cov.class.getMethod("get").getDeclaringClass().getName();
  }

  /** getModifiers keeps the bridge, varargs and synthetic flags. */
  public static String methodModifiers() throws Exception {
    Method bridge = Cmp.class.getDeclaredMethod("compareTo", Object.class);
    return bridge.getModifiers() + "," + bridge.isBridge() + bridge.isSynthetic() + "," + Cmp.class.getMethod("compareTo", Object.class).isBridge()
        + "," + Refl.class.getMethod("fmt", String.class, Object[].class).getModifiers()
        + "," + Cmp.class.getDeclaredMethod("compareTo", Cmp.class).getModifiers();
  }

  /** A JDK method has no body in the run: counted. */
  public static String jdkInvoke() throws Exception {
    return "" + String.class.getMethod("length").invoke("abcd");
  }

  /** A JDK constructor has no body in the run: counted. */
  public static String jdkNewInstance() throws Exception {
    return "" + StringBuilder.class.getConstructor(String.class).newInstance("xy").append('z');
  }

  static String declaring(Class<?> c, String name) {
    try {
      Method m = c.getMethod(name);
      return m.getDeclaringClass().getName() + ":" + m.getReturnType().getName();
    } catch (NoSuchMethodException e) {
      return "NSME:" + e.getMessage();
    }
  }

  /** getMethod's most specific return type among JDK types of Java 25's generated table: exact. */
  public static String returnJdk() { return declaring(SeqStr.class, "seq") + "|" + declaring(StrSeq.class, "seq"); }

  /** ... and among JDK types outside it (StringBuilder): counted. */
  public static String returnJdkUnknown() { return declaring(SeqSb.class, "seq"); }

  /** ... among arrays (by element type) and held classes: exact. */
  public static String returnKnown() { return declaring(ArrBoth.class, "arr") + "|" + declaring(HeldBoth.class, "get"); }

  /** Method.invoke selects the receiver's override, as invokevirtual/invokeinterface do. */
  public static String virtualInvoke() throws Exception {
    PrivBase privSub = new PrivSub();
    Method p = PrivBase.class.getDeclaredMethod("p");
    return Sub.class.getMethod("foo").invoke(new Leaf()) + "," + Super.class.getMethod("foo").invoke(new Leaf())
        + "," + Abs.class.getMethod("size").invoke(new AbsLeaf()) + "," + Abs.class.getMethod("name").invoke(new AbsLeaf())
        + "," + Api.class.getMethod("name").invoke(new Sub()) + "," + Api.class.getMethod("size").invoke(new Sub())
        + "," + Over.class.getMethod("f", long.class).invoke(new OverSub(), 3L) + "," + I1.class.getMethod("d").invoke(new Dia())
        + "," + GI.class.getMethod("g").invoke(new GSub()) + "," + p.invoke(privSub)
        + "," + StaticBase.class.getMethod("s").invoke(new StaticSub()) + "," + Sub.class.getMethod("foo").invoke(new Sub());
  }

  static String lookupNamed(Class<?> c, String name, boolean declared) {
    try {
      return "found " + (declared ? c.getDeclaredMethod(name) : c.getMethod(name));
    } catch (NoSuchMethodException e) {
      return "NSME:" + e.getMessage();
    }
  }

  /** Constructors and initializers are not methods. */
  public static String initNotMethod() {
    return lookupNamed(Super.class, "<init>", false) + "|" + lookupNamed(Super.class, "<init>", true)
        + "|" + lookupNamed(Refl.class, "<clinit>", true);
  }

  static String methodSet(Class<?> c) {
    Set<String> names = new TreeSet<>();
    for (Method m : c.getMethods()) {
      names.add(m.getDeclaringClass().getName() + "." + m.getName() + "/" + m.getParameterCount());
    }
    return c.getMethods().length + ":" + String.join(";", names);
  }

  static String fieldSet(Class<?> c) {
    Set<String> names = new TreeSet<>();
    for (Field f : c.getFields()) {
      names.add(f.getDeclaringClass().getName() + "." + f.getName());
    }
    return c.getFields().length + ":" + String.join(";", names);
  }

  /** getMethods: public members of the class, superclasses (Object too) and superinterfaces, as sets. */
  public static String publicMethods() {
    return methodSet(Sub.class) + "|" + methodSet(Api.class) + "|" + methodSet(GSub.class) + "|" + methodSet(Dia.class);
  }

  /** getFields: public fields of the class, superinterfaces and superclasses, as sets. */
  public static String publicFields() {
    return fieldSet(FSub.class) + "|" + fieldSet(FBase.class) + "|" + fieldSet(Consts.class);
  }

  /** A JDK class's members are not in any class file the run holds: counted. */
  public static String jdkMethods() { return "" + String.class.getMethods().length; }

  public static String jdkFields() { return "" + Integer.class.getFields().length; }

  public static String jdkDeclaredMethods() { return "" + String.class.getDeclaredMethods().length; }

  public static void main(String[] args) throws Exception {
    for (String name : new String[] {"returnJdk", "returnJdkUnknown", "returnKnown", "virtualInvoke", "initNotMethod", "publicMethods", "publicFields", "jdkMethods", "jdkFields", "jdkDeclaredMethods", "getMethodRules", "methodModifiers", "jdkInvoke", "jdkNewInstance", "spelled", "text", "real", "collections", "sameHandle", "fieldCopies", "methodCopies", "constructorCopies", "fieldStream", "methodMap", "forgedNames", "forgedCopies", "inheritedFields", "jdkFieldModifiers", "jdkFieldDeclaring"}) {
      System.out.println("== " + name);
      System.out.println(Refl.class.getMethod(name).invoke(null));
    }
  }
}

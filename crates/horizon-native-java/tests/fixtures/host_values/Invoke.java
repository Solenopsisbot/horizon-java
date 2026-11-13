package demo;

import java.lang.invoke.*;
import java.lang.reflect.*;
import java.util.*;
import java.util.function.Supplier;

/** MethodType, MethodHandle, Lookup and CallSite as host handles (#156 P10b): each method is one case. */
public class Invoke {
  public static String s(int x) { return "s" + x; }
  public String v(String a) { return "v" + a; }
  public static String hello() { return "hi"; }

  static MethodType type() { return MethodType.methodType(String.class, int.class); }
  static MethodHandle handle() throws Exception { return MethodHandles.lookup().findStatic(Invoke.class, "s", type()); }

  static String castType(Object o) {
    try { return "ok" + ((MethodType) o).parameterCount(); } catch (ClassCastException e) { return e.getMessage(); }
  }

  /** Strings spelled like the old markers, or the handles' encodings, stay Strings. */
  public static String spelled() throws Exception {
    Object[] texts = {
      "method-type/(I)Ljava/lang/String;", "method-handle/virtual/-/reflect-method/demo/Invoke#s(I)Ljava/lang/String;",
      "java/lang/invoke/MethodHandle#mh{}", "java/lang/invoke/ConstantCallSite#cs{}",
      "method-handles-lookup", "hmtype:(I)Ljava/lang/String;", "hmhlk:rpublic,null"
    };
    MethodType t = type();
    MethodHandle h = handle();
    MethodHandles.Lookup p = MethodHandles.publicLookup();
    StringBuilder out = new StringBuilder();
    Map<Object, String> map = new HashMap<>();
    for (Object x : texts) {
      // Text with a `#` is an object of the class before it to `instanceof`
      // until #156 P12 (the `class#n` rule), so those spellings skip it.
      if (!x.toString().contains("#")) {
        out.append(x instanceof MethodType).append(x instanceof MethodHandle).append(x instanceof MethodHandles.Lookup)
            .append(x instanceof CallSite);
      }
      out.append(x.equals(t)).append(t.equals(x)).append(h.equals(x)).append(p.equals(x))
          .append('|').append("" + x).append(',');
      // `word/rest` text is still classified by is_plain_string until #156 P17,
      // and any text with a `#` is an object to getClass until P12.
      if (!x.toString().contains("#") && !x.toString().startsWith("method-")
          || x.equals("method-handles-lookup")) {
        out.append(x instanceof String).append(x.getClass().getName()).append(x.hashCode()).append(',');
        map.put(x, "s");
      }
    }
    map.put(t, "t");
    return out.append(map.size()).append(map.get("method-handles-lookup")).append(map.get("hmtype:(I)Ljava/lang/String;"))
        .append('|').append(castType("hmtype:(I)Ljava/lang/String;")).append('|').append(castType(t)).toString();
  }

  /** MethodType: interned, printed with simple names, its own queries. */
  public static String types() {
    MethodType t = type();
    MethodType arr = MethodType.methodType(Map.Entry[].class, int[].class, List.class, String[][].class);
    Object o = t;
    return t + "|" + arr + "|" + MethodType.methodType(void.class) + "|" + (t == type()) + t.equals(type()) + t.equals(arr)
        + "|" + t.parameterCount() + t.returnType().getName() + t.parameterType(0) + t.toMethodDescriptorString()
        + "|" + t.changeReturnType(Object.class) + "|" + o.getClass().getName() + (o instanceof java.lang.constant.Constable)
        + (o instanceof java.io.Serializable) + (o instanceof TypeDescriptor.OfMethod) + (o instanceof Comparable);
  }

  /** MethodHandle: `MethodHandle` + its type, its uses; equal only to itself. */
  public static String handles() throws Throwable {
    MethodHandle h = handle();
    MethodHandle v = MethodHandles.lookup().findVirtual(Invoke.class, "v", MethodType.methodType(String.class, String.class));
    MethodHandle b = v.bindTo(new Invoke());
    Object o = h;
    return h + "|" + v + "|" + b + "|" + h.type() + (h.type() == type()) + "|" + h.invoke(5) + b.invoke("x") + h.invokeWithArguments(7)
        + (String) h.invokeExact(3) + "|" + h.equals(h) + h.equals(type()) + "|" + (o instanceof MethodHandle)
        + (o instanceof java.lang.constant.Constable);
  }

  /** Lookup: the public singleton prints and compares; a privateLookupIn prints its class. */
  public static String lookups() throws Exception {
    MethodHandles.Lookup p = MethodHandles.publicLookup();
    Object o = p;
    return p + "|" + (p == MethodHandles.publicLookup()) + p.equals(MethodHandles.publicLookup()) + "|"
        + MethodHandles.privateLookupIn(Invoke.class, MethodHandles.lookup()) + "|" + o.getClass().getName()
        + (o instanceof MethodHandles.Lookup);
  }

  /** The same handle on both sides is one object; method types are interned. */
  public static String sameHandle() throws Exception {
    MethodHandle h = handle();
    Object g = h;
    MethodHandles.Lookup l = MethodHandles.lookup();
    Object m = l;
    Map<Object, Integer> identity = new IdentityHashMap<>();
    identity.put(type(), 1);
    identity.put(type(), 2);
    identity.put(h, 3);
    identity.put(g, 4);
    return "" + (h == g) + (l == m) + (type() == type()) + identity.size();
  }

  /** Two lookups of one method are two MethodHandles: Java says false; counted. */
  public static String twoHandles() throws Exception { return "" + (handle() == handle()); }

  /** ... and not equal (identity): counted. */
  public static String twoHandlesEqual() throws Exception { return "" + handle().equals(handle()); }

  /** Two MethodHandles.lookup() are two objects: counted. */
  public static String twoLookups() { return "" + (MethodHandles.lookup() == MethodHandles.lookup()); }

  /** A handle's exact class (DirectMethodHandle) is not known: counted. */
  public static String handleClass() throws Exception { return handle().getClass().getName(); }

  /** MethodType.hashCode mixes Class identity hashes: counted. */
  public static String typeHash() { return "" + (type().hashCode() == type().hashCode()); }

  /** lookup()'s class is its caller, which the runtime does not record: counted. */
  public static String callerLookup() { return MethodHandles.lookup().toString(); }

  /** Type switches, patterns, Class.isInstance and host collections reach the kind's row. */
  public static String generic() throws Exception {
    Object[] values = {type(), handle(), MethodHandles.publicLookup(), "x"};
    StringBuilder out = new StringBuilder();
    for (Object x : values) {
      out.append(switch (x) {
        case MethodType t -> "type" + t.parameterCount();
        case MethodHandle h -> "handle";
        case MethodHandles.Lookup l -> "lookup";
        default -> "other";
      }).append(',');
    }
    Object t = type();
    return out + "|" + (t instanceof MethodType mt && mt.parameterCount() == 1) + MethodType.class.isInstance(t)
        + MethodHandle.class.isInstance(t) + "|" + List.of(type()) + List.of(type()).contains(type()) + "|"
        + Map.of(type(), 1).get(type()) + new ArrayList<>(List.of(type())).indexOf(type());
  }

  /** A HashSet of method types hashes them (identity hashes): counted. */
  public static String typeSet() { return "" + new HashSet<>(List.of(type(), type())).size(); }

  public interface Cfg {
    int size();
    default int twice() { return size() * 2; }
  }

  /** DazzleConf's default-method path: privateLookupIn, unreflectSpecial, bindTo, invokeWithArguments; and a virtual handle on the proxy. */
  public static String proxies() throws Throwable {
    InvocationHandler handler = new InvocationHandler() {
      public Object invoke(Object proxy, Method method, Object[] args) throws Throwable {
        if (method.isDefault()) {
          return MethodHandles.privateLookupIn(Cfg.class, MethodHandles.lookup()).unreflectSpecial(method, Cfg.class)
              .bindTo(proxy).invokeWithArguments(args);
        }
        return 21;
      }
    };
    Cfg cfg = (Cfg) Proxy.newProxyInstance(Invoke.class.getClassLoader(), new Class<?>[] {Cfg.class}, handler);
    MethodHandle size = MethodHandles.lookup().findVirtual(Cfg.class, "size", MethodType.methodType(int.class));
    return cfg.twice() + "|" + size.invoke(cfg) + "|" + size.type().parameterCount();
  }

  /** LambdaMetafactory called directly: a CallSite whose target spins a Supplier. */
  @SuppressWarnings("unchecked")
  public static String metafactory() throws Throwable {
    MethodHandles.Lookup l = MethodHandles.lookup();
    CallSite site = LambdaMetafactory.metafactory(l, "get", MethodType.methodType(Supplier.class),
        MethodType.methodType(Object.class), l.findStatic(Invoke.class, "hello", MethodType.methodType(String.class)),
        MethodType.methodType(String.class));
    Supplier<String> supplier = (Supplier<String>) site.getTarget().invoke();
    return supplier.get() + "|" + site.getTarget().type() + "|" + (site instanceof CallSite);
  }

  static String ex(Throwable e) {
    return e.getClass().getName() + ":" + e.getMessage();
  }

  static CallSite site() throws Throwable {
    MethodHandles.Lookup l = MethodHandles.lookup();
    return LambdaMetafactory.metafactory(l, "get", MethodType.methodType(Supplier.class),
        MethodType.methodType(Object.class), l.findStatic(Invoke.class, "hello", MethodType.methodType(String.class)),
        MethodType.methodType(String.class));
  }

  /** Two lookups' handles in a HashSet: Java keeps both; never merged by key (counted). */
  public static String handleSet() throws Exception { return "" + new HashSet<>(List.of(handle(), handle())).size(); }

  /** ... a HashMap key looked up with another lookup's handle (Java: false; counted). */
  public static String handleMapKey() throws Exception {
    Map<Object, Integer> m = new HashMap<>();
    m.put(handle(), 1);
    return "" + m.containsKey(handle());
  }

  /** ... List.contains/indexOf (Java: false, -1; counted). */
  public static String handleListContains() throws Exception {
    return "" + new ArrayList<>(List.of(handle())).contains(handle()) + List.of(handle()).indexOf(handle());
  }

  /** ... Set.of/Map.of keep both (Java: 2, 2; counted, never "duplicate"). */
  public static String handleSetOf() throws Exception { return "" + Set.of(handle(), handle()).size(); }

  public static String handleMapOf() throws Exception { return "" + Map.of(handle(), 1, handle(), 2).size(); }

  /** Two privateLookupIn lookups and two call sites are distinct objects too (Java: 2, 2; counted). */
  public static String lookupSet() throws Exception {
    return "" + new HashSet<>(List.of(MethodHandles.privateLookupIn(Invoke.class, MethodHandles.lookup()),
        MethodHandles.privateLookupIn(Invoke.class, MethodHandles.lookup()))).size();
  }

  public static String siteSet() throws Throwable { return "" + new HashSet<>(List.of(site(), site())).size(); }

  /** unreflectSpecial types the receiver as the special caller. */
  public static String specialCaller() throws Throwable {
    MethodHandle h = MethodHandles.privateLookupIn(TopImpl.class, MethodHandles.lookup())
        .unreflectSpecial(TopD.class.getMethod("d"), TopImpl.class);
    return h + "|" + h.type() + "|" + h.invoke(new TopImpl()) + "|" + h.bindTo(new TopImpl()) + "|" + h.bindTo(new TopImpl()).invoke();
  }

  static String privateIn(Class<?> c, MethodHandles.Lookup caller) {
    try { return "" + MethodHandles.privateLookupIn(c, caller); } catch (Throwable e) { return ex(e); }
  }

  /** privateLookupIn throws for a primitive, an array, a caller without full privilege. */
  public static String privateLookups() {
    return privateIn(int.class, MethodHandles.lookup()) + "|" + privateIn(String[].class, MethodHandles.lookup()) + "|"
        + privateIn(Invoke.class, MethodHandles.publicLookup()) + "|" + privateIn(int.class, MethodHandles.publicLookup())
        + "|" + privateIn(TopImpl.class, MethodHandles.lookup());
  }

  /** ... and on a JDK class it depends on the launch's modules: counted (Java: IllegalAccessException). */
  public static String privateLookupJdk() { return privateIn(Object.class, MethodHandles.lookup()).replaceAll("@[0-9a-f]+", "@H"); }

  /** Nothing to drop or insert: the same handle. */
  public static String noOpCombinators() throws Exception {
    MethodHandle h = handle();
    MethodHandle d = MethodHandles.dropArguments(h, 0);
    MethodHandle i = MethodHandles.insertArguments(h, 0);
    return "" + (d == h) + d.equals(h) + (i == h) + i.equals(h);
  }

  /** methodType(Class, List) reads the list; a void parameter throws. */
  public static String typeFromList() {
    MethodType t = MethodType.methodType(String.class, List.of(int.class));
    String v;
    try { v = "" + MethodType.methodType(String.class, void.class); } catch (Throwable e) { v = ex(e); }
    return t + "|" + (t == type()) + "|" + v;
  }

  /** A wrong arity for invokeWithArguments: Java's message names the handle. */
  public static String wrongArity() throws Throwable { return "" + handle().invokeWithArguments(1, 2); }

  /** Null arguments throw Java 25's helpful NullPointerExceptions. */
  public static String nullArguments() {
    String out = privateIn(null, MethodHandles.lookup()) + "|" + privateIn(Invoke.class, null);
    try { out += "|" + MethodType.methodType(null, int.class); } catch (Throwable e) { out += "|" + ex(e); }
    try { out += "|" + MethodType.methodType(String.class, int.class, null); } catch (Throwable e) { out += "|" + ex(e); }
    return out;
  }

  /** The metafactory's call site is a ConstantCallSite. */
  public static String siteClass() throws Throwable { return site().getClass().getName(); }

  public static void main(String[] args) throws Throwable {
    for (String name : new String[] {"spelled", "types", "handles", "lookups", "sameHandle", "twoHandles", "twoHandlesEqual", "twoLookups",
        "handleClass", "typeHash", "callerLookup", "generic", "typeSet", "proxies", "metafactory", "handleSet", "handleMapKey",
        "handleListContains", "handleSetOf", "handleMapOf", "lookupSet", "siteSet", "specialCaller", "privateLookups",
        "privateLookupJdk", "noOpCombinators", "typeFromList", "wrongArity", "siteClass", "nullArguments"}) {
      System.out.println("== " + name);
      try {
        System.out.println(Invoke.class.getMethod(name).invoke(null));
      } catch (java.lang.reflect.InvocationTargetException e) {
        System.out.println("EX " + ex(e.getCause()));
      }
    }
  }
}

interface TopD {
  default String d() { return "dflt"; }
}

class TopImpl implements TopD {
}

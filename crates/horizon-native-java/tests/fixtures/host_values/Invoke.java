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

  public static void main(String[] args) throws Throwable {
    for (String name : new String[] {"spelled", "types", "handles", "lookups", "sameHandle", "twoHandles", "twoHandlesEqual", "twoLookups",
        "handleClass", "typeHash", "callerLookup", "generic", "typeSet", "proxies", "metafactory"}) {
      System.out.println("== " + name);
      System.out.println(Invoke.class.getMethod(name).invoke(null));
    }
  }
}

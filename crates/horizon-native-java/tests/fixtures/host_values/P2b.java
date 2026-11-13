package demo;

import java.util.*;
import org.apache.logging.log4j.LogManager;
import org.slf4j.LoggerFactory;

/**
 * #156 P2 (second half): the main thread, loggers, stack walking and modules
 * as host handles. Each method is one case; the expected strings are what
 * OpenJDK 25.0.5 printed for it.
 */
public final class P2b {
  static String ex(Throwable e) { return "EXC " + e.getClass().getName(); }

  /** Strings spelled like an old marker with no `word/` head stay Strings. */
  static String plain(Object t, Object real) {
    Map<Object, String> m = new HashMap<>();
    m.put(t, "s");
    m.put(real, "r");
    return t.getClass().getName() + (t instanceof String) + t.equals(real) + real.equals(t) + t.hashCode()
        + "|" + ("" + t) + "|" + t.toString() + "|" + m.size() + m.get(t) + m.get(real);
  }

  /** The same, for a spelling with a `word/` head (getClass/instanceof/hashCode wait for #156 P17). */
  static String worded(Object t, Object real) {
    Map<Object, String> m = new HashMap<>();
    m.put(t, "s");
    m.put(real, "r");
    return t.equals(real) + "" + real.equals(t) + "|" + ("" + t) + "|" + t.toString() + "|" + m.size() + m.get(t) + m.get(real);
  }

  static String wordedClass(Object t) {
    return t.getClass().getName() + (t instanceof String) + t.hashCode();
  }

  public static String threadSpelled() {
    try { return worded("thread/main", Thread.currentThread()); } catch (Throwable e) { return ex(e); }
  }

  public static String threadReal() {
    try {
      Thread c = Thread.currentThread();
      Object o = c;
      Map<Object, Integer> m = new IdentityHashMap<>();
      m.put(Thread.currentThread(), 1);
      m.put(Thread.currentThread(), 2);
      return c.getName() + "|" + (c == Thread.currentThread())
          + c.equals(Thread.currentThread()) + c.equals("thread/main") + "|" + (o instanceof Runnable) + (o instanceof Thread)
          + (o instanceof String) + o.getClass().getName() + "|" + m.size();
    } catch (Throwable e) { return ex(e); }
  }

  /** A thread's text names its id, which is the process's. */
  public static String threadText() { return "" + Thread.currentThread(); }

  public static String loggerSpelled() {
    try { return worded("logger/demo", LogManager.getLogger("demo")); } catch (Throwable e) { return ex(e); }
  }

  public static String loggerReal() {
    try {
      Object a = LogManager.getLogger("demo");
      Object j = java.util.logging.Logger.getLogger("demo");
      return "" + (a == LogManager.getLogger("demo")) + a.equals(LogManager.getLogger("demo")) + (a == LogManager.getLogger("other"))
          + a.equals(j) + LogManager.getLogger("demo").getName() + "|" + (a instanceof org.apache.logging.log4j.Logger)
          + (a instanceof String) + "|" + (j == java.util.logging.Logger.getLogger("demo")) + j.equals(java.util.logging.Logger.getLogger("demo"))
          + (j instanceof java.util.logging.Logger) + java.util.logging.Logger.getLogger("demo").getName();
    } catch (Throwable e) { return ex(e); }
  }

  /** Two anonymous loggers are two objects: identity is counted. */
  public static String loggerTwo() {
    return "" + (java.util.logging.Logger.getAnonymousLogger() == java.util.logging.Logger.getAnonymousLogger());
  }

  public static String stackSpelled() {
    try {
      return plain("stack-walker", StackWalker.getInstance()) + "|" + worded("stack-frame/stackSpelled@demo/P2", "x");
    } catch (Throwable e) { return ex(e); }
  }

  public static String stackReal() {
    try {
      StackWalker w = StackWalker.getInstance();
      Object o = w;
      StackWalker.StackFrame f = w.walk(s -> s.findFirst().get());
      Object fo = f;
      StackTraceElement e = new Throwable().getStackTrace()[0];
      Object eo = e;
      return "" + (w == StackWalker.getInstance()) + w.equals(StackWalker.getInstance()) + (o instanceof StackWalker)
          + (o instanceof String) + o.getClass().getName() + "|" + f.getMethodName() + f.getClassName() + (fo == f) + fo.equals(f)
          + (fo instanceof StackWalker.StackFrame) + (fo instanceof String) + fo.getClass().getName() + "|" + e.getMethodName()
          + e.getClassName() + (eo == e) + eo.equals(e) + e.equals("stackReal") + (eo instanceof StackTraceElement)
          + (eo instanceof String) + eo.getClass().getName();
    } catch (Throwable e) { return ex(e); }
  }

  /** Two walks hand out two frame objects: identity is counted. */
  public static String stackTwo() {
    StackWalker w = StackWalker.getInstance();
    return "" + (w.walk(s -> s.findFirst().get()) == w.walk(s -> s.findFirst().get()));
  }

  /** Two elements of one method made by two throwables: equals needs their lines, counted. */
  public static String stackElements() {
    StackTraceElement a = new Throwable().getStackTrace()[0];
    StackTraceElement b = new Throwable().getStackTrace()[0];
    return "" + a.equals(b);
  }

  /** A stack trace element's text: Java names the file and line, which the runtime does not keep. */
  public static String stackText() {
    StackTraceElement e = new Throwable().getStackTrace()[0];
    return e + "|" + e.toString();
  }

  /** An unnamed module prints its identity hash. */
  public static String moduleText() { return "" + P2b.class.getModule(); }

  public static String moduleSpelled() {
    try { return worded("module/unnamed", P2b.class.getModule()) + "|" + worded("module/java.base", String.class.getModule()); }
    catch (Throwable e) { return ex(e); }
  }

  public static String moduleReal() {
    try {
      Module m = P2b.class.getModule();
      Module b = String.class.getModule();
      Object o = b;
      Map<Object, Integer> id = new IdentityHashMap<>();
      id.put(m, 1);
      id.put(P2b.class.getModule(), 2);
      return "" + (m == P2b.class.getModule()) + (m == b) + m.equals(P2b.class.getModule()) + m.equals(b) + m.isNamed()
          + b.isNamed() + b.getName() + "|" + b + "|" + ("" + b) + b.toString() + "|" + (o instanceof Module) + (o instanceof String)
          + o.getClass().getName() + "|" + id.size();
    } catch (Throwable e) { return ex(e); }
  }
  /** Spellings with a `word/` head: is_plain_string still reads them as markers (#156 P17). */
  public static String wordedClasses() {
    return wordedClass("thread/main") + wordedClass("logger/demo") + wordedClass("module/unnamed")
        + wordedClass("stack-frame/m@demo/P2b") + wordedClass("stack-trace-element/m@demo/P2b");
  }

  /** Type switches reach each kind's row. */
  public static String genSwitch() {
    StackWalker w = StackWalker.getInstance();
    Object[] xs = {Thread.currentThread(), w, String.class.getModule(), new Throwable().getStackTrace()[0],
        w.walk(s -> s.findFirst().get()), "thread/main"};
    StringBuilder sb = new StringBuilder();
    for (Object x : xs) {
      sb.append(switch (x) {
        case String s -> "S";
        case Runnable r -> "R";
        case StackWalker s -> "W";
        case Module m -> "M";
        case StackTraceElement e -> "E";
        case StackWalker.StackFrame f -> "F";
        case org.apache.logging.log4j.Logger l -> "L";
        case java.util.logging.Logger l -> "J";
        default -> "D";
      });
    }
    return sb.toString();
  }

  /** A logger's class is its backend's: a label it might implement (`Runnable`) is counted. */
  public static String loggerSwitch() {
    Object[] xs = {LogManager.getLogger("demo"), java.util.logging.Logger.getLogger("demo")};
    StringBuilder sb = new StringBuilder();
    for (Object x : xs) {
      sb.append(switch (x) {
        case String s -> "S";
        case Runnable r -> "R";
        case org.apache.logging.log4j.Logger l -> "L";
        case java.util.logging.Logger l -> "J";
        default -> "D";
      });
    }
    return sb.toString();
  }

  /** A logger matched by its declared type, with no label it might be first. */
  public static String loggerDeclared() {
    Object l = LogManager.getLogger("demo");
    Object j = java.util.logging.Logger.getLogger("demo");
    String a = switch (l) { case String s -> "S"; case org.apache.logging.log4j.Logger x -> "L" + x.getName(); default -> "D"; };
    String b = switch (j) { case String s -> "S"; case java.util.logging.Logger x -> "J" + x.getName(); default -> "D"; };
    return a + b + (l instanceof org.apache.logging.log4j.Logger x ? x.getName() : "-");
  }

  /** Class.isInstance / isAssignableFrom / instanceof on each kind. */
  public static String genClassChecks() {
    Object t = Thread.currentThread();
    Object w = StackWalker.getInstance();
    Object m = String.class.getModule();
    Object e = new Throwable().getStackTrace()[0];
    Object l = LogManager.getLogger("demo");
    Object j = java.util.logging.Logger.getLogger("demo");
    return "" + Thread.class.isInstance(t) + Runnable.class.isInstance(t) + String.class.isInstance(t) + StackWalker.class.isInstance(w)
        + Module.class.isInstance(m) + java.lang.reflect.AnnotatedElement.class.isInstance(m) + StackTraceElement.class.isInstance(e)
        + java.io.Serializable.class.isInstance(e) + String.class.isInstance(e) + "|" + Runnable.class.isAssignableFrom(t.getClass())
        + Thread.class.isAssignableFrom(t.getClass()) + String.class.isAssignableFrom(w.getClass()) + Object.class.isAssignableFrom(m.getClass())
        + java.lang.reflect.AnnotatedElement.class.isAssignableFrom(m.getClass()) + java.io.Serializable.class.isAssignableFrom(e.getClass())
        + "|" + (l instanceof org.apache.logging.log4j.Logger) + (l instanceof String) + org.apache.logging.log4j.Logger.class.isInstance(l)
        + (j instanceof java.util.logging.Logger) + (j instanceof String) + "|" + t.getClass().getName() + w.getClass().getName()
        + m.getClass().getName() + e.getClass().getName();
  }

  /** A logger's exact class is its backend's: counted. */
  public static String loggerClass() { return LogManager.getLogger("demo").getClass().getName(); }

  /** None of these kinds is Comparable: Java throws ClassCastException, the runtime counts. */
  public static String genOrder() {
    try {
      List<StackTraceElement> l = new ArrayList<>(List.of(new Throwable().getStackTrace()[0], new Throwable().getStackTrace()[0]));
      Collections.sort((List) l);
      return "sorted";
    } catch (ClassCastException e) { return "CCE"; }
  }

  public static String genTreeSet() {
    try { new TreeSet<Object>().add(Thread.currentThread()); return "added"; } catch (ClassCastException e) { return "CCE"; }
  }

  /** Inside host collections. */
  public static String genCollections() {
    Module b = String.class.getModule();
    Map<Object, Integer> m = new HashMap<>();
    m.put(b, 1);
    return List.of(b) + "|" + Map.of(b, 1) + "|" + m.get(String.class.getModule()) + m.containsKey(b) + List.of(b).contains(b)
        + new HashSet<>(List.of(b)).contains(String.class.getModule()) + new ArrayList<>(List.of(b)).indexOf(b);
  }

  /** A thread's hash is an identity hash: a HashSet of it is counted. */
  public static String genThreadSet() { return "" + new HashSet<>(List.of(Thread.currentThread())).contains(Thread.currentThread()); }

  /** Names with a key's separators stay their own literal names. */
  public static String loggerNames() {
    String[] names = {"a,b", "a\\b", "x@y", "a/b", "rlog4j,ra"};
    StringBuilder sb = new StringBuilder();
    for (String n : names) sb.append(LogManager.getLogger(n).getName()).append('|').append(java.util.logging.Logger.getLogger(n).getName()).append(';');
    return sb.toString() + (LogManager.getLogger("a,b") == LogManager.getLogger("a\\b"));
  }

  /** A member class, for loggers named by a class. */
  public static final class Inner {}

  /** `getInstance(Set)` of an empty set is the default walker (#173 B1). */
  public static String walkerEmptySet() {
    StackWalker d = StackWalker.getInstance();
    return "" + (StackWalker.getInstance(Set.of()) == d) + (StackWalker.getInstance(new HashSet<>()) == d)
        + (StackWalker.getInstance(Set.of(), 5) == d) + (StackWalker.getInstance(Set.of()) == StackWalker.getInstance(Set.of()));
  }

  /** log4j's getLogger() names the caller's class; Class/Object overloads use the canonical name. */
  public static String log4jNames() {
    Object anonymous = new Object() {};
    return LogManager.getLogger().getName() + "|" + (LogManager.getLogger() == LogManager.getLogger("demo.P2b"))
        + (LogManager.getLogger() == LogManager.getRootLogger()) + "|" + LogManager.getLogger(Inner.class).getName()
        + (LogManager.getLogger(Inner.class) == LogManager.getLogger("demo.P2b.Inner"))
        + (LogManager.getLogger(Inner.class) == LogManager.getLogger("demo.P2b$Inner")) + "|" + LogManager.getLogger(P2b.class).getName()
        + "|" + LogManager.getLogger(anonymous.getClass()).getName() + "|" + LogManager.getLogger((Object) "x").getName()
        + LogManager.getLogger((Object) new Inner()).getName() + (LogManager.getLogger((Object) "x") == LogManager.getLogger("x"))
        + "|" + LogManager.getFormatterLogger().getName() + (LogManager.getFormatterLogger() == LogManager.getLogger());
  }

  /** Mojang's LogUtils.getLogger() is slf4j's logger of the caller's class. */
  public static String logUtilsName() {
    return com.mojang.logging.LogUtils.getLogger().getName() + (com.mojang.logging.LogUtils.getLogger() == LoggerFactory.getLogger(P2b.class))
        + LoggerFactory.getLogger(Inner.class).getName();
  }

  /** A JDK class is in the module Java 25 puts its package in; an array in its element's. */
  public static String moduleNames() {
    Module b = String.class.getModule();
    Module l = java.util.logging.Logger.class.getModule();
    return l.getName() + l.isNamed() + l + "|" + (l == b) + (l == java.util.logging.Level.class.getModule()) + l.equals(b) + "|"
        + java.sql.Connection.class.getModule().getName() + "|" + P2b[].class.getModule().getName() + int[].class.getModule().getName()
        + (String[].class.getModule() == b) + (P2b[][].class.getModule() == P2b.class.getModule()) + "|"
        + P2b.class.getModule().getDescriptor() + "|" + l.getLayer().equals(b.getLayer());
  }

  /** A named module's descriptor is an object the runtime does not model: counted. */
  public static String moduleDescriptor() { return "" + (String.class.getModule().getDescriptor() != null); }

  /** A module other than java.base: its loader and exports are counted. */
  public static String moduleExports() { return "" + java.util.logging.Logger.class.getModule().isExported("java.util.logging"); }

  /** Unmodelled Thread members are counted, not run stoppers. */
  public static String threadMembers() {
    Thread t = Thread.currentThread();
    return "" + t.isAlive() + t.getPriority() + t.getState() + t.isInterrupted() + t.isVirtual();
  }

  /** log4j's getLogger(null name) is the caller's logger. */
  public static String log4jNullName() { return LogManager.getLogger((String) null).getName(); }

  /** A MessageFactory overload: one logger per name and factory, counted (the logger still answers). */
  public static String log4jFactory() {
    return LogManager.getLogger("x", new org.apache.logging.log4j.message.ParameterizedMessageFactory()).getName();
  }

  /** Spelled like a stack trace element; the element's hash (file and line) is counted. */
  public static String steSpelled() {
    try { return worded("stack-trace-element/stackSpelled@demo/P2", new Throwable().getStackTrace()[0]); } catch (Throwable e) { return ex(e); }
  }
}

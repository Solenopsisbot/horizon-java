import java.lang.module.ModuleFinder;
import java.lang.reflect.Modifier;
import java.util.Set;
import java.util.TreeSet;

/**
 * Prints `src/jdk_supertypes_25.txt`: for each class the runtime's JDK hierarchy answers for, its
 * flags, every supertype (getSuperclass/getInterfaces, transitively, the class itself left out),
 * its direct superclass (`-` for none), its direct interfaces in declaration order, and which of
 * `equals`/`hashCode`/`toString`/`compareTo` its objects run some JDK class's own body for
 * rather than `Object`'s (`-` for none, and for an interface), then every package of every system module and the module that holds it (a JDK class's
 * supertypes can only come from these).
 * Run on OpenJDK 25: `java JdkSupertypes.java > ../../../src/jdk_supertypes_25.txt`.
 */
public final class JdkSupertypes {
  static final String[] CLASSES = {
    // jdk_model::platform_supertypes
    "java.lang.Integer", "java.lang.Long", "java.lang.Short", "java.lang.Byte", "java.lang.Float",
    "java.lang.Double", "java.lang.Boolean", "java.lang.Character", "java.util.ArrayList",
    "java.util.Vector", "java.util.Stack", "java.util.concurrent.CopyOnWriteArrayList",
    "java.util.LinkedList", "java.util.HashSet", "java.util.concurrent.CopyOnWriteArraySet",
    "java.util.LinkedHashSet", "java.util.TreeSet", "java.util.concurrent.ConcurrentSkipListSet",
    "java.util.HashMap", "java.util.IdentityHashMap", "java.util.WeakHashMap", "java.util.EnumMap",
    "java.util.Hashtable", "java.util.LinkedHashMap", "java.util.TreeMap",
    "java.util.concurrent.ConcurrentSkipListMap", "java.util.concurrent.ConcurrentHashMap",
    "java.util.ArrayDeque", "java.util.concurrent.ConcurrentLinkedDeque", "java.util.PriorityQueue",
    "java.util.concurrent.ConcurrentLinkedQueue", "java.io.File",
    // java_immutable::supertypes
    "java.util.ImmutableCollections$ListN", "java.util.Collections$UnmodifiableRandomAccessList",
    "java.util.ImmutableCollections$SetN", "java.util.Collections$UnmodifiableSet",
    "java.util.ImmutableCollections$MapN", "java.util.Collections$UnmodifiableMap",
    // the classes the host's collections are made as (#156 P14c)
    "java.util.ImmutableCollections$List12", "java.util.ImmutableCollections$Set12",
    "java.util.ImmutableCollections$Map1", "java.util.Collections$UnmodifiableList",
    "java.util.Collections$EmptyList", "java.util.Collections$EmptySet", "java.util.Collections$EmptyMap",
    "java.util.Collections$SingletonList", "java.util.Collections$SingletonSet",
    "java.util.Collections$SingletonMap",
    // host_ref's closed classes
    "java.lang.Object", "java.lang.String", "java.lang.Class", "java.lang.Record", "java.lang.Enum",
    "java.lang.Number", "java.util.Optional", "java.util.OptionalInt", "java.util.OptionalLong",
    "java.util.OptionalDouble",
    // host_jdk's kinds (#156 P2)
    "java.lang.Thread", "java.lang.StackWalker", "java.lang.StackFrameInfo",
    "java.lang.StackTraceElement", "java.lang.Module", "java.util.logging.Logger",
    "java.lang.System$Logger",
    // host_invoke's and method_handles' kinds (#156 P10b), and CharSequence for getMethod's
    // most specific return type
    "java.lang.invoke.MethodType", "java.lang.invoke.MethodHandle",
    "java.lang.invoke.MethodHandles$Lookup", "java.lang.invoke.ConstantCallSite",
    "java.lang.CharSequence",
    // The private JDK classes the skeletons allocate for a jar's collection (`jdk_skeletons/`).
    "java.util.AbstractList$Itr", "java.util.AbstractList$ListItr", "java.util.AbstractList$SubList",
    "java.util.AbstractList$RandomAccessSubList", "java.util.AbstractList$SubList$1",
    "java.util.AbstractMap$1", "java.util.AbstractMap$2", "java.util.AbstractMap$KeyIterator",
    "java.util.AbstractMap$ValueIterator", "java.util.AbstractSequentialList",
    "java.util.AbstractMap$SimpleImmutableEntry",
    // The JDK classes jars construct (`new`, or `super()` from a subclass) in at least 10
    // corpus jars (corpus-xl, mega, neoforge; constructor references in the class files), and
    // the interfaces they are cast to: a jar object of one is answered exactly (#156 P12b1).
    "java.lang.IllegalArgumentException", "java.lang.RuntimeException",
    "java.lang.IllegalStateException", "java.lang.StringBuilder", "java.lang.AssertionError",
    "java.lang.MatchException", "java.lang.UnsupportedOperationException",
    "java.lang.NullPointerException", "java.io.InputStreamReader", "java.io.IOException",
    "java.util.concurrent.atomic.AtomicInteger", "java.io.BufferedReader",
    "java.lang.Exception", "java.util.concurrent.atomic.AtomicBoolean",
    "java.util.NoSuchElementException", "java.lang.IndexOutOfBoundsException",
    "java.io.FileInputStream", "java.util.Random",
    "java.util.concurrent.atomic.AtomicReference", "java.io.OutputStreamWriter",
    "java.util.Properties", "java.io.StringReader", "java.lang.ThreadLocal",
    "java.io.FileWriter", "java.io.ByteArrayInputStream", "java.lang.ref.WeakReference",
    "java.io.FileReader", "java.net.URI", "java.util.concurrent.atomic.AtomicLong",
    "java.io.BufferedWriter", "java.io.ByteArrayOutputStream",
    "java.util.concurrent.CompletableFuture", "java.io.FileOutputStream",
    "java.io.StringWriter", "java.text.DecimalFormat", "java.math.BigInteger",
    "java.math.BigDecimal", "java.io.PrintWriter", "java.util.AbstractList",
    "java.util.BitSet", "java.lang.ClassCastException", "java.util.Date",
    "java.util.AbstractCollection", "java.io.BufferedInputStream",
    "java.util.AbstractMap$SimpleEntry", "java.net.URL", "java.text.SimpleDateFormat",
    "java.io.DataInputStream", "java.util.UUID", "java.util.ConcurrentModificationException",
    "java.io.Reader", "java.util.AbstractMap", "java.io.BufferedOutputStream",
    "java.util.concurrent.locks.ReentrantLock", "java.io.EOFException", "java.lang.Error",
    "java.io.FileNotFoundException", "java.lang.ArrayIndexOutOfBoundsException",
    "java.nio.file.SimpleFileVisitor", "java.lang.NumberFormatException",
    "java.io.DataOutputStream", "java.util.AbstractSet",
    "java.util.concurrent.locks.ReentrantReadWriteLock",
    "java.util.concurrent.LinkedBlockingQueue", "java.lang.OutOfMemoryError",
    "java.awt.Color", "java.nio.file.NoSuchFileException",
    "java.util.concurrent.ThreadPoolExecutor", "java.util.concurrent.CancellationException",
    "java.io.Writer", "java.util.StringJoiner", "java.lang.Throwable",
    "java.util.concurrent.locks.StampedLock", "java.util.TimerTask", "java.util.Timer",
    "java.awt.image.BufferedImage", "java.lang.ref.ReferenceQueue", "java.io.InputStream",
    "java.util.zip.GZIPOutputStream", "java.io.OutputStream",
    "java.lang.IncompatibleClassChangeError", "java.io.RandomAccessFile",
    "java.text.ParsePosition", "java.net.InetSocketAddress",
    "java.util.concurrent.ScheduledThreadPoolExecutor", "java.lang.StringBuffer",
    "java.lang.ArithmeticException", "java.lang.ClassNotFoundException",
    "java.text.ParseException", "java.util.GregorianCalendar",
    "java.util.zip.GZIPInputStream", "java.lang.ref.SoftReference",
    "java.io.InvalidObjectException", "java.util.concurrent.CountDownLatch",
    "java.lang.ThreadGroup", "java.security.SecureRandom",
    "javax.annotation.processing.AbstractProcessor", "java.lang.ExceptionInInitializerError",
    "java.lang.InterruptedException", "java.util.EmptyStackException",
    "java.io.UncheckedIOException", "java.util.concurrent.atomic.AtomicIntegerArray",
    "java.util.concurrent.CompletionException", "java.io.FilterInputStream",
    "java.util.zip.ZipEntry", "java.util.concurrent.atomic.AtomicReferenceArray",
    "java.util.concurrent.ArrayBlockingQueue", "java.util.concurrent.Semaphore",
    "java.util.jar.Manifest", "java.util.concurrent.FutureTask", "java.util.StringTokenizer",
    "java.io.PushbackInputStream", "java.util.AbstractQueue",
    "java.io.InvalidClassException", "java.util.concurrent.LinkedBlockingDeque",
    "java.util.zip.ZipFile", "java.lang.ProcessBuilder", "java.io.PrintStream",
    "java.security.InvalidParameterException", "java.util.concurrent.atomic.AtomicLongArray",
    "java.util.concurrent.TimeoutException", "java.nio.charset.CharsetEncoder",
    "java.nio.charset.CharsetDecoder", "java.nio.charset.Charset",
    "javax.crypto.spec.SecretKeySpec", "java.util.Scanner",
    "java.text.StringCharacterIterator", "java.awt.Rectangle",
    "java.util.AbstractMap$SimpleImmutableEntry", "java.util.Locale",
    "java.util.zip.Inflater", "java.lang.InternalError", "java.lang.ClassLoader",
    "java.util.zip.InflaterInputStream", "java.net.PasswordAuthentication",
    "java.util.concurrent.atomic.LongAdder", "java.util.zip.Deflater", "javax.swing.JFrame",
    "java.net.Proxy", "java.util.zip.ZipOutputStream",
    "java.security.spec.X509EncodedKeySpec", "java.text.MessageFormat",
    "java.net.Authenticator", "java.io.FilterOutputStream", "javax.swing.JPanel",
    "java.util.concurrent.locks.AbstractQueuedSynchronizer", "javax.swing.JFileChooser",
    "java.util.concurrent.RejectedExecutionException", "java.awt.Dimension",
    "javax.swing.JLabel", "java.util.zip.ZipInputStream", "java.lang.NoSuchFieldException",
    "java.nio.BufferOverflowException", "java.util.Spliterators$AbstractSpliterator",
    "java.io.UTFDataFormatException", "java.lang.NoSuchMethodException",
    "java.util.concurrent.ForkJoinPool", "java.lang.ref.PhantomReference",
    "java.io.NotSerializableException", "java.text.Format",
    "java.util.concurrent.ExecutionException", "java.sql.Date", "java.sql.Timestamp",
    "java.lang.reflect.InvocationTargetException", "java.util.zip.CRC32",
    "java.nio.file.FileAlreadyExistsException", "java.util.concurrent.ForkJoinWorkerThread",
    "java.net.DatagramSocket", "java.security.DigestInputStream",
    "java.lang.IllegalAccessException", "java.net.URISyntaxException", "java.net.Socket",
    "java.util.concurrent.SynchronousQueue", "java.util.concurrent.PriorityBlockingQueue",
    "java.sql.Time", "java.awt.BorderLayout", "javax.swing.JButton",
    "java.awt.event.WindowAdapter", "java.lang.SecurityException",
    "java.awt.event.MouseAdapter", "java.awt.datatransfer.UnsupportedFlavorException",
    "java.util.Formatter", "javax.swing.filechooser.FileNameExtensionFilter",
    "javax.swing.JScrollPane", "java.util.jar.JarFile", "java.awt.Point",
    "java.lang.UnsatisfiedLinkError", "java.time.DateTimeException",
    "java.io.UnsupportedEncodingException", "java.lang.ClassValue",
    "java.util.concurrent.ForkJoinTask", "java.util.jar.JarOutputStream",
    "javax.crypto.spec.IvParameterSpec", "java.io.InterruptedIOException",
    "java.lang.InstantiationException", "java.nio.BufferUnderflowException",
    "java.lang.SecurityManager", "java.util.concurrent.Executor",
    "java.util.concurrent.ExecutorService", "java.util.concurrent.ScheduledExecutorService",
    "java.util.concurrent.BlockingQueue", "java.lang.AutoCloseable", "java.lang.Runnable",
    "java.util.concurrent.Callable", "java.lang.Iterable", "java.util.Collection",
    "java.util.List", "java.util.Set", "java.util.Map", "java.util.Queue", "java.util.Deque",
    "java.lang.Comparable", "java.lang.Appendable",
  };

  static void walk(Class<?> c, Set<String> out) {
    if (c == null || !out.add(c.getName().replace('.', '/'))) return;
    walk(c.getSuperclass(), out);
    for (Class<?> i : c.getInterfaces()) walk(i, out);
  }

  /**
   * The members of `equals`, `hashCode`, `toString` and `compareTo` that `c` declares or inherits
   * from a superclass other than `Object` (`compareTo`: any `Comparable`), comma-separated.
   */
  static String overrides(Class<?> c) throws Exception {
    if (c.isInterface()) return "-";
    java.util.List<String> own = new java.util.ArrayList<>();
    if (c.getMethod("equals", Object.class).getDeclaringClass() != Object.class) own.add("equals");
    if (c.getMethod("hashCode").getDeclaringClass() != Object.class) own.add("hashCode");
    if (c.getMethod("toString").getDeclaringClass() != Object.class) own.add("toString");
    if (Comparable.class.isAssignableFrom(c)) own.add("compareTo");
    return own.isEmpty() ? "-" : String.join(",", own);
  }

  public static void main(String[] args) throws Exception {
    System.out.println("# Java " + System.getProperty("java.version")
        + " supertypes and system-module packages: tests/fixtures/jdk_supertypes/JdkSupertypes.java");
    Set<String> sorted = new TreeSet<>();
    for (String n : CLASSES) sorted.add(n);
    for (String n : sorted) {
      Class<?> c = Class.forName(n);
      Set<String> supers = new TreeSet<>();
      walk(c, supers);
      supers.remove(c.getName().replace('.', '/'));
      String flags = c.isInterface() ? "interface" : Modifier.isFinal(c.getModifiers()) ? "final" : "class";
      Class<?> superclass = c.getSuperclass();
      String direct = superclass == null ? "-" : superclass.getName().replace('.', '/');
      Set<String> declared = new java.util.LinkedHashSet<>();
      for (Class<?> i : c.getInterfaces()) declared.add(i.getName().replace('.', '/'));
      System.out.println("class\t" + c.getName().replace('.', '/') + "\t" + flags + "\t" + String.join(" ", supers)
          + "\t" + direct + "\t" + String.join(" ", declared) + "\t" + overrides(c));
    }
    // Each system package with the module that holds it (`Class.getModule()` of a JDK class).
    Set<String> packages = new TreeSet<>();
    ModuleFinder.ofSystem().findAll().forEach(m -> m.descriptor().packages()
        .forEach(p -> packages.add(p.replace('.', '/') + "\t" + m.descriptor().name())));
    for (String p : packages) System.out.println("package\t" + p);
  }
}

//! Executors, `ServiceLoader`s and Guava's `Splitter`/`Joiner`/
//! `CacheBuilder`/`CacheLoader` are typed handles (`Value::Host`, #156 P9;
//! `host_executors.rs`, `host_services.rs`, `host_guava.rs`).
//!
//! They used to be text a jar `String` could spell (`executor-callable/…`,
//! `service-loader/…`, `guava-splitter/…`, the shared `guava-cache-builder`).
//! The Guava stubs exist only for `javac`: only `demo/` ships, so every call
//! reaches the host's model as a real mod's does.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::StdHost;

/// `java` (Temurin 25.0.5) running this `Probe`, compiled with `javac
/// --release 21` against Guava 33.6.0-jre (the 26.3 server's), with
/// `META-INF/services/demo.Greeter` naming `demo.Hello`, printed these lines,
/// the same on two runs but for `executorHash` (an identity hash); pasted
/// verbatim.
const EXPECTED: &str = "\
spelled=Stringtrue242617473falsefalsefalsefalsefalsefalse,Stringtrue-1467112479falsefalsefalsefalsefalsefalse,Stringtrue1921539161falsefalsefalsefalsefalsefalse,Stringtrue-1306580884falsefalsefalsefalsefalsefalse,Stringtrue777244203falsefalsefalsefalsefalsefalse,6ss|guava-cache-builderexecutor-thread-factory\n\
slashStrings=falsefalse,falsefalse,falsefalse,falsefalse,falsefalse,falsefalse,falsefalse,falsefalse,falsefalse,falsefalse,10s\n\
executors=falsefalsetruetrue|truetruetruetruetrue|5truefalsenullxtruetruetruetruefalse|true1false|truetruetruetrue\n\
callables=done|null|truefalsetruetruetruetruefalse\n\
services=java.util.ServiceLoader[demo.Greeter]|falsefalsetruetruetrue|hellofalsetrue|demo.Hellohellotruetruefalse|hellotruefalse\n\
guava=[a, b, , c]|x-y|truetruetrue|truetrue|k!\n\
identity=6|247\n\
splitterIdentity=false\n\
callableIdentity=false\n\
providerIdentity=true\n\
splitterEquals=false\n\
joinerContains=false\n\
sameJoinerContains=true0\n\
executorText=java.util.concurrent.Executors$AutoShutdownDelegatedExecutorService\n\
executorHash=292938459\n\
providerHashes=1|true1true\n\
reloadIdentity=falsetrue\n\
loaderIterable=1\n\
loaderJoined=demo.Hello\n\
forgedExecutor=CCECCECCECCECCECCE|false|CCE|secret\n\
freshEquality=2,2,2,2,falsefalse-1,2falsefalse\n\
";

const SPLITTER_STUB: &str = r#"package com.google.common.base;

public final class Splitter {
    public static Splitter on(char separator) { return null; }
    public java.util.List<String> splitToList(CharSequence text) { return null; }
}
"#;

const JOINER_STUB: &str = r#"package com.google.common.base;

public class Joiner {
    public static Joiner on(String separator) { return null; }
    public final String join(Iterable<?> parts) { return null; }
}
"#;

const LISTS_STUB: &str = r#"package com.google.common.collect;

public final class Lists {
    public static <E> java.util.ArrayList<E> newArrayList(Iterable<? extends E> elements) { return null; }
}
"#;

const FUNCTION_STUB: &str = r#"package com.google.common.base;

public interface Function<F, T> {
    T apply(F input);
}
"#;

const CACHE_BUILDER_STUB: &str = r#"package com.google.common.cache;

public final class CacheBuilder<K, V> {
    public static CacheBuilder<Object, Object> newBuilder() { return null; }
    public CacheBuilder<K, V> maximumSize(long size) { return null; }
    public <K1 extends K, V1 extends V> LoadingCache<K1, V1> build(CacheLoader<? super K1, V1> loader) { return null; }
}
"#;

const CACHE_LOADER_STUB: &str = r#"package com.google.common.cache;

public abstract class CacheLoader<K, V> {
    public static <K, V> CacheLoader<K, V> from(com.google.common.base.Function<K, V> function) { return null; }
}
"#;

const LOADING_CACHE_STUB: &str = r#"package com.google.common.cache;

public interface LoadingCache<K, V> {
    V getUnchecked(K key);
}
"#;

const GREETER: &str = r#"package demo;

public interface Greeter {
    String greet();
}
"#;

const HELLO: &str = r#"package demo;

public class Hello implements Greeter {
    public Hello() {}

    public String greet() { return "hello"; }
}
"#;

/// The probe the oracle ran (its `main` left out).
const PROBE: &str = r##"package demo;

import com.google.common.base.Joiner;
import com.google.common.base.Splitter;
import com.google.common.cache.CacheBuilder;
import com.google.common.cache.CacheLoader;
import com.google.common.cache.LoadingCache;
import java.util.*;
import java.util.concurrent.*;

@SuppressWarnings({"rawtypes", "unchecked"})
public final class Probe {
    /** Strings spelled like the old markers (and a handle's text) stay Strings. */
    public static String spelled() {
        Object[] texts = {"executor-thread-factory", "guava-cache-builder", "hgsplit:r,", "hgjoin:r-", "service-loader"};
        StringBuilder out = new StringBuilder();
        Map<Object, String> m = new HashMap<>();
        for (Object t : texts) {
            out.append(t.getClass().getSimpleName()).append(t instanceof String).append(t.hashCode())
                .append(t instanceof ExecutorService).append(t instanceof ThreadFactory).append(t instanceof Callable)
                .append(t instanceof CacheBuilder).append(t instanceof Splitter).append(t instanceof ServiceLoader).append(',');
            m.put(t, "s");
        }
        m.put(Executors.defaultThreadFactory(), "f");
        return out.append(m.size()).append(m.get("executor-thread-factory")).append(m.get("guava-cache-builder"))
            .append('|').append("" + "guava-cache-builder" + "executor-thread-factory").toString();
    }

    /** The old composite markers (`word/rest` text): equals and map keys (their class and hash wait for #156 P17). */
    public static String slashStrings() {
        Object[] texts = {"executor-callable/r@indy:x", "executor-callable-result/rx|ry", "service-loader/demo/Greeter;0",
            "service-iterator/0", "service-provider/demo/Greeter;demo/Hello", "guava-splitter/,", "guava-joiner/-",
            "guava-cache-loader/@indy:f", "hexe:executor#1", "hsvl:rdemo/Greeter,i0"};
        StringBuilder out = new StringBuilder();
        Map<Object, String> m = new HashMap<>();
        for (Object t : texts) {
            out.append(t.equals(Splitter.on(','))).append(t.equals(ServiceLoader.load(Greeter.class))).append(',');
            m.put(t, "s");
        }
        return out.append(m.size()).append(m.get("guava-splitter/,")).toString();
    }

    /** Executors: fresh pools; tasks run, futures complete, shutdown state. */
    public static String executors() throws Exception {
        ExecutorService e = Executors.newSingleThreadExecutor();
        ExecutorService f = Executors.newFixedThreadPool(2);
        ScheduledExecutorService s = Executors.newScheduledThreadPool(1);
        Object o = e;
        Future<Integer> answer = e.submit(() -> 5);
        Future<?> ran = e.submit(() -> {});
        ScheduledFuture<String> later = s.schedule(() -> "x", 0, TimeUnit.MILLISECONDS);
        String out = (e == f) + "" + e.equals(f) + e.equals(e) + o.equals(o) + "|" + (o instanceof ExecutorService)
            + (o instanceof Executor) + (o instanceof AutoCloseable) + (s instanceof ScheduledExecutorService)
            + (f instanceof ExecutorService) + "|" + answer.get() + answer.isDone() + answer.isCancelled() + ran.get()
            + later.get() + (answer instanceof Future) + (later instanceof ScheduledFuture) + (answer == answer)
            + answer.equals(answer) + answer.equals(ran) + "|" + List.of(e, f).contains(e) + List.of(e, f).indexOf(f)
            + List.of(answer).contains(ran);
        e.shutdown();
        f.shutdown();
        s.shutdown();
        return out + "|" + e.isShutdown() + f.isShutdown() + e.awaitTermination(1, TimeUnit.SECONDS) + s.isShutdown();
    }

    /** Executors.callable and defaultThreadFactory: fresh objects. */
    public static String callables() throws Exception {
        Runnable r = () -> {};
        Callable c = Executors.callable(r, "done");
        Callable plain = Executors.callable(r);
        Object o = c;
        ThreadFactory t = Executors.defaultThreadFactory();
        return c.call() + "|" + plain.call() + "|" + c.equals(c) + c.equals(plain) + (o instanceof Callable)
            + (t instanceof ThreadFactory) + t.equals(t) + List.of(c).contains(c) + List.of(c).contains(plain);
    }

    /** ServiceLoader: one loader per load; providers built by the JDK's rules; Provider equality by classes. */
    public static String services() {
        ServiceLoader<Greeter> l = ServiceLoader.load(Greeter.class);
        ServiceLoader<Greeter> m = ServiceLoader.load(Greeter.class);
        Iterator<Greeter> it = l.iterator();
        Greeter first = it.next();
        ServiceLoader.Provider<Greeter> p = l.stream().findFirst().get();
        ServiceLoader.Provider<Greeter> q = l.stream().findFirst().get();
        Object o = l;
        return l + "|" + (l == m) + l.equals(m) + l.equals(l) + (o instanceof ServiceLoader) + (o instanceof Iterable)
            + "|" + first.greet() + it.hasNext() + (l.iterator().next() == first) + "|" + p.type().getName()
            + p.get().greet() + p.equals(q) + p.equals(p) + (p.get() == p.get()) + "|" + l.findFirst().get().greet()
            + List.of(l).contains(l) + List.of(l).contains(m);
    }

    /** Guava: Splitter/Joiner are fresh objects with Object's equals; CacheBuilder's setters return it. */
    public static String guava() {
        Splitter s = Splitter.on(',');
        Joiner j = Joiner.on("-");
        CacheBuilder b = CacheBuilder.newBuilder();
        Object o = s;
        LoadingCache<String, String> cache = b.build(CacheLoader.from((String k) -> k + "!"));
        return s.splitToList("a,b,,c") + "|" + j.join(List.of("x", "y")) + "|" + s.equals(s)
            + (o instanceof Splitter) + (j instanceof Joiner) + "|" + (b.maximumSize(10) == b) + (b instanceof CacheBuilder)
            + "|" + cache.getUnchecked("k");
    }

    /** Providers of two loaders are equal: hash collections keep one. */
    public static String providerHashes() {
        ServiceLoader.Provider<Greeter> p = ServiceLoader.load(Greeter.class).stream().findFirst().get();
        ServiceLoader.Provider<Greeter> q = ServiceLoader.load(Greeter.class).stream().findFirst().get();
        Map<Object, Integer> m = new HashMap<>();
        m.put(p, 1);
        return new HashSet<>(List.of(p, q)).size() + "|" + m.containsKey(q) + m.get(q) + new HashSet<>(List.of(p)).contains(q);
    }

    /** After reload() a loader makes new providers: equal, not the same. */
    public static String reloadIdentity() {
        ServiceLoader<Greeter> l = ServiceLoader.load(Greeter.class);
        Object p = l.stream().findFirst().get();
        l.reload();
        Object q = l.stream().findFirst().get();
        return "" + (p == q) + p.equals(q);
    }

    /** A loader is an Iterable to library code. */
    public static String loaderIterable() {
        return "" + com.google.common.collect.Lists.newArrayList(ServiceLoader.load(Greeter.class)).size();
    }

    public static String loaderJoined() {
        String joined = Joiner.on(",").join(ServiceLoader.load(Greeter.class));
        return joined == null ? "null" : joined.substring(0, joined.indexOf('@'));
    }

    /** A String spelled like a pool's or a future's old marker is a String: casting it throws, and no real pool or future changes. */
    public static String forgedExecutor() throws Exception {
        ExecutorService e = Executors.newFixedThreadPool(1);
        StringBuilder out = new StringBuilder();
        for (int i = 0; i < 6; i++) {
            try {
                ((ExecutorService) (Object) ("executor#" + i)).shutdown();
                out.append("ok");
            } catch (ClassCastException c) {
                out.append("CCE");
            }
        }
        Future<String> f = e.submit(() -> "secret");
        Object forged;
        try {
            forged = ((Future<?>) (Object) "future#1").get();
        } catch (ClassCastException c) {
            forged = "CCE";
        }
        boolean shut = e.isShutdown();
        e.shutdown();
        return out + "|" + shut + "|" + forged + "|" + f.get();
    }

    /** Two splitters, joiners, callables, cache loaders made separately: Object's equals, identity. */
    public static String freshEquality() {
        Splitter a = Splitter.on(',');
        Splitter b = Splitter.on(',');
        Map<Object, Integer> m = new HashMap<>();
        m.put(a, 1);
        m.put(b, 2);
        Runnable r = () -> {};
        Callable<Object> c = Executors.callable(r);
        Callable<Object> d = Executors.callable(r);
        String sets;
        try {
            sets = Set.of(a, b).size() + "," + Map.of(a, 1, b, 2).size();
        } catch (IllegalArgumentException e) {
            sets = "IAE";
        }
        return new HashSet<>(List.of(a, b)).size() + "," + m.size() + "," + sets + "," + Objects.equals(a, b)
            + new ArrayList<>(List.of(a)).contains(b) + List.of(a, b).indexOf(Splitter.on(',')) + ","
            + new HashSet<>(List.of(c, d)).size() + List.of(c).contains(d)
            + List.of(CacheLoader.from((String k) -> k)).contains(CacheLoader.from((String k) -> k));
    }

    /** Separate constructions are separate objects: an IdentityHashMap keeps each. */
    public static String identity() {
        ExecutorService e = Executors.newSingleThreadExecutor();
        ServiceLoader<Greeter> l = ServiceLoader.load(Greeter.class);
        CacheBuilder b = CacheBuilder.newBuilder();
        Map<Object, Integer> m = new IdentityHashMap<>();
        m.put(e, 1);
        m.put(e, 2);
        m.put(Executors.newSingleThreadExecutor(), 3);
        m.put(l, 4);
        m.put(ServiceLoader.load(Greeter.class), 5);
        m.put(b, 6);
        m.put(b.maximumSize(1), 7);
        m.put(CacheBuilder.newBuilder(), 8);
        e.shutdown();
        return m.size() + "|" + m.get(e) + m.get(l) + m.get(b);
    }

    /** Equal-looking fresh objects: identity on two of them is the JVM's to say. */
    public static String splitterIdentity() {
        return "" + (Splitter.on(',') == Splitter.on(','));
    }

    public static String callableIdentity() {
        Runnable r = () -> {};
        return "" + (Executors.callable(r) == Executors.callable(r));
    }

    public static String providerIdentity() {
        ServiceLoader<Greeter> l = ServiceLoader.load(Greeter.class);
        return "" + (l.stream().findFirst().get() == l.stream().findFirst().get());
    }

    /** Two `Joiner.on("-")` are two objects with `Object`'s equals. */
    public static String joinerContains() {
        return "" + List.of(Joiner.on("-")).contains(Joiner.on("-"));
    }

    /** Two `Splitter.on(',')`: `Object.equals`, identity. */
    public static String splitterEquals() {
        return "" + Splitter.on(',').equals(Splitter.on(','));
    }

    /** The same joiner, through a host list. */
    public static String sameJoinerContains() {
        Joiner j = Joiner.on("-");
        return "" + List.of(j).contains(j) + new ArrayList<>(List.of(j)).indexOf(j);
    }

    public static String executorText() {
        ExecutorService e = Executors.newSingleThreadExecutor();
        String text = String.valueOf(e);
        e.shutdown();
        return text == null ? "null" : text.substring(0, text.indexOf('@'));
    }

    public static String executorHash() {
        ExecutorService e = Executors.newSingleThreadExecutor();
        e.shutdown();
        return "" + e.hashCode();
    }
}
"##;

fn fixture(test: &str) -> javac_fixture::Fixture {
    javac_fixture::Fixture::build_with_lib(
        test,
        &[
            ("com/google/common/base/Splitter.java", SPLITTER_STUB),
            ("com/google/common/base/Joiner.java", JOINER_STUB),
            ("com/google/common/base/Function.java", FUNCTION_STUB),
            ("com/google/common/collect/Lists.java", LISTS_STUB),
            (
                "com/google/common/cache/CacheBuilder.java",
                CACHE_BUILDER_STUB,
            ),
            (
                "com/google/common/cache/CacheLoader.java",
                CACHE_LOADER_STUB,
            ),
            (
                "com/google/common/cache/LoadingCache.java",
                LOADING_CACHE_STUB,
            ),
            ("demo/Greeter.java", GREETER),
            ("demo/Hello.java", HELLO),
            ("demo/Probe.java", PROBE),
        ],
        &[("META-INF/services/demo.Greeter", b"demo.Hello\n")],
    )
}

/// The JVM's line for `method` in [`EXPECTED`].
fn expected(method: &str) -> &'static str {
    EXPECTED
        .lines()
        .find_map(|line| line.strip_prefix(method)?.strip_prefix('='))
        .unwrap_or_else(|| panic!("no expected line for {method}"))
}

/// Run `method`: its text and the unknown calls it counted.
fn run(fixture: &javac_fixture::Fixture, method: &str) -> (String, Vec<(String, usize)>) {
    let mut host = fixture.jar.seed_host(StdHost::new());
    let text = fixture.text_on(&mut host, method);
    (text, host.unknown_host_calls())
}

/// Executors, futures, `Executors.callable`, thread factories, service
/// loaders and their providers, Guava splitters, joiners and cache builders
/// answer as Java 25 does with nothing counted: the executor model's inline
/// tasks, completed futures and shutdown state; a fresh object per factory
/// call (`IdentityHashMap`, `List.contains`); `ServiceLoader`'s text, lookup
/// and cached providers (equal by their classes); a builder's setters
/// answering it; and no string spelled like an old marker or a handle's
/// text is one.
#[test]
fn executors_services_and_guava_answer_as_java_25() {
    let fixture = fixture("host-services");
    for method in [
        "spelled",
        "slashStrings",
        "executors",
        "callables",
        "services",
        "guava",
        "identity",
    ] {
        let (text, unknown) = run(&fixture, method);
        assert_eq!(text, expected(method), "{method}: {unknown:?}");
        assert_eq!(unknown, Vec::<(String, usize)>::new(), "{method}");
    }
}

/// What only a JVM knows is counted, never answered from equal keys:
/// identity of two `Splitter.on(',')` or two `Executors.callable(r)` and
/// `equals` between two splitters (Java 25: `false`), an executor's `toString` (its implementation class and identity
/// hash) and its `hashCode`.
#[test]
fn what_only_a_jvm_knows_is_counted() {
    let fixture = fixture("host-services-counted");
    for (method, call) in [
        ("splitterIdentity", "java/lang/Object#if_acmp("),
        ("callableIdentity", "java/lang/Object#if_acmp("),
        ("splitterEquals", "java/lang/Object#equals("),
        ("executorText", "#toString()Ljava/lang/String;"),
        ("executorHash", "#hashCode()I"),
    ] {
        let (_, unknown) = run(&fixture, method);
        assert!(
            unknown.iter().any(|(counted, _)| counted.contains(call)),
            "{method}: {unknown:?}"
        );
    }
}

/// Answers that need what only a JVM knows are Java 25's or counted, never
/// a silent wrong one: providers of two loaders in a `HashSet`/`HashMap`
/// (equal, so one entry), a provider's identity across one loader's
/// `stream()` calls and across `reload()` (fresh objects), a loader read as
/// an `Iterable` by Guava, and two separately made joiners compared through
/// host lists (`Object.equals`, identity) once #175 is in (the next test).
#[test]
fn answers_only_a_jvm_knows_are_java_25s_or_counted() {
    let fixture = fixture("host-services-lists");
    for method in [
        "providerIdentity",
        "providerHashes",
        "reloadIdentity",
        "loaderIterable",
        "loaderJoined",
    ] {
        let (text, unknown) = run(&fixture, method);
        assert!(
            text == expected(method) || !unknown.is_empty(),
            "{method}: {text} {unknown:?}"
        );
    }
}

/// Two separately made splitters, joiners, callables and cache loaders have
/// equal keys but are two objects with `Object`'s `equals`: in a `HashSet`, as
/// `HashMap` keys, in `Set.of`/`Map.of`, through `Objects.equals` and host
/// lists, the answer is Java 25's or counted. The rows answer `None` (unknown
/// identity); shared code then compared their keys until #175.
#[test]
#[ignore = "#175: shared collection code answers an unknown equals from key equality"]
fn fresh_objects_with_equal_keys_are_never_merged_silently() {
    let fixture = fixture("host-services-fresh");
    for method in ["joinerContains", "sameJoinerContains", "freshEquality"] {
        let (text, unknown) = run(&fixture, method);
        assert!(
            text == expected(method) || !unknown.is_empty(),
            "{method}: {text} {unknown:?}"
        );
    }
}

/// A `String` spelled like a pool's or a future's old marker never reaches
/// the real objects' state: Java 25 refuses the casts, and here the cast may
/// pass (`"executor#0"` still looks like an allocation until #156 P12) but
/// the call on it is counted, the real pool is not shut down, and the forged
/// "future" does not hand out the real future's value.
#[test]
fn a_string_spelled_like_an_executor_never_drives_a_real_one() {
    let fixture = fixture("host-services-forged");
    let (text, unknown) = run(&fixture, "forgedExecutor");
    let java = expected("forgedExecutor");
    if text != java {
        let parts: Vec<&str> = text.split('|').collect();
        assert_eq!(parts.get(1), Some(&"false"), "{text} {unknown:?}");
        assert_ne!(parts.get(2), Some(&"secret"), "{text} {unknown:?}");
        assert_eq!(parts.get(3), Some(&"secret"), "{text} {unknown:?}");
        assert!(!unknown.is_empty(), "{text}");
    }
}

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
        "providerIdentity",
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

/// A joiner inside a host list: `contains`/`indexOf` run `Object.equals`,
/// which is identity. The kind answers it from `same_object`, and an answer
/// the handle cannot give (two separately made joiners, or one whose origin
/// a list's text form dropped) is counted. Shared list code may still have
/// its own fallback (#175 removes the one that compared keys), so this
/// checks only that the answer is Java 25's or counted, never a silent
/// wrong one.
///
/// Ignored until #175 merges: an immutable list's `contains` reaches
/// `java_equals`, which answers an unknown `equals` from key equality, so
/// `List.of(Joiner.on("-")).contains(Joiner.on("-"))` is `true` here without
/// a count (Java 25: `false`). This kind answers `None` there; the fallback
/// is #175's to remove.
#[test]
#[ignore = "#175: java_equals falls back to key equality on an unknown equals"]
fn a_joiner_in_a_host_list_is_found_by_identity_or_counted() {
    let fixture = fixture("host-services-lists");
    for method in ["joinerContains", "sameJoinerContains"] {
        let (text, unknown) = run(&fixture, method);
        assert!(
            text == expected(method) || !unknown.is_empty(),
            "{method}: {text} {unknown:?}"
        );
    }
}

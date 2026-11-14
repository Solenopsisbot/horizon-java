//! The review of #132 (`c5b143e`): a live stream's casts and `instanceof`,
//! `sorted()` on a natural-`SORTED` pipeline, `Head.forEach` over a jar
//! spliterator, `RandomAccessSpliterator`'s late `modCount`, `Collector.of`
//! handles as hash keys, and sinks kept past their push.
//!
//! Every expected line is OpenJDK 25.0.5's output for the same probe
//! (`support/jdk_differential.rs`); the counted cases name Java's answer.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;
#[path = "support/jdk_differential.rs"]
mod jdk_differential;

use horizon_native_java::{StdHost, Value};

const PROBE: &str = r#"package demo;

import java.util.*;
import java.util.function.*;
import java.util.stream.*;

public final class Probe {
    static final StringBuilder LOG = new StringBuilder();
    static void log(String event) { LOG.append(event).append(' '); }
    static String take(Object result) {
        String text = LOG.toString().trim() + " => " + result;
        LOG.setLength(0);
        return text;
    }
    static String ex(Throwable t) { return "!" + t.getClass().getSimpleName(); }

    /// A jar spliterator: every member logs; `comparator` answers `getComparator()`.
    static class Split implements Spliterator<Object> {
        final Object[] items;
        int next;
        final int flags;
        Split(int flags, Object... items) { this.flags = flags; this.items = items; }
        @Override public boolean tryAdvance(Consumer<? super Object> action) {
            log("ta" + next);
            if (next >= items.length) return false;
            action.accept(items[next++]);
            return true;
        }
        @Override public Spliterator<Object> trySplit() { return null; }
        @Override public long estimateSize() { log("es"); return items.length - next; }
        @Override public int characteristics() { log("ch"); return flags; }
        @Override public Comparator<? super Object> getComparator() { log("gc"); return null; }
    }

    /// Keeps the action it is handed.
    static Consumer<? super Object> SAVED;
    static final class SaveSplit extends Split {
        SaveSplit(Object... items) { super(0, items); }
        @Override public boolean tryAdvance(Consumer<? super Object> action) { SAVED = action; return super.tryAdvance(action); }
    }

    static final class Key implements Comparable<Key> {
        final int id;
        Key(int id) { this.id = id; }
        @Override public int compareTo(Key o) { log("cmp" + id + o.id); return Integer.compare(id, o.id); }
        @Override public String toString() { return "K" + id; }
    }

    /// A `RandomAccess` jar list whose `add` moves `modCount`.
    static final class Row extends AbstractList<Object> implements RandomAccess {
        final List<Object> items = new ArrayList<>();
        Row(Object... xs) { items.addAll(Arrays.asList(xs)); }
        @Override public int size() { log("size"); return items.size(); }
        @Override public Object get(int i) { log("get" + i); return items.get(i); }
        @Override public boolean add(Object x) { log("add"); modCount++; return items.add(x); }
    }

    /// A list's elements through their own `toString`.
    static String ids(List<?> list) {
        StringBuilder out = new StringBuilder("[");
        for (Object o : list) out.append(o).append(';');
        return out.append(']').toString();
    }

    static Collector<Object, ?, List<Object>> coll() {
        return Collector.of(ArrayList::new, List::add, (a, b) -> { a.addAll(b); return a; });
    }

    public static String streamTypes() {
        Object a = List.<Object>of(7, 8).stream();
        Object b = StreamSupport.stream(new Split(0, 1), false);
        Object c = IntStream.range(0, 2);
        Object d = Stream.of(1).map(x -> x);
        String t = (a instanceof Stream) + "," + (a instanceof BaseStream) + "," + (a instanceof AutoCloseable) + ","
            + (a instanceof IntStream) + "," + (a instanceof String) + "," + Stream.class.isInstance(a) + "|"
            + (b instanceof Stream) + "," + (b instanceof String) + "|"
            + (c instanceof IntStream) + "," + (c instanceof Stream) + "," + (c instanceof BaseStream) + "|"
            + (d instanceof Stream) + "," + (d instanceof CharSequence);
        return take(t);
    }

    public static String castStreamToString() {
        Stream<Object> s = List.<Object>of(7, 8).stream();
        String r;
        try { String t = (String) (Object) s; r = "noCCE:" + t.length(); } catch (ClassCastException e) { r = "CCE"; }
        String q;
        try { Object o = String.class.cast(s); q = "noCCE"; } catch (ClassCastException e) { q = "CCE"; }
        return take(r + "," + q + "," + s.toList());
    }

    public static String castHandlesToString() {
        String r;
        try { String t = (String) (Object) Stream.builder(); r = "noCCE"; } catch (ClassCastException e) { r = e.getMessage(); }
        String q;
        try { String t = (String) (Object) coll(); q = "noCCE"; } catch (ClassCastException e) { q = e.getMessage(); }
        return take(r + "|" + q);
    }

    public static String castSinkToString() {
        StreamSupport.stream(new SaveSplit(1), false).map(x -> x).forEach(x -> { });
        String r;
        try { String t = (String) (Object) SAVED; r = "noCCE"; } catch (ClassCastException e) { r = "CCE"; }
        return take(r);
    }

    public static String sortedNoop() {
        List<Object> a = StreamSupport.stream(new Split(Spliterator.SORTED | Spliterator.ORDERED, 3, 1, 2), false).sorted().toList();
        log("|");
        List<Object> b = StreamSupport.stream(new Split(Spliterator.SORTED | Spliterator.ORDERED, 3, 1, 2), false).filter(x -> true).sorted().toList();
        log("|");
        Optional<Object> c = StreamSupport.stream(new Split(Spliterator.SORTED | Spliterator.ORDERED, 3, 1, 2), false).sorted().findFirst();
        log("|");
        Optional<Object> d = StreamSupport.stream(new Split(Spliterator.SORTED, new Key(1), new Key(2), new Key(3)), false).sorted().findFirst();
        log("|");
        List<Object> e = StreamSupport.stream(new Split(Spliterator.SORTED, new Key(2), new Key(1)), false)
            .sorted((Comparator<Object>) (Comparator<?>) Comparator.<Key>naturalOrder()).toList();
        return take(a + "," + b + "," + c + "," + d + "," + ids(e));
    }

    public static String treeSetSorted() {
        TreeSet<Key> ts = new TreeSet<>();
        ts.add(new Key(2));
        ts.add(new Key(1));
        log("|");
        Optional<Key> a = ts.stream().sorted().findFirst();
        log("|");
        TreeSet<Key> rev = new TreeSet<>(Comparator.reverseOrder());
        rev.add(new Key(1));
        rev.add(new Key(2));
        log("|");
        List<Key> b = rev.stream().sorted().toList();
        return take(a + "," + ids(b));
    }

    public static String headForEach() {
        StreamSupport.stream(new Split(0, 1, 2), false).forEachOrdered(x -> log("e" + x));
        log("|");
        Consumer<Object> c = x -> log("f" + x);
        StreamSupport.stream(new SaveSplit(1), false).forEach(c);
        boolean same = SAVED == c;
        SAVED.accept("late");
        return take(same);
    }

    public static String rowLateModCount() {
        Row r = new Row(1, 2);
        Stream<Object> s = r.stream();
        r.add(3);
        List<Object> a = s.toList();
        log("|");
        Row q = new Row(1, 2);
        Spliterator<Object> sp = q.spliterator();
        q.add(3);
        String b;
        try { sp.forEachRemaining(x -> log("e" + x)); b = "x"; } catch (ConcurrentModificationException e) { b = "!CME"; }
        return take(a + "," + b);
    }

    public static String nullCharacteristic() {
        String r;
        try { Collector.of(ArrayList::new, List::add, (a, b) -> a, (Collector.Characteristics) null); r = "ok"; } catch (NullPointerException e) { r = "NPE"; }
        return take(r);
    }

    public static String collectorKeys() {
        Set<Object> set = new HashSet<>(Arrays.asList(coll(), coll()));
        Map<Object, Integer> m = new HashMap<>();
        m.put(coll(), 1);
        return take(set.size() + "," + m.containsKey(coll()) + "," + coll().equals(coll()));
    }

    public static String deadSinks() {
        StreamSupport.stream(new SaveSplit(1), false).map(x -> "m" + x).forEach(x -> log("e" + x));
        log("done");
        SAVED.accept("late");
        log("|");
        List<Object> l = StreamSupport.stream(new SaveSplit(1), false).toList();
        SAVED.accept("late2");
        boolean java = SAVED.getClass().getName().startsWith("java.util.stream.");
        return take(l + "," + java);
    }
}
"#;

#[test]
fn spliterator_review_cases_run_as_jdk_25_does() {
    jdk_differential::check(
        "stream-spliterator-review",
        PROBE,
        &[
            // A live stream is its stream interface, `BaseStream` and
            // `AutoCloseable`, never a `String`.
            (
                "streamTypes",
                "ch => true,true,true,false,false,true|true,false|true,false,true|true,false",
            ),
            // The builder and `Collector.of`'s collector know their class.
            (
                "castHandlesToString",
                " => class java.util.stream.Streams$StreamBuilderImpl cannot be cast to class java.lang.String (java.util.stream.Streams$StreamBuilderImpl and java.lang.String are in module java.base of loader 'bootstrap')|class java.util.stream.Collectors$CollectorImpl cannot be cast to class java.lang.String (java.util.stream.Collectors$CollectorImpl and java.lang.String are in module java.base of loader 'bootstrap')",
            ),
            // A natural `sorted()` on a natural-`SORTED` pipeline is the
            // downstream itself: no compare, `findFirst` pulls one element;
            // `sorted(Comparator.naturalOrder())` still sorts.
            (
                "sortedNoop",
                "ch gc ch ta0 ta1 ta2 ta3 | ch gc ch ta0 ta1 ta2 ta3 | ch gc ch ta0 | ch gc ch ta0 | ch gc ch ta0 ta1 ta2 cmp12 => [3, 1, 2],[3, 1, 2],Optional[3],Optional[K1],[K1;K2;]",
            ),
            // A natural `TreeSet` is `SORTED`; a reverse-ordered one is not.
            (
                "treeSetSorted",
                "cmp22 cmp12 | | cmp11 cmp12 | cmp12 => Optional[K1],[K1;K2;]",
            ),
            // `Head.forEach`: the jar spliterator gets the caller's own
            // `Consumer`, with no size query first.
            (
                "headForEach",
                "ch ta0 e1 ta1 e2 ta2 | ch ta0 f1 ta1 flate => true",
            ),
            // `Collector.of`'s characteristics go through `EnumSet.of`.
            ("nullCharacteristic", " => NPE"),
            // `RandomAccessSpliterator.getFence()` binds `modCount` late.
            (
                "rowLateModCount",
                "add size get0 get1 get2 | add size get0 e1 get1 e2 get2 e3 => [1, 2, 3],x",
            ),
        ],
    );
}

/// Runs `method`: its answer and the unknown host calls it counted.
fn run(method: &str) -> (Option<String>, Vec<(String, usize)>) {
    let fixture = javac_fixture::Fixture::build(
        "stream-spliterator-review-counted",
        &[("demo/Probe.java", PROBE)],
    );
    let mut host = fixture.jar.seed_host(StdHost::new());
    let answer = fixture.jar.invoke_method_with_args(
        "demo/Probe",
        method,
        "()Ljava/lang/String;",
        None,
        &[],
        &mut host,
    );
    let text = match answer {
        Ok(Value::Ref(Some(text))) => Some(text),
        _ => None,
    };
    (text, host.unknown_host_calls())
}

/// A live stream cast to `String` (`checkcast` or `Class.cast`) throws, as
/// in Java (JDK 25.0.5: ` => CCE,CCE,[7, 8]`), and never hands the jar its
/// marker. The message would name a pipeline stage class the runtime does
/// not model, so the casts are counted.
#[test]
fn a_stream_never_passes_a_cast_to_string() {
    let (text, unknown) = run("castStreamToString");
    assert_eq!(text.as_deref(), Some(" => CCE,CCE,[7, 8]"));
    assert!(!unknown.is_empty(), "{unknown:?}");
}

/// The same for a stored sink, whose class is unknown (JDK 25.0.5:
/// `ch ch ta0 ta1 => CCE`): it throws, counted.
#[test]
fn a_sink_never_passes_a_cast_to_string() {
    let (text, unknown) = run("castSinkToString");
    assert_eq!(text.as_deref(), Some("ch ch ta0 ta1 => CCE"));
    assert!(
        unknown
            .iter()
            .any(|(call, _)| call.contains("java/lang/String#<checkcast>")),
        "{unknown:?}"
    );
}

/// Counted, not answered: two `Collector.of` records of the same lambdas
/// are equal with equal hashes (JDK 25.0.5: ` => 1,true,true`); the runtime
/// cannot hash them, so a hash table asks the row's `equals`, which counts.
#[test]
fn collector_handles_as_hash_keys_are_counted() {
    let (_, unknown) = run("collectorKeys");
    assert!(!unknown.is_empty(), "{unknown:?}");
}

/// Counted, not answered: a sink kept past its push still runs its stages
/// in Java (JDK 25.0.5: `ch ch ta0 em1 ta1 done emlate | ch ch ta0 ta1 =>
/// [1],true`); here the late `accept` and the sink's `getClass()` are
/// counted, and the run is not aborted.
#[test]
fn a_sink_kept_past_its_push_is_counted() {
    let (_, unknown) = run("deadSinks");
    assert!(
        unknown
            .iter()
            .any(|(call, _)| call.starts_with("java/util/stream/Sink#accept")),
        "{unknown:?}"
    );
}

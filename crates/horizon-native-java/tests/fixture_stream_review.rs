//! The review of #128 (head `c641080`): `Collectors.toMap`'s map supplier,
//! a `Stream.Builder` through `Consumer.andThen`, `Stream.concat` sides with
//! stages under a short-circuiting push, `distinct()` on a `SORTED` pipeline,
//! and `stream/...` Strings that are no stream (`stream_pipeline.rs`).
//!
//! The probe is the reviewer's (`~/ava/results/rv128/Probe2.java` on
//! elysium) plus two methods; every expected line is OpenJDK 25.0.5's output
//! for it (`support/jdk_differential.rs`).

#[path = "support/javac_fixture.rs"]
mod javac_fixture;
#[path = "support/jdk_differential.rs"]
mod jdk_differential;

use horizon_native_java::StdHost;

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
    static String fail(Throwable t) { return take("!" + t.getClass().getName() + ": " + t.getMessage()); }
    static String ex(Throwable t) { return "!" + t.getClass().getSimpleName(); }

    static final class K implements Comparable<K> {
        final int id; final int h;
        K(int id, int h) { this.id = id; this.h = h; }
        @Override public boolean equals(Object o) { log("eq" + id + (o instanceof K k ? "/" + k.id : "/x")); return o instanceof K k && k.id == id; }
        @Override public int hashCode() { log("h" + id); return h; }
        @Override public int compareTo(K o) { log("c" + id + o.id); return Integer.compare(id, o.id); }
    }

    static final class Bag extends AbstractCollection<Integer> {
        final List<Integer> items;
        Bag(Integer... xs) { items = new ArrayList<>(Arrays.asList(xs)); }
        @Override public Iterator<Integer> iterator() {
            log("it");
            Iterator<Integer> in = items.iterator();
            return new Iterator<>() {
                public boolean hasNext() { boolean b = in.hasNext(); log("hn" + (b ? 1 : 0)); return b; }
                public Integer next() { Integer v = in.next(); log("nx" + v); return v; }
            };
        }
        @Override public int size() { log("sz"); return items.size(); }
    }

    public static String a02ToMapSuppliers() {
        StringBuilder out = new StringBuilder();
        try { Map<String, Integer> m = Stream.of("b", "a", "b").collect(Collectors.toMap(x -> x, x -> 1, Integer::sum, TreeMap::new)); out.append(m).append(m.getClass().getSimpleName()).append(m instanceof TreeMap); } catch (Throwable t) { out.append(ex(t)); }
        out.append(',');
        try { Map<String, Integer> m = Stream.of("b", "a").collect(Collectors.toMap(x -> x, x -> 1, (p, q) -> p, LinkedHashMap::new)); out.append(m).append(m.getClass().getSimpleName()); } catch (Throwable t) { out.append(ex(t)); }
        out.append(',');
        try { Map<String, Integer> m = Stream.of("zz", "b", "a").collect(Collectors.toMap(x -> x, x -> 1, (p, q) -> p, LinkedHashMap::new)); out.append(m.keySet()); } catch (Throwable t) { out.append(ex(t)); }
        out.append(',');
        try { Map<String, Integer> m = Stream.of("b", "a").collect(Collectors.toMap(x -> x, x -> 1)); m.put("c", 3); out.append(m); } catch (Throwable t) { out.append(ex(t)); }
        out.append(',');
        try { Map<String, Integer> m = Stream.of("b", "a").collect(Collectors.toMap(x -> x, x -> 1, (p, q) -> p, () -> { log("sup"); return new TreeMap<String, Integer>(Comparator.<String>reverseOrder()); })); out.append(m); } catch (Throwable t) { out.append(ex(t)); }
        return take(out);
    }

    public static String a03BuilderAndThen() {
        try {
            Stream.Builder<Integer> b = Stream.builder();
            Consumer<Integer> c = b;
            c.andThen(x -> log("then" + x)).accept(3);
            Consumer<Integer> d = x -> log("first" + x);
            d.andThen(b).accept(4);
            return take(b.build().toList());
        } catch (Throwable t) { return fail(t); }
    }

    public static String a05IntBuilder() {
        try {
            IntStream.Builder b = IntStream.builder();
            b.add(1).add(2);
            b.accept(5);
            Object o = b;
            String cls; try { cls = o.getClass().getName(); } catch (Throwable t) { cls = ex(t); }
            return take(cls + "," + (o instanceof IntConsumer) + "," + (o instanceof Consumer) + "," + (o instanceof Stream.Builder)
                + "," + (o instanceof IntStream.Builder) + "," + b.build().boxed().toList());
        } catch (Throwable t) { return fail(t); }
    }

    public static String a09ConcatSorted() {
        try {
            return take(Stream.concat(Stream.of(3, 1, 2).sorted().peek(x -> log("q" + x)), Stream.of(9).peek(x -> log("r" + x))).findFirst());
        } catch (Throwable t) { return fail(t); }
    }

    public static String a10ConcatSortedAnyMatchAndLimit() {
        try {
            boolean any = Stream.concat(Stream.of(2, 1).sorted().peek(x -> log("q" + x)), Stream.of(9)).anyMatch(x -> x == 1);
            List<Integer> l = Stream.concat(Stream.of(2, 1).sorted().peek(x -> log("s" + x)), Stream.of(9)).limit(1).toList();
            return take(any + "," + l);
        } catch (Throwable t) { return fail(t); }
    }

    public static String a11ConcatFlatMapInner() {
        try {
            return take(Stream.concat(Stream.of(1).flatMap(x -> Stream.of(x, x + 1)).peek(x -> log("q" + x)), Stream.of(9)).findFirst());
        } catch (Throwable t) { return fail(t); }
    }

    public static String a12SortedDistinctNoToString() {
        try {
            K a = new K(1, 7), b = new K(2, 7), a2 = new K(1, 7);
            List<K> r = Stream.of(b, a, a2).sorted().distinct().toList();
            return take(r.size() + "," + (r.get(0) == a) + "," + (r.get(1) == b));
        } catch (Throwable t) { return fail(t); }
    }

    public static String a13SortedDistinctKeepsFirst() {
        try {
            K a = new K(1, 7), a2 = new K(1, 7);
            List<K> r = Stream.of(a, a2).sorted().distinct().toList();
            return take(r.size() + "," + (r.get(0) == a) + "," + (r.get(0) == a2));
        } catch (Throwable t) { return fail(t); }
    }

    public static String a14DistinctKeepsFirst() {
        try {
            K a = new K(1, 7), a2 = new K(1, 7);
            List<K> r = Stream.of(a, a2).distinct().toList();
            return take(r.size() + "," + (r.get(0) == a));
        } catch (Throwable t) { return fail(t); }
    }

    public static String a15TreeSetDistinctNoHash() {
        try {
            TreeSet<K> t = new TreeSet<>();
            K a = new K(1, 7), b = new K(2, 8);
            t.add(a); t.add(b);
            log("|");
            long n = t.stream().distinct().count();
            return take(n);
        } catch (Throwable t) { return fail(t); }
    }

    public static String forgedStream() {
        StringBuilder out = new StringBuilder();
        Stream<Integer> live = Stream.of(1, 2).map(x -> x);
        Object o = "stream/demo/Probe/forgedStream/1";
        out.append(o.getClass().getName()).append(',').append(o instanceof String).append(',');
        try { Object s = (Stream<?>) o; out.append("noCCE"); } catch (ClassCastException e) { out.append(e.getMessage()); }
        out.append(',');
        try { Stream<?> s = (Stream<?>) o; out.append(s.count()); } catch (ClassCastException e) { out.append("CCE"); }
        out.append(',').append(live.toList());
        return take(out);
    }

    public static String liveStreamClass() {
        Stream<Integer> s = Stream.of(1).map(x -> x);
        String name = s.getClass().getName();
        return take(name + "," + s.toList());
    }
}
"#;

fn check(cases: &[(&str, &str)]) {
    jdk_differential::check("stream-review", PROBE, cases);
}

#[test]
fn review_cases_run_as_jdk_25_does() {
    check(&[
        // `toMap(k, v, merge, mapFactory)`: the supplier's map, `merge` per
        // element.
        (
            "a02ToMapSuppliers",
            "sup => {a=1, b=2}TreeMaptrue,{b=1, a=1}LinkedHashMap,[zz, b, a],{a=1, b=1, c=3},{b=1, a=1}",
        ),
        // A builder on either side of `Consumer.andThen`.
        ("a03BuilderAndThen", "then3 first4 => [3, 4]"),
        // A `concat` side with stages is a `WrappingSpliterator`: one source
        // element runs through the side before the outer push may stop.
        ("a09ConcatSorted", "q1 q2 q3 => Optional[1]"),
        ("a10ConcatSortedAnyMatchAndLimit", "q1 q2 s1 s2 => true,[1]"),
        ("a11ConcatFlatMapInner", "q1 q2 => Optional[1]"),
        // `distinct()` on a `SORTED` pipeline: adjacent `equals`, no hash.
        (
            "a12SortedDistinctNoToString",
            "c12 c11 c12 c11 eq1/1 eq2/1 => 2,true,true",
        ),
        ("a13SortedDistinctKeepsFirst", "c11 eq1/1 => 1,true,false"),
        ("a14DistinctKeepsFirst", "h1 h1 eq1/1 => 1,true"),
        ("a15TreeSetDistinctNoHash", "c11 c21 | => 2"),
        // A String spelled like a live stream's marker is a String.
        (
            "forgedStream",
            " => java.lang.String,true,class java.lang.String cannot be cast to class java.util.stream.Stream (java.lang.String and java.util.stream.Stream are in module java.base of loader 'bootstrap'),CCE,[1, 2]",
        ),
    ]);
}

/// Runs `method` and answers the unknown host calls it counted.
fn counted(method: &str) -> Vec<(String, usize)> {
    let fixture =
        javac_fixture::Fixture::build("stream-review-counted", &[("demo/Probe.java", PROBE)]);
    let mut host = fixture.jar.seed_host(StdHost::new());
    let _ = fixture.jar.invoke_method_with_args(
        "demo/Probe",
        method,
        "()Ljava/lang/String;",
        None,
        &[],
        &mut host,
    );
    host.unknown_host_calls()
}

/// Counted, not answered: `IntStream.builder()` is
/// `Streams$IntStreamBuilderImpl`, an `IntConsumer` (JDK 25.0.5: ` =>
/// java.util.stream.Streams$IntStreamBuilderImpl,true,false,false,true,[1, 2,
/// 5]`), which is not modelled.
#[test]
fn primitive_stream_builders_are_counted() {
    let unknown = counted("a05IntBuilder");
    assert!(
        unknown
            .iter()
            .any(|(call, _)| call.starts_with("java/util/stream/IntStream#builder")),
        "{unknown:?}"
    );
}

/// Counted, not answered: a stream's class is a pipeline stage's own
/// (JDK 25.0.5: ` => java.util.stream.ReferencePipeline$3,[1]`).
#[test]
fn a_streams_runtime_class_is_counted() {
    let unknown = counted("liveStreamClass");
    assert!(
        unknown
            .iter()
            .any(|(call, _)| call.contains("#getClass()Ljava/lang/Class;")),
        "{unknown:?}"
    );
}

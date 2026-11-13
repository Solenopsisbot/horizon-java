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

    /// The paths that render an object as text (or its hash, which a text
    /// carries on): `text(s, i)` for `i` below `TEXT_PATHS`.
    static final int TEXT_PATHS = 23;

    static String text(Object s, int i) {
        return switch (i) {
            case 0 -> s.toString();
            case 1 -> String.valueOf(s);
            case 2 -> "" + s;
            case 3 -> "<" + s + ">";
            case 4 -> new StringBuilder().append(s).toString();
            case 5 -> new StringBuilder("x").append(s).append(1).toString();
            case 6 -> Objects.toString(s);
            case 7 -> Objects.toString(s, "d");
            case 8 -> String.format("%s", s);
            case 9 -> List.of(s).toString();
            case 10 -> new ArrayList<>(List.of(s)).toString();
            case 11 -> Arrays.asList(s).toString();
            case 12 -> Arrays.toString(new Object[] {s});
            case 13 -> Set.of(s).toString();
            case 14 -> Map.of("k", s).toString();
            case 15 -> Map.of(s, "v").toString();
            case 16 -> new HashMap<>(Map.of("k", s)).toString();
            case 17 -> Map.entry("k", s).toString();
            case 18 -> Optional.of(s).toString();
            case 19 -> String.valueOf(s.hashCode());
            case 20 -> String.valueOf(System.identityHashCode(s));
            case 21 -> String.valueOf(Objects.hashCode(s));
            default -> String.valueOf(List.of(s).hashCode());
        };
    }

    static List<String> texts(Object s) {
        List<String> t = new ArrayList<>();
        for (int i = 0; i < TEXT_PATHS; i++) t.add(text(s, i));
        return t;
    }

    /// One path's text of a live stream (`builder` false) or builder.
    public static String oneText(int i, boolean builder) {
        Object s = builder ? Stream.builder() : Stream.of(1, 2).map(x -> x);
        return text(s, i);
    }

    /// The number of texts holding a marker, and the casts of any substring
    /// of them back to `Stream` that did not throw.
    static String scan(List<String> texts) {
        int leaks = 0, casts = 0;
        for (String t : texts) {
            // A counted text answers the neutral `null` here.
            if (t == null) continue;
            for (String needle : new String[] {"stream/", "hsbld:", "@fn:", "~"}) {
                int i = t.indexOf(needle);
                if (i < 0) continue;
                leaks++;
                for (int start = 0; start <= i; start++) {
                    for (int end = i + 1; end <= t.length(); end++) {
                        Object o = t.substring(start, end);
                        try { Stream<?> x = (Stream<?>) o; casts++; } catch (ClassCastException e) { }
                    }
                }
            }
        }
        return leaks + "," + casts;
    }

    public static String streamTextForgery() {
        Stream<Integer> s = Stream.of(1, 2).map(x -> x);
        Stream.Builder<Integer> b = Stream.builder();
        b.add(3);
        String a = scan(texts(s));
        String c = scan(texts(b));
        return take(a + "|" + c + "|" + s.toList() + b.build().toList());
    }

    static final class Jar { }

    public static String castMessages() {
        Stream<Integer> s = Stream.of(1).map(x -> x);
        Object o = s;
        String a, b;
        try { Jar j = (Jar) o; a = "noCCE"; } catch (ClassCastException e) { a = e.getMessage(); }
        try { Jar.class.cast(o); b = "noCCE"; } catch (ClassCastException e) { b = e.getMessage(); }
        return take(a.contains("stream/") + "," + b.contains("stream/") + "," + s.toList());
    }

    @SuppressWarnings("unchecked")
    public static String forgedBuilderCallback() {
        Stream.Builder<Integer> b = Stream.builder();
        String r;
        try {
            Consumer<Integer> f = (Consumer<Integer>) (Object) "hsbld:0";
            ((Consumer<Integer>) x -> { }).andThen(f).accept(7);
            r = "noCCE";
        } catch (ClassCastException e) { r = "CCE"; }
        return take(r + "," + b.build().toList());
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

/// Runs `method`: its answer and the unknown host calls it counted.
fn run(method: &str) -> (Option<String>, Vec<(String, usize)>) {
    let fixture =
        javac_fixture::Fixture::build("stream-review-text", &[("demo/Probe.java", PROBE)]);
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
        Ok(horizon_native_java::Value::Ref(Some(text))) => Some(text),
        other => Some(format!("{other:?}")),
    };
    (text, host.unknown_host_calls())
}

/// Not JDK behaviour, counted instead: a stream's or builder's text and
/// hash are its class and identity hash, which no run reproduces. Every path
/// that renders one is counted, and none prints its marker: a printed marker
/// would be a capability a jar could cut out of the text and cast back to a
/// stream. JDK 25.0.5 for `streamTextForgery`: ` => 0,0|0,0|[1, 2][3]` (no
/// text holds a marker, no substring casts to `Stream`).
#[test]
fn a_streams_text_is_counted_and_never_its_marker() {
    let (text, _) = run("streamTextForgery");
    assert_eq!(text.as_deref(), Some(" => 0,0|0,0|[1, 2][3]"));
    let fixture =
        javac_fixture::Fixture::build("stream-review-paths", &[("demo/Probe.java", PROBE)]);
    for builder in [false, true] {
        for path in 0..23 {
            let mut host = fixture.jar.seed_host(StdHost::new());
            let answer = fixture.jar.invoke_method_with_args(
                "demo/Probe",
                "oneText",
                "(IZ)Ljava/lang/String;",
                None,
                &[
                    horizon_native_java::Value::Int(path),
                    horizon_native_java::Value::Int(i32::from(builder)),
                ],
                &mut host,
            );
            let unknown = host.unknown_host_calls();
            assert!(
                !unknown.is_empty(),
                "path {path} (builder {builder}) answered {answer:?} uncounted"
            );
        }
    }
}

/// Counted: a failing cast of a stream names its stage class (JDK 25.0.5:
/// ` => false,false,[1]`, neither message holds a marker); the runtime counts
/// the cast instead and never puts the marker in a message.
#[test]
fn a_streams_cast_messages_never_hold_its_marker() {
    let (text, unknown) = run("castMessages");
    assert_eq!(text.as_deref(), Some(" => false,false,[1]"));
    assert!(
        unknown.iter().any(|(call, _)| call.contains("<checkcast>")),
        "{unknown:?}"
    );
}

/// A jar `String` spelled like a builder handle's encoding never reaches the
/// builder through a `Consumer.andThen` composite (JDK 25.0.5: ` => CCE,[]`;
/// the cast itself passing here is #168's legacy-marker gap).
#[test]
fn a_forged_builder_encoding_is_no_builder() {
    let (text, _) = run("forgedBuilderCallback");
    assert!(
        text.as_deref().is_some_and(|text| text.ends_with(",[]")),
        "{text:?}"
    );
}

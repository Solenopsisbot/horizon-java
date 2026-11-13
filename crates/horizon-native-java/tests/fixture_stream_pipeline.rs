//! `java.util.stream` pipelines run lazily, one element at a time, with the
//! JDK's short-circuiting, `sorted` barrier, sized `count()` and per-element
//! terminals (`stream_pipeline.rs`).
//!
//! Every lambda logs; each expected line is OpenJDK 25.0.5's output for the
//! same probe method (`support/jdk_differential.rs`). The eager model ran
//! each stage over the whole list before the next, so the logs came out
//! grouped by stage and `findFirst`/`anyMatch`/`limit` ran every upstream
//! lambda.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;
#[path = "support/jdk_differential.rs"]
mod jdk_differential;

use horizon_native_java::{StdHost, Value};

fn check(cases: &[(&str, &str)]) {
    jdk_differential::check("stream-pipeline", PROBE, cases);
}

const PROBE: &str = r#"package demo;

import java.util.ArrayList;
import java.util.Comparator;
import java.util.List;
import java.util.Optional;
import java.util.stream.Collectors;
import java.util.stream.IntStream;
import java.util.stream.LongStream;
import java.util.stream.Stream;

public final class Probe {
    static final StringBuilder LOG = new StringBuilder();

    static void log(String event) { LOG.append(event).append(' '); }

    static String take(Object result) {
        String text = LOG.toString().trim() + " => " + result;
        LOG.setLength(0);
        return text;
    }

    static <T> T peekLog(String tag, T value) { log(tag + value); return value; }

    public static String parallelCounted() {
        return take(List.of(1, 2, 3).parallelStream().map(x -> x * 2).toList() + ","
            + Stream.of(4, 5).parallel().filter(x -> x > 4).count());
    }

    public static String collectorOf() {
        java.util.stream.Collector<Object, List<Object>, String> c = java.util.stream.Collector.of(
            () -> { log("supply"); return new ArrayList<>(); },
            (l, x) -> { log("acc" + x); l.add(x); },
            (a, b) -> { log("comb"); return a; },
            l -> { log("finish"); return l.toString(); });
        String a = Stream.of("x", "y").peek(x -> log("p" + x)).collect(c);
        java.util.stream.Collector<Object, List<Object>, List<Object>> id = java.util.stream.Collector.of(
            ArrayList::new, List::add, (l, r) -> l, java.util.stream.Collector.Characteristics.IDENTITY_FINISH);
        List<Object> b = Stream.of(1, 2).collect(id);
        java.util.stream.Collector<Object, List<Object>, Object> idf = java.util.stream.Collector.of(
            ArrayList::new, List::add, (l, r) -> l, l -> { log("never"); return l; },
            java.util.stream.Collector.Characteristics.IDENTITY_FINISH);
        Object d = Stream.of(3).collect(idf);
        String nul;
        try { java.util.stream.Collector.<Object, List<Object>>of(null, List::add, (l, r) -> l); nul = "ok"; } catch (NullPointerException e) { nul = "npe"; }
        return take(a + "," + b + "," + d + "," + nul);
    }

    public static String interleave() {
        return take(Stream.of("a", "bb", "c")
            .peek(x -> log("p" + x))
            .map(x -> peekLog("m", x.toUpperCase()))
            .filter(x -> { log("f" + x); return x.length() == 1; })
            .collect(Collectors.toList()));
    }

    public static String findFirstStops() {
        return take(Stream.of(1, 2, 3, 4).peek(x -> log("p" + x)).filter(x -> x > 1).findFirst());
    }

    public static String anyAllNone() {
        boolean any = Stream.of(1, 2, 3).peek(x -> log("a" + x)).anyMatch(x -> x == 2);
        boolean all = Stream.of(1, 2, 3).peek(x -> log("b" + x)).allMatch(x -> x < 2);
        boolean none = Stream.of(1, 2, 3).peek(x -> log("c" + x)).noneMatch(x -> x == 9);
        return take(any + "," + all + "," + none);
    }

    public static String limitStops() {
        Stream.of(1, 2, 3, 4).peek(x -> log("p" + x)).limit(2).forEach(x -> log("e" + x));
        return take(Stream.of(1, 2, 3).peek(x -> log("q" + x)).limit(0).count());
    }

    public static String skipThenLimit() {
        return take(Stream.of(1, 2, 3, 4, 5).peek(x -> log("p" + x)).skip(1).limit(2).map(x -> peekLog("m", x * 10)).toList());
    }

    public static String sortedBarrier() {
        return take(Stream.of(3, 1, 2).peek(x -> log("p" + x)).sorted().peek(x -> log("s" + x)).findFirst());
    }

    public static String sortedForEach() {
        Stream.of("b", "a").peek(x -> log("p" + x)).sorted(Comparator.reverseOrder()).forEach(x -> log("e" + x));
        return take("done");
    }

    public static String sortedThenLimit() {
        Stream.of(3, 1, 2).peek(x -> log("p" + x)).sorted().peek(x -> log("s" + x)).limit(1).forEach(x -> log("e" + x));
        return take(Stream.of("b", "a", "c").sorted().skip(1).toList());
    }

    public static String distinctLazy() {
        return take(Stream.of(1, 2, 1, 3, 2).peek(x -> log("p" + x)).distinct().map(x -> peekLog("m", x)).toList());
    }

    public static String flatMapStops() {
        return take(Stream.of(1, 2).peek(x -> log("o" + x))
            .flatMap(x -> Stream.of(x * 10, x * 10 + 1).peek(y -> log("i" + y)))
            .filter(y -> y % 10 == 1).findFirst());
    }

    public static String flatMapAll() {
        return take(Stream.of(1, 2).flatMap(x -> Stream.of(x, -x).peek(y -> log("i" + y))).map(y -> peekLog("m", y)).toList());
    }

    public static String countSized() {
        long a = List.of(1, 2, 3).stream().peek(x -> log("p" + x)).map(x -> peekLog("m", x)).count();
        long b = List.of(1, 2, 3).stream().peek(x -> log("q" + x)).filter(x -> x > 1).count();
        long c = Stream.of(1, 2).flatMap(x -> Stream.of(x, x)).peek(x -> log("r" + x)).count();
        return take(a + "," + b + "," + c);
    }

    public static String countSlicesConcat() {
        long a = Stream.of(1, 2, 3).peek(x -> log("p" + x)).skip(1).count();
        long b = Stream.concat(Stream.of(1, 2).peek(x -> log("q" + x)), Stream.of(3)).count();
        long c = Stream.concat(Stream.of(1, 2).filter(x -> { log("f" + x); return true; }), Stream.of(3)).count();
        long d = Stream.of(3, 1, 2).sorted((x, y) -> { log("c"); return x - y; }).count();
        return take(a + "," + b + "," + c + "," + d);
    }

    public static String reduceForms() {
        int three = Stream.of(1, 2, 3).reduce(10, (a, b) -> { log("acc" + a + "+" + b); return a + b; },
            (a, b) -> { log("comb"); return a * b; });
        int two = Stream.of(1, 2, 3).peek(x -> log("p" + x)).reduce(0, (a, b) -> { log("r" + b); return a + b; });
        Optional<Integer> one = Stream.of(4, 5).reduce((a, b) -> { log("o" + a + b); return a * b; });
        Optional<Integer> none = Stream.<Integer>empty().reduce((a, b) -> a);
        String text = Stream.of("x", "y").reduce("", (acc, s) -> acc + s, String::concat);
        return take(three + "," + two + "," + one + "," + none + "," + text);
    }

    public static String maxMin() {
        Optional<Integer> max = Stream.of(2, 9, 4).peek(x -> log("p" + x))
            .max((a, b) -> { log("c" + a + b); return Integer.compare(a, b); });
        return take(max + "," + Stream.of(5, 3).min(Comparator.naturalOrder()));
    }

    public static String forEachAndCollect() {
        List<Object> out = Stream.of("a", "b").peek(x -> log("p" + x))
            .collect(() -> { log("supply"); return new ArrayList<>(); },
                     (list, x) -> { log("add" + x); list.add(x); },
                     (l, r) -> log("combine"));
        return take(out);
    }

    public static String builder() {
        Stream.Builder<String> b = Stream.builder();
        b.add("a").add("b");
        b.accept("c");
        Stream<String> s = b.build();
        String after;
        try { b.add("d"); after = "added"; } catch (IllegalStateException e) { after = e.getClass().getName() + ":" + e.getMessage(); }
        String built;
        try { b.build(); built = "built"; } catch (IllegalStateException e) { built = e.getClass().getName() + ":" + e.getMessage(); }
        return take(s.map(x -> peekLog("m", x)).collect(Collectors.joining("|")) + "," + after + "," + built);
    }

    public static String builderEmptyAndOne() {
        return take(Stream.builder().build().count() + "," + Stream.builder().add(7).build().toList());
    }

    public static String builderAsConsumer() {
        Stream.Builder<Object> b = Stream.builder();
        List.of("x", "y").forEach(b);
        return take(b.build().peek(x -> log("p" + x)).count());
    }

    public static String concatLazy() {
        return take(Stream.concat(Stream.of(1, 2).peek(x -> log("a" + x)), Stream.of(3).peek(x -> log("b" + x)))
            .filter(x -> x > 1).findFirst());
    }

    public static String parallelSequential() {
        return take(Stream.of(1, 2).parallel().isParallel() + "," + Stream.of(1, 2).parallel().sequential().isParallel());
    }

    public static String primitiveLazy() {
        Optional<Integer> first = IntStream.range(0, 5).peek(x -> log("p" + x)).map(x -> x * 2)
            .filter(x -> { log("f" + x); return x > 2; }).boxed().findFirst();
        long sum = IntStream.of(1, 2, 3).asLongStream().map(x -> peekLog("m", x) * 10).sum();
        boolean any = LongStream.rangeClosed(1, 4).peek(x -> log("l" + x)).anyMatch(x -> x == 2);
        IntStream.of(3, 1).sorted().forEach(x -> log("e" + x));
        int reduced = IntStream.of(1, 2, 3).reduce(5, (a, b) -> { log("r" + b); return a + b; });
        return take(first + "," + sum + "," + any + "," + reduced);
    }

    public static String nullCallbacks() {
        String map;
        try { Stream.of(1).map(null); map = "ok"; } catch (NullPointerException e) { map = "npe"; }
        String first;
        try { Stream.of((Object) null).findFirst(); first = "ok"; } catch (NullPointerException e) { first = "npe"; }
        String reduce;
        try { Stream.of(1, 2).reduce((a, b) -> null).get(); reduce = "ok"; } catch (NullPointerException e) { reduce = "npe"; }
        String nullFlat = Stream.of(1, 2).flatMap(x -> x == 1 ? null : Stream.of(x)).toList().toString();
        String limit;
        try { Stream.of(1).limit(-1); limit = "ok"; } catch (IllegalArgumentException e) { limit = e.getClass().getName() + ":" + e.getMessage(); }
        String sorted;
        try { Stream.of(2, 1).sorted(null); sorted = "ok"; } catch (NullPointerException e) { sorted = "npe"; }
        String max;
        try { Stream.of(2, 1).max(null); max = "ok"; } catch (NullPointerException e) { max = "npe"; }
        return take(map + "," + first + "," + reduce + "," + nullFlat + "," + limit + "," + sorted + "," + max);
    }
}
"#;

#[test]
fn stages_run_one_element_at_a_time_as_jdk_25_does() {
    check(&[
        ("interleave", "pa mA fA pbb mBB fBB pc mC fC => [A, C]"),
        ("distinctLazy", "p1 m1 p2 m2 p1 p3 m3 p2 => [1, 2, 3]"),
        (
            "flatMapAll",
            "i1 m1 i-1 m-1 i2 m2 i-2 m-2 => [1, -1, 2, -2]",
        ),
        ("skipThenLimit", "p1 p2 m20 p3 m30 => [20, 30]"),
        (
            "primitiveLazy",
            "p0 f0 p1 f2 p2 f4 m1 m2 m3 l1 l2 e1 e3 r1 r2 r3 => Optional[4],60,true,11",
        ),
    ]);
}

#[test]
fn short_circuits_stop_the_source_as_jdk_25_does() {
    check(&[
        ("findFirstStops", "p1 p2 => Optional[2]"),
        ("anyAllNone", "a1 a2 b1 b2 c1 c2 c3 => true,false,true"),
        ("limitStops", "p1 e1 p2 e2 => 0"),
        ("flatMapStops", "o1 i10 i11 => Optional[11]"),
        ("concatLazy", "a1 a2 => Optional[2]"),
        ("sortedBarrier", "p3 p1 p2 s1 => Optional[1]"),
        ("sortedForEach", "pb pa eb ea => done"),
        ("sortedThenLimit", "p3 p1 p2 s1 e1 => [b, c]"),
    ]);
}

#[test]
fn count_skips_sized_pipelines_as_jdk_25_does() {
    check(&[
        ("countSized", "q1 q2 q3 r1 r1 r2 r2 => 3,2,4"),
        ("countSlicesConcat", "f1 f2 => 2,3,3,3"),
    ]);
}

#[test]
fn per_element_terminals_run_as_jdk_25_does() {
    check(&[
        (
            "reduceForms",
            "acc10+1 acc11+2 acc13+3 p1 r1 p2 r2 p3 r3 o45 => 16,6,Optional[20],Optional.empty,xy",
        ),
        ("maxMin", "p2 p9 c29 p4 c94 => Optional[9],Optional[3]"),
        (
            "collectorOf",
            "supply px accx py accy finish => [x, y],[1, 2],[3],npe",
        ),
        ("forEachAndCollect", "supply pa adda pb addb => [a, b]"),
        (
            "nullCallbacks",
            " => npe,npe,npe,[2],java.lang.IllegalArgumentException:-1,npe,npe",
        ),
        ("parallelSequential", " => true,false"),
    ]);
}

#[test]
fn stream_builders_run_as_jdk_25_does() {
    check(&[
        (
            "builder",
            "ma mb mc => a|b|c,java.lang.IllegalStateException:null,java.lang.IllegalStateException:null",
        ),
        ("builderEmptyAndOne", " => 0,[7]"),
        ("builderAsConsumer", " => 2"),
    ]);
}

/// Not JDK behaviour: a parallel pipeline runs sequentially here, so its
/// terminals are counted (`Stream#parallel`). The answers are the sequential
/// ones, which a JVM's parallel run of these associative terminals also gives.
#[test]
fn parallel_pipelines_run_sequentially_and_are_counted() {
    let fixture =
        javac_fixture::Fixture::build("stream-pipeline-parallel", &[("demo/Probe.java", PROBE)]);
    let mut host = fixture.jar.seed_host(StdHost::new());
    let answer = fixture.jar.invoke_method_with_args(
        "demo/Probe",
        "parallelCounted",
        "()Ljava/lang/String;",
        None,
        &[],
        &mut host,
    );
    assert_eq!(answer.unwrap(), Value::Ref(Some(" => [2, 4, 6],1".into())));
    assert_eq!(
        host.unknown_host_calls(),
        vec![(
            "java/util/stream/Stream#parallel()Ljava/util/stream/BaseStream;".to_owned(),
            2
        )]
    );
}

//! A `Collectors.toMap` collector runs its functions per element, and a
//! stream over a host collection binds late and checks it for modification
//! (`stream_pipeline.rs`).
//!
//! Every lambda logs; each expected line is OpenJDK 25.0.5's output for the
//! same probe method (`support/jdk_differential.rs`).

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

    static String err(Throwable e) { return e.getClass().getName() + ":" + e.getMessage(); }

    static <T> T peekLog(String tag, T value) { log(tag + value); return value; }

    public static String toMapInterleaved() {
        Map<String, String> map = Stream.of("a", "b").peek(x -> log("p" + x))
            .collect(Collectors.toMap(x -> peekLog("k", x), x -> peekLog("v", x + "!")));
        String dup;
        try {
            Stream.of("a", "b", "a", "c").peek(x -> log("q" + x))
                .collect(Collectors.toMap(x -> peekLog("k", x), x -> x + "1"));
            dup = "ok";
        } catch (IllegalStateException e) { dup = err(e); }
        Map<String, Integer> merged = Stream.of("a", "b", "a").peek(x -> log("r" + x))
            .collect(Collectors.toMap(x -> x, x -> 1, (u, v) -> { log("m" + u + v); return u + v; }));
        return take(map + "," + dup + "," + merged);
    }

    public static String lateBinding() {
        List<Integer> list = new ArrayList<>(List.of(1));
        Stream<Integer> s = list.stream().map(x -> peekLog("m", x));
        list.add(2);
        s.forEach(x -> log("e" + x));
        Stream<Integer> c = list.stream();
        list.add(3);
        long n = c.count();
        Set<String> set = new HashSet<>(Set.of("a"));
        Stream<String> t = set.stream();
        set.add("b");
        return take(n + "," + t.toList());
    }

    public static String addDuringForEach() {
        List<Integer> list = new ArrayList<>(List.of(1, 2));
        String a;
        try { list.stream().forEach(x -> { log("e" + x); if (x == 1) list.add(9); }); a = "ok"; } catch (ConcurrentModificationException e) { a = err(e); }
        List<Integer> list2 = new ArrayList<>(List.of(1, 2, 3));
        String b;
        try { list2.stream().peek(x -> { log("p" + x); if (x == 1) list2.add(9); }).anyMatch(x -> false); b = "ok"; } catch (ConcurrentModificationException e) { b = err(e); }
        LinkedList<Integer> linked = new LinkedList<>(List.of(1, 2));
        String d;
        try { linked.stream().map(x -> { log("l" + x); if (x == 1) linked.add(7); return x; }).findFirst(); d = "ok"; } catch (ConcurrentModificationException e) { d = err(e); }
        return take(a + "," + b + "," + d + "," + list + "," + list2);
    }


    public static String reuseRefused() {
        Stream<Integer> s = Stream.of(1, 2);
        s.map(x -> x);
        String a;
        try { s.filter(x -> true); a = "ok"; } catch (IllegalStateException e) { a = err(e); }
        Stream<Integer> t = Stream.of(3);
        t.count();
        String b;
        try { t.forEach(x -> log("e" + x)); b = "ok"; } catch (IllegalStateException e) { b = err(e); }
        Stream<Integer> u = Stream.of(4);
        Stream.concat(u, Stream.of(5));
        String c;
        try { u.toList(); c = "ok"; } catch (IllegalStateException e) { c = err(e); }
        return take(a + "," + b + "," + c);
    }

    public static String collectedListIsOwn() {
        List<Integer> l = Stream.of(1, 2).collect(Collectors.toList());
        l.add(3);
        List<Integer> m = Stream.of(1, 2).map(x -> x * 10).collect(Collectors.toList());
        m.set(0, 5);
        return take(l + "," + m);
    }

    public static String toListUnmodifiable() {
        String a;
        try { Stream.of(1, 2).toList().add(3); a = "ok"; } catch (UnsupportedOperationException e) { a = err(e); }
        String b;
        try { Stream.of(1, 2).map(x -> x).toList().set(0, 5); b = "ok"; } catch (UnsupportedOperationException e) { b = err(e); }
        String c;
        try { IntStream.range(0, 2).boxed().toList().add(3); c = "ok"; } catch (UnsupportedOperationException e) { c = err(e); }
        boolean same = Stream.of(1, 2, 3).toList().getClass() == List.of(1, 2, 3).getClass();
        List<Integer> nulls = Stream.of(1, null).toList();
        return take(a + "," + b + "," + c + "," + same + "," + nulls);
    }

    public static String unmodifiableCollectors() {
        String a;
        try { Stream.of(1, 2).collect(Collectors.toUnmodifiableList()).add(3); a = "ok"; } catch (UnsupportedOperationException e) { a = err(e); }
        String b;
        try { Stream.of(1, null).collect(Collectors.toUnmodifiableList()); b = "ok"; } catch (NullPointerException e) { b = "npe"; }
        return take(a + "," + b);
    }

    public static String joiningOrder() {
        String a = IntStream.rangeClosed(1, 4).mapToObj(i -> "v" + i).collect(Collectors.joining("|", "<", ">"));
        String b = Stream.of("x").collect(Collectors.joining(",", "[", "]"));
        String c = Stream.<String>empty().collect(Collectors.joining(",", "[", "]"));
        return take(a + " " + b + " " + c);
    }


    public static String restoreDuringForEach() {
        List<Integer> l = new ArrayList<>(List.of(1, 2, 3));
        l.stream().forEach(x -> { log("e" + x); if (x == 1) { l.add(9); l.remove(3); } });
        return take(l);
    }

    public static String restoreDuringAnyMatch() {
        List<Integer> l = new LinkedList<>(List.of(1, 2, 3));
        return take(l.stream().peek(x -> { if (x == 1) { l.add(9); l.remove(3); } }).anyMatch(x -> x == 3));
    }

    public static String mapViewPutDuringForEach() {
        Map<String, Integer> m = new TreeMap<>(Map.of("a", 1, "b", 2));
        m.values().stream().forEach(x -> { log("e" + x); if (x == 1) m.put("c", 3); });
        return take(m);
    }

    public static String mapViewPutBeforeTerminal() {
        Map<String, Integer> m = new HashMap<>(Map.of("a", 1));
        Stream<String> s = m.keySet().stream();
        m.put("b", 2);
        return take(s.toList());
    }

    public static String sortedLambda() {
        return take(Stream.of(3, 1, 2).sorted((a, b) -> { log("c" + a + b); return a - b; }).toList());
    }

    public static String hashSetAddDuringForEach() {
        Set<Integer> set = new HashSet<>(Set.of(1, 2));
        String c;
        try { set.stream().forEach(x -> { log("s" + x); if (x == 1) set.add(5); }); c = "ok"; } catch (ConcurrentModificationException e) { c = err(e); }
        return take(c);
    }

    public static String listSetDuringFindFirst() {
        List<Integer> list = new ArrayList<>(List.of(1, 2, 3));
        Optional<Integer> f = list.stream().peek(x -> { if (x == 1) list.set(2, 8); }).filter(x -> x > 2).findFirst();
        return take(f);
    }
}
"#;

#[test]
fn to_map_and_host_collection_streams_run_as_jdk_25_does() {
    jdk_differential::check(
        "stream-collections",
        PROBE,
        &[
            (
                "toMapInterleaved",
                "pa ka va! pb kb vb! qa ka qb kb qa ka ra rb ra m11 => {a=a!, b=b!},java.lang.IllegalStateException:Duplicate key a (attempted merging values a1 and a1),{a=2, b=1}",
            ),
            ("lateBinding", "m1 e1 m2 e2 => 3,[a, b]"),
            (
                "addDuringForEach",
                "e1 e2 p1 l1 => java.util.ConcurrentModificationException:null,java.util.ConcurrentModificationException:null,java.util.ConcurrentModificationException:null,[1, 2, 9],[1, 2, 3, 9]",
            ),
            (
                "reuseRefused",
                " => java.lang.IllegalStateException:stream has already been operated upon or closed,java.lang.IllegalStateException:stream has already been operated upon or closed,java.lang.IllegalStateException:stream has already been operated upon or closed",
            ),
            ("collectedListIsOwn", " => [1, 2, 3],[5, 20]"),
            ("joiningOrder", " => <v1|v2|v3|v4> [x] []"),
            (
                "toListUnmodifiable",
                " => java.lang.UnsupportedOperationException:null,java.lang.UnsupportedOperationException:null,java.lang.UnsupportedOperationException:null,true,[1, null]",
            ),
            (
                "unmodifiableCollectors",
                " => java.lang.UnsupportedOperationException:null,npe",
            ),
        ],
    );
}

/// Runs `method` and answers its text and the unknown host calls it counted.
fn run(method: &str) -> (Option<String>, Vec<(String, usize)>) {
    let fixture =
        javac_fixture::Fixture::build("stream-collections-counted", &[("demo/Probe.java", PROBE)]);
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

/// Not JDK behaviour, counted instead: a `HashSet`'s bulk traversal visits
/// an element added behind its cursor (JDK 25: `s1 s2 s5`, then the CME),
/// which depends on the table; the change is counted and the CME still
/// thrown.
#[test]
fn a_set_changed_during_a_bulk_traversal_is_counted() {
    let (text, unknown) = run("hashSetAddDuringForEach");
    assert_eq!(
        text.as_deref(),
        Some("s1 s2 => java.util.ConcurrentModificationException:null")
    );
    assert_eq!(
        unknown,
        vec![(
            "java/util/HashSet#spliterator()Ljava/util/Spliterator;".to_owned(),
            1
        )]
    );
}

/// Not decidable without `modCount`: a list of the same size with different
/// elements (here a `set`, which the JDK does not count as a modification)
/// is counted.
#[test]
fn a_same_size_list_change_during_a_stream_is_counted() {
    let (_, unknown) = run("listSetDuringFindFirst");
    assert_eq!(
        unknown,
        vec![(
            "java/util/ArrayList#spliterator()Ljava/util/Spliterator;".to_owned(),
            1
        )]
    );
}

/// Counted until #105's `modCount` is under this stack: a change that writes
/// a collection and restores its contents moves the JDK's `modCount` (JDK 25:
/// CME), which the contents cannot show; a write to the source during the
/// traversal is counted.
#[test]
fn restoring_changes_during_a_stream_are_counted() {
    for method in ["restoreDuringForEach", "restoreDuringAnyMatch"] {
        let (_, unknown) = run(method);
        assert!(
            unknown
                .iter()
                .any(|(call, _)| call.ends_with("#spliterator()Ljava/util/Spliterator;")),
            "{method}: {unknown:?}"
        );
    }
}

/// Counted: a map view's stream reads a copy, where the JDK's view sees a
/// `put` to its map (and its iterator fails fast).
#[test]
fn map_writes_behind_a_view_stream_are_counted() {
    for method in ["mapViewPutDuringForEach", "mapViewPutBeforeTerminal"] {
        let (_, unknown) = run(method);
        assert!(
            unknown
                .iter()
                .any(|(call, _)| call.ends_with("#spliterator()Ljava/util/Spliterator;")),
            "{method}: {unknown:?}"
        );
    }
}

/// Counted until #110's TimSort is under this stack: a comparator lambda would
/// see the JDK's comparisons in another order.
#[test]
fn sorting_with_a_jar_comparator_is_counted() {
    let (_, unknown) = run("sortedLambda");
    assert!(
        unknown
            .iter()
            .any(|(call, _)| call.starts_with("java/util/stream/Stream#sorted")),
        "{unknown:?}"
    );
}

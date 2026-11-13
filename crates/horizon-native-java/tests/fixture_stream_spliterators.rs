//! Streams over spliterators: a jar's own `Spliterator` and
//! `Spliterators.AbstractSpliterator`, and the JDK's `IteratorSpliterator`
//! and `RandomAccessSpliterator` over jar collections (`Collection.stream()`,
//! `Iterable.spliterator()`, `Spliterators.spliterator`), pulled lazily by
//! the terminal (`jdk_spliterators.rs`, `stream_pipeline.rs`).
//!
//! Every jar member logs; each expected line is OpenJDK 25.0.5's output for
//! the same probe method (`support/jdk_differential.rs`). Before this,
//! `StreamSupport.stream` drained a jar spliterator when the stream was made,
//! `Collection.stream()` of a jar collection drained its iterator, and
//! `Spliterators.spliterator` over a jar collection was counted.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;
#[path = "support/jdk_differential.rs"]
mod jdk_differential;

use horizon_native_java::StdHost;

fn check(cases: &[(&str, &str)]) {
    jdk_differential::check("stream-spliterators", PROBE, cases);
}

const PROBE: &str = r#"package demo;

import java.util.AbstractCollection;
import java.util.AbstractList;
import java.util.ArrayList;
import java.util.AbstractSet;
import java.util.Arrays;
import java.util.Comparator;
import java.util.Iterator;
import java.util.List;
import java.util.Optional;
import java.util.RandomAccess;
import java.util.Spliterator;
import java.util.Spliterators;
import java.util.function.Consumer;
import java.util.stream.Collectors;
import java.util.stream.Stream;
import java.util.stream.StreamSupport;

public final class Probe {
    static final StringBuilder LOG = new StringBuilder();

    static void log(String event) { LOG.append(event).append(' '); }

    static String take(Object result) {
        String text = LOG.toString().trim() + " => " + result;
        LOG.setLength(0);
        return text;
    }

    static <T> T peekLog(String tag, T value) { log(tag + value); return value; }

    /// A jar collection: its iterator and size log.
    static class Bag extends AbstractCollection<Object> {
        final Object[] items;
        Bag(Object... items) { this.items = items; }
        @Override public int size() { log("size"); return items.length; }
        @Override public Iterator<Object> iterator() {
            log("iterator");
            return new Iterator<>() {
                int next;
                @Override public boolean hasNext() { log("hasNext" + next); return next < items.length; }
                @Override public Object next() { log("next" + next); return items[next++]; }
            };
        }
    }

    /// A jar collection whose spliterator() is its own, as Guava's ImmutableCollection does.
    static final class OwnSplit extends Bag {
        OwnSplit(Object... items) { super(items); }
        @Override public Spliterator<Object> spliterator() {
            log("spliterator");
            return Spliterators.spliterator(this, Spliterator.IMMUTABLE | Spliterator.NONNULL | Spliterator.ORDERED);
        }
    }

    /// An Iterable that is not a Collection.
    static final class Walk implements Iterable<Object> {
        final Object[] items;
        Walk(Object... items) { this.items = items; }
        @Override public Iterator<Object> iterator() { return new Bag(items).iterator(); }
    }

    /// A jar spliterator: every member logs.
    static class Split implements Spliterator<Object> {
        final Object[] items;
        int next;
        final int flags;
        Split(int flags, Object... items) { this.flags = flags; this.items = items; }
        @Override public boolean tryAdvance(Consumer<? super Object> action) {
            log("tryAdvance" + next);
            if (next >= items.length) return false;
            action.accept(items[next++]);
            return true;
        }
        @Override public Spliterator<Object> trySplit() { log("trySplit"); return null; }
        @Override public long estimateSize() { log("estimateSize"); return items.length - next; }
        @Override public int characteristics() { log("characteristics"); return flags; }
    }

    /// A RandomAccess jar list: get and size log.
    static final class Row extends AbstractList<Object> implements RandomAccess {
        final Object[] items;
        Row(Object... items) { this.items = items; }
        @Override public int size() { log("size"); return items.length; }
        @Override public Object get(int i) { log("get" + i); return items[i]; }
    }

    /// A jar spliterator whose forEachRemaining is its own.
    static final class BulkSplit implements Spliterator<Object> {
        final Object[] items;
        int next;
        BulkSplit(Object... items) { this.items = items; }
        @Override public boolean tryAdvance(Consumer<? super Object> action) {
            log("tryAdvance" + next);
            if (next >= items.length) return false;
            action.accept(items[next++]);
            return true;
        }
        @Override public void forEachRemaining(Consumer<? super Object> action) {
            log("bulk");
            while (next < items.length) { action.accept(items[next++]); log("after"); }
        }
        @Override public Spliterator<Object> trySplit() { return null; }
        @Override public long estimateSize() { log("estimateSize"); return items.length - next; }
        @Override public int characteristics() { log("characteristics"); return Spliterator.ORDERED; }
    }

    /// A jar collection whose iterator has its own forEachRemaining.
    static final class BulkBag extends Bag {
        BulkBag(Object... items) { super(items); }
        @Override public Iterator<Object> iterator() {
            log("iterator");
            Iterator<Object> inner = super.iterator();
            LOG.setLength(LOG.length() - "iterator ".length());
            return new Iterator<>() {
                @Override public boolean hasNext() { return inner.hasNext(); }
                @Override public Object next() { return inner.next(); }
                @Override public void forEachRemaining(Consumer<? super Object> action) { log("bulk"); Iterator.super.forEachRemaining(action); }
            };
        }
    }

    static final class Countdown extends Spliterators.AbstractSpliterator<Object> {
        int left;
        Countdown(int n) { super(n, Spliterator.SIZED | Spliterator.ORDERED); left = n; }
        @Override public boolean tryAdvance(Consumer<? super Object> action) {
            log("adv" + left);
            if (left == 0) return false;
            action.accept(left--);
            return true;
        }
    }

    /// A jar collection whose iterator is a host list's.
    static final class HBag extends AbstractCollection<Integer> {
        final List<Integer> items;
        HBag(Integer... items) { this.items = new ArrayList<>(Arrays.asList(items)); }
        @Override public int size() { log("size"); return items.size(); }
        @Override public Iterator<Integer> iterator() { log("iter"); return items.iterator(); }
    }

    /// A jar set whose iterator is `Arrays.asList(...).iterator()`.
    static final class ASet extends AbstractSet<Integer> {
        @Override public int size() { return 2; }
        @Override public Iterator<Integer> iterator() { return Arrays.asList(1, 2).iterator(); }
    }

    /// A jar Iterable over `List.of`.
    static final class HostWalk implements Iterable<String> {
        @Override public Iterator<String> iterator() { log("giter"); return List.of("x", "y").iterator(); }
    }

    public static String hostIteratorToList() { return take(new HBag(1, 2, 3).stream().toList()); }

    public static String hostIteratorForEach() {
        new HBag(1, 2).stream().forEach(x -> log("e" + x));
        return take("done");
    }

    public static String arraysAsListSet() { return take(new ASet().stream().toList()); }

    public static String sizedHostIterator() {
        return take(StreamSupport.stream(Spliterators.spliterator(List.of(1, 2).iterator(), 2, Spliterator.ORDERED), false).toList());
    }

    public static String unknownSizeForEach() {
        Spliterators.spliteratorUnknownSize(List.of(1, 2).iterator(), 0).forEachRemaining(x -> log("e" + x));
        return take("done");
    }

    public static String iterableOverListOf() {
        HostWalk it = new HostWalk();
        Spliterator<String> sp = it.spliterator();
        log("made");
        log("c" + sp.characteristics());
        log("e" + sp.estimateSize());
        return take(StreamSupport.stream(sp, false).toList());
    }


    public static String concatOfJarSpliterator() {
        return take(Stream.concat(StreamSupport.stream(new Split(Spliterator.ORDERED, "a"), false), Stream.of("b")).toList());
    }

    public static String jarSpliteratorForEach() {
        Stream<Object> s = StreamSupport.stream(new Split(Spliterator.ORDERED, "a", "b"), false);
        log("made");
        s.map(x -> peekLog("m", x)).forEach(x -> log("e" + x));
        return take("done");
    }

    public static String jarSpliteratorFindFirst() {
        Stream<Object> s = StreamSupport.stream(new Split(Spliterator.ORDERED | Spliterator.SIZED, "a", "b", "c"), false);
        log("made");
        return take(s.filter(x -> !x.equals("a")).findFirst());
    }

    public static String jarSpliteratorCount() {
        long sized = StreamSupport.stream(new Split(Spliterator.SIZED, "a", "b"), false).count();
        log("|");
        long unsized = StreamSupport.stream(new Split(0, "a", "b"), false).count();
        return take(sized + "," + unsized);
    }

    public static String jarSpliteratorToList() {
        return take(StreamSupport.stream(new Split(Spliterator.ORDERED, 1, 2), false).map(x -> peekLog("m", x)).toList());
    }

    public static String jarSpliteratorSizedToList() {
        return take(StreamSupport.stream(new Split(Spliterator.SIZED, 1, 2), false).map(x -> peekLog("m", x)).toList());
    }

    public static String jarSpliteratorBulk() {
        StreamSupport.stream(new BulkSplit("a", "b"), false).map(x -> peekLog("m", x)).forEach(x -> log("e" + x));
        log("|");
        Optional<Object> f = StreamSupport.stream(new BulkSplit("a", "b"), false).findFirst();
        return take(f);
    }

    public static String jarSpliteratorSorted() {
        Split sp = new Split(Spliterator.SORTED | Spliterator.ORDERED, "a") {
            @Override public Comparator<? super Object> getComparator() { log("getComparator"); return null; }
        };
        Stream<Object> s = StreamSupport.stream(sp, false);
        log("made");
        return take(s.toList());
    }

    public static String abstractSpliterator() {
        Countdown c = new Countdown(2);
        String flags = c.characteristics() + "," + c.estimateSize() + "," + c.getExactSizeIfKnown() + "," + c.hasCharacteristics(Spliterator.SUBSIZED);
        long n = StreamSupport.stream(new Countdown(3), false).count();
        List<Object> all = StreamSupport.stream(new Countdown(2), false).map(x -> peekLog("m", x)).toList();
        return take(flags + "," + n + "," + all);
    }

    public static String collectionStreamForEach() {
        Stream<Object> s = new Bag("a", "b").stream();
        log("made");
        s.map(x -> peekLog("m", x)).forEach(x -> log("e" + x));
        return take("done");
    }

    public static String collectionStreamFindFirst() {
        Stream<Object> s = new Bag("a", "b", "c").stream();
        log("made");
        return take(s.peek(x -> log("p" + x)).findFirst());
    }

    public static String collectionStreamCount() {
        long a = new Bag("a", "b", "c").stream().map(x -> peekLog("m", x)).count();
        log("|");
        long b = new Bag("a", "b", "c").stream().filter(x -> true).count();
        return take(a + "," + b);
    }

    public static String collectionStreamToList() {
        return take(new Bag("a", "b").stream().map(x -> peekLog("m", x)).collect(Collectors.toList()));
    }

    public static String collectOrderWithSource() {
        List<Object> out = new Bag("a").stream().collect(() -> { log("supply"); return new ArrayList<>(); }, List::add, List::addAll);
        return take(out);
    }

    public static String iteratorBulk() {
        new BulkBag("a", "b").stream().forEach(x -> log("e" + x));
        return take("done");
    }

    public static String ownSpliteratorStream() {
        Stream<Object> s = new OwnSplit("a", "b").stream();
        log("made");
        return take(s.map(x -> peekLog("m", x)).toList());
    }

    public static String ownSpliteratorFindFirst() {
        Stream<Object> s = new OwnSplit("a", "b").stream();
        log("made");
        return take(s.findFirst());
    }

    public static String iterableSpliterator() {
        Walk walk = new Walk("a", "b");
        Spliterator<Object> sp = walk.spliterator();
        log("split");
        String flags = sp.characteristics() + "," + sp.estimateSize() + "," + sp.getExactSizeIfKnown();
        log(flags);
        return take(StreamSupport.stream(sp, false).map(x -> peekLog("m", x)).toList());
    }

    public static String spliteratorMembers() {
        Spliterator<Object> sp = new Bag("a", "b", "c").spliterator();
        log("split");
        int flags = sp.characteristics();
        long est = sp.estimateSize();
        boolean first = sp.tryAdvance(x -> log("t" + x));
        long est2 = sp.estimateSize();
        sp.forEachRemaining(x -> log("f" + x));
        boolean more = sp.tryAdvance(x -> log("never"));
        String cmp;
        try { sp.getComparator(); cmp = "cmp"; } catch (IllegalStateException e) { cmp = e.getClass().getName(); }
        return take(flags + "," + est + "," + first + "," + est2 + "," + more + "," + cmp + "," + sp.hasCharacteristics(Spliterator.SIZED));
    }

    public static String unknownSizeMembers() {
        Spliterator<Object> sp = Spliterators.spliteratorUnknownSize(new Bag("a", "b").iterator(), Spliterator.ORDERED | Spliterator.SIZED);
        log("split");
        return take(sp.characteristics() + "," + sp.estimateSize() + "," + sp.getExactSizeIfKnown() + "," + sp.tryAdvance(x -> log("t" + x)));
    }

    public static String spliteratorSizedIterator() {
        Spliterator<Object> sp = Spliterators.spliterator(new Bag("a", "b").iterator(), 5, Spliterator.ORDERED);
        log("split");
        return take(sp.characteristics() + "," + sp.estimateSize() + "," + sp.tryAdvance(x -> log("t" + x)) + "," + sp.estimateSize());
    }

    public static String randomAccessStream() {
        Stream<Object> s = new Row("a", "b", "c").stream();
        log("made");
        Optional<Object> first = s.peek(x -> log("p" + x)).findFirst();
        log("|");
        long n = new Row("a", "b").stream().map(x -> peekLog("m", x)).count();
        log("|");
        List<Object> all = new Row("a", "b").stream().map(x -> peekLog("m", x)).toList();
        return take(first + "," + n + "," + all);
    }

    public static String randomAccessMembers() {
        Spliterator<Object> sp = new Row("a", "b").spliterator();
        log("split");
        String a = sp.characteristics() + "," + sp.estimateSize();
        sp.tryAdvance(x -> log("t" + x));
        String b = sp.estimateSize() + "";
        sp.forEachRemaining(x -> log("f" + x));
        return take(a + "," + b);
    }
}
"#;

#[test]
fn jar_spliterators_stream_lazily_as_jdk_25_does() {
    check(&[
        (
            "jarSpliteratorForEach",
            "characteristics made characteristics tryAdvance0 ma ea tryAdvance1 mb eb tryAdvance2 => done",
        ),
        (
            "jarSpliteratorFindFirst",
            "characteristics made characteristics estimateSize tryAdvance0 tryAdvance1 => Optional[b]",
        ),
        (
            "jarSpliteratorCount",
            "characteristics characteristics estimateSize | characteristics characteristics tryAdvance0 tryAdvance1 tryAdvance2 => 2,2",
        ),
        (
            "jarSpliteratorToList",
            "characteristics characteristics tryAdvance0 m1 tryAdvance1 m2 tryAdvance2 => [1, 2]",
        ),
        (
            "jarSpliteratorSizedToList",
            "characteristics characteristics estimateSize characteristics estimateSize tryAdvance0 m1 tryAdvance1 m2 tryAdvance2 => [1, 2]",
        ),
        (
            "jarSpliteratorBulk",
            "characteristics characteristics bulk ma ea after mb eb after | characteristics characteristics tryAdvance0 => Optional[a]",
        ),
        (
            "jarSpliteratorSorted",
            "characteristics getComparator made characteristics tryAdvance0 tryAdvance1 => [a]",
        ),
        (
            "abstractSpliterator",
            "adv2 m2 adv1 m1 adv0 => 16464,2,2,true,3,[2, 1]",
        ),
    ]);
}

#[test]
fn collection_streams_pull_the_iterator_as_jdk_25_does() {
    check(&[
        (
            "collectionStreamForEach",
            "made iterator size hasNext0 next0 ma ea hasNext1 next1 mb eb hasNext2 => done",
        ),
        (
            "collectionStreamFindFirst",
            "made iterator size hasNext0 next0 pa => Optional[a]",
        ),
        (
            "collectionStreamCount",
            "iterator size | iterator size hasNext0 next0 hasNext1 next1 hasNext2 next2 hasNext3 => 3,3",
        ),
        (
            "collectionStreamToList",
            "iterator size hasNext0 next0 ma hasNext1 next1 mb hasNext2 => [a, b]",
        ),
        (
            "collectOrderWithSource",
            "iterator size supply hasNext0 next0 hasNext1 => [a]",
        ),
        (
            "iteratorBulk",
            "iterator size bulk hasNext0 next0 ea hasNext1 next1 eb hasNext2 => done",
        ),
        (
            "ownSpliteratorStream",
            "spliterator made iterator size hasNext0 next0 ma hasNext1 next1 mb hasNext2 => [a, b]",
        ),
        (
            "ownSpliteratorFindFirst",
            "spliterator made iterator size hasNext0 next0 => Optional[a]",
        ),
        (
            "randomAccessStream",
            "made size get0 pa | size | size get0 ma get1 mb => Optional[a],2,[a, b]",
        ),
    ]);
}

#[test]
fn jdk_spliterators_run_as_jdk_25_does() {
    check(&[
        (
            "iterableSpliterator",
            "iterator split 0,9223372036854775807,-1 hasNext0 next0 ma hasNext1 next1 mb hasNext2 => [a, b]",
        ),
        (
            "spliteratorMembers",
            "split iterator size hasNext0 next0 ta hasNext1 next1 fb hasNext2 next2 fc hasNext3 hasNext3 => 16448,3,true,3,false,java.lang.IllegalStateException,true",
        ),
        (
            "unknownSizeMembers",
            "iterator split hasNext0 next0 ta => 16,9223372036854775807,-1,true",
        ),
        (
            "spliteratorSizedIterator",
            "iterator split hasNext0 next0 ta => 16464,5,true,5",
        ),
        (
            "randomAccessMembers",
            "split size get0 ta get1 fb => 16464,2,1",
        ),
    ]);
}

/// Not JDK behaviour, counted instead: `Stream.concat` asks a jar
/// spliterator side's `estimateSize()`/`characteristics()` when it is made,
/// which the lazy push asks later.
#[test]
fn concat_over_a_jar_spliterator_is_counted() {
    let fixture =
        javac_fixture::Fixture::build("stream-spliterators-concat", &[("demo/Probe.java", PROBE)]);
    let mut host = fixture.jar.seed_host(StdHost::new());
    let _ = fixture.jar.invoke_method_with_args(
        "demo/Probe",
        "concatOfJarSpliterator",
        "()Ljava/lang/String;",
        None,
        &[],
        &mut host,
    );
    assert!(
        host.unknown_host_calls()
            .iter()
            .any(|(call, _)| call.starts_with("java/util/stream/Stream#concat")),
        "{:?}",
        host.unknown_host_calls()
    );
}

/// A jar collection or iterable whose iterator is a host collection's: the
/// spliterator hands the action to the host iterator's `forEachRemaining`,
/// which runs it (#141). Review of #132 found each streaming nothing.
#[test]
fn host_iterators_under_jar_spliterators_stream_as_jdk_25_does() {
    check(&[
        ("arraysAsListSet", " => [1, 2]"),
        ("hostIteratorForEach", "iter size e1 e2 => done"),
        ("hostIteratorToList", "iter size => [1, 2, 3]"),
        (
            "iterableOverListOf",
            "giter made c0 e9223372036854775807 => [x, y]",
        ),
        ("sizedHostIterator", " => [1, 2]"),
        ("unknownSizeForEach", "e1 e2 => done"),
    ]);
}

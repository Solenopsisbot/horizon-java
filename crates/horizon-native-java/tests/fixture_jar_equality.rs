//! Host collections and modelled comparators compare jar objects with the
//! jar's own `equals` and `compareTo` (`jar_equality.rs`).
//!
//! Before this, a record key was found by identity (`map.get(new Key(1))`
//! answered `null`, `Set.add` stored a duplicate, `indexOf` answered `-1`),
//! and jar `Comparable`s sorted in construction order (by their marker
//! strings). Every one of those was uncounted.
//!
//! Every expected value is OpenJDK 25.0.5's output for the same `Probe`
//! (`javac --release 21`, then `java -cp` calling each method), pasted
//! verbatim.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::StdHost;
use javac_fixture::Fixture;

const PROBE: &str = r#"package demo;

import java.util.ArrayList;
import java.util.Collections;
import java.util.Comparator;
import java.util.HashMap;
import java.util.HashSet;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.stream.Collectors;
import java.util.stream.Stream;

public final class Probe {
    record Key(int id) { }

    /// A class that keeps `Object.equals`: two instances are two elements.
    static final class Plain { }

    record Ver(int n) implements Comparable<Ver> {
        @Override public int compareTo(Ver other) { return Integer.compare(n, other.n); }
    }

    enum Mode { LOW, MID, HIGH }

    public static String mapKeys() {
        Map<Key, String> map = new HashMap<>();
        map.put(new Key(1), "a");
        String replaced = map.put(new Key(1), "b");
        return map.get(new Key(1)) + "|" + replaced + "|" + map.containsKey(new Key(1))
            + "|" + map.containsKey(new Key(2)) + "|" + map.size()
            + "|" + map.getOrDefault(new Key(2), "none");
    }

    public static String setAndList() {
        Set<Key> set = new HashSet<>();
        boolean first = set.add(new Key(3));
        boolean second = set.add(new Key(3));
        List<Key> list = new ArrayList<>();
        list.add(new Key(5));
        list.add(new Key(6));
        boolean removed = list.remove(new Key(5));
        return first + "|" + second + "|" + set.size() + "|" + set.contains(new Key(3))
            + "|" + list.indexOf(new Key(6)) + "|" + removed + "|" + list.size();
    }

    public static String identityKeys() {
        Set<Plain> set = new HashSet<>();
        Plain one = new Plain();
        set.add(one);
        set.add(new Plain());
        return set.size() + "|" + set.contains(one) + "|" + set.contains(new Plain());
    }

    public static String naturalOrder() {
        String streamed = Stream.of(new Ver(3), new Ver(1), new Ver(2))
            .sorted()
            .map(v -> String.valueOf(v.n()))
            .collect(Collectors.joining(","));
        List<Ver> list = new ArrayList<>(List.of(new Ver(9), new Ver(7), new Ver(8)));
        list.sort(null);
        List<Ver> sorted = new ArrayList<>(List.of(new Ver(20), new Ver(10)));
        Collections.sort(sorted);
        int direct = Comparator.<Ver>naturalOrder().compare(new Ver(5), new Ver(4));
        return streamed + "|" + list.get(0).n() + list.get(1).n() + list.get(2).n()
            + "|" + sorted.get(0).n() + "|" + direct;
    }

    public static String enumOrder() {
        return Stream.of(Mode.HIGH, Mode.LOW, Mode.MID)
            .sorted()
            .map(Mode::name)
            .collect(Collectors.joining(","));
    }
}
"#;

/// Runs `method`, checking that nothing was counted as unknown.
fn exact(method: &str) -> String {
    let fixture = Fixture::build(
        &format!("jar-equality-{method}"),
        &[("demo/Probe.java", PROBE)],
    );
    let mut host = fixture.jar.seed_host(StdHost::new());
    let text = fixture.text_on(&mut host, method);
    assert_eq!(
        host.unknown_host_call_count(),
        0,
        "{method}: {:?}",
        host.unknown_host_calls()
    );
    text
}

/// `get`, `put` (replacing the value under the original key), `containsKey`,
/// `size` and `getOrDefault` with record keys.
#[test]
fn a_host_map_finds_a_record_key_by_equals() {
    assert_eq!(exact("mapKeys"), "b|a|true|false|1|none");
}

/// `Set.add` of an equal record answers `false`; `List.remove(Object)` and
/// `indexOf` find the equal element.
#[test]
fn host_sets_and_lists_compare_records_by_equals() {
    assert_eq!(exact("setAndList"), "true|false|1|true|0|true|1");
}

/// A class that keeps `Object.equals` is still compared by identity.
#[test]
fn identity_equality_is_unchanged() {
    assert_eq!(exact("identityKeys"), "2|true|false");
}

/// `sorted()`, `List.sort(null)`, `Collections.sort` and
/// `Comparator.naturalOrder()` run the record's `compareTo`.
#[test]
fn natural_order_runs_the_jar_compare_to() {
    assert_eq!(exact("naturalOrder"), "1,2,3|789|10|1");
}

/// Enum constants sort by ordinal, and `Mode::name` as a method reference
/// answers the constant's name as the direct call does.
#[test]
fn enum_constants_sort_by_ordinal() {
    assert_eq!(exact("enumOrder"), "LOW,MID,HIGH");
}

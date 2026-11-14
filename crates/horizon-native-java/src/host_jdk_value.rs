//! JDK value objects as host handles (#156 P1a): `java.util.UUID`, `java.time.Duration` and
//! `java.net.URI`. They were the text `uuid/<hex>`, `duration/<nanos>` and `uri/<text>`, which a
//! String could spell: `"uuid/…"` answered `equals`/`toString` as that UUID, and
//! `String.valueOf("duration/5")` printed `PT0.000000005S`.
//!
//! All three classes are `final` with Java 25's whole supertype list known (`jdk_closure.rs`),
//! so `instanceof` and the class are exact (`ClassRule::Fixed`). Their `equals` are specified by
//! content; their identities are not, so a key names a value, not an object:
//!
//! * **`UUID`**: key the canonical lowercase text. `equals` compares the 128 bits, `hashCode` is
//!   `(int) (hilo >> 32) ^ (int) hilo` with `hilo = msb ^ lsb`, `toString` the canonical text,
//!   `compareTo` compares `msb` then `lsb` as **signed** longs (Java 25 `Long.compare`). Every
//!   `fromString`/`randomUUID` makes a new object: never canonical.
//! * **`Duration`**: key the total nanoseconds (an `i128`, wider than `long`: seconds and nanos).
//!   `equals` by value, `hashCode` `(int) (seconds ^ (seconds >>> 32)) + 51 * nanos`, `compareTo`
//!   `Long.compare(seconds)` then `nanos - other.nanos` (not a sign), `toString` ISO-8601. Every
//!   zero duration is `Duration.ZERO` (`create(0, 0)` answers the constant), so key `0` is
//!   canonical; any other may be one object or two (`d.plus(ZERO)` is `d`): never canonical.
//! * **`URI`**: key its text. `toString` is that text. `equals` compares the parsed components
//!   (scheme and host case-insensitively, escapes by value): the same text is equal, two texts
//!   are counted. `hashCode` and `compareTo` are the components': counted. Never canonical.

use crate::host_ref::{HostKind, HostRef, Methods, Nested, instance_of_class};
use crate::{NativeError, Value};

/// `UUID.fromString`/`randomUUID`'s object of the canonical `text`.
pub(crate) fn uuid(canonical: &str) -> Value {
    Value::Host(HostRef::new(HostKind::Uuid, canonical))
}

/// The canonical text of a UUID handle, `None` for any other value.
pub(crate) fn uuid_text(value: &Value) -> Option<&str> {
    match value {
        Value::Host(handle) if handle.kind() == HostKind::Uuid => Some(handle.key()),
        _ => None,
    }
}

/// The two halves of a canonical UUID text.
fn uuid_bits(text: &str) -> Option<(i64, i64)> {
    let hex: String = text.chars().filter(|c| *c != '-').collect();
    let bits = u128::from_str_radix(&hex, 16).ok()?;
    Some(((bits >> 64) as u64 as i64, bits as u64 as i64))
}

fn uuid_hash(handle: &HostRef, _: &mut dyn Nested) -> Result<Option<i32>, NativeError> {
    Ok(uuid_bits(handle.key()).map(|(msb, lsb)| {
        let hilo = msb ^ lsb;
        ((hilo >> 32) as i32) ^ (hilo as i32)
    }))
}

fn same_kind_equals(
    handle: &HostRef,
    other: &Value,
    _: &mut dyn Nested,
) -> Result<Option<bool>, NativeError> {
    Ok(Some(matches!(
        other,
        Value::Host(other) if other.kind() == handle.kind() && other.key() == handle.key()
    )))
}

fn uuid_compare(
    handle: &HostRef,
    other: &Value,
    _: &mut dyn Nested,
) -> Result<Option<i32>, NativeError> {
    let (Some(own), Some(theirs)) = (
        uuid_bits(handle.key()),
        uuid_text(other).and_then(uuid_bits),
    ) else {
        return Ok(None);
    };
    Ok(Some(match own.0.cmp(&theirs.0).then(own.1.cmp(&theirs.1)) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }))
}

pub(crate) const UUID_METHODS: Methods = Methods {
    to_string: |handle, _| Ok(Some(handle.key().to_owned())),
    hash_code: uuid_hash,
    equals: same_kind_equals,
    instance_of: instance_of_class,
    // A value any jar reads the same way.
    crosses_jars: |_, _| true,
    compare_to: Some(uuid_compare),
};

/// A `Duration` of `nanos` nanoseconds.
pub(crate) fn duration(nanos: i128) -> Value {
    Value::Host(HostRef::new(HostKind::Duration, nanos.to_string()))
}

/// The nanoseconds a `Duration` handle holds, `None` for any other value.
pub(crate) fn duration_nanos(value: &Value) -> Option<i128> {
    match value {
        Value::Host(handle) if handle.kind() == HostKind::Duration => handle.key().parse().ok(),
        _ => None,
    }
}

/// `(seconds, nanos)` as `Duration` keeps them: `nanos` in `0..1e9`.
fn seconds_nanos(nanos: i128) -> (i64, i32) {
    (
        nanos.div_euclid(1_000_000_000) as i64,
        nanos.rem_euclid(1_000_000_000) as i32,
    )
}

fn duration_hash(handle: &HostRef, _: &mut dyn Nested) -> Result<Option<i32>, NativeError> {
    Ok(handle.key().parse::<i128>().ok().map(|nanos| {
        let (seconds, nanos) = seconds_nanos(nanos);
        ((seconds ^ ((seconds as u64) >> 32) as i64) as i32).wrapping_add(nanos.wrapping_mul(51))
    }))
}

fn duration_compare(
    handle: &HostRef,
    other: &Value,
    _: &mut dyn Nested,
) -> Result<Option<i32>, NativeError> {
    let (Ok(own), Some(theirs)) = (handle.key().parse::<i128>(), duration_nanos(other)) else {
        return Ok(None);
    };
    let ((own_seconds, own_nanos), (their_seconds, their_nanos)) =
        (seconds_nanos(own), seconds_nanos(theirs));
    Ok(Some(match own_seconds.cmp(&their_seconds) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Greater => 1,
        std::cmp::Ordering::Equal => own_nanos - their_nanos,
    }))
}

pub(crate) const DURATION_METHODS: Methods = Methods {
    to_string: |handle, _| {
        Ok(handle
            .key()
            .parse::<i128>()
            .ok()
            .map(crate::duration_text))
    },
    hash_code: duration_hash,
    equals: same_kind_equals,
    instance_of: instance_of_class,
    crosses_jars: |_, _| true,
    compare_to: Some(duration_compare),
};

/// `Duration.ZERO` is every zero duration; any other key may be one object or two.
pub(crate) fn duration_canonical(key: &str) -> bool {
    key == "0"
}

/// `URI.create(text)`'s object.
pub(crate) fn uri(text: &str) -> Value {
    Value::Host(HostRef::new(HostKind::Uri, text))
}

/// The text of a URI handle, `None` for any other value.
pub(crate) fn uri_text(value: &Value) -> Option<&str> {
    match value {
        Value::Host(handle) if handle.kind() == HostKind::Uri => Some(handle.key()),
        _ => None,
    }
}

fn uri_equals(
    handle: &HostRef,
    other: &Value,
    _: &mut dyn Nested,
) -> Result<Option<bool>, NativeError> {
    Ok(match uri_text(other) {
        // One text parses to equal components; two texts may still be equal (`HTTP://a` and
        // `http://a`, `%7e` and `%7E`): counted.
        Some(theirs) if theirs == handle.key() => Some(true),
        Some(_) => None,
        None => Some(false),
    })
}

pub(crate) const URI_METHODS: Methods = Methods {
    to_string: |handle, _| Ok(Some(handle.key().to_owned())),
    hash_code: |_, _| Ok(None),
    equals: uri_equals,
    instance_of: instance_of_class,
    crosses_jars: |_, _| true,
    compare_to: None,
};

#[cfg(test)]
#[path = "host_jdk_value_tests.rs"]
mod tests;

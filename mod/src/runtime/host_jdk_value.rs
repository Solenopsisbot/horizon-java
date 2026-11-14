//! JDK value objects as host handles (#156 P1a): `java.util.UUID`, `java.time.Duration` and
//! `java.net.URI`. They were the text `uuid/<hex>`, `duration/<nanos>` and `uri/<text>`, which a
//! String could spell: `"uuid/…"` answered `equals`/`toString` as that UUID, and
//! `String.valueOf("duration/5")` printed `PT0.000000005S`.
//!
//! All three classes are `final`, so the class is exact (`ClassRule::Fixed`). `UUID`'s and
//! `URI`'s Java 25 supertypes are in `jdk_supertypes_25.txt`, so their `instanceof` is exact;
//! `Duration` is not there yet, so its `instanceof Comparable` and the like are counted. Their
//! `equals` are specified by content; their identities are not, so a key names a value, not
//! an object:
//!
//! * **`UUID`**: key the canonical lowercase text. `equals` compares the 128 bits, `hashCode` is
//!   `(int) (hilo >> 32) ^ (int) hilo` with `hilo = msb ^ lsb`, `toString` the canonical text,
//!   `compareTo` compares `msb` then `lsb` as **signed** longs (Java 25 `Long.compare`). Every
//!   `fromString`/`randomUUID` makes a new object: never canonical.
//!
//!   A jar's `new UUID(msb, lsb)` is no handle: `new` made a jar object (`java/util/UUID#n`)
//!   before the constructor ran. The host keeps its two halves by the object's key
//!   (`StdHost::constructed_uuids`, read through `Host::constructed_uuid` and
//!   `Nested::uuid_bits`), and its methods answer from them with the same formulas, so
//!   `new UUID(1, 2)` equals `fromString("00000000-0000-0001-0000-000000000002")` both ways.
//!   One whose constructor the host did not see is counted, never printed as its key.
//! * **`Duration`**: key the total nanoseconds (an `i128`, wider than `long`: seconds and nanos).
//!   `equals` by value, `hashCode` `(int) (seconds ^ (seconds >>> 32)) + 51 * nanos`, `compareTo`
//!   `Long.compare(seconds)` then `nanos - other.nanos` (not a sign), `toString` ISO-8601. Every
//!   zero duration is `Duration.ZERO` (`create(0, 0)` answers the constant), so key `0` is
//!   canonical; any other may be one object or two (`d.plus(ZERO)` is `d`): never canonical.
//!   A value past `long` seconds throws Java's `ArithmeticException` ("long overflow" from the
//!   factories, `plus`, `minus`, `toNanos`, `toMillis`; "Exceeds capacity of Duration" from the
//!   BigDecimal paths `multipliedBy`, `dividedBy`, `negated`, `abs`).
//! * **`URI`**: key its text. `toString` is that text. `equals` compares the parsed components
//!   (scheme and host case-insensitively, escapes by value): the same text is equal, two texts
//!   are counted. `hashCode` and `compareTo` are the components': counted. Never canonical.
//!   The accessors read [`uri_parts`], RFC 2396 as `java.net.URI` parses it; text it refuses
//!   or this model does not parse (IPv6, a bad escape) is counted at `URI.create`, and a
//!   decoded getter of a component with an escape is counted.

use crate::runtime::host_ref::{HostKind, HostRef, Methods, Nested, instance_of_class};
use crate::runtime::{MethodRef, NativeError, StdHost, Value};

/// Whether `value` is a jar's `new UUID(..)` object (constructed or not).
pub(crate) fn is_uuid_object(value: &Value) -> bool {
    crate::runtime::jar_object::object_key(value).is_some()
        && crate::runtime::jar_object::object_class(value) == Some("java/util/UUID")
}

/// The halves of a UUID handle, `None` for any other value.
pub(crate) fn uuid_bits_of(value: &Value) -> Option<(i64, i64)> {
    uuid_text(value).and_then(uuid_bits)
}

/// `UUID.toString`: the 32 hex digits of `msb` then `lsb`, dashed 8-4-4-4-12.
pub(crate) fn uuid_text_of(msb: i64, lsb: i64) -> String {
    let hex = format!("{:016x}{:016x}", msb as u64, lsb as u64);
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

/// `UUID.hashCode`: `(int) (hilo >> 32) ^ (int) hilo`, `hilo = msb ^ lsb`.
pub(crate) fn uuid_hash_of(msb: i64, lsb: i64) -> i32 {
    let hilo = msb ^ lsb;
    ((hilo >> 32) as i32) ^ (hilo as i32)
}

/// `UUID.compareTo`: `msb`, then `lsb`, each a **signed** `Long.compare`.
pub(crate) fn uuid_order(own: (i64, i64), theirs: (i64, i64)) -> i32 {
    match own.0.cmp(&theirs.0).then(own.1.cmp(&theirs.1)) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}

/// `UUID.version`: `(msb >> 12) & 0x0f`.
fn uuid_version(msb: i64) -> i32 {
    ((msb >> 12) & 0x0f) as i32
}

/// `UUID.variant` (Java 25): `(lsb >>> (64 - (lsb >>> 62))) & (lsb >> 63)`, a shift count of
/// 64 being Java's (and `wrapping_shr`'s) shift by 0.
fn uuid_variant(lsb: i64) -> i32 {
    let unsigned = lsb as u64;
    let shift = 64 - (unsigned >> 62) as u32;
    ((unsigned.wrapping_shr(shift) as i64) & (lsb >> 63)) as i32
}

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
    Ok(uuid_bits(handle.key()).map(|(msb, lsb)| uuid_hash_of(msb, lsb)))
}

/// A UUID handle's `equals`: another handle by its text, a jar's `new UUID` by the halves the
/// host kept (counted when it kept none), anything else `false`.
fn uuid_equals(
    handle: &HostRef,
    other: &Value,
    nested: &mut dyn Nested,
) -> Result<Option<bool>, NativeError> {
    Ok(match other {
        Value::Host(other) if other.kind() == HostKind::Uuid => Some(other.key() == handle.key()),
        other if is_uuid_object(other) => nested
            .uuid_bits(other)
            .map(|theirs| uuid_bits(handle.key()) == Some(theirs)),
        _ => Some(false),
    })
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
    nested: &mut dyn Nested,
) -> Result<Option<i32>, NativeError> {
    let theirs = uuid_bits_of(other).or_else(|| nested.uuid_bits(other));
    let (Some(own), Some(theirs)) = (uuid_bits(handle.key()), theirs) else {
        return Ok(None);
    };
    Ok(Some(uuid_order(own, theirs)))
}

pub(crate) const UUID_METHODS: Methods = Methods {
    to_string: |handle, _| Ok(Some(handle.key().to_owned())),
    hash_code: uuid_hash,
    equals: uuid_equals,
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
    to_string: |handle, _| Ok(handle.key().parse::<i128>().ok().map(crate::runtime::duration_text)),
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

/// A URI text's components as `java.net.URI`'s RFC 2396 parser splits them (raw: escapes
/// kept). `None` for text this model does not parse exactly: anything `URI` would refuse
/// (an illegal character, a bad escape), an IPv6 host, an empty scheme-specific part.
#[derive(Debug, PartialEq)]
pub(crate) struct UriParts<'a> {
    pub(crate) scheme: Option<&'a str>,
    /// `scheme:` followed by no `/`: no authority, path or query.
    pub(crate) opaque: bool,
    pub(crate) authority: Option<&'a str>,
    /// Set only for a server-based authority (`[userinfo@]host[:port]`); a registry-based
    /// one (`a_b`) has none.
    pub(crate) user_info: Option<&'a str>,
    pub(crate) host: Option<&'a str>,
    pub(crate) port: i32,
    pub(crate) path: Option<&'a str>,
    pub(crate) query: Option<&'a str>,
    pub(crate) fragment: Option<&'a str>,
}

/// Whether `byte` is legal anywhere in a URI (RFC 2396 `uric`, `#`, `%`); a non-ASCII
/// character is `URI`'s "other" category, legal too.
fn uri_byte(byte: u8) -> bool {
    !byte.is_ascii() || byte.is_ascii_alphanumeric() || b"-_.!~*'();/?:@&=+$,#%".contains(&byte)
}

pub(crate) fn uri_parts(text: &str) -> Option<UriParts<'_>> {
    let bytes = text.as_bytes();
    if !bytes.iter().all(|byte| uri_byte(*byte)) || text.matches('#').count() > 1 {
        return None;
    }
    // Every `%` starts an escape of two hex digits.
    for (index, _) in text.match_indices('%') {
        if !bytes
            .get(index + 1..index + 3)
            .is_some_and(|hex| hex.iter().all(u8::is_ascii_hexdigit))
        {
            return None;
        }
    }
    let (rest, fragment) = match text.split_once('#') {
        Some((rest, fragment)) => (rest, Some(fragment)),
        None => (text, None),
    };
    // A scheme: a letter, then letters, digits, `+`, `-`, `.`, before the first `:` that
    // precedes any `/`, `?`.
    let scheme = rest
        .find([':', '/', '?'])
        .filter(|&end| rest.as_bytes()[end] == b':')
        .map(|end| &rest[..end]);
    let scheme = match scheme {
        Some(scheme)
            if scheme
                .as_bytes()
                .first()
                .is_some_and(u8::is_ascii_alphabetic)
                && scheme
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"+-.".contains(&b)) =>
        {
            Some(scheme)
        }
        // `:` first (`:a`), or a scheme with another character: `URI` refuses it.
        Some(_) => return None,
        None => None,
    };
    let ssp = scheme.map_or(rest, |scheme| &rest[scheme.len() + 1..]);
    if scheme.is_some() && ssp.is_empty() {
        return None;
    }
    let mut parts = UriParts {
        scheme,
        opaque: false,
        authority: None,
        user_info: None,
        host: None,
        port: -1,
        path: None,
        query: None,
        fragment,
    };
    if scheme.is_some() && !ssp.starts_with('/') {
        parts.opaque = true;
        return Some(parts);
    }
    let (hierarchy, query) = match ssp.split_once('?') {
        Some((hierarchy, query)) => (hierarchy, Some(query)),
        None => (ssp, None),
    };
    parts.query = query;
    let path = match hierarchy.strip_prefix("//") {
        Some(after) => {
            let end = after.find('/').unwrap_or(after.len());
            let authority = &after[..end];
            if !authority.is_empty() {
                parts.authority = Some(authority);
                if let Some((user_info, host, port)) = server_authority(authority)? {
                    parts.user_info = user_info;
                    parts.host = Some(host);
                    parts.port = port;
                }
            }
            &after[end..]
        }
        None => hierarchy,
    };
    parts.path = Some(path);
    Some(parts)
}

/// `[userinfo@]host[:port]` as `URI.Parser.parseServer` reads it: `Some(None)` when the
/// authority is registry-based (not a server: `URI` then keeps no host), `None` (counted)
/// for an IPv6 literal or a port past an `int`.
#[allow(clippy::type_complexity)]
fn server_authority(authority: &str) -> Option<Option<(Option<&str>, &str, i32)>> {
    let (user_info, server) = match authority.split_once('@') {
        Some((user_info, server))
            if user_info
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_.!~*'();:&=+$,%".contains(&b)) =>
        {
            (Some(user_info), server)
        }
        Some(_) => return Some(None),
        None => (None, authority),
    };
    if server.starts_with('[') {
        return None;
    }
    let (host, port) = match server.split_once(':') {
        Some((host, port)) => (host, Some(port)),
        None => (server, None),
    };
    let port = match port {
        None | Some("") => -1,
        Some(digits) if digits.bytes().all(|b| b.is_ascii_digit()) => digits.parse().ok()?,
        Some(_) => return Some(None),
    };
    if !(is_ipv4(host) || is_hostname(host)) {
        return Some(None);
    }
    Some(Some((user_info, host, port)))
}

fn is_ipv4(host: &str) -> bool {
    let octets: Vec<&str> = host.split('.').collect();
    octets.len() == 4
        && octets.iter().all(|octet| {
            !octet.is_empty()
                && octet.len() <= 3
                && octet.bytes().all(|b| b.is_ascii_digit())
                && octet.parse::<u16>().is_ok_and(|value| value <= 255)
        })
}

/// RFC 2396 `hostname` as `URI.Parser.parseHostname` checks it: alphanumeric labels with
/// inner `-`, dot-separated (a trailing dot allowed), the rightmost of several starting
/// with a letter.
fn is_hostname(host: &str) -> bool {
    let host = host.strip_suffix('.').unwrap_or(host);
    let labels: Vec<&str> = host.split('.').collect();
    let label_ok = |label: &str| {
        let bytes = label.as_bytes();
        !bytes.is_empty()
            && bytes[0].is_ascii_alphanumeric()
            && bytes[bytes.len() - 1].is_ascii_alphanumeric()
            && bytes
                .iter()
                .all(|b| b.is_ascii_alphanumeric() || *b == b'-')
    };
    labels.iter().all(|label| label_ok(label))
        && (labels.len() == 1
            || labels
                .last()
                .and_then(|label| label.bytes().next())
                .is_some_and(|b| b.is_ascii_alphabetic()))
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

impl StdHost {
    /// `UUID`'s members a kind's row does not answer: a jar's `new UUID(msb, lsb)` (its
    /// constructor keeps the halves; its `toString`/`hashCode`/`equals`/`compareTo` read them),
    /// and the field accessors of either kind. A jar's UUID whose halves the host did not keep
    /// is counted for every member; a handle's other members fall through.
    pub(crate) fn uuid_member(
        &mut self,
        r: &MethodRef,
        receiver: &Value,
        args: &[Value],
    ) -> Option<Result<Value, NativeError>> {
        if r.owner != "java/util/UUID" {
            return None;
        }
        let object = is_uuid_object(receiver);
        if r.name == "<init>" {
            let key = crate::runtime::jar_object::object_key(receiver)?.to_owned();
            return Some(match (r.descriptor.as_str(), args) {
                ("(JJ)V", [Value::Long(msb), Value::Long(lsb)]) => {
                    self.constructed_uuids.insert(key, (*msb, *lsb));
                    Ok(Value::Void)
                }
                _ => self.unknown_host_call(r),
            });
        }
        let halves = |host: &Self, value: &Value| {
            uuid_bits_of(value).or_else(|| crate::runtime::Host::constructed_uuid(host, value))
        };
        let Some((msb, lsb)) = halves(self, receiver) else {
            return object.then(|| self.unknown_host_call(r));
        };
        // The other operand's halves; `None` (counted) for a jar's UUID the host did not see
        // constructed.
        let other = |host: &Self| {
            let other = args.first().unwrap_or(&Value::Ref(None));
            match halves(host, other) {
                Some(bits) => Some(Some(bits)),
                None if is_uuid_object(other) => None,
                None => Some(None),
            }
        };
        Some(match (r.name.as_str(), r.descriptor.as_str()) {
            ("getMostSignificantBits", "()J") => Ok(Value::Long(msb)),
            ("getLeastSignificantBits", "()J") => Ok(Value::Long(lsb)),
            ("version", "()I") => Ok(Value::Int(uuid_version(msb))),
            ("variant", "()I") => Ok(Value::Int(uuid_variant(lsb))),
            // A handle's are its row's, dispatched before this model.
            ("toString", "()Ljava/lang/String;") if object => {
                Ok(Value::Ref(Some(uuid_text_of(msb, lsb))))
            }
            ("hashCode", "()I") if object => Ok(Value::Int(uuid_hash_of(msb, lsb))),
            ("equals", "(Ljava/lang/Object;)Z") if object => match other(self) {
                Some(theirs) => Ok(Value::Int(i32::from(theirs == Some((msb, lsb))))),
                None => self.unknown_host_call(r),
            },
            ("compareTo", "(Ljava/util/UUID;)I" | "(Ljava/lang/Object;)I") if object => {
                match other(self) {
                    Some(Some(theirs)) => Ok(Value::Int(uuid_order((msb, lsb), theirs))),
                    // `null` throws, another class casts: the runtime's to model.
                    _ => self.unknown_host_call(r),
                }
            }
            _ if object => self.unknown_host_call(r),
            _ => return None,
        })
    }
}

#[cfg(test)]
#[path = "host_jdk_value_tests.rs"]
mod tests;

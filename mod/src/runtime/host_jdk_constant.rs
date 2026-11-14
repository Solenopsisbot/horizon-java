//! JDK constants and small value objects as host handles (#156 P1b): `TimeUnit`,
//! `CodingErrorAction`, `Character.UnicodeBlock`, the `Locale` constants, a compiled `Pattern`
//! and NeoForge's `ArtifactVersion`. They were the text `time-unit/<NAME>`,
//! `coding-error-action/<NAME>`, `unicode-block/<NAME>`, `locale/<US|ROOT|ENGLISH>`,
//! `regex-pattern/<text>` and `artifact-version/<text>`, which a String could spell
//! (`String.valueOf("regex-pattern/x")` printed `x`). `charset/` is not here: open
//! contributor work (Gabbotampo's #4 and #6) is on it.
//!
//! * **`TimeUnit`** (an enum without constant bodies, so `final`), **`CodingErrorAction`** and
//!   **`UnicodeBlock`** (`final`, one instance per constant): key the constant's name, one key
//!   one object (canonical). `toString` is the name, `equals` identity, `hashCode` an identity
//!   hash (counted); `TimeUnit.compareTo` is the ordinals' difference (`Enum.compareTo`).
//! * **`Locale`** (`final`): key `<language>_<country>` (`Locale.US` is `en_US`, `ROOT` `_`).
//!   `equals` compares language and country, `hashCode` is `BaseLocale`'s
//!   (`jdk_locale_thread::locale_hash`), `toString` `en_US`. A `Locale` may be one object or
//!   two for one key (`new Locale("en", "US")` is not `Locale.US`): never canonical.
//! * **`Pattern`** (`final`): key `[serial, text]`, a serial per `compile`: one key one object.
//!   `toString`/`pattern()` is the text, `equals` identity, `hashCode` identity (counted).
//! * **`ArtifactVersion`** (NeoForge's `DefaultArtifactVersion`, Maven's `ComparableVersion`
//!   behind it): key its text, class unknown (`ArtifactVersion`, `Comparable` declared).
//!   `toString` is the text; `equals` is `compareTo == 0`: `true` for one text, counted for two
//!   (`1.0` equals `1.0.0`); `hashCode` counted. Never canonical.

use std::sync::atomic::{AtomicU64, Ordering};

use crate::runtime::host_ref::{HostKind, HostRef, Methods, Nested, instance_of_class};
use crate::runtime::{NativeError, Value};

/// `TimeUnit`'s constants in declaration order (their ordinals).
pub(crate) const TIME_UNITS: [&str; 7] = [
    "NANOSECONDS",
    "MICROSECONDS",
    "MILLISECONDS",
    "SECONDS",
    "MINUTES",
    "HOURS",
    "DAYS",
];

fn key_of(value: &Value, kind: HostKind) -> Option<&str> {
    match value {
        Value::Host(handle) if handle.kind() == kind => Some(handle.key()),
        _ => None,
    }
}

/// The `TimeUnit` constant `name`.
pub(crate) fn time_unit(name: &str) -> Value {
    Value::Host(HostRef::new(HostKind::TimeUnit, name))
}

pub(crate) fn time_unit_name(value: &Value) -> Option<&str> {
    key_of(value, HostKind::TimeUnit)
}

/// The `CodingErrorAction` constant `name`.
pub(crate) fn coding_error_action(name: &str) -> Value {
    Value::Host(HostRef::new(HostKind::CodingErrorAction, name))
}

pub(crate) fn coding_error_action_name(value: &Value) -> Option<&str> {
    key_of(value, HostKind::CodingErrorAction)
}

/// The `Character.UnicodeBlock` constant `name`.
pub(crate) fn unicode_block(name: &str) -> Value {
    Value::Host(HostRef::new(HostKind::UnicodeBlock, name))
}

pub(crate) fn unicode_block_name(value: &Value) -> Option<&str> {
    key_of(value, HostKind::UnicodeBlock)
}

/// The `Locale` of `language` and `country`.
pub(crate) fn locale(language: &str, country: &str) -> Value {
    Value::Host(HostRef::new(
        HostKind::Locale,
        format!("{language}_{country}"),
    ))
}

/// The language and country of a `Locale` handle. The key is the runtime's: a language and a
/// country never hold `_`.
pub(crate) fn locale_parts(value: &Value) -> Option<(&str, &str)> {
    key_of(value, HostKind::Locale)?.split_once('_')
}

static NEXT_PATTERN: AtomicU64 = AtomicU64::new(0);

/// `Pattern.compile(text)`: a new object.
pub(crate) fn pattern(text: &str) -> Value {
    Value::Host(HostRef::of_values(
        HostKind::RegexPattern,
        &[
            Value::Long(NEXT_PATTERN.fetch_add(1, Ordering::Relaxed) as i64),
            Value::Ref(Some(text.to_owned())),
        ],
    ))
}

/// The text of a `Pattern` handle.
pub(crate) fn pattern_text(value: &Value) -> Option<String> {
    let Value::Host(handle) = value else {
        return None;
    };
    handle_pattern(handle)
}

fn handle_pattern(handle: &HostRef) -> Option<String> {
    if handle.kind() != HostKind::RegexPattern {
        return None;
    }
    match <[Value; 2]>::try_from(handle.values()?) {
        Ok([_, Value::Ref(Some(text))]) => Some(text),
        _ => None,
    }
}

/// NeoForge's `ArtifactVersion` of `text`.
pub(crate) fn artifact_version(text: &str) -> Value {
    Value::Host(HostRef::new(HostKind::ArtifactVersion, text))
}

pub(crate) fn artifact_version_text(value: &Value) -> Option<&str> {
    key_of(value, HostKind::ArtifactVersion)
}

fn identity_equals(
    handle: &HostRef,
    other: &Value,
    _: &mut dyn Nested,
) -> Result<Option<bool>, NativeError> {
    Ok(match other {
        Value::Host(other) => handle.same_object(other),
        _ => Some(false),
    })
}

fn name_text(handle: &HostRef, _: &mut dyn Nested) -> Result<Option<String>, NativeError> {
    Ok(Some(handle.key().to_owned()))
}

fn unknown_hash(_: &HostRef, _: &mut dyn Nested) -> Result<Option<i32>, NativeError> {
    Ok(None)
}

const CONSTANT_METHODS: Methods = Methods {
    to_string: name_text,
    hash_code: unknown_hash,
    equals: identity_equals,
    instance_of: instance_of_class,
    crosses_jars: |_, _| true,
    compare_to: None,
};

/// `Enum.compareTo`: the ordinals' difference.
fn time_unit_compare(
    handle: &HostRef,
    other: &Value,
    _: &mut dyn Nested,
) -> Result<Option<i32>, NativeError> {
    let ordinal = |name: &str| TIME_UNITS.iter().position(|unit| *unit == name);
    Ok(
        match (
            ordinal(handle.key()),
            time_unit_name(other).and_then(ordinal),
        ) {
            (Some(own), Some(theirs)) => Some(own as i32 - theirs as i32),
            _ => None,
        },
    )
}

pub(crate) const TIME_UNIT_METHODS: Methods = Methods {
    compare_to: Some(time_unit_compare),
    ..CONSTANT_METHODS
};
pub(crate) const CODING_ERROR_ACTION_METHODS: Methods = CONSTANT_METHODS;
pub(crate) const UNICODE_BLOCK_METHODS: Methods = CONSTANT_METHODS;

pub(crate) const LOCALE_METHODS: Methods =
    Methods {
        to_string: |handle, _| {
            Ok(handle.key().split_once('_').map(|(language, country)| {
                crate::runtime::jdk_locale_thread::locale_text(language, country)
            }))
        },
        hash_code: |handle, _| {
            Ok(handle.key().split_once('_').map(|(language, country)| {
                crate::runtime::jdk_locale_thread::locale_hash(language, country)
            }))
        },
        equals: |handle, other, _| {
            Ok(match other {
                Value::Host(other) if other.kind() == HostKind::Locale => {
                    Some(other.key() == handle.key())
                }
                // A jar's `new Locale(..)` is the host's to compare.
                other if crate::runtime::jar_object::object_key(other).is_some() => None,
                _ => Some(false),
            })
        },
        instance_of: instance_of_class,
        crosses_jars: |_, _| true,
        compare_to: None,
    };

pub(crate) const PATTERN_METHODS: Methods = Methods {
    to_string: |handle, _| Ok(handle_pattern(handle)),
    ..CONSTANT_METHODS
};

pub(crate) fn artifact_version_types(_: &str) -> &'static [&'static str] {
    &[
        "org/apache/maven/artifact/versioning/ArtifactVersion",
        "java/lang/Comparable",
    ]
}

pub(crate) const ARTIFACT_VERSION_METHODS: Methods = Methods {
    to_string: name_text,
    equals: |handle, other, _| {
        Ok(match other {
            Value::Host(other) if other.kind() == HostKind::ArtifactVersion => {
                (other.key() == handle.key()).then_some(true)
            }
            // A jar's own `ArtifactVersion` compares through its own body.
            other if crate::runtime::jar_object::object_key(other).is_some() => None,
            _ => Some(false),
        })
    },
    crosses_jars: |_, _| true,
    ..crate::runtime::host_events::DECLARED_TYPE_METHODS
};

#[cfg(test)]
#[path = "host_jdk_constant_tests.rs"]
mod tests;

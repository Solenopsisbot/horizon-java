//! Guava's `Splitter`, `Joiner`, `CacheBuilder` and `CacheLoader.from` as
//! host handles (#156 P9). Their behaviour stays in `guava_text.rs` and
//! `guava_model.rs`; this module is their keys and rows.
//!
//! These were `guava-splitter/<separator>`, `guava-joiner/<separator>`,
//! `guava-cache-loader/<function>` and the one `guava-cache-builder` every
//! builder shared, text a jar `String` could spell (the String
//! `"guava-cache-builder"` *was* a builder). Guava 33.6.0 (the 26.3 server's)
//! on Java 25 makes each a fresh object with `Object`'s `equals` (two
//! `Splitter.on(',')` are not equal) and an identity `hashCode`:
//!
//! * splitter, joiner: key [`HostRef::of_values`] of the separator: two with
//!   one separator have equal keys and are two objects, so not canonical and
//!   identity on them is counted.
//! * cache loader: key the function `CacheLoader.from` was given; not
//!   canonical either.
//! * cache builder: key its allocation marker (`guava-cache-builder#<n>`),
//!   one per `newBuilder()`; its setters answer the same builder: canonical.
//!
//! Their classes (`Splitter` is final; `Joiner`'s `useForNull` makes a
//! subclass; the loader is a private `FunctionToCacheLoader`) are
//! [`ClassRule::Unknown`] here, so `getClass()` is counted, never answered
//! without the rest of the `Class` surface.
//!
//! [`ClassRule::Unknown`]: crate::runtime::host_ref::ClassRule

use crate::runtime::Value;
use crate::runtime::host_ref::{HostKind, HostRef};

pub(crate) fn splitter_types(_: &str) -> &'static [&'static str] {
    &["com/google/common/base/Splitter"]
}
pub(crate) fn joiner_types(_: &str) -> &'static [&'static str] {
    &["com/google/common/base/Joiner"]
}
pub(crate) fn cache_builder_types(_: &str) -> &'static [&'static str] {
    &["com/google/common/cache/CacheBuilder"]
}
pub(crate) fn cache_loader_types(_: &str) -> &'static [&'static str] {
    &["com/google/common/cache/CacheLoader"]
}

/// A handle of `kind` holding one value.
fn holding(kind: HostKind, value: &Value) -> Value {
    Value::Host(HostRef::of_values(kind, std::slice::from_ref(value)))
}

/// The value a one-value handle of `kind` holds.
fn held(value: &Value, kind: HostKind) -> Option<Value> {
    match value {
        Value::Host(handle) if handle.kind() == kind => match handle.values()?.as_slice() {
            [held] => Some(held.clone()),
            _ => None,
        },
        _ => None,
    }
}

/// `Splitter.on(separator)`.
pub(crate) fn splitter(separator: &str) -> Value {
    holding(
        HostKind::GuavaSplitter,
        &Value::Ref(Some(separator.to_owned())),
    )
}

/// `Joiner.on(separator)`.
pub(crate) fn joiner(separator: &str) -> Value {
    holding(
        HostKind::GuavaJoiner,
        &Value::Ref(Some(separator.to_owned())),
    )
}

/// The separator of a splitter (or, with `joiner`, a joiner) handle.
pub(crate) fn separator(value: &Value, joiner: bool) -> Option<String> {
    let kind = if joiner {
        HostKind::GuavaJoiner
    } else {
        HostKind::GuavaSplitter
    };
    match held(value, kind)? {
        Value::Ref(Some(separator)) => Some(separator),
        _ => None,
    }
}

/// `CacheLoader.from(function)`.
pub(crate) fn cache_loader(function: &Value) -> Value {
    holding(HostKind::GuavaCacheLoader, function)
}

/// The function a `CacheLoader.from` handle wraps.
pub(crate) fn cache_loader_function(value: &Value) -> Option<Value> {
    held(value, HostKind::GuavaCacheLoader)
}

/// A `CacheBuilder` from its allocation marker.
pub(crate) fn cache_builder(marker: Value) -> Value {
    crate::runtime::host_executors::allocated(HostKind::GuavaCacheBuilder, marker)
}

/// Whether `value` is a `CacheBuilder` handle.
pub(crate) fn is_cache_builder(value: &Value) -> bool {
    matches!(value, Value::Host(handle) if handle.kind() == HostKind::GuavaCacheBuilder)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A separator (jar text) with every character a key layer uses reads back
    /// as itself, and two different separators never make one key ("keys
    /// never parse jar text", #156); the same goes for a service loader's
    /// service name and a provider's class name.
    #[test]
    fn jar_text_in_keys_round_trips_and_never_aliases() {
        let alphabet = [
            "a", ",", "\\", ":", "|", "#", "?", ";", "/", "hgjoin:", "r", "",
        ];
        let texts: Vec<String> = alphabet
            .iter()
            .flat_map(|a| alphabet.iter().map(move |b| format!("{a}{b}")))
            .collect();
        let mut seen = std::collections::BTreeMap::new();
        for text in &texts {
            assert_eq!(
                separator(&splitter(text), false).as_deref(),
                Some(text.as_str())
            );
            assert_eq!(
                separator(&joiner(text), true).as_deref(),
                Some(text.as_str())
            );
            let loader = crate::runtime::host_services::loader(text, 7);
            assert_eq!(
                crate::runtime::host_services::loader_parts(&loader),
                Some((text.clone(), 7))
            );
            let provider = crate::runtime::host_services::provider(text, text, 7);
            assert_eq!(
                crate::runtime::host_services::provider_parts(&provider),
                Some((text.clone(), text.clone()))
            );
            let Value::Host(handle) = splitter(text) else {
                panic!("{text:?}")
            };
            if let Some(other) = seen.insert(handle.key().to_owned(), text.clone()) {
                assert_eq!(&other, text);
            }
        }
    }
}

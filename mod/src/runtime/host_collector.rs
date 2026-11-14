//! `Collectors`' built-in collectors as `Collector` handles (#156 P14f).
//!
//! `Collectors.toList()`, `toSet()`, `joining(..)`, `toMap(..)` (and Guava's `toImmutableList`/
//! `toImmutableSet`/`toImmutableMap`) were the text `collector/list`, `collector/joining/<a>␟<b>`,
//! `collector/to-map/<functions>`. A `String` could spell one, and two calls were one object.
//! Java 25 makes a new `Collectors.CollectorImpl` (a record) per call, so each is a
//! [`HostKind::Collector`] handle, like `Collector.of`'s (`stream_pipeline.rs`), with
//! that kind's row (`COLLECTOR_IMPL`). Its key is the escaping encoder's
//! (`HostRef::of_values`): `builtin`, a serial (one per call), the collector's name, and its
//! arguments (the joining texts, the `toMap` functions). Jar text reaches the key only through
//! that encoder. `Collector.of`'s keys are bare numbers, so the two never meet.

use std::sync::atomic::{AtomicU64, Ordering};

use crate::runtime::Value;
use crate::runtime::host_ref::{HostKind, HostRef};

/// The first value of a built-in collector's key.
const BUILTIN: &str = "builtin";

/// One serial per built-in collector made: a new object per call.
static NEXT: AtomicU64 = AtomicU64::new(0);

/// What a built-in collector collects into.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Builtin {
    /// `toList()`: an `ArrayList`.
    List,
    /// `toSet()`: a `HashSet`.
    Set,
    /// `toUnmodifiableSet()`: `Set.of` of the collected elements.
    UnmodifiableSet,
    /// `toUnmodifiableList()`: `List.of`'s, rejecting `null`.
    UnmodifiableList,
    /// Guava's `toImmutableList()`.
    ImmutableList,
    /// Guava's `toImmutableSet()`: encounter order.
    OrderedSet,
    /// `joining(delimiter, prefix, suffix)` (`joining()` and `joining(d)` with the empty ones).
    Joining {
        delimiter: String,
        prefix: String,
        suffix: String,
    },
    /// `toMap(key, value[, merge[, supplier]])` and Guava's immutable-map collectors: the
    /// functions in that order (`jdk_model.rs`'s `ToMap`).
    ToMap { guava: bool, functions: Vec<Value> },
}

impl Builtin {
    fn name(&self) -> &'static str {
        match self {
            Builtin::List => "list",
            Builtin::Set => "set",
            Builtin::UnmodifiableSet => "unmodifiable-set",
            Builtin::UnmodifiableList => "unmodifiable-list",
            Builtin::ImmutableList => "immutable-list",
            Builtin::OrderedSet => "ordered-set",
            Builtin::Joining { .. } => "joining",
            Builtin::ToMap { guava: false, .. } => "to-map",
            Builtin::ToMap { guava: true, .. } => "guava-to-map",
        }
    }
}

fn text(value: &str) -> Value {
    Value::Ref(Some(value.to_owned()))
}

/// A new collector handle for `builtin`.
pub(crate) fn collector(builtin: Builtin) -> Value {
    let serial = NEXT.fetch_add(1, Ordering::Relaxed);
    let mut values = vec![
        text(BUILTIN),
        Value::Long(serial as i64),
        text(builtin.name()),
    ];
    match builtin {
        Builtin::Joining {
            delimiter,
            prefix,
            suffix,
        } => values.extend([text(&delimiter), text(&prefix), text(&suffix)]),
        Builtin::ToMap { functions, .. } => values.extend(functions),
        _ => {}
    }
    Value::Host(HostRef::of_values(HostKind::Collector, &values))
}

/// The built-in collector a handle is; `None` for any other value (a `Collector.of` handle, a
/// `String` spelling an old marker, a jar's own collector).
pub(crate) fn builtin(value: &Value) -> Option<Builtin> {
    let Value::Host(handle) = value else {
        return None;
    };
    if handle.kind() != HostKind::Collector {
        return None;
    }
    let values = handle.values()?;
    let [
        Value::Ref(Some(tag)),
        Value::Long(_),
        Value::Ref(Some(name)),
        rest @ ..,
    ] = values.as_slice()
    else {
        return None;
    };
    if tag != BUILTIN {
        return None;
    }
    let texts = |values: &[Value]| -> Option<Vec<String>> {
        values
            .iter()
            .map(|value| match value {
                Value::Ref(Some(text)) => Some(text.clone()),
                _ => None,
            })
            .collect()
    };
    Some(match (name.as_str(), rest) {
        ("list", []) => Builtin::List,
        ("set", []) => Builtin::Set,
        ("unmodifiable-set", []) => Builtin::UnmodifiableSet,
        ("unmodifiable-list", []) => Builtin::UnmodifiableList,
        ("immutable-list", []) => Builtin::ImmutableList,
        ("ordered-set", []) => Builtin::OrderedSet,
        ("joining", parts) => {
            let [delimiter, prefix, suffix] = <[String; 3]>::try_from(texts(parts)?).ok()?;
            Builtin::Joining {
                delimiter,
                prefix,
                suffix,
            }
        }
        ("to-map" | "guava-to-map", functions) => Builtin::ToMap {
            guava: name == "guava-to-map",
            functions: functions.to_vec(),
        },
        _ => return None,
    })
}

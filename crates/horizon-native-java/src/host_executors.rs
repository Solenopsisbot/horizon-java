//! Executors, their futures, `Executors.callable` and the default thread
//! factory as host handles (#156 P9). The executor model itself (tasks run
//! inline, futures complete, shutdown flags) is `executors.rs`, unchanged;
//! this module is the objects' identity and their rows in the kind table.
//!
//! These were text a jar `String` could spell: `executor#3`/`future#4`
//! allocation markers, `executor-callable/<task>`,
//! `executor-callable-result/<task>|<result>` (a task's marker joined with
//! its result's by `|`) and the one `executor-thread-factory` every factory
//! shared.
//!
//! Every one is a fresh object in Java 25 (`Executors.newX()`, `submit`,
//! `Executors.callable`, `defaultThreadFactory`), with `Object`'s `equals`
//! and an identity `hashCode`, and its class an implementation the host does
//! not name (`ThreadPoolExecutor`, `Executors$AutoShutdownDelegatedExecutorService`,
//! `FutureTask`, `Executors$RunnableAdapter`): [`ClassRule::Unknown`], so
//! `getClass()`, `toString` and `hashCode` are counted.
//!
//! * executor, future, thread factory: key the allocation marker
//!   (`executor#3`, `scheduled-executor#3`, `future#4`, `scheduled-future#4`,
//!   `thread-factory#5`), one per allocation: canonical. The executor
//!   model's state stays in the runtime's field table under that same text.
//! * callable: key [`HostRef::of_values`] of the task (and the result), the
//!   values `Executors.callable` was given. Two calls with the same task are
//!   two objects with equal keys: not canonical, so identity on them is
//!   counted unless the same handle flows.
//!
//! [`ClassRule::Unknown`]: crate::host_ref::ClassRule

use crate::host_ref::{HostKind, HostRef, Methods, Nested, instance_of_class};
use crate::{NativeError, Value};

/// The executor kinds' declared types, by allocation prefix.
const EXECUTOR_TYPES: &[&str] = &[
    "java/util/concurrent/ExecutorService",
    "java/util/concurrent/Executor",
    "java/lang/AutoCloseable",
];
const SCHEDULED_EXECUTOR_TYPES: &[&str] = &[
    "java/util/concurrent/ScheduledExecutorService",
    "java/util/concurrent/ExecutorService",
    "java/util/concurrent/Executor",
    "java/lang/AutoCloseable",
];
const FUTURE_TYPES: &[&str] = &["java/util/concurrent/Future"];
const SCHEDULED_FUTURE_TYPES: &[&str] = &[
    "java/util/concurrent/ScheduledFuture",
    "java/util/concurrent/Future",
    "java/util/concurrent/Delayed",
    "java/lang/Comparable",
];
const CALLABLE_TYPES: &[&str] = &["java/util/concurrent/Callable"];
const THREAD_FACTORY_TYPES: &[&str] = &["java/util/concurrent/ThreadFactory"];

/// An allocation marker (`<prefix>#<n>`) as a handle of `kind`.
pub(crate) fn allocated(kind: HostKind, marker: Value) -> Value {
    match marker {
        Value::Ref(Some(marker)) => Value::Host(HostRef::new(kind, marker)),
        other => other,
    }
}

/// The allocation text of a handle of `kind`, the key its state is kept
/// under in the runtime's field table.
pub(crate) fn allocation(value: &Value, kind: HostKind) -> Option<&str> {
    match value {
        Value::Host(handle) if handle.kind() == kind => Some(handle.key()),
        _ => None,
    }
}

pub(crate) fn executor_types(key: &str) -> &'static [&'static str] {
    if key.starts_with("scheduled-") {
        SCHEDULED_EXECUTOR_TYPES
    } else {
        EXECUTOR_TYPES
    }
}

pub(crate) fn future_types(key: &str) -> &'static [&'static str] {
    if key.starts_with("scheduled-") {
        SCHEDULED_FUTURE_TYPES
    } else {
        FUTURE_TYPES
    }
}

pub(crate) fn callable_types(_: &str) -> &'static [&'static str] {
    CALLABLE_TYPES
}

pub(crate) fn thread_factory_types(_: &str) -> &'static [&'static str] {
    THREAD_FACTORY_TYPES
}

/// `Executors.callable(task)` / `callable(task, result)`.
pub(crate) fn callable(task: &Value, result: Option<&Value>) -> Value {
    let mut values = vec![task.clone()];
    values.extend(result.cloned());
    Value::Host(HostRef::of_values(HostKind::ExecutorCallable, &values))
}

/// The task and result an `Executors.callable` handle wraps (`None` result
/// for the one-argument form, whose `call()` answers `null`).
pub(crate) fn callable_parts(value: &Value) -> Option<(Value, Option<Value>)> {
    match value {
        Value::Host(handle) if handle.kind() == HostKind::ExecutorCallable => {
            match handle.values()?.as_slice() {
                [task] => Some((task.clone(), None)),
                [task, result] => Some((task.clone(), Some(result.clone()))),
                _ => None,
            }
        }
        _ => None,
    }
}

/// `Object.equals`: identity, as far as it is known (`None`: counted).
pub(crate) fn identity_equals(
    handle: &HostRef,
    other: &Value,
    _: &mut dyn Nested,
) -> Result<Option<bool>, NativeError> {
    Ok(match other {
        Value::Host(other) => handle.same_object(other),
        _ => Some(false),
    })
}

/// An object of an unnamed implementation class with `Object`'s methods:
/// `toString` (class name and identity hash) and `hashCode` are counted.
pub(crate) const OBJECT_METHODS: Methods = Methods {
    to_string: |_, _| Ok(None),
    hash_code: |_, _| Ok(None),
    equals: identity_equals,
    instance_of: instance_of_class,
    crosses_jars: |_, _| false,
    compare_to: None,
};

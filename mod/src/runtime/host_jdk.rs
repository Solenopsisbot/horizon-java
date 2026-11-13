//! JDK runtime objects as host handles (#156 P2): the main thread, loggers,
//! the stack walker and its frames, stack trace elements and modules. Their
//! text forms (`thread/main`, `logger/<name>`, `stack-walker`,
//! `stack-frame/…`, `stack-trace-element/…`, `module/…`) were Strings a jar
//! could spell. Expected answers: `tests/fixture_host_values_p2.rs` (Java 25).
//!
//! None of these classes overrides `Object.hashCode`, and only `Module` (for
//! a named module) and `StackTraceElement` override `toString`, so most
//! `hashCode`/`toString` answers are identity hashes no run can reproduce:
//! counted. `equals` is identity ([`identity_equals`]) except for
//! `StackTraceElement`'s.
//!
//! | kind | key | canonical | class |
//! |---|---|---|---|
//! | `Thread` | `main` | yes: callbacks run on one thread | `java/lang/Thread` |
//! | `Logger` | `<api>/<name>` | per API (below) | the API's implementation |
//! | `StackWalker` | `""` (`getInstance()`), `options` | `getInstance()`'s `DEFAULT_WALKER` only | `java/lang/StackWalker` |
//! | `StackFrame` | `<method>@<class>` | no: each walk makes new frames | `java/lang/StackFrameInfo` |
//! | `StackTraceElement` | `<method>@<class>` | no | `java/lang/StackTraceElement` |
//! | `Module` | `unnamed`, `java.base` | yes | `java/lang/Module` |
//!
//! `Thread` and `StackTraceElement` objects are also made by `new` (a jar's
//! own thread is a `java/lang/Thread#n` object), so their kinds read their
//! class from the key (`ClassRule::FromKey`) rather than claiming the class
//! for handles alone: a fixed class would make every cast of a
//! `java/lang/Thread#n` object to `Thread` throw.
//!
//! Loggers keep the API they came from, which decides their class and
//! whether one name is one object: log4j's `LogManager.getLogger` (per
//! message factory, so `getFormatterLogger` is a separate API), slf4j's
//! `LoggerFactory.getLogger` and `java.util.logging.Logger.getLogger` cache
//! one logger per name; `LogUtils.getLogger()` names its caller (unknown
//! here), `Logger.getAnonymousLogger()` and `System.getLogger` hand out new
//! objects.

use crate::runtime::NativeError;
use crate::runtime::Value;
use crate::runtime::host_ref::{HostKind, HostRef, Methods, Nested};

/// `self == other` as `Object.equals` answers it: `None` when only the JVM
/// knows (equal keys of a kind that is not canonical, two constructions).
pub(crate) fn identity_equals(handle: &HostRef, other: &Value) -> Option<bool> {
    match other {
        Value::Host(other) => handle.same_object(other),
        _ => Some(false),
    }
}

fn unknown_text(_: &HostRef, _: &mut dyn Nested) -> Result<Option<String>, NativeError> {
    Ok(None)
}

fn identity_hash(_: &HostRef, _: &mut dyn Nested) -> Result<Option<i32>, NativeError> {
    Ok(None)
}

/// The supertypes of each kind's class (`javap` of JDK 25, log4j-core and
/// log4j-slf4j2-impl 2.26.0).
fn instance_of(
    handle: &HostRef,
    wanted: &str,
    _: &dyn crate::runtime::host_ref::ClassHierarchy,
) -> Option<bool> {
    if wanted == "java/lang/Object" || wanted == handle.class() {
        return Some(true);
    }
    let types: &[&str] = match handle.kind() {
        HostKind::Thread => &["java/lang/Runnable"],
        HostKind::StackFrame => &[
            "java/lang/StackWalker$StackFrame",
            "java/lang/ClassFrameInfo",
        ],
        HostKind::StackTraceElement => &["java/io/Serializable"],
        HostKind::Module => &["java/lang/reflect/AnnotatedElement"],
        HostKind::Logger => logger_supertypes(handle.key()),
        _ => &[],
    };
    Some(types.contains(&wanted))
}

const fn methods(to_string: crate::runtime::host_ref::ToStringFn) -> Methods {
    Methods {
        to_string,
        hash_code: identity_hash,
        equals: |handle, other, _| Ok(identity_equals(handle, other)),
        instance_of,
        crosses_jars: |_, _| true,
    }
}

/// `Thread.currentThread()`: the one thread a native run's callbacks run on.
pub(crate) fn main_thread() -> Value {
    Value::Host(HostRef::new(HostKind::Thread, "main"))
}

pub(crate) fn is_main_thread(value: &Value) -> bool {
    matches!(value, Value::Host(handle) if handle.kind() == HostKind::Thread)
}

/// `Thread.toString` prints the thread's id and name, which are the
/// process's: Java 25 printed `Thread[#3,main,5,main]` for a plain program's
/// main thread, and a game's callbacks run on its server thread. Counted.
pub(crate) const THREAD: Methods = methods(unknown_text);

/// A logger of `api` named `name` (see the module docs for the APIs).
pub(crate) fn logger_value(api: &str, name: &str) -> Value {
    Value::Host(HostRef::new(HostKind::Logger, format!("{api}/{name}")))
}

/// The name of a logger handle, `None` for any other value.
pub(crate) fn logger_name(value: &Value) -> Option<&str> {
    match value {
        Value::Host(handle) if handle.kind() == HostKind::Logger => {
            handle.key().split_once('/').map(|(_, name)| name)
        }
        _ => None,
    }
}

fn logger_api(key: &str) -> &str {
    key.split_once('/').map_or(key, |(api, _)| api)
}

/// The class `getClass()` answers on Minecraft's logging stack (log4j-core,
/// its slf4j binding, the JDK's `java.util.logging` and its `System.Logger`
/// wrapper).
pub(crate) fn logger_class(key: &str) -> &'static str {
    match logger_api(key) {
        "log4j" | "log4j-formatter" => "org/apache/logging/log4j/core/Logger",
        "slf4j" | "mojang" => "org/apache/logging/slf4j/Log4jLogger",
        "jul" | "jul-anonymous" => "java/util/logging/Logger",
        _ => "sun/util/logging/internal/LoggingProviderImpl$JULWrapper",
    }
}

fn logger_supertypes(key: &str) -> &'static [&'static str] {
    match logger_api(key) {
        "log4j" | "log4j-formatter" => &[
            "org/apache/logging/log4j/spi/AbstractLogger",
            "org/apache/logging/log4j/spi/ExtendedLogger",
            "org/apache/logging/log4j/Logger",
            "org/apache/logging/log4j/spi/LocationAwareLogger",
            "org/apache/logging/log4j/util/Supplier",
            "java/util/function/Supplier",
            "java/io/Serializable",
        ],
        "slf4j" | "mojang" => &[
            "org/slf4j/spi/LocationAwareLogger",
            "org/slf4j/Logger",
            "java/io/Serializable",
        ],
        "jul" | "jul-anonymous" => &[],
        _ => &[
            "sun/util/logging/PlatformLogger$ConfigurableBridge$LoggerConfiguration",
            "java/lang/System$Logger",
            "sun/util/logging/PlatformLogger$Bridge",
            "sun/util/logging/PlatformLogger$ConfigurableBridge",
        ],
    }
}

/// One object per name for the caching factories.
pub(crate) fn logger_canonical(key: &str) -> bool {
    matches!(
        logger_api(key),
        "log4j" | "log4j-formatter" | "slf4j" | "jul"
    )
}

pub(crate) const LOGGER: Methods = methods(unknown_text);

/// `StackWalker.getInstance(...)`: `options` is false for the no-argument
/// overload, which answers the JDK's one `DEFAULT_WALKER`.
pub(crate) fn stack_walker(options: bool) -> Value {
    Value::Host(HostRef::new(
        HostKind::StackWalker,
        if options { "options" } else { "" },
    ))
}

pub(crate) const STACK_WALKER: Methods = methods(unknown_text);

/// One stack frame (`StackFrame`) or element (`StackTraceElement`) of the
/// jar's own frames.
pub(crate) fn frame_value(kind: HostKind, class: &str, method: &str) -> Value {
    Value::Host(HostRef::new(kind, format!("{method}@{class}")))
}

/// `(class, method)` of a frame handle of `kind`.
pub(crate) fn frame_parts(value: &Value, kind: HostKind) -> Option<(&str, &str)> {
    match value {
        Value::Host(handle) if handle.kind() == kind => {
            let (method, class) = handle.key().split_once('@')?;
            Some((class, method))
        }
        _ => None,
    }
}

/// `StackFrameInfo.toString` names the frame's file and line, which the
/// runtime does not keep: counted.
pub(crate) const STACK_FRAME: Methods = methods(unknown_text);

/// `StackTraceElement.equals` compares the class, method, file and line
/// (and loader and module): elements of two methods are unequal, two of one
/// method may be two lines, so only the same element is known equal. Its
/// `toString`/`hashCode` read the file and line: counted.
pub(crate) const STACK_TRACE_ELEMENT: Methods = Methods {
    equals: |handle, other, _| {
        Ok(match other {
            Value::Host(other) if other.kind() == HostKind::StackTraceElement => {
                handle.same_object(other)
            }
            _ => Some(false),
        })
    },
    ..methods(unknown_text)
};

/// The unnamed module every jar class is in.
pub(crate) const UNNAMED_MODULE: &str = "unnamed";
/// `java.base`.
pub(crate) const JAVA_BASE_MODULE: &str = "java.base";

pub(crate) fn module_value(name: &str) -> Value {
    Value::Host(HostRef::new(HostKind::Module, name))
}

pub(crate) fn module_name(value: &Value) -> Option<&str> {
    match value {
        Value::Host(handle) if handle.kind() == HostKind::Module => Some(handle.key()),
        _ => None,
    }
}

/// `Module.toString`: `module <name>`; an unnamed module prints its identity
/// hash (counted).
pub(crate) const MODULE: Methods = methods(|handle, _| {
    Ok((handle.key() != UNNAMED_MODULE).then(|| format!("module {}", handle.key())))
});

//! `net.minecraft.resources.Identifier` (26.3), and `ResourceLocation`, its
//! name before the 1.21 rename, answered from the class's own rules.
//!
//! An id is an `Identifier` handle keyed `<namespace>:<path>` (`host_ids.rs`).
//! The factories run
//! vanilla's checks (`javap -c` of 26.3's `Identifier`): a namespace is
//! `[a-z0-9_.-]*` and not `..`, a path `[a-z0-9/._-]*`. `bySeparator` (and
//! `parse`, which is `bySeparator(s, ':')`) splits at the first separator; no
//! namespace, or an empty one, is `minecraft` and checks only the path
//! (`withDefaultNamespace`), otherwise the namespace is checked first. The
//! `try*` forms answer `null` where those throw `IdentifierException`.
//!
//! `ResourceLocation` throws `ResourceLocationException` instead. No pre-1.21
//! jar is on hand to check it against, so its messages are 26.3's with the
//! namespace message's "identifier" read as "location", the word 26.3's path
//! message still uses.
//!
//! Members this model does not answer (`read`, `resolveAgainst`) are counted as
//! unknown host calls rather than answered with a default.

use crate::runtime::{Host, MethodRef, NativeError, Runtime, StdHost, Value, expect_ref, expect_string};

/// The two owner spellings that name the class across versions.
pub(crate) fn owns(owner: &str) -> bool {
    matches!(
        owner,
        "net/minecraft/resources/Identifier" | "net/minecraft/resources/ResourceLocation"
    )
}

fn raise(owner: &str, message: String) -> NativeError {
    let class = if owner.ends_with("ResourceLocation") {
        "net/minecraft/ResourceLocationException"
    } else {
        "net/minecraft/IdentifierException"
    };
    NativeError::UncaughtException {
        class: class.into(),
        message: Some(escape_java(&message)),
        frame: None,
    }
}

/// commons-lang3 `StringEscapeUtils.escapeJava`, which the exception's
/// constructor applies to its message (26.3 `IdentifierException`): `"` and
/// `\` escaped, `\b \t \n \f \r` by name, and every other UTF-16 unit
/// below `0x20` or above `0x7f` as `\uXXXX` (upper-case hex, a surrogate pair
/// as two).
fn escape_java(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for unit in text.encode_utf16() {
        match unit {
            0x22 => out.push_str("\\\""),
            0x5c => out.push_str("\\\\"),
            0x08 => out.push_str("\\b"),
            0x09 => out.push_str("\\t"),
            0x0a => out.push_str("\\n"),
            0x0c => out.push_str("\\f"),
            0x0d => out.push_str("\\r"),
            0x20..=0x7f => out.push(char::from(unit as u8)),
            _ => out.push_str(&format!("\\u{unit:04X}")),
        }
    }
    out
}

fn valid_namespace_char(c: char) -> bool {
    matches!(c, '_' | '-' | '.' | 'a'..='z' | '0'..='9')
}

fn valid_path_char(c: char) -> bool {
    valid_namespace_char(c) || c == '/'
}

fn is_valid_namespace(namespace: &str) -> bool {
    namespace != ".." && namespace.chars().all(valid_namespace_char)
}

fn is_valid_path(path: &str) -> bool {
    path.chars().all(valid_path_char)
}

/// `assertValidNamespace` then `assertValidPath`, with their messages.
pub(crate) fn checked(owner: &str, namespace: &str, path: &str) -> Result<Value, NativeError> {
    if !is_valid_namespace(namespace) {
        let word = if owner.ends_with("ResourceLocation") {
            "location"
        } else {
            "identifier"
        };
        return Err(raise(
            owner,
            format!("Non [a-z0-9_.-] character in namespace of {word}: {namespace}:{path}"),
        ));
    }
    if !is_valid_path(path) {
        return Err(raise(
            owner,
            format!("Non [a-z0-9/._-] character in path of location: {namespace}:{path}"),
        ));
    }
    Ok(crate::runtime::identifier_value(namespace, path))
}

/// Split at the first `separator`: no separator or an empty namespace is
/// `minecraft`.
fn split(identifier: &str, separator: char) -> (&str, &str) {
    match identifier.split_once(separator) {
        Some(("", path)) => ("minecraft", path),
        Some((namespace, path)) => (namespace, path),
        None => ("minecraft", identifier),
    }
}

/// The `namespace:path` an id value names: an `Identifier` handle, or an
/// object `new ResourceLocation(..)` built (its id kept by the host).
pub(crate) fn id_text(host: &StdHost, value: &Value) -> Option<String> {
    crate::runtime::host_ids::identifier_key(value)
        .or_else(|| {
            host.identifier_objects
                .get(expect_ref(value)?)
                .map(String::as_str)
        })
        .map(str::to_owned)
}

/// [`id_text`] split into `(namespace, path)`.
fn id_parts(host: &StdHost, value: &Value) -> Option<(String, String)> {
    let id = id_text(host, value)?;
    let (namespace, path) = id.split_once(':')?;
    Some((namespace.to_owned(), path.to_owned()))
}

/// Java's `String.compareTo`: UTF-16 code units, then length.
pub(crate) fn java_compare(left: &str, right: &str) -> i32 {
    let (mut left, mut right) = (left.encode_utf16(), right.encode_utf16());
    loop {
        match (left.next(), right.next()) {
            (Some(a), Some(b)) if a == b => {}
            (Some(a), Some(b)) => return i32::from(a) - i32::from(b),
            (Some(_), None) => return 1 + left.count() as i32,
            (None, Some(_)) => return -1 - right.count() as i32,
            (None, None) => return 0,
        }
    }
}

fn bool_value(value: bool) -> Value {
    Value::Int(i32::from(value))
}

/// The static factories and predicates.
pub(crate) fn invoke_static(
    host: &mut StdHost,
    r: &MethodRef,
    args: &[Value],
) -> Option<Result<Value, NativeError>> {
    if !owns(&r.owner) {
        return None;
    }
    // A `null` the factory dereferences: Java 25's helpful message names
    // the 26.3 method's own parameter (`bySeparator(identifier, ..)`,
    // `isValidNamespace(namespace)` via `namespace.equals("..")`,
    // `isValidPath(path)` via `path.length()`).
    let null = |index: usize| matches!(args.get(index), Some(Value::Ref(None)));
    let npe = |call: &str, name: &str| {
        Some(Err(NativeError::UncaughtException {
            class: "java/lang/NullPointerException".into(),
            message: Some(format!(
                "Cannot invoke \"{call}\" because \"{name}\" is null"
            )),
            frame: None,
        }))
    };
    match r.name.as_str() {
        "parse" | "tryParse" | "bySeparator" | "tryBySeparator" if null(0) => {
            return npe("String.indexOf(int)", "identifier");
        }
        "fromNamespaceAndPath" | "tryBuild" if null(0) => {
            return npe("String.equals(Object)", "namespace");
        }
        "fromNamespaceAndPath" | "tryBuild" if null(1) => {
            return npe("String.length()", "path");
        }
        "withDefaultNamespace" if null(0) => return npe("String.length()", "path"),
        _ => {}
    }
    let text = |index: usize| args.get(index).map(expect_string).unwrap_or("");
    let char_arg = |index: usize| match args.get(index) {
        Some(Value::Int(code)) => char::from_u32(*code as u32),
        _ => None,
    };
    let separator = match r.name.as_str() {
        "parse" | "tryParse" => Some(':'),
        "bySeparator" | "tryBySeparator" => char_arg(1),
        _ => None,
    };
    let answer = match (r.name.as_str(), separator) {
        ("fromNamespaceAndPath", _) => checked(&r.owner, text(0), text(1)),
        ("tryBuild", _) => Ok(try_value(text(0), text(1))),
        ("withDefaultNamespace", _) => checked(&r.owner, "minecraft", text(0)),
        ("parse" | "bySeparator", Some(separator)) => {
            let (namespace, path) = split(text(0), separator);
            checked(&r.owner, namespace, path)
        }
        ("tryParse" | "tryBySeparator", Some(separator)) => {
            let (namespace, path) = split(text(0), separator);
            Ok(try_value(namespace, path))
        }
        ("isValidPath", _) => Ok(bool_value(is_valid_path(text(0)))),
        ("isValidNamespace", _) => Ok(bool_value(is_valid_namespace(text(0)))),
        ("validPathChar", _) => Ok(bool_value(char_arg(0).is_some_and(valid_path_char))),
        ("isAllowedInIdentifier", _) => Ok(bool_value(
            char_arg(0).is_some_and(|c| valid_path_char(c) || c == ':'),
        )),
        // `read(String)` answers a `DataResult`, `read(StringReader)` reads a
        // command argument: neither is modelled.
        _ => return Some(host.unknown_host_call(r)),
    };
    Some(answer)
}

/// `tryBuild`/`tryBySeparator`: the id, or `null` when either half is invalid.
fn try_value(namespace: &str, path: &str) -> Value {
    if is_valid_namespace(namespace) && is_valid_path(path) {
        crate::runtime::identifier_value(namespace, path)
    } else {
        Value::Ref(None)
    }
}

/// Instance members: the derived text, path operations and ordering.
pub(crate) fn invoke_instance(
    host: &mut StdHost,
    r: &MethodRef,
    receiver: &Value,
    args: &[Value],
) -> Option<Result<Value, NativeError>> {
    if !owns(&r.owner) {
        return None;
    }
    let text = |index: usize| args.get(index).map(expect_string).unwrap_or("");
    // `new ResourceLocation(...)` (public before 1.21): validate as the
    // constructor did and keep the id under the object's marker.
    if r.name == "<init>" {
        let marker = expect_ref(receiver)?.clone();
        let built = match r.descriptor.as_str() {
            "(Ljava/lang/String;)V" => {
                let (namespace, path) = split(text(0), ':');
                checked(&r.owner, namespace, path)
            }
            "(Ljava/lang/String;Ljava/lang/String;)V" => checked(&r.owner, text(0), text(1)),
            _ => return Some(host.unknown_host_call(r)),
        };
        return Some(built.map(|id| {
            if let Some(id) = crate::runtime::host_ids::identifier_key(&id) {
                host.identifier_objects.insert(marker, id.to_owned());
            }
            Value::Void
        }));
    }
    let Some((namespace, path)) = id_parts(host, receiver) else {
        return Some(host.unknown_host_call(r));
    };
    let string = |text: String| Ok(Value::Ref(Some(text)));
    let language_key = format!("{namespace}.{path}");
    let short = |full: String| {
        if namespace == "minecraft" {
            path.clone()
        } else {
            full
        }
    };
    let returns_id = r
        .descriptor
        .ends_with(")Lnet/minecraft/resources/Identifier;")
        || r.descriptor
            .ends_with(")Lnet/minecraft/resources/ResourceLocation;");
    let answer = match (r.name.as_str(), r.descriptor.as_str()) {
        ("getNamespace", "()Ljava/lang/String;") => string(namespace.clone()),
        ("getPath", "()Ljava/lang/String;") => string(path.clone()),
        ("toString", "()Ljava/lang/String;") => string(format!("{namespace}:{path}")),
        ("toShortString", "()Ljava/lang/String;") => string(short(format!("{namespace}:{path}"))),
        ("toLanguageKey", "()Ljava/lang/String;") => string(language_key),
        ("toShortLanguageKey", "()Ljava/lang/String;") => string(short(language_key)),
        ("toLanguageKey", "(Ljava/lang/String;)Ljava/lang/String;") => {
            string(format!("{}.{language_key}", text(0)))
        }
        ("toLanguageKey", "(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;") => {
            string(format!("{}.{language_key}.{}", text(0), text(1)))
        }
        ("toDebugFileName", "()Ljava/lang/String;") => {
            string(format!("{namespace}:{path}").replace(['/', ':'], "_"))
        }
        ("withPath", descriptor) if returns_id && descriptor.starts_with("(Ljava/lang/String;") => {
            checked(&r.owner, &namespace, text(0))
        }
        ("withPrefix", _) if returns_id => {
            checked(&r.owner, &namespace, &(text(0).to_owned() + &path))
        }
        ("withSuffix", _) if returns_id => checked(&r.owner, &namespace, &(path.clone() + text(0))),
        ("hashCode", "()I") => Ok(Value::Int(
            crate::runtime::stable_hash(&namespace)
                .wrapping_mul(31)
                .wrapping_add(crate::runtime::stable_hash(&path)),
        )),
        ("equals", "(Ljava/lang/Object;)Z") => Ok(bool_value(
            args.first().and_then(|other| id_parts(host, other)) == Some((namespace, path)),
        )),
        // Vanilla compares the paths, then the namespaces on a tie.
        ("compareTo", _) => {
            let other = args.first().unwrap_or(&Value::Ref(None));
            if matches!(other, Value::Ref(None)) {
                return Some(Err(NativeError::UncaughtException {
                    class: "java/lang/NullPointerException".into(),
                    message: None,
                    frame: None,
                }));
            }
            let Some((other_namespace, other_path)) = id_parts(host, other) else {
                return Some(host.unknown_host_call(r));
            };
            let order = match java_compare(&path, &other_path) {
                0 => java_compare(&namespace, &other_namespace),
                order => order,
            };
            Ok(Value::Int(order))
        }
        // `withPath(UnaryOperator)` runs a jar lambda, which the runtime does
        // before it gets here ([`Runtime::invoke_identifier_runtime`]);
        // `resolveAgainst` needs a real filesystem `Path`.
        _ => return Some(host.unknown_host_call(r)),
    };
    Some(answer)
}

impl<H: Host> Runtime<'_, H> {
    /// `Identifier.withPath(UnaryOperator)`, which is
    /// `withPath(modifier.apply(path))`: the runtime runs the operator (often
    /// a jar lambda, `p -> "block/" + p`) and the host's `withPath(String)`
    /// validates the result.
    pub(crate) fn invoke_identifier_runtime(
        &mut self,
        r: &MethodRef,
        receiver: Option<&Value>,
        args: &[Value],
    ) -> Result<Option<Value>, NativeError> {
        if !owns(&r.owner)
            || r.name != "withPath"
            || !r
                .descriptor
                .starts_with("(Ljava/util/function/UnaryOperator;)")
        {
            return Ok(None);
        }
        let (Some(receiver), Some(modifier)) = (receiver, args.first()) else {
            return Ok(None);
        };
        let Some((_, path)) =
            crate::runtime::host_ids::identifier_key(receiver).and_then(|id| id.split_once(':'))
        else {
            return Ok(None);
        };
        let new_path = self.apply_callback(modifier, &[Value::Ref(Some(path.to_owned()))])?;
        let with_path = MethodRef {
            owner: r.owner.clone(),
            name: "withPath".into(),
            descriptor: format!("(Ljava/lang/String;){}", crate::runtime::return_desc(&r.descriptor)),
        };
        self.host
            .invoke_instance(&with_path, receiver, &[new_path])
            .map(Some)
    }
}

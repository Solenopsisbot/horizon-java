//! `java.lang.invoke.MethodType` and `MethodHandles.Lookup` as host handles
//! (#156 P10b), and the method-type text the method-handle kinds print.
//! `MethodHandle` and the call sites `LambdaMetafactory` builds are
//! `method_handles.rs`'s kinds.
//!
//! These used to be text (`method-type/<descriptor>`, the bare word
//! `method-handles-lookup`), so a jar string spelled like one was a
//! `MethodType` to `findStatic` and printed as one. A handle is not a
//! `String`.
//!
//! * `MethodType`: key the method descriptor, which the runtime builds from
//!   `Class` values, never from text a jar spelled. Java interns method
//!   types (`methodType(String.class, int.class)` twice is `==` on Java 25),
//!   so the kind is canonical. `MethodType.hashCode` mixes the classes'
//!   identity hashes: counted.
//! * `Lookup`: key `[mode, class]` in the nested-key encoding. `"public"` is
//!   `MethodHandles.publicLookup()`, one object on Java 25 (canonical);
//!   `"full"` with a class is `privateLookupIn(class, ..)`'s full-privilege
//!   lookup; `"full"` without one is a lookup whose class the runtime does not
//!   know (`MethodHandles.lookup()`'s caller), whose `toString` is counted.

use crate::runtime::host_ref::{HostKind, HostRef, Methods, instance_of_class};
use crate::runtime::{Value, descriptor_arguments, return_desc};

pub(crate) const METHOD_TYPE: &str = "java/lang/invoke/MethodType";
pub(crate) const LOOKUP: &str = "java/lang/invoke/MethodHandles$Lookup";

/// `MethodType.methodType(..)` of a method descriptor.
pub(crate) fn method_type(descriptor: &str) -> Value {
    Value::Host(HostRef::new(HostKind::MethodType, descriptor))
}

/// The descriptor a `MethodType` handle holds.
pub(crate) fn method_type_of(value: &Value) -> Option<&str> {
    match value {
        Value::Host(handle) if handle.kind() == HostKind::MethodType => Some(handle.key()),
        _ => None,
    }
}

/// `Class.getSimpleName()` of a field descriptor, where the name alone tells
/// it: a primitive, an array of one, a top-level class, or a JDK member class
/// (`Map$Entry` is `Entry`). A jar's nested class's simple name lives in its
/// `InnerClasses` attribute, so it is `None` (the caller counts).
pub(crate) fn simple_name(descriptor: &str) -> Option<String> {
    if let Some(component) = descriptor.strip_prefix('[') {
        return Some(format!("{}[]", simple_name(component)?));
    }
    let primitive = match descriptor {
        "Z" => "boolean",
        "B" => "byte",
        "C" => "char",
        "S" => "short",
        "I" => "int",
        "J" => "long",
        "F" => "float",
        "D" => "double",
        "V" => "void",
        _ => "",
    };
    if !primitive.is_empty() {
        return Some(primitive.into());
    }
    let class = descriptor.strip_prefix('L')?.strip_suffix(';')?;
    let tail = class.rsplit('/').next().unwrap_or(class);
    match tail.rsplit_once('$') {
        None => Some(tail.into()),
        Some((_, member))
            if crate::runtime::jdk_closure::in_jdk_package(class)
                && !member.is_empty()
                && !member.starts_with(|c: char| c.is_ascii_digit()) =>
        {
            Some(member.into())
        }
        Some(_) => None,
    }
}

/// `MethodType.toString()`: `(int,String[])List`, the classes' simple names.
pub(crate) fn method_type_text(descriptor: &str) -> Option<String> {
    let parameters = descriptor_arguments(descriptor)
        .iter()
        .map(|field| simple_name(field))
        .collect::<Option<Vec<_>>>()?;
    Some(format!(
        "({}){}",
        parameters.join(","),
        simple_name(return_desc(descriptor))?
    ))
}

const PUBLIC_LOOKUP_KEY: &str = "rpublic,null";

/// `MethodHandles.publicLookup()`.
pub(crate) fn public_lookup() -> Value {
    Value::Host(HostRef::of_values(
        HostKind::MethodHandlesLookup,
        &[Value::Ref(Some("public".into())), Value::Ref(None)],
    ))
}

/// A full-privilege lookup on `class` (`privateLookupIn`), or on a class the
/// runtime does not know (`None`: `MethodHandles.lookup()`, `Lookup.in`).
pub(crate) fn lookup(class: Option<&str>) -> Value {
    Value::Host(HostRef::of_values(
        HostKind::MethodHandlesLookup,
        &[
            Value::Ref(Some("full".into())),
            Value::Ref(class.map(str::to_owned)),
        ],
    ))
}

/// The publicLookup singleton is one object; any other lookup is a new one.
pub(crate) fn lookup_canonical(key: &str) -> bool {
    key == PUBLIC_LOOKUP_KEY
}

/// `Lookup.toString()`: `java.lang.Object/publicLookup`, or the lookup
/// class's name for a full-privilege lookup on a class the runtime knows.
fn lookup_text(handle: &HostRef) -> Option<String> {
    if handle.key() == PUBLIC_LOOKUP_KEY {
        return Some("java.lang.Object/publicLookup".into());
    }
    match handle.values()?.as_slice() {
        [_, Value::Ref(Some(class))] => Some(class.replace('/', ".")),
        _ => None,
    }
}

pub(crate) const METHOD_TYPE_METHODS: Methods = Methods {
    to_string: |handle, _| Ok(method_type_text(handle.key())),
    // `MethodType.hashCode` mixes the classes' identity hashes.
    hash_code: |_, _| Ok(None),
    // `MethodType.equals`: the same return and parameter classes.
    equals: |handle, other, _| {
        Ok(Some(
            matches!(other, Value::Host(other) if other.kind() == HostKind::MethodType && other.key() == handle.key()),
        ))
    },
    instance_of: instance_of_class,
    // Classes mean nothing in another jar's runtime.
    crosses_jars: |_, _| false,
    compare_to: None,
};

pub(crate) const LOOKUP_METHODS: Methods = Methods {
    to_string: |handle, _| Ok(lookup_text(handle)),
    // `Lookup.hashCode` mixes the lookup class's identity hash.
    hash_code: |_, _| Ok(None),
    // `Lookup.equals` compares lookup class and modes: known for the one
    // object and for the public singleton; otherwise counted.
    equals: |handle, other, _| {
        Ok(match other {
            Value::Host(other) if other.kind() == HostKind::MethodHandlesLookup => {
                (handle.same_object(other) == Some(true)
                    || (handle.key() == PUBLIC_LOOKUP_KEY && other.key() == PUBLIC_LOOKUP_KEY))
                    .then_some(true)
            }
            _ => Some(false),
        })
    },
    instance_of: instance_of_class,
    crosses_jars: |_, _| false,
    compare_to: None,
};

#[cfg(test)]
mod tests {
    use super::*;

    /// Simple names as `MethodType.toString` prints them (Java 25:
    /// `(int[],List,String[][])Inner[]` for a jar's member class `Inner`,
    /// whose name the runtime leaves to its `InnerClasses` attribute).
    #[test]
    fn method_type_text_uses_simple_names() {
        assert_eq!(
            method_type_text("(I)Ljava/lang/String;").as_deref(),
            Some("(int)String")
        );
        assert_eq!(method_type_text("()V").as_deref(), Some("()void"));
        assert_eq!(
            method_type_text("([ILjava/util/List;[[Ljava/lang/String;Ljava/util/Map$Entry;)J")
                .as_deref(),
            Some("(int[],List,String[][],Entry)long")
        );
        assert_eq!(method_type_text("()[Ldemo/Mh$Inner;"), None);
        assert!(lookup_canonical(match &public_lookup() {
            Value::Host(handle) => handle.key(),
            _ => unreachable!(),
        }));
    }
}

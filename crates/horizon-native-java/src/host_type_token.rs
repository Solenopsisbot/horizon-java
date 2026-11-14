//! Guava's `TypeToken.of(..)` objects as host handles (#156 P5b). They were the text
//! `typetoken/<type>`, which a String could spell: `"typetoken/java/lang/String"` answered
//! `getType`, `equals` and `toString` as a token.
//!
//! **Class** (guava 33.6.0, `javap -p`): `TypeToken.of` makes a `TypeToken$SimpleTypeToken`, a
//! `final` class with a package-private constructor, `extends TypeToken extends TypeCapture
//! extends Object`, and `TypeToken implements Serializable`. So the class is exact. `instanceof`
//! follows the other library classes: an interface a loaded Mixin plan adds is `true`, another
//! interface counted, any other class a proved `false`.
//!
//! **Key** (`HostRef::of_values`): `[serial, type]`, the serial minted per allocation (each `of`
//! is a new object: one key, one object) and `type` the runtime's `Type` value for the token,
//! still a text marker (a class name or a `type/` generic marker, #156 P10c's family). P10c must
//! key it as its handle instead.
//!
//! **Object methods** (`TypeToken`'s own): `equals` is `runtimeType.equals(that.runtimeType)`:
//! exact for one token, and for two whose types are both plain classes (equal exactly when the
//! class is); counted otherwise, since a generic marker may spell one type two ways.
//! `hashCode` is `runtimeType.hashCode()`, an identity hash for a class: counted (main answered
//! the marker text's hash). `toString` is `Types.toString(runtimeType)`, the type's name.

use std::sync::atomic::{AtomicU64, Ordering};

use crate::host_ref::{
    ClassHierarchy, ClassSide, HostKind, HostRef, Methods, Nested, exact_class_rules_out,
};
use crate::{NativeError, Value};

pub(crate) const SIMPLE_TYPE_TOKEN: &str = "com/google/common/reflect/TypeToken$SimpleTypeToken";
const SUPERTYPES: [&str; 4] = [
    "com/google/common/reflect/TypeToken",
    "com/google/common/reflect/TypeCapture",
    "java/io/Serializable",
    "java/lang/Object",
];

static NEXT_TOKEN: AtomicU64 = AtomicU64::new(0);

/// A new token of the type `inner` (`TypeToken.of`).
pub(crate) fn token(inner: &str) -> Value {
    Value::Host(HostRef::of_values(
        HostKind::TypeToken,
        &[
            Value::Long(NEXT_TOKEN.fetch_add(1, Ordering::Relaxed) as i64),
            Value::Ref(Some(inner.to_owned())),
        ],
    ))
}

/// The type a token handle holds, `None` for any other value.
pub(crate) fn token_type(value: &Value) -> Option<String> {
    let Value::Host(handle) = value else {
        return None;
    };
    handle_type(handle)
}

fn handle_type(handle: &HostRef) -> Option<String> {
    if handle.kind() != HostKind::TypeToken {
        return None;
    }
    match handle.values()?.as_slice() {
        [_, Value::Ref(Some(inner))] => Some(inner.clone()),
        _ => None,
    }
}

/// Whether a type marker is a plain class (no generic structure).
fn plain_class(inner: &str) -> bool {
    crate::decode_generic_type(inner).is_none()
}

/// `TypeToken.equals` between two types: equal markers are one type; two plain classes are
/// different types when their names differ; anything else is unknown.
pub(crate) fn types_equal(a: &str, b: &str) -> Option<bool> {
    if a == b {
        return Some(true);
    }
    (plain_class(a) && plain_class(b)).then_some(false)
}

/// `Types.toString(type)`: a class's name, a generic type as Java prints it.
pub(crate) fn type_text(inner: &str) -> String {
    crate::decode_generic_type(inner).map_or_else(
        || crate::class_type_display(inner),
        |node| crate::generic_type_display(&node),
    )
}

fn class_instance_of(class: &str, wanted: &str, hierarchy: &dyn ClassHierarchy) -> Option<bool> {
    if wanted == class
        || SUPERTYPES.contains(&wanted)
        || hierarchy.is_instance(class, wanted) == Some(true)
    {
        return Some(true);
    }
    (hierarchy.is_interface(wanted) == Some(false) || exact_class_rules_out(class, wanted))
        .then_some(false)
}

fn token_equals(
    handle: &HostRef,
    other: &Value,
    _: &mut dyn Nested,
) -> Result<Option<bool>, NativeError> {
    let Some(own) = handle_type(handle) else {
        return Ok(None);
    };
    Ok(match other {
        Value::Host(other) if other.kind() == HostKind::TypeToken => {
            handle_type(other).and_then(|theirs| types_equal(&own, &theirs))
        }
        // Another kind of handle, a box, null: not a `TypeToken`.
        Value::Host(_) | Value::Int(_) | Value::Long(_) | Value::Float(_) | Value::Boxed(_) => {
            Some(false)
        }
        Value::Ref(None) => Some(false),
        Value::Ref(Some(text)) if crate::value_class::is_plain_string(text) => Some(false),
        // A jar object may be a `TypeToken` subclass: the runtime answers it.
        _ => None,
    })
}

pub(crate) const TOKEN_METHODS: Methods = Methods {
    to_string: |handle, _| Ok(handle_type(handle).map(|inner| type_text(&inner))),
    hash_code: |_, _| Ok(None),
    equals: token_equals,
    instance_of: |_, wanted, hierarchy| class_instance_of(SIMPLE_TYPE_TOKEN, wanted, hierarchy),
    crosses_jars: |_, _| false,
    compare_to: None,
};

pub(crate) const TOKEN_CLASS_SIDE: ClassSide = ClassSide {
    owns: |class| class == SIMPLE_TYPE_TOKEN,
    instance_of: class_instance_of,
    superclass: |_| Some(SUPERTYPES[0].to_owned()),
    is_interface: |_| Some(false),
    classes: || vec![SIMPLE_TYPE_TOKEN.to_owned()],
};

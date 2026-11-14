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
//! is a new object: one key, one object) and `type` the token's `Type` value itself: a class
//! marker or #156 P10c's type handle (`host_reflect_type.rs`), never its text. `getType()` hands
//! that very value back, as Guava's `runtimeType` field does.
//!
//! **Object methods** (`TypeToken`'s own): `equals` is `runtimeType.equals(that.runtimeType)`:
//! the JDK's structural `Type.equals` (`host_reflect_type::nodes_equal`): exact where it can
//! compare, counted where it cannot (a variable whose declaration is unknown...).
//! `hashCode` is `runtimeType.hashCode()`, an identity hash for a class: counted (main answered
//! the marker text's hash). `toString` is `Types.toString(runtimeType)`, the type's name.

use std::sync::atomic::{AtomicU64, Ordering};

use crate::host_ref::{
    ClassHierarchy, ClassSide, HostKind, HostRef, Methods, Nested, exact_class_rules_out,
};
use crate::{NativeError, Value};

pub(crate) const SIMPLE_TYPE_TOKEN: &str = "com/google/common/reflect/TypeToken$SimpleTypeToken";
const TYPE_TOKEN: &str = "com/google/common/reflect/TypeToken";
const TYPE_CAPTURE: &str = "com/google/common/reflect/TypeCapture";
const SUPERTYPES: [&str; 4] = [
    "com/google/common/reflect/TypeToken",
    "com/google/common/reflect/TypeCapture",
    "java/io/Serializable",
    "java/lang/Object",
];

static NEXT_TOKEN: AtomicU64 = AtomicU64::new(0);

/// A new token of the type `inner` (`TypeToken.of`): a class marker or a type handle.
pub(crate) fn token(inner: &Value) -> Value {
    Value::Host(HostRef::of_values(
        HostKind::TypeToken,
        &[
            Value::Long(NEXT_TOKEN.fetch_add(1, Ordering::Relaxed) as i64),
            inner.clone(),
        ],
    ))
}

/// The `Type` a token handle holds, `None` for any other value.
pub(crate) fn token_type(value: &Value) -> Option<Value> {
    let Value::Host(handle) = value else {
        return None;
    };
    handle_type(handle)
}

fn handle_type(handle: &HostRef) -> Option<Value> {
    if handle.kind() != HostKind::TypeToken {
        return None;
    }
    match <[Value; 2]>::try_from(handle.values()?) {
        Ok([_, inner]) => Some(inner),
        Err(_) => None,
    }
}

pub(crate) fn token_class(_: &str) -> &str {
    SIMPLE_TYPE_TOKEN
}

/// `runtimeType.equals(that.runtimeType)`: the JDK's structural equality of the two types,
/// `None` where it cannot be told.
pub(crate) fn types_equal(a: &Value, b: &Value) -> Option<bool> {
    let (a, b) = (
        crate::host_gson_type::any_node(a)?,
        crate::host_gson_type::any_node(b)?,
    );
    crate::host_reflect_type::nodes_equal(&a, &b)
}

/// `Types.toString(type)`: a class's name, a generic type as the JDK prints it.
pub(crate) fn type_text(inner: &Value) -> Option<String> {
    Some(guava_type_text(&crate::host_gson_type::any_node(inner)?))
}

/// `Types.toString(type)`: `Class.getName()` for a class (`[I` for `int[]`), the type's own
/// `toString` otherwise (a Gson type's text is Gson's).
pub(crate) fn guava_type_text(node: &crate::GenericType) -> String {
    match node {
        crate::GenericType::Class(name) => name.replace('/', "."),
        other => crate::generic_type_display(other),
    }
}

/// The supertypes of each class in `SimpleTypeToken`'s chain (guava 33.6.0, `javap`):
/// `SimpleTypeToken extends TypeToken extends TypeCapture extends Object`, and `TypeToken
/// implements Serializable`.
fn supertypes(class: &str) -> &'static [&'static str] {
    match class {
        SIMPLE_TYPE_TOKEN => &SUPERTYPES,
        TYPE_TOKEN => &SUPERTYPES,
        _ => &[TYPE_CAPTURE, "java/lang/Object"],
    }
}

fn class_instance_of(class: &str, wanted: &str, hierarchy: &dyn ClassHierarchy) -> Option<bool> {
    if wanted == class
        || supertypes(class).contains(&wanted)
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
    nested: &mut dyn Nested,
) -> Result<Option<bool>, NativeError> {
    let Some(own) = handle_type(handle) else {
        return Ok(None);
    };
    // A jar's `new TypeToken<T>() {}`: its captured type, where this context reads it.
    if crate::jar_object::object_key(other).is_some() {
        return Ok(nested
            .guava_token_type(other)
            .and_then(|theirs| types_equal(&own, &theirs)));
    }
    Ok(match other {
        Value::Host(other) if other.kind() == HostKind::TypeToken => {
            handle_type(other).and_then(|theirs| types_equal(&own, &theirs))
        }
        // Another kind of handle, a box, null: not a `TypeToken`.
        Value::Host(_) | Value::Int(_) | Value::Long(_) | Value::Float(_) | Value::Boxed(_) => {
            Some(false)
        }
        Value::Ref(None) | Value::Void => Some(false),
        // A jar object may be a `TypeToken` subclass (its captured type is the
        // runtime's to read); any other reference (a string, a host model's
        // object) is not a `TypeToken`.
        Value::Ref(Some(_)) => crate::jar_object::object_key(other)
            .is_none()
            .then_some(false),
    })
}

pub(crate) const TOKEN_METHODS: Methods = Methods {
    to_string: |handle, _| Ok(handle_type(handle).and_then(|inner| type_text(&inner))),
    hash_code: |_, _| Ok(None),
    equals: token_equals,
    instance_of: |_, wanted, hierarchy| class_instance_of(SIMPLE_TYPE_TOKEN, wanted, hierarchy),
    crosses_jars: |_, _| false,
    compare_to: None,
};

/// `SimpleTypeToken`, `TypeToken` and `TypeCapture`: a `getClass()` walk up the chain stays
/// exact (#209).
pub(crate) const TOKEN_CLASS_SIDE: ClassSide = ClassSide {
    owns: |class| [SIMPLE_TYPE_TOKEN, TYPE_TOKEN, TYPE_CAPTURE].contains(&class),
    instance_of: class_instance_of,
    superclass: |class| {
        Some(
            match class {
                SIMPLE_TYPE_TOKEN => TYPE_TOKEN,
                TYPE_TOKEN => TYPE_CAPTURE,
                _ => "java/lang/Object",
            }
            .to_owned(),
        )
    },
    is_interface: |_| Some(false),
    classes: || {
        [SIMPLE_TYPE_TOKEN, TYPE_TOKEN, TYPE_CAPTURE]
            .map(str::to_owned)
            .to_vec()
    },
};

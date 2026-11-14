//! Gson 2.14.0's `TypeToken` and its canonical types as host handles (#156 P5b).
//!
//! **`GsonTypes.canonicalize`** (`javap -c -p`, gson-2.14.0): a `Class` stays itself, except an
//! array class, which becomes a `GenericArrayTypeImpl` of its canonical component; a
//! `ParameterizedType`, `GenericArrayType` or `WildcardType` becomes a **new** Gson object of
//! the matching `GsonTypes$…Impl` class (`final`, implementing only its `java.lang.reflect`
//! interface), its parts canonical too; a `TypeVariable` stays the JDK's. So
//! `TypeToken.getType()` is equal to the JDK node it came from, both ways (the JDK's
//! `ParameterizedTypeImpl.equals` and Gson's both compare owner, raw type and arguments), but
//! `==` is `false`, its class is Gson's, and it prints with `GsonTypes.typeToString`: a
//! parameterized type is `rawType.getName()` and its arguments, so a member type prints
//! `g.G$Outer$Inner<java.lang.Integer>` where the JDK prints its owner's arguments too.
//!
//! **Key.** A canonical type is `[id, node]` (`HostRef::of_values`): `id` names one Gson object
//! (a serial per canonicalisation, then `.<n>` per part it holds, so `getActualTypeArguments()[0]`
//! is the same object on every call, as the array's elements are), and `node` is the type itself,
//! kept in #156 P10c's node encoding (`host_reflect_type::type_value`) inside the key only, never
//! handed out as a JDK type. One key, one object. A token made by a factory is `[serial, type]`;
//! a jar's `new TypeToken<T>() {}` keeps its type in its own `type` field, as Gson's does.
//!
//! **Object methods.** `equals` is `GsonTypes.equals`, structural (`host_reflect_type::
//! nodes_equal`, against a Gson or a JDK node); `hashCode` hashes classes' identity hashes:
//! counted. A token's `equals` compares the two types the same way, its `hashCode` is its type's
//! (counted), its `toString` is `typeToString(type)`.

use std::sync::atomic::{AtomicU64, Ordering};

use crate::host_ref::{
    ClassHierarchy, ClassSide, HostKind, HostRef, Methods, Nested, exact_class_rules_out,
};
use crate::host_reflect_type::{nodes_equal, type_node, type_value};
use crate::{GenericType, NativeError, Value};

pub(crate) const TOKEN: &str = "com/google/gson/reflect/TypeToken";
const PARAMETERIZED: &str = "com/google/gson/internal/GsonTypes$ParameterizedTypeImpl";
const GENERIC_ARRAY: &str = "com/google/gson/internal/GsonTypes$GenericArrayTypeImpl";
const WILDCARD: &str = "com/google/gson/internal/GsonTypes$WildcardTypeImpl";
const OBJECT: &str = "java/lang/Object";
const TYPE: &str = "java/lang/reflect/Type";

static NEXT: AtomicU64 = AtomicU64::new(0);

fn serial() -> String {
    NEXT.fetch_add(1, Ordering::Relaxed).to_string()
}

/// A class array's node as Gson canonicalises it: `GenericArrayTypeImpl` of the canonical
/// component (`[[I` is an array of an array of `int`).
fn canonical_node(node: GenericType) -> GenericType {
    match node {
        GenericType::Class(name) => match name.strip_prefix('[') {
            Some(component) => GenericType::Array(Box::new(canonical_node(GenericType::Class(
                crate::erased_field_type(component),
            )))),
            None => GenericType::Class(name),
        },
        GenericType::Parameterized { raw, owner, args } => GenericType::Parameterized {
            raw,
            owner: owner.map(|owner| Box::new(canonical_node(*owner))),
            args: args.into_iter().map(canonical_node).collect(),
        },
        GenericType::Array(component) => GenericType::Array(Box::new(canonical_node(*component))),
        GenericType::Wildcard { upper, lower } => GenericType::Wildcard {
            upper: upper.into_iter().map(canonical_node).collect(),
            lower: lower.into_iter().map(canonical_node).collect(),
        },
        variable => variable,
    }
}

/// The value a canonical node is, the Gson object `id` for one that is no class or variable.
fn node_value(id: String, node: &GenericType) -> Value {
    match node {
        // A class is its `Class` object (#156 P13b1).
        GenericType::Class(name) => crate::host_class::class_value(name),
        GenericType::Variable { .. } => type_value(node),
        _ => Value::Host(HostRef::of_values(
            HostKind::GsonType,
            &[Value::Ref(Some(id)), type_value(node)],
        )),
    }
}

/// `GsonTypes.canonicalize(type)`: `None` for a value that is no `Type`.
pub(crate) fn canonicalize(value: &Value) -> Option<Value> {
    if gson_node(value).is_some() {
        // Already Gson's: canonicalize makes a new object of the same content.
        return Some(node_value(serial(), &gson_node(value)?));
    }
    let node = type_node(value)?;
    Some(match &node {
        GenericType::Class(name) if !name.starts_with('[') => value.clone(),
        GenericType::Variable { .. } => value.clone(),
        _ => node_value(serial(), &canonical_node(node)),
    })
}

fn gson_parts(handle: &HostRef) -> Option<(String, GenericType)> {
    if handle.kind() != HostKind::GsonType {
        return None;
    }
    match <[Value; 2]>::try_from(handle.values()?) {
        Ok([Value::Ref(Some(id)), node]) => Some((id, type_node(&node)?)),
        _ => None,
    }
}

/// The node of a Gson canonical type, `None` for any other value.
pub(crate) fn gson_node(value: &Value) -> Option<GenericType> {
    match value {
        Value::Host(handle) => gson_parts(handle).map(|(_, node)| node),
        _ => None,
    }
}

/// The node of any `Type` value: a class marker, a JDK node or a Gson one.
pub(crate) fn any_node(value: &Value) -> Option<GenericType> {
    gson_node(value).or_else(|| type_node(value))
}

/// `Class.getName()` of a class marker (`a.b.C$D`, `int`).
fn class_name(name: &str) -> String {
    name.replace('/', ".")
}

/// `GsonTypes.typeToString`: a class's `getName()`, a Gson type's own `toString`, a variable's
/// name.
pub(crate) fn type_to_string(node: &GenericType) -> String {
    match node {
        GenericType::Class(name) => class_name(name),
        GenericType::Parameterized { raw, args, .. } => {
            if args.is_empty() {
                return class_name(raw);
            }
            let args: Vec<String> = args.iter().map(type_to_string).collect();
            format!("{}<{}>", class_name(raw), args.join(", "))
        }
        GenericType::Array(component) => format!("{}[]", type_to_string(component)),
        GenericType::Wildcard { upper, lower } => {
            if let Some(lower) = lower.first() {
                format!("? super {}", type_to_string(lower))
            } else {
                match upper.first() {
                    None => "?".to_owned(),
                    Some(GenericType::Class(name)) if name == OBJECT => "?".to_owned(),
                    Some(upper) => format!("? extends {}", type_to_string(upper)),
                }
            }
        }
        GenericType::Variable { name, .. } => name.clone(),
    }
}

/// A class marker's descriptor, for an array class built from it.
fn descriptor(name: &str) -> String {
    match name {
        "byte" => "B".into(),
        "char" => "C".into(),
        "double" => "D".into(),
        "float" => "F".into(),
        "int" => "I".into(),
        "long" => "J".into(),
        "short" => "S".into(),
        "boolean" => "Z".into(),
        other if other.starts_with('[') => other.into(),
        other => format!("L{other};"),
    }
}

/// `GsonTypes.getRawType(type)`: the class a type erases to.
pub(crate) fn raw_type(node: &GenericType) -> String {
    match node {
        GenericType::Class(name) => name.clone(),
        GenericType::Parameterized { raw, .. } => raw.clone(),
        GenericType::Array(component) => format!("[{}", descriptor(&raw_type(component))),
        GenericType::Wildcard { upper, .. } => upper.first().map_or(OBJECT.into(), raw_type),
        GenericType::Variable { .. } => OBJECT.into(),
    }
}

/// A part of the Gson object `handle` (an argument, the owner, a component, a bound): the same
/// object on every call.
pub(crate) fn part(handle: &Value, index: usize, node: &GenericType) -> Value {
    let id = match handle {
        Value::Host(handle) => gson_parts(handle).map(|(id, _)| id),
        _ => None,
    }
    .unwrap_or_else(serial);
    node_value(format!("{id}.{index}"), node)
}

/// A new token (`TypeToken.get`, `getArray`) of the canonical type `canonical`.
pub(crate) fn token(canonical: Value) -> Value {
    Value::Host(HostRef::of_values(
        HostKind::GsonTypeToken,
        &[Value::Ref(Some(serial())), canonical],
    ))
}

/// The canonical type a token handle holds.
pub(crate) fn token_type(value: &Value) -> Option<Value> {
    let Value::Host(handle) = value else {
        return None;
    };
    token_parts(handle)
}

fn token_parts(handle: &HostRef) -> Option<Value> {
    if handle.kind() != HostKind::GsonTypeToken {
        return None;
    }
    match <[Value; 2]>::try_from(handle.values()?) {
        Ok([_, canonical]) => Some(canonical),
        Err(_) => None,
    }
}

/// `GsonTypes.equals(a, b)` of two `Type` values, `None` where it cannot be told.
pub(crate) fn types_equal(a: &Value, b: &Value) -> Option<bool> {
    nodes_equal(&any_node(a)?, &any_node(b)?)
}

fn shape_class(node: &GenericType) -> &'static str {
    match node {
        GenericType::Array(_) => GENERIC_ARRAY,
        GenericType::Wildcard { .. } => WILDCARD,
        _ => PARAMETERIZED,
    }
}

pub(crate) fn gson_type_class(key: &str) -> &str {
    HostRef::new(HostKind::GsonType, key)
        .values()
        .and_then(|values| type_node(values.get(1)?))
        .map_or(PARAMETERIZED, |node| shape_class(&node))
}

fn shape_interface(class: &str) -> &'static str {
    match class {
        GENERIC_ARRAY => "java/lang/reflect/GenericArrayType",
        WILDCARD => "java/lang/reflect/WildcardType",
        _ => "java/lang/reflect/ParameterizedType",
    }
}

/// `class instanceof wanted` for an object of exactly `class` (one of Gson's `final` type
/// classes, or `TypeToken`): its own types are exact, an interface a loaded Mixin plan adds is
/// `true`, another interface counted, another class a proved `false`.
fn class_instance_of(class: &str, wanted: &str, hierarchy: &dyn ClassHierarchy) -> Option<bool> {
    let own = if class == TOKEN {
        wanted == TOKEN
    } else {
        wanted == class || wanted == shape_interface(class) || wanted == TYPE
    };
    if own || wanted == OBJECT || hierarchy.is_instance(class, wanted) == Some(true) {
        return Some(true);
    }
    (hierarchy.is_interface(wanted) == Some(false) || exact_class_rules_out(class, wanted))
        .then_some(false)
}

fn gson_type_equals(
    handle: &HostRef,
    other: &Value,
    _: &mut dyn Nested,
) -> Result<Option<bool>, NativeError> {
    if let Value::Host(other) = other
        && handle.same_object(other) == Some(true)
    {
        return Ok(Some(true));
    }
    let Some((_, node)) = gson_parts(handle) else {
        return Ok(None);
    };
    Ok(match other {
        Value::Host(_) => match any_node(other) {
            Some(theirs) => nodes_equal(&node, &theirs),
            None if crate::jar_object::object_key(other).is_some() => None,
            None => Some(false),
        },
        // A class: no Gson shape is one.
        Value::Ref(Some(_)) => Some(false),
        _ => Some(false),
    })
}

pub(crate) const GSON_TYPE_METHODS: Methods = Methods {
    to_string: |handle, _| Ok(gson_parts(handle).map(|(_, node)| type_to_string(&node))),
    hash_code: |_, _| Ok(None),
    equals: gson_type_equals,
    instance_of: |handle, wanted, hierarchy| {
        class_instance_of(gson_type_class(handle.key()), wanted, hierarchy)
    },
    crosses_jars: |_, _| false,
    compare_to: None,
};

fn token_equals(
    handle: &HostRef,
    other: &Value,
    nested: &mut dyn Nested,
) -> Result<Option<bool>, NativeError> {
    if let Value::Host(other) = other
        && handle.same_object(other) == Some(true)
    {
        return Ok(Some(true));
    }
    let Some(own) = token_parts(handle) else {
        return Ok(None);
    };
    Ok(match other {
        Value::Host(other_handle) if other_handle.kind() == HostKind::GsonTypeToken => {
            token_parts(other_handle).and_then(|theirs| types_equal(&own, &theirs))
        }
        // A jar's `TypeToken` subclass: its `type` field, where this context reads it.
        other if crate::jar_object::object_key(other).is_some() => nested
            .gson_token_type(other)
            .and_then(|theirs| types_equal(&own, &theirs)),
        _ => Some(false),
    })
}

pub(crate) const TOKEN_METHODS: Methods = Methods {
    to_string: |handle, _| {
        Ok(token_parts(handle)
            .and_then(|canonical| any_node(&canonical))
            .map(|node| type_to_string(&node)))
    },
    hash_code: |_, _| Ok(None),
    equals: token_equals,
    instance_of: |_, wanted, hierarchy| class_instance_of(TOKEN, wanted, hierarchy),
    crosses_jars: |_, _| false,
    compare_to: None,
};

pub(crate) fn token_class(_: &str) -> &str {
    TOKEN
}

pub(crate) const GSON_TYPE_CLASS_SIDE: ClassSide = ClassSide {
    owns: |class| [PARAMETERIZED, GENERIC_ARRAY, WILDCARD].contains(&class),
    instance_of: class_instance_of,
    superclass: |_| Some(OBJECT.to_owned()),
    is_interface: |_| Some(false),
    classes: || {
        [PARAMETERIZED, GENERIC_ARRAY, WILDCARD]
            .map(str::to_owned)
            .to_vec()
    },
};

pub(crate) const TOKEN_CLASS_SIDE: ClassSide = ClassSide {
    owns: |class| class == TOKEN,
    instance_of: class_instance_of,
    superclass: |_| Some(OBJECT.to_owned()),
    is_interface: |_| Some(false),
    classes: || vec![TOKEN.to_owned()],
};

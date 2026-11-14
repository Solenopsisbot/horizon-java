//! `java.lang.reflect.Type` nodes and `AnnotatedType`s as host handles (#156
//! P10c). The signature parser and the reflection calls that hand these out
//! stay in `lib.rs` and `reflect_generic.rs`; this module is their keys and
//! rows.
//!
//! These were text a jar `String` could spell: `type/<length-prefixed node>`
//! and `annotated-type/<type marker>`.
//!
//! What Java 25 does (`sun.reflect.generics.reflectiveObjects`, and the
//! oracle in `tests/fixture_host_reflect_type.rs`):
//!
//! * a type that is a class is that `Class`: it stays main's class marker
//!   (the internal name), so `getGenericType() == String.class` holds. #156
//!   P13 (`Class` objects) must turn both into one `Class` handle.
//! * `ParameterizedTypeImpl`, `TypeVariableImpl`, `WildcardTypeImpl` and
//!   `GenericArrayTypeImpl`: two lookups of one member usually answer the
//!   **same** object on Java 25 (`getField("ls").getGenericType() ==
//!   getField("ls").getGenericType()` is `true`: a `Field` or `Method` copy
//!   shares its root's generic repository), but that cache is soft-referenced
//!   and may be rebuilt, so the kind is not canonical and identity between
//!   two answers is counted unless the same handle flows. Their `equals` is
//!   specified and structural:
//!   * `ParameterizedTypeImpl`: owner, raw type and arguments equal (an
//!     implicit owner is the raw class's declaring class);
//!   * `TypeVariableImpl`: the same generic declaration and name;
//!   * `WildcardTypeImpl`: upper and lower bounds equal (`?` has the upper
//!     bound `Object`, so it equals `? extends Object`);
//!   * `GenericArrayTypeImpl`: the component types equal.
//!
//!   A part this host cannot compare (a variable whose declaration the
//!   signature does not name, an owner it cannot infer, a jar's own `Type`
//!   implementation) is counted. Their `hashCode`s hash `Class` objects'
//!   identity hashes: counted. `toString`/`getTypeName` are the JDK's
//!   (`java.util.Map<java.lang.String, ? extends java.lang.Number>`).
//! * The implementations are JDK-internal and final in effect (only
//!   `CoreReflectionFactory` makes them), so `instanceof` is exact: each
//!   shape's interface, `Type`, and for a variable `AnnotatedElement`.
//!   `getClass()` is counted ([`ClassRule::Unknown`]).
//! * `AnnotatedType`: an `AnnotatedTypeFactory` implementation per shape,
//!   built afresh per call (not canonical); its `equals` compares types,
//!   annotations and owners (counted here unless the same handle flows).
//!   Type-use annotations are not read: a member with no
//!   `RuntimeVisibleTypeAnnotations` has none (exact); one with them marks
//!   its `AnnotatedType`s ([`annotations_unread`]), whose annotation reads and
//!   text are counted, nested ones included (Java 25 reports them).
//!
//! Keys hold a node through [`HostRef::of_values`]: a class is its name, a
//! nested node its own handle. Class and variable names (jar text with `<`,
//! `>`, `,`, `$`) are never split back.
//!
//! **For #156 P5b (Gson `TypeToken`):** `new TypeToken<List<String>>(){}`
//! reads the anonymous class's `getGenericSuperclass()`, which answers this
//! module's handle; read its type argument with [`type_node`]. Gson 2.14.0's
//! `TypeToken` stores `GsonTypes.canonicalize(arg)`, a **new Gson object**:
//! `getType() == arg` is `false`, `equals` and `hashCode` agree with the JDK
//! node both ways, its class is `com.google.gson.internal.GsonTypes$
//! ParameterizedTypeImpl`, and a nested generic prints differently
//! (`g.G$Outer$Inner<java.lang.Integer>`, where the JDK prints
//! `g.G$Outer<java.lang.String>$Inner<java.lang.Integer>`); a `Class` stays
//! itself. So P5b must model Gson's canonical type (its own kind or a flag:
//! equal by content, Gson's class and `toString`), never hand this module's
//! handle back, and never build one with [`type_value`] (that would claim
//! the JDK's class). It must not re-encode the type as text either.
//!
//! [`ClassRule::Unknown`]: crate::host_ref::ClassRule

use crate::host_ref::{ClassHierarchy, HostKind, HostRef, Methods, Nested};
use crate::{GenericType, NativeError, TypeDeclaration, Value};

const OBJECT: &str = "java/lang/Object";
const TYPE: &str = "java/lang/reflect/Type";

fn text(value: &str) -> Value {
    Value::Ref(Some(value.to_owned()))
}

/// The value a type node is: a class's marker, or a `Type` handle.
pub(crate) fn type_value(node: &GenericType) -> Value {
    let values = match node {
        GenericType::Class(name) => return text(name),
        GenericType::Parameterized { raw, owner, args } => {
            let mut values = vec![
                text("P"),
                text(raw),
                owner.as_deref().map_or(Value::Ref(None), type_value),
            ];
            values.extend(args.iter().map(type_value));
            values
        }
        GenericType::Variable { name, declaration } => {
            let (class, method, descriptor) = match declaration {
                Some(TypeDeclaration::Class(class)) => (Some(class.clone()), None, None),
                Some(TypeDeclaration::Method {
                    class,
                    name,
                    descriptor,
                }) => (
                    Some(class.clone()),
                    Some(name.clone()),
                    Some(descriptor.clone()),
                ),
                None => (None, None, None),
            };
            vec![
                text("V"),
                text(name),
                Value::Ref(class),
                Value::Ref(method),
                Value::Ref(descriptor),
            ]
        }
        GenericType::Wildcard { upper, lower } => {
            // `WildcardTypeImpl`: `?` has the upper bound `Object`.
            let object = [GenericType::Class(OBJECT.into())];
            let upper = if upper.is_empty() { &object[..] } else { upper };
            let mut values = vec![text("W"), Value::Int(upper.len() as i32)];
            values.extend(upper.iter().map(type_value));
            values.extend(lower.iter().map(type_value));
            values
        }
        GenericType::Array(component) => vec![text("A"), type_value(component)],
    };
    Value::Host(HostRef::of_values(HostKind::ReflectType, &values))
}

/// The node a `Type` value names: a type handle's, or a class marker's.
pub(crate) fn type_node(value: &Value) -> Option<GenericType> {
    match value {
        Value::Ref(Some(name)) => Some(GenericType::Class(name.clone())),
        Value::Host(handle) if handle.kind() == HostKind::ReflectType => {
            node_of_values(&handle.values()?)
        }
        _ => None,
    }
}

/// The text Guava's `TypeToken` model keeps a type under (`guava_model.rs`,
/// a later phase's family): a class's name, or a type handle's own encoding,
/// which only [`type_from_text`] reads back.
pub(crate) fn type_text(value: &Value) -> Option<String> {
    match value {
        Value::Ref(Some(name)) => Some(name.clone()),
        Value::Host(handle) if handle.kind() == HostKind::ReflectType => Some(handle.encode()),
        _ => None,
    }
}

/// The `Type` value [`type_text`] wrote.
pub(crate) fn type_from_text(text: &str) -> Value {
    match HostRef::decode(text) {
        Some(handle) if handle.kind() == HostKind::ReflectType => Value::Host(handle),
        _ => Value::Ref(Some(text.to_owned())),
    }
}

/// The node a type handle names (no class marker).
pub(crate) fn handle_node(value: &Value) -> Option<GenericType> {
    match value {
        Value::Host(handle) if handle.kind() == HostKind::ReflectType => {
            node_of_values(&handle.values()?)
        }
        _ => None,
    }
}

fn node_of_values(values: &[Value]) -> Option<GenericType> {
    let name = |value: &Value| match value {
        Value::Ref(Some(name)) => Some(name.clone()),
        _ => None,
    };
    Some(match values {
        [tag, raw, owner, args @ ..] if name(tag)? == "P" => GenericType::Parameterized {
            raw: name(raw)?,
            owner: match owner {
                Value::Ref(None) => None,
                owner => Some(Box::new(type_node(owner)?)),
            },
            args: args.iter().map(type_node).collect::<Option<_>>()?,
        },
        [tag, variable, class, method, descriptor] if name(tag)? == "V" => {
            let declaration = match (class, method, descriptor) {
                (Value::Ref(Some(class)), Value::Ref(None), Value::Ref(None)) => {
                    Some(TypeDeclaration::Class(class.clone()))
                }
                (
                    Value::Ref(Some(class)),
                    Value::Ref(Some(method)),
                    Value::Ref(Some(descriptor)),
                ) => Some(TypeDeclaration::Method {
                    class: class.clone(),
                    name: method.clone(),
                    descriptor: descriptor.clone(),
                }),
                _ => None,
            };
            GenericType::Variable {
                name: name(variable)?,
                declaration,
            }
        }
        [tag, Value::Int(count), bounds @ ..] if name(tag)? == "W" => {
            let count = usize::try_from(*count)
                .ok()
                .filter(|count| *count <= bounds.len())?;
            let (upper, lower) = bounds.split_at(count);
            GenericType::Wildcard {
                upper: upper.iter().map(type_node).collect::<Option<_>>()?,
                lower: lower.iter().map(type_node).collect::<Option<_>>()?,
            }
        }
        [tag, component] if name(tag)? == "A" => {
            GenericType::Array(Box::new(type_node(component)?))
        }
        _ => return None,
    })
}

/// Java's `equals` between two type nodes: `Some(false)` once a part
/// certainly differs, `None` when a part only a JVM could compare decides.
pub(crate) fn nodes_equal(a: &GenericType, b: &GenericType) -> Option<bool> {
    fn all(parts: impl IntoIterator<Item = Option<bool>>) -> Option<bool> {
        let mut known = true;
        for part in parts {
            match part {
                Some(false) => return Some(false),
                None => known = false,
                Some(true) => {}
            }
        }
        known.then_some(true)
    }
    fn lists(a: &[GenericType], b: &[GenericType]) -> Option<bool> {
        if a.len() != b.len() {
            return Some(false);
        }
        all(a.iter().zip(b).map(|(a, b)| nodes_equal(a, b)))
    }
    match (a, b) {
        (GenericType::Class(a), GenericType::Class(b)) => Some(a == b),
        (
            GenericType::Parameterized {
                raw: a_raw,
                owner: a_owner,
                args: a_args,
            },
            GenericType::Parameterized {
                raw: b_raw,
                owner: b_owner,
                args: b_args,
            },
        ) => {
            // An absent owner is the raw class's declaring class: the same
            // for the same raw class, unknown against a written owner.
            let owners = match (a_owner, b_owner) {
                (None, None) => Some(true),
                (Some(a), Some(b)) => nodes_equal(a, b),
                _ => None,
            };
            all([owners, Some(a_raw == b_raw), lists(a_args, b_args)])
        }
        (
            GenericType::Variable {
                name: a_name,
                declaration: a_declaration,
            },
            GenericType::Variable {
                name: b_name,
                declaration: b_declaration,
            },
        ) => {
            let declarations = match (a_declaration, b_declaration) {
                (Some(a), Some(b)) => Some(a == b),
                _ => None,
            };
            all([declarations, Some(a_name == b_name)])
        }
        (
            GenericType::Wildcard {
                upper: a_upper,
                lower: a_lower,
            },
            GenericType::Wildcard {
                upper: b_upper,
                lower: b_lower,
            },
        ) => {
            let object = [GenericType::Class(OBJECT.into())];
            let upper = |bounds: &'_ [GenericType]| -> Vec<GenericType> {
                if bounds.is_empty() {
                    object.to_vec()
                } else {
                    bounds.to_vec()
                }
            };
            all([
                lists(a_lower, b_lower),
                lists(&upper(a_upper), &upper(b_upper)),
            ])
        }
        (GenericType::Array(a), GenericType::Array(b)) => nodes_equal(a, b),
        // Different shapes: each implementation's `equals` asks for its own
        // interface, which no other shape implements.
        _ => Some(false),
    }
}

/// The interfaces a node's JDK implementation class has, all of them (the
/// class is JDK-internal: no other type can be added to it).
fn shape_types(node: &GenericType) -> &'static [&'static str] {
    match node {
        GenericType::Parameterized { .. } => &[
            "sun/reflect/generics/reflectiveObjects/ParameterizedTypeImpl",
            "java/lang/reflect/ParameterizedType",
            TYPE,
        ],
        GenericType::Variable { .. } => &[
            "sun/reflect/generics/reflectiveObjects/TypeVariableImpl",
            "sun/reflect/generics/reflectiveObjects/LazyReflectiveObjectGenerator",
            "java/lang/reflect/TypeVariable",
            "java/lang/reflect/AnnotatedElement",
            TYPE,
        ],
        GenericType::Wildcard { .. } => &[
            "sun/reflect/generics/reflectiveObjects/WildcardTypeImpl",
            "sun/reflect/generics/reflectiveObjects/LazyReflectiveObjectGenerator",
            "java/lang/reflect/WildcardType",
            TYPE,
        ],
        GenericType::Array(_) => &[
            "sun/reflect/generics/reflectiveObjects/GenericArrayTypeImpl",
            "java/lang/reflect/GenericArrayType",
            TYPE,
        ],
        GenericType::Class(_) => &[],
    }
}

/// [`crate::host_ref::ClassRule::Unknown`]'s declared types of a type handle.
pub(crate) fn type_types(key: &str) -> &'static [&'static str] {
    handle_node(&Value::Host(HostRef::new(HostKind::ReflectType, key)))
        .map_or(&[TYPE], |node| shape_types(&node))
}

fn type_instance_of(handle: &HostRef, wanted: &str, _: &dyn ClassHierarchy) -> Option<bool> {
    let node = handle_node(&Value::Host(handle.clone()))?;
    Some(wanted == OBJECT || shape_types(&node).contains(&wanted))
}

/// Whether `value` is a jar object (one whose `equals` a jar defines).
fn jar_object(value: &Value) -> bool {
    crate::jar_object::object_key(value).is_some()
}

fn type_equals(
    handle: &HostRef,
    other: &Value,
    _: &mut dyn Nested,
) -> Result<Option<bool>, NativeError> {
    if let Value::Host(other) = other
        && handle.same_object(other) == Some(true)
    {
        return Ok(Some(true));
    }
    let Some(node) = handle_node(&Value::Host(handle.clone())) else {
        return Ok(None);
    };
    Ok(match other {
        Value::Host(_) => match handle_node(other) {
            Some(other) => nodes_equal(&node, &other),
            // No other handle kind is a `java.lang.reflect.Type` shape.
            None => Some(false),
        },
        // A jar's own `ParameterizedType` (Gson's, Guava's) answers through
        // its own methods: counted.
        other if jar_object(other) => None,
        // `null`, a `String`, a `Class` (final, no shape interface), a box.
        _ => Some(false),
    })
}

/// `Type.getTypeName`/`toString`.
fn type_name(handle: &HostRef, _: &mut dyn Nested) -> Result<Option<String>, NativeError> {
    Ok(handle_node(&Value::Host(handle.clone())).map(|node| crate::generic_type_display(&node)))
}

pub(crate) const TYPE_METHODS: Methods = Methods {
    to_string: type_name,
    // Hashes of `Class` objects (identity hashes): counted.
    hash_code: |_, _| Ok(None),
    equals: type_equals,
    instance_of: type_instance_of,
    // A reflective view of the run's own classes.
    crosses_jars: |_, _| false,
    compare_to: None,
};

/// The mark an `AnnotatedType`'s key carries when its member has type-use
/// annotations the runtime does not read (`RuntimeVisibleTypeAnnotations`).
const UNREAD: &str = "type-annotations";

/// The `AnnotatedType` of the type `inner` (a class marker or a type handle).
/// `unread`: its member carries type-use annotations the runtime does not
/// read, so its annotations and text are counted (Java 25 reports them).
pub(crate) fn annotated(inner: Value, unread: bool) -> Value {
    let values = if unread {
        vec![inner, Value::Ref(Some(UNREAD.into()))]
    } else {
        vec![inner]
    };
    Value::Host(HostRef::of_values(HostKind::AnnotatedType, &values))
}

/// Whether an `AnnotatedType` handle's annotations are unknown here.
pub(crate) fn annotations_unread(value: &Value) -> bool {
    match value {
        Value::Host(handle) if handle.kind() == HostKind::AnnotatedType => {
            handle.values().is_some_and(|values| values.len() == 2)
        }
        _ => false,
    }
}

/// The type an `AnnotatedType` handle annotates.
pub(crate) fn annotated_inner(value: &Value) -> Option<Value> {
    match value {
        Value::Host(handle) if handle.kind() == HostKind::AnnotatedType => {
            match handle.values()?.as_slice() {
                [inner] | [inner, _] => Some(inner.clone()),
                _ => None,
            }
        }
        _ => None,
    }
}

const ANNOTATED: &[&str] = &[
    "java/lang/reflect/AnnotatedType",
    "java/lang/reflect/AnnotatedElement",
];

/// The interfaces an `AnnotatedTypeFactory` implementation has for each shape.
fn annotated_shape(node: &GenericType) -> Option<&'static str> {
    Some(match node {
        GenericType::Parameterized { .. } => "java/lang/reflect/AnnotatedParameterizedType",
        GenericType::Wildcard { .. } => "java/lang/reflect/AnnotatedWildcardType",
        GenericType::Variable { .. } => "java/lang/reflect/AnnotatedTypeVariable",
        GenericType::Array(_) => "java/lang/reflect/AnnotatedArrayType",
        GenericType::Class(name) if name.starts_with('[') => "java/lang/reflect/AnnotatedArrayType",
        GenericType::Class(_) => return None,
    })
}

pub(crate) fn annotated_types(_: &str) -> &'static [&'static str] {
    ANNOTATED
}

fn annotated_instance_of(handle: &HostRef, wanted: &str, _: &dyn ClassHierarchy) -> Option<bool> {
    let node = type_node(&annotated_inner(&Value::Host(handle.clone()))?)?;
    Some(wanted == OBJECT || ANNOTATED.contains(&wanted) || annotated_shape(&node) == Some(wanted))
}

/// Whether a node names a nested class anywhere (an `AnnotatedType` prints a
/// nested class through its annotated owner, `Map.Entry`, not as the type
/// does).
fn names_nested_class(node: &GenericType) -> bool {
    match node {
        GenericType::Class(name) => name.contains('$'),
        GenericType::Parameterized { raw, owner, args } => {
            raw.contains('$') || owner.is_some() || args.iter().any(names_nested_class)
        }
        GenericType::Variable { .. } => false,
        GenericType::Wildcard { upper, lower } => upper.iter().chain(lower).any(names_nested_class),
        GenericType::Array(component) => names_nested_class(component),
    }
}

/// `AnnotatedType.toString` of a type nobody annotated: the type's name
/// (Java 25, the fixture's `annotated` line); a nested class is counted.
fn annotated_text(handle: &HostRef, _: &mut dyn Nested) -> Result<Option<String>, NativeError> {
    // Unread type-use annotations print before the type (`@demo.TU()
    // java.lang.String`): counted.
    if annotations_unread(&Value::Host(handle.clone())) {
        return Ok(None);
    }
    Ok(annotated_inner(&Value::Host(handle.clone()))
        .and_then(|inner| type_node(&inner))
        .filter(|node| !names_nested_class(node))
        .map(|node| crate::generic_type_display(&node)))
}

pub(crate) const ANNOTATED_METHODS: Methods = Methods {
    to_string: annotated_text,
    hash_code: |_, _| Ok(None),
    // `AnnotatedTypeBaseImpl.equals` compares types, annotations and owners.
    equals: |handle, other, _| {
        Ok(match other {
            Value::Host(other) if handle.same_object(other) == Some(true) => Some(true),
            Value::Host(other) if other.kind() == HostKind::AnnotatedType => None,
            other if jar_object(other) => None,
            _ => Some(false),
        })
    },
    instance_of: annotated_instance_of,
    crosses_jars: |_, _| false,
    compare_to: None,
};

#[cfg(test)]
mod tests {
    use super::*;

    /// Class and variable names (jar text) with every separator a layer uses
    /// read back as themselves, and two different names never make one key
    /// (`List<A,B>` is no class name, `Map$Entry` no owner): the old
    /// `type/` text was length-prefixed, the key now nests values ("keys
    /// never parse jar text", #156).
    #[test]
    fn names_round_trip_and_never_alias() {
        let alphabet = ["a", "<", ">", ",", "$", "/", ";", "\\", "hrtyp:", "P", ""];
        let names: Vec<String> = alphabet
            .iter()
            .flat_map(|a| alphabet.iter().map(move |b| format!("x{a}{b}")))
            .collect();
        let mut seen = std::collections::BTreeMap::new();
        for (index, name) in names.iter().enumerate() {
            let other = &names[(index * 7 + 3) % names.len()];
            let node = GenericType::Parameterized {
                raw: name.clone(),
                owner: None,
                args: vec![
                    GenericType::Variable {
                        name: other.clone(),
                        declaration: Some(TypeDeclaration::Class(name.clone())),
                    },
                    GenericType::Array(Box::new(GenericType::Wildcard {
                        upper: vec![GenericType::Class(other.clone())],
                        lower: Vec::new(),
                    })),
                ],
            };
            let value = type_value(&node);
            assert_eq!(type_node(&value), Some(node.clone()));
            let Value::Host(handle) = value else {
                unreachable!()
            };
            if let Some(earlier) = seen.insert(handle.key().to_owned(), node.clone()) {
                assert_eq!(earlier, node);
            }
        }
    }
}

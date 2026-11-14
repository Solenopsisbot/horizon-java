//! Gson 2.14.0's `TypeToken` members and its canonical types' `java.lang.reflect` members, run
//! by the interpreter (`host_gson_type.rs` holds the kinds and their rows).

use super::*;
use crate::runtime::host_gson_type::{
    TOKEN, any_node, canonicalize, gson_node, part, raw_type, token, token_type, type_to_string,
    types_equal,
};

/// A jar subclass token's `type` field (the real field's key, so a reflective read sees it).
const TYPE_FIELD: &str = "#typeLjava/lang/reflect/Type;";

impl<H: Host> Runtime<'_, H> {
    /// The canonical type a Gson token holds: a factory's handle, or a jar subclass object whose
    /// constructor ran.
    pub(crate) fn gson_token_type(&self, token_value: &Value) -> Option<Value> {
        if let Some(canonical) = token_type(token_value) {
            return Some(canonical);
        }
        let marker = crate::runtime::jar_object::object_key(token_value)?;
        self.fields
            .get(&format!("{marker}|{TOKEN}{TYPE_FIELD}"))
            .cloned()
    }

    /// Gson's `TypeToken` (static factories, the subclass constructor, the members) and the
    /// members of its canonical types; `None` for every other call.
    pub(super) fn invoke_gson_types(
        &mut self,
        r: &MethodRef,
        receiver: Option<&Value>,
        args: &[Value],
    ) -> Result<Option<Value>, NativeError> {
        if let Some(receiver) = receiver
            && let Some(node) = gson_node(receiver)
        {
            return self.gson_type_member(r, receiver, &node);
        }
        let token_call = r.owner == TOKEN
            || receiver.is_some_and(|receiver| {
                token_type(receiver).is_some()
                    || (matches!(
                        r.name.as_str(),
                        "equals" | "hashCode" | "toString" | "getType" | "getRawType"
                    ) && crate::runtime::jar_object::object_key(receiver).is_some()
                        && self.gson_token_type(receiver).is_some()
                        // A member the jar's subclass does not declare itself is
                        // `TypeToken`'s (whose members are `final` or `Object`'s).
                        && crate::runtime::jar_object::object_class(receiver).is_some_and(|class| {
                            self.member_owner(class, &r.name, &r.descriptor)
                                .is_none_or(|(_, jar)| !jar)
                        }))
            });
        if !token_call {
            return Ok(None);
        }
        let npe = || NativeError::UncaughtException {
            class: "java/lang/NullPointerException".into(),
            message: None,
            frame: None,
        };
        match (receiver, r.name.as_str(), r.descriptor.as_str()) {
            (
                None,
                "get",
                "(Ljava/lang/Class;)Lcom/google/gson/reflect/TypeToken;"
                | "(Ljava/lang/reflect/Type;)Lcom/google/gson/reflect/TypeToken;",
            ) => {
                let Some(value) = args.first().filter(|value| **value != Value::Ref(None)) else {
                    return Err(npe());
                };
                return Ok(match canonicalize(value) {
                    Some(canonical) => Some(token(canonical)),
                    None => Some(self.host.unknown_operand_call(r)?),
                });
            }
            // `getArray(component)`: a token of `new GenericArrayTypeImpl(component)`.
            (None, "getArray", "(Ljava/lang/reflect/Type;)Lcom/google/gson/reflect/TypeToken;") => {
                let Some(node) = args.first().and_then(any_node) else {
                    return Err(npe());
                };
                let array =
                    crate::runtime::host_reflect_type::type_value(&GenericType::Array(Box::new(node)));
                return Ok(canonicalize(&array).map(token));
            }
            // `new TypeToken<T>() {}`: `getGenericSuperclass()`'s argument, canonical.
            (Some(receiver), "<init>", "()V") => {
                let Some(marker) = crate::runtime::jar_object::object_key(receiver).map(str::to_owned)
                else {
                    return Ok(None);
                };
                let Some(class) = crate::runtime::jar_object::key_class(&marker).map(str::to_owned) else {
                    return Ok(None);
                };
                let scope = crate::runtime::reflect_generic::class_scope(&class, self.class(&class));
                let superclass = self
                    .class(&class)
                    .and_then(ClassFile::class_signature)
                    .as_deref()
                    .and_then(|signature| {
                        crate::runtime::reflect_generic::parse_class_signature(signature, scope)
                    })
                    .map(|(superclass, _)| superclass);
                let captured = match superclass {
                    Some(GenericType::Parameterized { raw, args, .. }) if raw == TOKEN => {
                        args.into_iter().next()
                    }
                    _ => None,
                };
                // A raw or indirect subclass, or a captured type variable (Gson throws
                // unless a system property allows it): counted.
                let Some(captured) = captured.filter(|node| !holds_variable(node)) else {
                    return self.host.unknown_operand_call(r).map(Some);
                };
                let Some(canonical) = canonicalize(&generic_type_value(&captured)) else {
                    return self.host.unknown_operand_call(r).map(Some);
                };
                self.fields
                    .insert(format!("{marker}|{TOKEN}{TYPE_FIELD}"), canonical);
                return Ok(Some(Value::Void));
            }
            (None, _, _) => return self.host.unknown_operand_call(r).map(Some),
            _ => {}
        }
        let Some(receiver) = receiver else {
            return Ok(None);
        };
        let Some(canonical) = self.gson_token_type(receiver) else {
            return Ok(None);
        };
        let Some(node) = any_node(&canonical) else {
            return Ok(None);
        };
        Ok(Some(match (r.name.as_str(), r.descriptor.as_str()) {
            ("getType", "()Ljava/lang/reflect/Type;") => canonical,
            ("getRawType", "()Ljava/lang/Class;") => Value::Ref(Some(raw_type(&node))),
            ("toString", "()Ljava/lang/String;") => Value::Ref(Some(type_to_string(&node))),
            // `o instanceof TypeToken && GsonTypes.equals(type, o.type)`.
            ("equals", "(Ljava/lang/Object;)Z") => {
                let other = args.first().cloned().unwrap_or(Value::Ref(None));
                let equal = match self.gson_token_type(&other) {
                    Some(theirs) => types_equal(&canonical, &theirs),
                    None if crate::runtime::jar_object::object_key(&other).is_some() => None,
                    None => Some(false),
                };
                match equal {
                    Some(equal) => Value::Int(i32::from(equal)),
                    None => return self.host.unknown_operand_call(r).map(Some),
                }
            }
            // `type.hashCode()`: classes' identity hashes.
            _ => return self.host.unknown_operand_call(r).map(Some),
        }))
    }

    /// A Gson canonical type's `java.lang.reflect` members.
    fn gson_type_member(
        &mut self,
        r: &MethodRef,
        receiver: &Value,
        node: &GenericType,
    ) -> Result<Option<Value>, NativeError> {
        let types = |runtime: &mut Self, values: Vec<Value>| {
            runtime.new_array_value("[Ljava/lang/reflect/Type;", values)
        };
        Ok(Some(match (r.name.as_str(), r.descriptor.as_str(), node) {
            ("getTypeName" | "toString", "()Ljava/lang/String;", _) => {
                Value::Ref(Some(type_to_string(node)))
            }
            (
                "getRawType",
                "()Ljava/lang/reflect/Type;",
                GenericType::Parameterized { raw, .. },
            ) => Value::Ref(Some(raw.clone())),
            (
                "getOwnerType",
                "()Ljava/lang/reflect/Type;",
                GenericType::Parameterized { owner, .. },
            ) => match owner {
                Some(owner) => part(receiver, 0, owner),
                None => Value::Ref(None),
            },
            (
                "getActualTypeArguments",
                "()[Ljava/lang/reflect/Type;",
                GenericType::Parameterized { args, .. },
            ) => {
                let values = args
                    .iter()
                    .enumerate()
                    .map(|(index, arg)| part(receiver, index + 1, arg))
                    .collect();
                types(self, values)
            }
            (
                "getGenericComponentType",
                "()Ljava/lang/reflect/Type;",
                GenericType::Array(component),
            ) => part(receiver, 1, component),
            (
                "getUpperBounds",
                "()[Ljava/lang/reflect/Type;",
                GenericType::Wildcard { upper, .. },
            ) => {
                let object = GenericType::Class("java/lang/Object".into());
                let bound = upper.first().unwrap_or(&object);
                let value = part(receiver, 1, bound);
                types(self, vec![value])
            }
            (
                "getLowerBounds",
                "()[Ljava/lang/reflect/Type;",
                GenericType::Wildcard { lower, .. },
            ) => {
                let values = lower
                    .first()
                    .map(|bound| part(receiver, 2, bound))
                    .into_iter()
                    .collect();
                types(self, values)
            }
            _ => return Ok(None),
        }))
    }
}

/// Whether a node holds a type variable anywhere.
fn holds_variable(node: &GenericType) -> bool {
    match node {
        GenericType::Variable { .. } => true,
        GenericType::Class(_) => false,
        GenericType::Parameterized { owner, args, .. } => {
            owner.as_deref().is_some_and(holds_variable) || args.iter().any(holds_variable)
        }
        GenericType::Array(component) => holds_variable(component),
        GenericType::Wildcard { upper, lower } => upper.iter().chain(lower).any(holds_variable),
    }
}

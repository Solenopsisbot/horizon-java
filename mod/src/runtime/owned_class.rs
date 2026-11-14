//! The reflective `Class` surface on a class name a family kind owns (`host_ref::ClassSide`, #156):
//! `getClass()` of a handle whose kind knows its exact class (a game event's
//! `ServerTickEvent$Post`), or an `ldc` of such a class.
//!
//! The runtime holds no class file for those names, and the generic `Class` models answer a class
//! they do not hold as if it were a bare top-level class extending `Object` with any method asked
//! for. For an owned name that would be a silent wrong answer (`getSuperclass()` = `Object`,
//! `getSimpleName()` = `ServerTickEvent$Post`, `getMethod("isCanceled")` found). So every `Class`
//! call on one answers from the kind's `ClassSide`, or from the name itself where Java's answer
//! follows from it alone, or is counted.

use super::*;

/// `Class` members whose generic model answers an owned name exactly from the name and the row
/// (`getName`, `isAssignableFrom` through `HostKind::class_instance_of`...).
const EXACT_ELSEWHERE: &[(&str, &str)] = &[
    ("getName", "()Ljava/lang/String;"),
    ("getTypeName", "()Ljava/lang/String;"),
    ("getPackageName", "()Ljava/lang/String;"),
    ("isInstance", "(Ljava/lang/Object;)Z"),
    ("isAssignableFrom", "(Ljava/lang/Class;)Z"),
    ("cast", "(Ljava/lang/Object;)Ljava/lang/Object;"),
    ("isPrimitive", "()Z"),
    ("isArray", "()Z"),
    ("hashCode", "()I"),
    ("equals", "(Ljava/lang/Object;)Z"),
];

/// Whether `text` is a class name (internal form, the text a `Class` is until #156 P13) a family
/// kind owns. A path that prints a value cannot tell such a `Class` from a String spelled like it,
/// and Java prints the two differently (`class X` vs the text): it counts.
pub(crate) fn owned_class_name(text: &str) -> bool {
    HostKind::owning_class(text)
        .and_then(HostKind::class_side)
        .is_some()
}

impl<H: Host> Runtime<'_, H> {
    /// The class side that answers for `class`, unless the run holds a class file of that name: a
    /// jar's own copy (CreativeCore's bundled bus-7 `Event` on Fabric) always wins.
    pub(crate) fn owned_class_side(&self, class: &str) -> Option<&'static host_ref::ClassSide> {
        if self.class(class).is_some() {
            return None;
        }
        HostKind::owning_class(class).and_then(HostKind::class_side)
    }

    /// A `Class` call on a class name a family kind owns: its answer, or counted. `None` for any
    /// other call (or a member the generic model answers exactly).
    pub(crate) fn invoke_owned_class(
        &mut self,
        r: &MethodRef,
        receiver: Option<&Value>,
    ) -> Result<Option<Value>, NativeError> {
        if r.owner != "java/lang/Class" {
            return Ok(None);
        }
        let Some(class) = receiver
            .and_then(expect_ref)
            .map(|name| name.replace('.', "/"))
        else {
            return Ok(None);
        };
        let Some(side) = self.owned_class_side(&class) else {
            return Ok(None);
        };
        let descriptor = r.descriptor.as_str();
        if EXACT_ELSEWHERE.contains(&(r.name.as_str(), descriptor)) {
            return Ok(None);
        }
        // The simple and canonical names follow from the binary name alone for a top-level class;
        // a nested one's need its `InnerClasses` (`$` may be part of a name).
        let simple = class.rsplit('/').next().unwrap_or(&class);
        let answer = match (r.name.as_str(), descriptor) {
            ("getSimpleName", "()Ljava/lang/String;") if !simple.contains('$') => {
                Some(Value::Ref(Some(simple.to_owned())))
            }
            ("getCanonicalName", "()Ljava/lang/String;") if !simple.contains('$') => {
                Some(Value::Ref(Some(class.replace('/', "."))))
            }
            ("getSuperclass", "()Ljava/lang/Class;") => {
                (side.superclass)(&class).map(|parent| Value::Ref(Some(parent)))
            }
            ("isInterface", "()Z") => {
                (side.is_interface)(&class).map(|is| Value::Int(i32::from(is)))
            }
            // `Class.toString`: `class X` or `interface X`.
            ("toString", "()Ljava/lang/String;") => (side.is_interface)(&class).map(|is| {
                let word = if is { "interface" } else { "class" };
                Value::Ref(Some(format!("{word} {}", class.replace('/', "."))))
            }),
            _ => None,
        };
        match answer {
            Some(answer) => Ok(Some(answer)),
            None => self.host.unknown_operand_call(r).map(Some),
        }
    }
}

impl<H: Host> Runtime<'_, H> {
    /// `String.valueOf(c)` for a `Class`-typed concatenation operand naming a class a family kind
    /// owns: `Class.toString` (`class X`/`interface X`) when the kind knows which, else counted.
    /// `None` for any other operand (a `Class` is still its name's text until #156 P13).
    pub(crate) fn owned_class_text(
        &mut self,
        value: &Value,
    ) -> Result<Option<String>, NativeError> {
        let Some(class) = crate::runtime::host_class::class_name(value).map(|name| name.replace('.', "/")) else {
            return Ok(None);
        };
        if self.owned_class_side(&class).is_none() {
            return Ok(None);
        }
        let to_string = MethodRef {
            owner: "java/lang/Class".into(),
            name: "toString".into(),
            descriptor: "()Ljava/lang/String;".into(),
        };
        Ok(match self.invoke_owned_class(&to_string, Some(value))? {
            Some(Value::Ref(text)) => Some(text.unwrap_or_else(|| "null".into())),
            _ => None,
        })
    }
}

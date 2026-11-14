//! Generic and annotated reflection over a class's and a method's
//! `Signature` attributes: `Class.getGenericSuperclass`/`getGenericInterfaces`,
//! `Method.getReturnType`/`getGenericReturnType`/`getParameterTypes`/
//! `getGenericParameterTypes`/`getAnnotatedReturnType`, and the
//! `AnnotatedType` nodes the last one answers.
//!
//! Config libraries build their whole model from these: DazzleConf reads each
//! config interface method's annotated return type to pick a serializer, and
//! Guava's `TypeToken` subclass idiom (`new TypeToken<List<String>>() {}`)
//! reads the anonymous class's generic superclass. The answers reuse the
//! `java.lang.reflect.Type` markers `Field.getGenericType` already builds, so a
//! raw type is its class marker and a parameterized one decodes through the
//! same surface.
//!
//! Type-use annotations are not read: an `AnnotatedType` answers an empty
//! annotation array, which is what the JVM answers for a type nobody
//! annotated. A method or class with no `Signature` attribute answers the
//! erased type, exactly as the JVM does.

use super::*;
use crate::jdk_model::class_access_flags;

/// `java.lang.reflect.AnnotatedType[]`.
const ANNOTATED_TYPES: &str = "[Ljava/lang/reflect/AnnotatedType;";

impl ClassFile {
    /// The class's own generic `Signature` attribute, when it carries one.
    pub(crate) fn class_signature(&self) -> Option<String> {
        self.read_signatures(None)
    }

    /// One method's generic `Signature` attribute, when it carries one.
    pub(crate) fn method_signature(&self, name: &str, descriptor: &str) -> Option<String> {
        self.read_signatures(Some((name, descriptor)))
    }

    /// Walk the raw class file to the class attributes (`method == None`) or
    /// to one method's attributes, and read its `Signature`. A malformed file
    /// answers `None`, so the caller falls back to the erased type.
    fn read_signatures(&self, method: Option<(&str, &str)>) -> Option<String> {
        let mut r = Reader::new(&self.raw);
        if r.u4().ok()? != 0xCAFE_BABE {
            return None;
        }
        r.skip(4).ok()?;
        let pool = read_pool(&mut r).ok()?;
        r.skip(6).ok()?; // access, this, super
        let interfaces = r.u2().ok()? as usize;
        r.skip(interfaces.checked_mul(2)?).ok()?;
        let fields = r.u2().ok()? as usize;
        for _ in 0..fields {
            r.skip(6).ok()?;
            skip_attributes(&mut r)?;
        }
        let methods = r.u2().ok()? as usize;
        for _ in 0..methods {
            r.skip(2).ok()?;
            let name = utf8(&pool, r.u2().ok()?).ok()?;
            let descriptor = utf8(&pool, r.u2().ok()?).ok()?;
            let matching = method.is_some_and(|(wanted, wanted_descriptor)| {
                name == wanted && descriptor == wanted_descriptor
            });
            if matching {
                return read_signature_attribute(&mut r, &pool);
            }
            skip_attributes(&mut r)?;
        }
        if method.is_some() {
            return None;
        }
        read_signature_attribute(&mut r, &pool)
    }
}

impl ClassFile {
    /// Whether one field (`field`) or method carries the attribute named
    /// `attribute` (`RuntimeVisibleTypeAnnotations`); `None` for a member the
    /// file does not hold or a malformed file.
    pub(crate) fn member_has_attribute(
        &self,
        field: bool,
        member: (&str, &str),
        attribute: &str,
    ) -> Option<bool> {
        let mut r = Reader::new(&self.raw);
        if r.u4().ok()? != 0xCAFE_BABE {
            return None;
        }
        r.skip(4).ok()?;
        let pool = read_pool(&mut r).ok()?;
        r.skip(6).ok()?; // access, this, super
        let interfaces = r.u2().ok()? as usize;
        r.skip(interfaces.checked_mul(2)?).ok()?;
        for table in [true, false] {
            let count = r.u2().ok()? as usize;
            for _ in 0..count {
                r.skip(2).ok()?;
                let name = utf8(&pool, r.u2().ok()?).ok()?;
                let descriptor = utf8(&pool, r.u2().ok()?).ok()?;
                if table != field || (name.as_str(), descriptor.as_str()) != member {
                    skip_attributes(&mut r)?;
                    continue;
                }
                let attributes = r.u2().ok()? as usize;
                let mut found = false;
                for _ in 0..attributes {
                    found |= utf8(&pool, r.u2().ok()?).ok()? == attribute;
                    let length = r.u4().ok()? as usize;
                    r.skip(length).ok()?;
                }
                return Some(found);
            }
        }
        None
    }
}

/// Whether the `AnnotatedType`s of a member's type may carry type-use
/// annotations the runtime does not read: its `RuntimeVisibleTypeAnnotations`
/// (a field's all target its type; a method's may target only a parameter,
/// which still makes its return type's answers counted). A member the
/// runtime does not hold, or a malformed file, may too.
pub(crate) fn type_annotations_unread(
    file: Option<&ClassFile>,
    field: bool,
    member: (&str, &str),
) -> bool {
    file.and_then(|file| file.member_has_attribute(field, member, "RuntimeVisibleTypeAnnotations"))
        .unwrap_or(true)
}

fn skip_attributes(r: &mut Reader<'_>) -> Option<()> {
    let count = r.u2().ok()? as usize;
    for _ in 0..count {
        r.skip(2).ok()?;
        let length = r.u4().ok()? as usize;
        r.skip(length).ok()?;
    }
    Some(())
}

fn read_signature_attribute(r: &mut Reader<'_>, pool: &[Cp]) -> Option<String> {
    let count = r.u2().ok()? as usize;
    let mut found = None;
    for _ in 0..count {
        let attribute = utf8(pool, r.u2().ok()?).ok()?;
        let length = r.u4().ok()? as usize;
        if attribute == "Signature" && length == 2 && found.is_none() {
            found = utf8(pool, r.u2().ok()?).ok();
        } else {
            r.skip(length).ok()?;
        }
    }
    found
}

fn signature_parser(text: &str, scope: TypeScope) -> Option<GenericSignatureParser<'_>> {
    (!text.is_empty() && text.len() <= MAX_GENERIC_SIGNATURE_BYTES).then_some(
        GenericSignatureParser {
            bytes: text.as_bytes(),
            pos: 0,
            depth: 0,
            scope,
        },
    )
}

/// The variables in scope in class `class` (its own formal type parameters).
pub(crate) fn class_scope(class: &str, file: Option<&ClassFile>) -> TypeScope {
    TypeScope {
        class: class.to_owned(),
        class_variables: file
            .and_then(ClassFile::class_signature)
            .map(|signature| type_parameter_names(&signature))
            .unwrap_or_default(),
        method: None,
        method_variables: Vec::new(),
    }
}

/// The variables in scope in method `name descriptor` of `class`: its own
/// formal type parameters, then the class's.
fn method_scope(class: &str, file: Option<&ClassFile>, name: &str, descriptor: &str) -> TypeScope {
    TypeScope {
        method: Some((name.to_owned(), descriptor.to_owned())),
        method_variables: file
            .and_then(|file| file.method_signature(name, descriptor))
            .map(|signature| type_parameter_names(&signature))
            .unwrap_or_default(),
        ..class_scope(class, file)
    }
}

/// The bounds of variable `name` that `declaration` declares
/// (`TypeVariable.getBounds`), from its formal type parameter list.
pub(crate) fn variable_bounds(
    declaration: &TypeDeclaration,
    name: &str,
    file: Option<&ClassFile>,
) -> Option<Vec<GenericType>> {
    let file = file?;
    let (signature, scope) = match declaration {
        TypeDeclaration::Class(class) => (file.class_signature()?, class_scope(class, Some(file))),
        TypeDeclaration::Method {
            class,
            name: method,
            descriptor,
        } => (
            file.method_signature(method, descriptor)?,
            method_scope(class, Some(file), method, descriptor),
        ),
    };
    let mut parser = signature_parser(&signature, scope)?;
    parser
        .parse_type_parameters()?
        .into_iter()
        .find_map(|(declared, bounds)| (declared == name).then_some(bounds))
}

/// A class `Signature`: the generic superclass and interfaces, read with the
/// class's own variables in `scope`.
pub(crate) fn parse_class_signature(
    text: &str,
    scope: TypeScope,
) -> Option<(GenericType, Vec<GenericType>)> {
    let mut parser = signature_parser(text, scope)?;
    parser.parse_type_parameters()?;
    let superclass = parser.parse_reference()?;
    let mut interfaces = Vec::new();
    while parser.pos < parser.bytes.len() {
        if interfaces.len() >= MAX_GENERIC_TYPE_ARGUMENTS {
            return None;
        }
        interfaces.push(parser.parse_reference()?);
    }
    Some((superclass, interfaces))
}

/// A method `Signature`: the generic parameter types and return type
/// (`void` is the class `void`). Thrown types are ignored.
fn parse_method_signature(text: &str, scope: TypeScope) -> Option<(Vec<GenericType>, GenericType)> {
    let mut parser = signature_parser(text, scope)?;
    parser.parse_type_parameters()?;
    if parser.peek()? != b'(' {
        return None;
    }
    parser.pos += 1;
    let mut parameters = Vec::new();
    while parser.peek()? != b')' {
        if parameters.len() >= MAX_GENERIC_TYPE_ARGUMENTS {
            return None;
        }
        parameters.push(parser.parse_java_type()?);
    }
    parser.pos += 1;
    let result = if parser.peek()? == b'V' {
        GenericType::Class("void".into())
    } else {
        parser.parse_java_type()?
    };
    Some((parameters, result))
}

/// The erased class of one descriptor (`I` is `int`, `[I` stays the array
/// descriptor, `Ljava/util/List;` is `java/util/List`): its `Class` object.
fn erased(descriptor: &str) -> Value {
    crate::host_class::class_value(&erased_field_type(descriptor))
}

/// The `(class, name, descriptor)` a `Method` handle names, when it carries
/// its descriptor.
///
/// A lookup the runtime could not settle carries only the parameters it was
/// asked for (`(I)`, no return type) and the class it was asked on (maybe a
/// subclass of the declaring one), so none of its facts is answered here
/// (counted).
fn method_marker(value: Option<&Value>) -> Option<(String, String, String)> {
    let method = host_reflect::method_of(value?)?;
    let descriptor = method
        .descriptor
        .filter(|descriptor| !descriptor.ends_with(')'))?;
    Some((method.class, method.name, descriptor))
}

/// The `AnnotatedType` of a type node (`host_reflect_type.rs`); `unread`:
/// its member carries type-use annotations the runtime does not read.
fn annotated(value: &GenericType, unread: bool) -> Value {
    crate::host_reflect_type::annotated(generic_type_value(value), unread)
}

impl<H: Host> Runtime<'_, H> {
    /// The generic reflection calls this module answers, or `None`.
    pub(super) fn invoke_reflect_generic(
        &mut self,
        r: &MethodRef,
        receiver: Option<&Value>,
    ) -> Result<Option<Value>, NativeError> {
        match r.owner.as_str() {
            "java/lang/Class" => self.class_generic(r, receiver),
            "java/lang/reflect/Method" => Ok(self.method_generic(r, receiver)),
            owner if owner.starts_with("java/lang/reflect/Annotated") => {
                Ok(self.annotated_type_member(r, receiver))
            }
            _ => Ok(None),
        }
    }

    fn class_generic(
        &mut self,
        r: &MethodRef,
        receiver: Option<&Value>,
    ) -> Result<Option<Value>, NativeError> {
        if r.name == "getModifiers" && r.descriptor == "()I" {
            let class = reflection_class_name(receiver, r)?.replace('.', "/");
            return Ok(self.class_modifiers(&class).map(Value::Int));
        }
        let wants_super =
            r.name == "getGenericSuperclass" && r.descriptor == "()Ljava/lang/reflect/Type;";
        let wants_interfaces =
            r.name == "getGenericInterfaces" && r.descriptor == "()[Ljava/lang/reflect/Type;";
        if !wants_super && !wants_interfaces {
            return Ok(None);
        }
        let class = reflection_class_name(receiver, r)?.replace('.', "/");
        let Some(file) = self.class(&class) else {
            // A class the runtime does not hold keeps the host's answer.
            return Ok(None);
        };
        let interface = class_access_flags(file).is_some_and(|flags| flags & 0x0200 != 0);
        let parsed = file.class_signature().as_deref().and_then(|signature| {
            parse_class_signature(signature, class_scope(&class, Some(file)))
        });
        let super_name = file.super_name.clone();
        let interface_names = file.interface_names();
        if wants_super {
            // An interface (and `Object`) has no superclass.
            if interface || super_name.is_empty() {
                return Ok(Some(Value::Ref(None)));
            }
            return Ok(Some(match parsed {
                Some((superclass, _)) => generic_type_value(&superclass),
                None => Value::Ref(Some(super_name)),
            }));
        }
        let types = match parsed {
            Some((_, interfaces)) => interfaces,
            None => interface_names
                .into_iter()
                .map(GenericType::Class)
                .collect(),
        };
        Ok(Some(self.generic_type_array(TYPE_ARRAY, &types)))
    }

    /// `Class.getModifiers()`: a jar class's access flags without
    /// `ACC_SUPER`, which is what the JVM reports for a top-level class (a
    /// member class's `InnerClasses` flags, such as `static` or `private`, are
    /// not read). A primitive or `void` is `public abstract final`, an array
    /// takes its element's visibility plus `abstract final`, and the common
    /// JDK types answer their declared modifiers. Any other class keeps the
    /// host's answer.
    fn class_modifiers(&self, class: &str) -> Option<i32> {
        const PUBLIC: i32 = 0x0001;
        const FINAL: i32 = 0x0010;
        const INTERFACE: i32 = 0x0200;
        const ABSTRACT: i32 = 0x0400;
        if let Some(element) = class.strip_prefix('[') {
            let element = erased_field_type(element.trim_start_matches('['));
            let visibility = self.class_modifiers(&element).unwrap_or(PUBLIC) & 0x0007;
            return Some(visibility | ABSTRACT | FINAL);
        }
        if matches!(
            class,
            "boolean" | "byte" | "char" | "short" | "int" | "long" | "float" | "double" | "void"
        ) {
            return Some(PUBLIC | ABSTRACT | FINAL);
        }
        // A nested class's are its own `InnerClasses` row's (`static` lives
        // only there), as JDK 25's `Class.getModifiers` reads them.
        if let Some(file) = self.class(class) {
            return file
                .nested_access_flags()
                .or_else(|| class_access_flags(file))
                .map(|flags| i32::from(flags) & !0x0020);
        }
        Some(match class {
            "java/lang/String"
            | "java/lang/Integer"
            | "java/lang/Long"
            | "java/lang/Short"
            | "java/lang/Byte"
            | "java/lang/Character"
            | "java/lang/Boolean"
            | "java/lang/Float"
            | "java/lang/Double"
            | "java/lang/Class"
            | "java/util/UUID"
            | "java/util/Optional" => PUBLIC | FINAL,
            "java/lang/Object" => PUBLIC,
            "java/lang/Number" | "java/lang/Enum" | "java/lang/Record" => PUBLIC | ABSTRACT,
            "java/util/List"
            | "java/util/Map"
            | "java/util/Set"
            | "java/util/Collection"
            | "java/lang/Iterable"
            | "java/lang/CharSequence"
            | "java/lang/Comparable"
            | "java/lang/Runnable" => PUBLIC | INTERFACE | ABSTRACT,
            other if other.starts_with("java/util/function/") => PUBLIC | INTERFACE | ABSTRACT,
            // The game's API a mod's config names (`TextColor`, `EntityType`)
            // is public; whether one is an interface is not known here.
            other if other.starts_with("net/minecraft/") || other.starts_with("com/mojang/") => {
                PUBLIC
            }
            _ => return None,
        })
    }

    fn method_generic(&mut self, r: &MethodRef, receiver: Option<&Value>) -> Option<Value> {
        let (class, name, descriptor) = method_marker(receiver)?;
        let generic = || {
            let file = self.class(&class);
            file.and_then(|file| file.method_signature(&name, &descriptor))
                .as_deref()
                .and_then(|signature| {
                    parse_method_signature(
                        signature,
                        method_scope(&class, file, &name, &descriptor),
                    )
                })
        };
        let erased_parameters = || {
            descriptor_arguments(&descriptor)
                .iter()
                .map(|parameter| GenericType::Class(erased_field_type(parameter)))
                .collect::<Vec<_>>()
        };
        let erased_return = return_desc(&descriptor).to_owned();
        Some(match (r.name.as_str(), r.descriptor.as_str()) {
            ("getReturnType", "()Ljava/lang/Class;") => erased(&erased_return),
            ("getGenericReturnType", "()Ljava/lang/reflect/Type;") => match generic() {
                Some((_, result)) => generic_type_value(&result),
                None => erased(&erased_return),
            },
            ("getAnnotatedReturnType", "()Ljava/lang/reflect/AnnotatedType;") => {
                let unread = type_annotations_unread(
                    self.class(&class),
                    false,
                    (name.as_str(), descriptor.as_str()),
                );
                match generic() {
                    Some((_, result)) => annotated(&result, unread),
                    None => annotated(
                        &GenericType::Class(erased_field_type(&erased_return)),
                        unread,
                    ),
                }
            }
            ("getParameterTypes", "()[Ljava/lang/Class;") => {
                let types = erased_parameters();
                self.generic_type_array("[Ljava/lang/Class;", &types)
            }
            ("getGenericParameterTypes", "()[Ljava/lang/reflect/Type;") => {
                let types = generic()
                    .map(|(parameters, _)| parameters)
                    .filter(|parameters| {
                        parameters.len() == descriptor_arguments(&descriptor).len()
                    })
                    .unwrap_or_else(erased_parameters);
                self.generic_type_array(TYPE_ARRAY, &types)
            }
            ("getDeclaringClass", "()Ljava/lang/Class;") => crate::host_class::class_value(&class),
            ("getModifiers", "()I") => {
                let file = self.class(&class)?;
                let method = file.method(&name, &descriptor)?;
                // The JVM's recognized method flags (`Method.getModifiers`),
                // bridge, varargs and synthetic included: a bridge is `4161`.
                Value::Int(i32::from(method.access) & 0x1dff)
            }
            ("isDefault", "()Z") => {
                let file = self.class(&class)?;
                let interface = class_access_flags(file).is_some_and(|flags| flags & 0x0200 != 0);
                let method = file.method(&name, &descriptor)?;
                // A default method is a public, non-abstract, non-static
                // instance method of an interface.
                Value::Int(i32::from(
                    interface
                        && method.code.is_some()
                        && !method.is_static()
                        && method.access & 0x0001 != 0,
                ))
            }
            ("isVarArgs", "()Z") => {
                let method = self.class(&class)?.method(&name, &descriptor)?;
                Value::Int(i32::from(method.access & 0x0080 != 0))
            }
            ("isBridge", "()Z") => {
                let method = self.class(&class)?.method(&name, &descriptor)?;
                Value::Int(i32::from(method.access & 0x0040 != 0))
            }
            ("isSynthetic", "()Z") => {
                let method = self.class(&class)?.method(&name, &descriptor)?;
                Value::Int(i32::from(method.access & 0x1000 != 0))
            }
            _ => return None,
        })
    }

    fn annotated_type_member(&mut self, r: &MethodRef, receiver: Option<&Value>) -> Option<Value> {
        let inner = crate::host_reflect_type::annotated_inner(receiver?)?;
        let node = crate::host_reflect_type::type_node(&inner)?;
        // A member with type-use annotations the runtime does not read: its
        // annotations are counted (`None`), and the nested `AnnotatedType`s
        // carry the same mark. Its type and shape stay exact.
        let unread = crate::host_reflect_type::annotations_unread(receiver?);
        let annotation_read = matches!(
            r.name.as_str(),
            "getAnnotations"
                | "getDeclaredAnnotations"
                | "getAnnotation"
                | "getDeclaredAnnotation"
                | "isAnnotationPresent"
        );
        if unread && annotation_read {
            return None;
        }
        let annotated = |value: &GenericType| annotated(value, unread);
        Some(match (r.name.as_str(), r.descriptor.as_str()) {
            ("getType", "()Ljava/lang/reflect/Type;") => inner,
            (
                "getAnnotations" | "getDeclaredAnnotations",
                "()[Ljava/lang/annotation/Annotation;",
            ) => self.new_array_value("[Ljava/lang/annotation/Annotation;", Vec::new()),
            (
                "getAnnotation" | "getDeclaredAnnotation",
                "(Ljava/lang/Class;)Ljava/lang/annotation/Annotation;",
            ) => Value::Ref(None),
            ("isAnnotationPresent", "(Ljava/lang/Class;)Z") => Value::Int(0),
            ("getAnnotatedActualTypeArguments", "()[Ljava/lang/reflect/AnnotatedType;") => {
                let GenericType::Parameterized { args, .. } = node else {
                    return None;
                };
                let values = args.iter().map(annotated).collect::<Vec<_>>();
                self.new_array_value(ANNOTATED_TYPES, values)
            }
            ("getAnnotatedUpperBounds", "()[Ljava/lang/reflect/AnnotatedType;") => {
                let GenericType::Wildcard { upper, .. } = node else {
                    return None;
                };
                let values = generic_type_bounds(&upper)
                    .iter()
                    .map(annotated)
                    .collect::<Vec<_>>();
                self.new_array_value(ANNOTATED_TYPES, values)
            }
            ("getAnnotatedLowerBounds", "()[Ljava/lang/reflect/AnnotatedType;") => {
                let GenericType::Wildcard { lower, .. } = node else {
                    return None;
                };
                let values = lower.iter().map(annotated).collect::<Vec<_>>();
                self.new_array_value(ANNOTATED_TYPES, values)
            }
            _ => return None,
        })
    }
}

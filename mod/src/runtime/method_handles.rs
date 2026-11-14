//! Method handles built at runtime: lookups of JDK methods, the
//! `MethodHandles` combinators, `MethodHandle.invoke`/`invokeExact`/`asType`,
//! `MethodType` queries, and `LambdaMetafactory.metafactory` called directly
//! (not through an `invokedynamic` site).
//!
//! WorldEdit's command framework (piston) builds every argument converter this
//! way: `publicLookup().findStatic(Integer.class, "valueOf", (String)Integer)`,
//! `dropArguments` to add the injected-values parameter, `asType` and
//! `filterReturnValue` to wrap the answer in a `ConversionResult`,
//! `catchException` to turn a throw into a failed conversion, and a `Converter`
//! lambda spun with `LambdaMetafactory.metafactory` over
//! `MethodHandles.invoker(type)`. The host answered all of this null, so every
//! converter lookup came back empty and the entrypoint stopped.
//!
//! A handle is a host handle (`Value::Host`, #156 P10b) of kind
//! `MethodHandle` whose key is the combinator tree as JSON, with bound values
//! in the runtime's value-marker form. Nothing is kept in a side table, so a
//! handle survives being captured by a lambda, stored in a collection, or
//! kept in a static for a later callback exactly like any other value. The
//! JSON is the runtime's own (names inside are JSON strings, never split), and
//! no `String` is a handle. A handle's exact class (`DirectMethodHandle`,
//! `BoundMethodHandle$Species_LL`, …) is not known, so `getClass` is counted;
//! `toString` is `MethodHandle` + its type, as on Java 25. The handles
//! `jdk_reflect_proxy.rs` makes for `unreflect` and lookups of jar methods are
//! the same direct handles (`direct_handle`).
//!
//! What a handle answers follows the JDK: `type()` is the combinator's type,
//! `invokeExact` with a call-site type other than `type()` throws
//! `WrongMethodTypeException`, `invoke` adapts arguments and the result by
//! boxing and unboxing, and `catchException` hands its handler the throw only
//! when the throw is an instance of the caught class. Lookups of JDK methods
//! answer from [`JDK_LOOKUPS`], which lists only members the JDK really has
//! (checked with `java` 27); any other JDK member is still
//! `NoSuchMethodException`, the answer a caller probing for a converter takes
//! its fallback on.

use super::*;
use serde_json::{Value as Json, json};

/// The class every handle is (the declared type of an `Unknown` kind).
const HANDLE_CLASS: &str = "java/lang/invoke/MethodHandle";
/// Combinators one handle may nest; deeper trees are refused, not walked.
const MAX_HANDLE_DEPTH: usize = 32;
/// Where a lambda spun by a direct `metafactory` call says it came from, in
/// its `@indy` marker.
const METAFACTORY_SITE: &str = "java/lang/invoke/LambdaMetafactory#metafactory";

/// JDK members a lookup finds: (class, name, descriptor, static). `<init>`
/// rows are constructors. Each was checked with `MethodHandles.publicLookup()`
/// on `java` 27; `Character` has neither `valueOf(String)` nor a
/// `(String)` constructor, and `String` has no `valueOf(String)`.
const JDK_LOOKUPS: &[(&str, &str, &str, bool)] = &[
    (
        "java/lang/Byte",
        "valueOf",
        "(Ljava/lang/String;)Ljava/lang/Byte;",
        true,
    ),
    (
        "java/lang/Short",
        "valueOf",
        "(Ljava/lang/String;)Ljava/lang/Short;",
        true,
    ),
    (
        "java/lang/Integer",
        "valueOf",
        "(Ljava/lang/String;)Ljava/lang/Integer;",
        true,
    ),
    (
        "java/lang/Long",
        "valueOf",
        "(Ljava/lang/String;)Ljava/lang/Long;",
        true,
    ),
    (
        "java/lang/Float",
        "valueOf",
        "(Ljava/lang/String;)Ljava/lang/Float;",
        true,
    ),
    (
        "java/lang/Double",
        "valueOf",
        "(Ljava/lang/String;)Ljava/lang/Double;",
        true,
    ),
    (
        "java/lang/Boolean",
        "valueOf",
        "(Ljava/lang/String;)Ljava/lang/Boolean;",
        true,
    ),
    ("java/lang/Byte", "<init>", "(Ljava/lang/String;)V", false),
    ("java/lang/Short", "<init>", "(Ljava/lang/String;)V", false),
    (
        "java/lang/Integer",
        "<init>",
        "(Ljava/lang/String;)V",
        false,
    ),
    ("java/lang/Long", "<init>", "(Ljava/lang/String;)V", false),
    ("java/lang/Float", "<init>", "(Ljava/lang/String;)V", false),
    ("java/lang/Double", "<init>", "(Ljava/lang/String;)V", false),
    (
        "java/lang/Boolean",
        "<init>",
        "(Ljava/lang/String;)V",
        false,
    ),
    ("java/lang/String", "<init>", "(Ljava/lang/String;)V", false),
    (
        "java/math/BigInteger",
        "<init>",
        "(Ljava/lang/String;)V",
        false,
    ),
    (
        "java/math/BigDecimal",
        "<init>",
        "(Ljava/lang/String;)V",
        false,
    ),
];

/// How a direct handle reaches its member.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Static,
    /// Dispatches on the receiver, as `invokevirtual`/`invokeinterface`.
    Virtual,
    /// Runs exactly the named class's body, as `invokespecial`.
    Special,
    Constructor,
}

impl Kind {
    fn name(self) -> &'static str {
        match self {
            Self::Static => "static",
            Self::Virtual => "virtual",
            Self::Special => "special",
            Self::Constructor => "new",
        }
    }

    fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "static" => Self::Static,
            "virtual" => Self::Virtual,
            "special" => Self::Special,
            "new" => Self::Constructor,
            _ => return None,
        })
    }
}

/// One method handle, as a combinator tree.
#[derive(Clone, Debug, PartialEq)]
enum Handle {
    /// A member: `findStatic`/`findVirtual`/`findConstructor`/`unreflect`.
    Direct {
        kind: Kind,
        owner: String,
        name: String,
        descriptor: String,
    },
    /// `insertArguments` (and `bindTo`, at position 0).
    Insert {
        target: Box<Handle>,
        position: usize,
        values: Vec<Value>,
    },
    /// `dropArguments`: parameters the target never sees.
    Drop {
        target: Box<Handle>,
        position: usize,
        types: Vec<String>,
    },
    /// `asType`: the same call under another type, boxing and unboxing.
    AsType {
        target: Box<Handle>,
        descriptor: String,
    },
    /// `filterReturnValue`.
    FilterReturn {
        target: Box<Handle>,
        filter: Box<Handle>,
    },
    /// `catchException`.
    Catch {
        target: Box<Handle>,
        exception: String,
        handler: Box<Handle>,
    },
    /// `invoker`/`exactInvoker`: calls its first argument, a handle of
    /// `descriptor`, with the rest.
    Invoker { descriptor: String, exact: bool },
    /// The target of a `metafactory` call site: given the captured values,
    /// answers a lambda whose single method runs `implementation`.
    LambdaFactory {
        descriptor: String,
        implementation: Box<Handle>,
    },
}

/// A field descriptor's class marker, as `ldc` and `Integer.TYPE` answer it.
fn class_marker(descriptor: &str) -> String {
    match descriptor {
        "Z" => "boolean".into(),
        "B" => "byte".into(),
        "C" => "char".into(),
        "S" => "short".into(),
        "I" => "int".into(),
        "J" => "long".into(),
        "F" => "float".into(),
        "D" => "double".into(),
        "V" => "void".into(),
        array if array.starts_with('[') => array.to_owned(),
        other => other
            .strip_prefix('L')
            .and_then(|name| name.strip_suffix(';'))
            .unwrap_or(other)
            .to_owned(),
    }
}

/// A class marker's field descriptor.
fn field_descriptor(class: &str) -> String {
    match class {
        "boolean" => "Z".into(),
        "byte" => "B".into(),
        "char" => "C".into(),
        "short" => "S".into(),
        "int" => "I".into(),
        "long" => "J".into(),
        "float" => "F".into(),
        "double" => "D".into(),
        "void" => "V".into(),
        array if array.starts_with('[') => array.replace('.', "/"),
        other => format!("L{};", other.replace('.', "/")),
    }
}

fn is_primitive(descriptor: &str) -> bool {
    descriptor.len() == 1 && "ZBCSIJFD".contains(descriptor)
}

/// `(params)ret` from its parts.
fn method_descriptor(parameters: &[String], result: &str) -> String {
    format!("({}){result}", parameters.concat())
}

fn method_type_value(descriptor: &str) -> Value {
    host_invoke::method_type(descriptor)
}

/// The descriptor of a `MethodType` handle.
fn method_type_of(value: &Value) -> Option<String> {
    host_invoke::method_type_of(value).map(str::to_owned)
}

/// The JVM's text for a method type, `(String,int)Integer`, as
/// `MethodType.toString` and `WrongMethodTypeException` print it
/// (`host_invoke::method_type_text`). A jar's member class, whose simple name
/// the runtime does not read, keeps its binary name's last segment.
fn method_type_text(descriptor: &str) -> String {
    host_invoke::method_type_text(descriptor).unwrap_or_else(|| {
        let simple = |field: &str| {
            let class = class_marker(field);
            class.rsplit('/').next().unwrap_or(&class).to_owned()
        };
        let parameters: Vec<String> = descriptor_arguments(descriptor)
            .iter()
            .map(|field| simple(field))
            .collect();
        format!(
            "({}){}",
            parameters.join(","),
            simple(return_desc(descriptor))
        )
    })
}

fn wrong_method_type(message: String) -> NativeError {
    NativeError::UncaughtException {
        class: "java/lang/invoke/WrongMethodTypeException".into(),
        message: Some(message),
        frame: None,
    }
}

fn npe() -> NativeError {
    NativeError::UncaughtException {
        class: "java/lang/NullPointerException".into(),
        message: None,
        frame: None,
    }
}

fn malformed(what: &str) -> NativeError {
    NativeError::Metadata(format!("malformed method handle: {what}"))
}

impl Handle {
    /// The handle's `MethodType`, as a method descriptor.
    fn descriptor(&self) -> Result<String, NativeError> {
        Ok(match self {
            Self::Direct {
                kind,
                owner,
                descriptor,
                ..
            } => {
                let parameters = descriptor_arguments(descriptor);
                match kind {
                    Kind::Static => descriptor.clone(),
                    Kind::Virtual | Kind::Special => {
                        let mut with_receiver = vec![field_descriptor(owner)];
                        with_receiver.extend(parameters);
                        method_descriptor(&with_receiver, return_desc(descriptor))
                    }
                    Kind::Constructor => method_descriptor(&parameters, &field_descriptor(owner)),
                }
            }
            Self::Insert {
                target,
                position,
                values,
            } => {
                let inner = target.descriptor()?;
                let mut parameters = descriptor_arguments(&inner);
                let end = position + values.len();
                if end > parameters.len() {
                    return Err(malformed("insertArguments past the parameters"));
                }
                parameters.drain(*position..end);
                method_descriptor(&parameters, return_desc(&inner))
            }
            Self::Drop {
                target,
                position,
                types,
            } => {
                let inner = target.descriptor()?;
                let mut parameters = descriptor_arguments(&inner);
                if *position > parameters.len() {
                    return Err(malformed("dropArguments past the parameters"));
                }
                for (offset, field) in types.iter().enumerate() {
                    parameters.insert(position + offset, field.clone());
                }
                method_descriptor(&parameters, return_desc(&inner))
            }
            Self::AsType { descriptor, .. } => descriptor.clone(),
            Self::FilterReturn { target, filter } => {
                let inner = target.descriptor()?;
                method_descriptor(
                    &descriptor_arguments(&inner),
                    return_desc(&filter.descriptor()?),
                )
            }
            Self::Catch { target, .. } => target.descriptor()?,
            Self::Invoker { descriptor, .. } => {
                let mut parameters = vec![field_descriptor(HANDLE_CLASS)];
                parameters.extend(descriptor_arguments(descriptor));
                method_descriptor(&parameters, return_desc(descriptor))
            }
            Self::LambdaFactory { descriptor, .. } => descriptor.clone(),
        })
    }

    fn to_json(&self) -> Json {
        match self {
            Self::Direct {
                kind,
                owner,
                name,
                descriptor,
            } => {
                json!({"k": "direct", "kind": kind.name(), "owner": owner, "name": name, "desc": descriptor})
            }
            Self::Insert {
                target,
                position,
                values,
            } => json!({
                "k": "insert",
                "target": target.to_json(),
                "pos": position,
                "values": values.iter().map(collection_item_marker).collect::<Vec<_>>(),
            }),
            Self::Drop {
                target,
                position,
                types,
            } => {
                json!({"k": "drop", "target": target.to_json(), "pos": position, "types": types})
            }
            Self::AsType { target, descriptor } => {
                json!({"k": "astype", "target": target.to_json(), "desc": descriptor})
            }
            Self::FilterReturn { target, filter } => {
                json!({"k": "filter", "target": target.to_json(), "filter": filter.to_json()})
            }
            Self::Catch {
                target,
                exception,
                handler,
            } => json!({
                "k": "catch",
                "target": target.to_json(),
                "exception": exception,
                "handler": handler.to_json(),
            }),
            Self::Invoker { descriptor, exact } => {
                json!({"k": "invoker", "desc": descriptor, "exact": exact})
            }
            Self::LambdaFactory {
                descriptor,
                implementation,
            } => json!({"k": "lambda", "desc": descriptor, "impl": implementation.to_json()}),
        }
    }

    fn from_json(json: &Json, depth: usize) -> Result<Self, NativeError> {
        if depth > MAX_HANDLE_DEPTH {
            return Err(malformed("nested past the combinator bound"));
        }
        let text = |key: &str| {
            json.get(key)
                .and_then(Json::as_str)
                .map(str::to_owned)
                .ok_or_else(|| malformed(key))
        };
        let index = |key: &str| {
            json.get(key)
                .and_then(Json::as_u64)
                .map(|value| value as usize)
                .ok_or_else(|| malformed(key))
        };
        let child = |key: &str| {
            json.get(key)
                .ok_or_else(|| malformed(key))
                .and_then(|inner| Self::from_json(inner, depth + 1))
                .map(Box::new)
        };
        let strings = |key: &str| -> Result<Vec<String>, NativeError> {
            json.get(key)
                .and_then(Json::as_array)
                .ok_or_else(|| malformed(key))?
                .iter()
                .map(|item| {
                    item.as_str()
                        .map(str::to_owned)
                        .ok_or_else(|| malformed(key))
                })
                .collect()
        };
        Ok(match json.get("k").and_then(Json::as_str) {
            Some("direct") => Self::Direct {
                kind: Kind::parse(&text("kind")?).ok_or_else(|| malformed("kind"))?,
                owner: text("owner")?,
                name: text("name")?,
                descriptor: text("desc")?,
            },
            Some("insert") => Self::Insert {
                target: child("target")?,
                position: index("pos")?,
                values: strings("values")?
                    .iter()
                    .map(|marker| parse_collection_value(marker))
                    .collect::<Result<_, _>>()?,
            },
            Some("drop") => Self::Drop {
                target: child("target")?,
                position: index("pos")?,
                types: strings("types")?,
            },
            Some("astype") => Self::AsType {
                target: child("target")?,
                descriptor: text("desc")?,
            },
            Some("filter") => Self::FilterReturn {
                target: child("target")?,
                filter: child("filter")?,
            },
            Some("catch") => Self::Catch {
                target: child("target")?,
                exception: text("exception")?,
                handler: child("handler")?,
            },
            Some("invoker") => Self::Invoker {
                descriptor: text("desc")?,
                exact: json.get("exact").and_then(Json::as_bool).unwrap_or(false),
            },
            Some("lambda") => Self::LambdaFactory {
                descriptor: text("desc")?,
                implementation: child("impl")?,
            },
            _ => return Err(malformed("unknown combinator")),
        })
    }

    fn value(&self) -> Value {
        Value::Host(HostRef::new(
            HostKind::MethodHandle,
            self.to_json().to_string(),
        ))
    }
}

/// The JSON of a handle of `kind`.
fn handle_json(value: &Value, kind: HostKind) -> Option<&str> {
    match value {
        Value::Host(handle) if handle.kind() == kind => Some(handle.key()),
        _ => None,
    }
}

fn parse_handle(json: &str) -> Result<Handle, NativeError> {
    let json: Json = serde_json::from_str(json).map_err(|error| malformed(&error.to_string()))?;
    Handle::from_json(&json, 0)
}

/// A direct handle on a member, for `jdk_reflect_proxy.rs`'s `unreflect`,
/// `findStatic`, `findVirtual` and `findConstructor` over jar classes: `kind`
/// is `static`, `virtual`, `special` or `new`.
pub(super) fn direct_handle(
    kind: &str,
    owner: &str,
    name: &str,
    descriptor: &str,
) -> Option<Value> {
    Some(
        Handle::Direct {
            kind: Kind::parse(kind)?,
            owner: owner.to_owned(),
            name: name.to_owned(),
            descriptor: descriptor.to_owned(),
        }
        .value(),
    )
}

/// `Lookup.unreflectSpecial(method, specialCaller)`: a special handle on
/// `owner`'s body whose receiver is typed as `caller` (Java 25:
/// `MethodHandle(Impl)String` for an interface default unreflected for
/// `Impl`). The receiver passes through unchanged, so a reference `asType`
/// over the direct handle is exactly that.
pub(super) fn special_handle(owner: &str, name: &str, descriptor: &str, caller: &str) -> Value {
    let direct = Handle::Direct {
        kind: Kind::Special,
        owner: owner.to_owned(),
        name: name.to_owned(),
        descriptor: descriptor.to_owned(),
    };
    if caller == owner {
        return direct.value();
    }
    let mut parameters = vec![field_descriptor(caller)];
    parameters.extend(descriptor_arguments(descriptor));
    Handle::AsType {
        target: Box::new(direct),
        descriptor: method_descriptor(&parameters, return_desc(descriptor)),
    }
    .value()
}

/// The types a `MethodHandle` surely is.
pub(crate) fn handle_types(_: &str) -> &'static [&'static str] {
    &[HANDLE_CLASS]
}

/// A metafactory call site's class: Java 25 builds a plain `ConstantCallSite`.
pub(crate) fn call_site_class(_: &str) -> &str {
    "java/lang/invoke/ConstantCallSite"
}

/// `Object.equals` on an object without its own: identity, known for the
/// same handle and for different ones (different keys); counted for two
/// equal keys from different constructions.
fn identity_equals(
    handle: &HostRef,
    other: &Value,
    _: &mut dyn host_ref::Nested,
) -> Result<Option<bool>, NativeError> {
    Ok(match other {
        Value::Host(other) => handle.same_object(other),
        _ => Some(false),
    })
}

/// `MethodHandle.toString()`: `MethodHandle` + its type (`MethodHandle(int)String`).
fn handle_text(
    handle: &HostRef,
    _: &mut dyn host_ref::Nested,
) -> Result<Option<String>, NativeError> {
    let Ok(descriptor) = parse_handle(handle.key()).and_then(|tree| tree.descriptor()) else {
        return Ok(None);
    };
    Ok(host_invoke::method_type_text(&descriptor).map(|text| format!("MethodHandle{text}")))
}

pub(crate) const HANDLE_METHODS: host_ref::Methods = host_ref::Methods {
    to_string: handle_text,
    // Identity hashes.
    hash_code: |_, _| Ok(None),
    equals: identity_equals,
    instance_of: host_ref::instance_of_class,
    crosses_jars: |_, _| false,
    compare_to: None,
};

pub(crate) const CALL_SITE_METHODS: host_ref::Methods = host_ref::Methods {
    // `Object.toString`: the identity hash.
    to_string: |_, _| Ok(None),
    hash_code: |_, _| Ok(None),
    equals: identity_equals,
    instance_of: host_ref::instance_of_class,
    crosses_jars: |_, _| false,
    compare_to: None,
};

impl<H: Host> Runtime<'_, H> {
    /// The method-handle calls this module answers, or `None` to leave the
    /// call to the rest of the dispatch chain.
    pub(super) fn invoke_method_handle_model(
        &mut self,
        r: &MethodRef,
        receiver: Option<&Value>,
        args: &[Value],
    ) -> Result<Option<Value>, NativeError> {
        match (r.owner.as_str(), r.name.as_str()) {
            ("java/lang/invoke/MethodHandles$Lookup", "findStatic" | "findVirtual") => {
                Ok(self.find_jdk_member(r.name == "findStatic", args)?)
            }
            ("java/lang/invoke/MethodHandles$Lookup", "findConstructor") => {
                Ok(self.find_jdk_constructor(args)?)
            }
            ("java/lang/invoke/MethodHandles", _) => self.method_handles_static(r, args),
            ("java/lang/invoke/LambdaMetafactory", "metafactory") => self.metafactory(args),
            (
                "java/lang/invoke/CallSite"
                | "java/lang/invoke/ConstantCallSite"
                | "java/lang/invoke/MutableCallSite"
                | "java/lang/invoke/VolatileCallSite",
                "dynamicInvoker" | "getTarget",
            ) => {
                // The call site's target: its key is the target handle's.
                Ok(receiver
                    .and_then(|receiver| handle_json(receiver, HostKind::CallSite))
                    .map(|json| Value::Host(HostRef::new(HostKind::MethodHandle, json))))
            }
            ("java/lang/invoke/MethodType", _) if let Some(receiver) = receiver => {
                Ok(self.method_type_call(r, receiver, args))
            }
            (HANDLE_CLASS, _) => self.handle_call(r, receiver, args),
            _ => Ok(None),
        }
    }

    /// The handle a `MethodHandle` handle names.
    fn handle_of(&self, value: &Value) -> Result<Option<Handle>, NativeError> {
        handle_json(value, HostKind::MethodHandle)
            .map(parse_handle)
            .transpose()
    }

    /// The argument at `index` as a handle, or the JDK's NPE for null.
    fn handle_arg(&self, args: &[Value], index: usize) -> Result<Option<Handle>, NativeError> {
        match args.get(index) {
            Some(Value::Ref(None)) | None => Err(npe()),
            Some(value) => self.handle_of(value),
        }
    }

    /// `findStatic`/`findVirtual` on a JDK class: a direct handle when the
    /// JDK has the member, `NoSuchMethodException` when it does not. A jar
    /// class is left to `jdk_reflect_proxy.rs`, which reads its class file.
    fn find_jdk_member(
        &mut self,
        wanted_static: bool,
        args: &[Value],
    ) -> Result<Option<Value>, NativeError> {
        let (Some(class), Some(name)) = (
            args.first().and_then(crate::runtime::host_class::class_name),
            args.get(1).and_then(expect_ref),
        ) else {
            return Ok(None);
        };
        let class = class.replace('.', "/");
        if self.has_class(&class) {
            return Ok(None);
        }
        let Some(descriptor) = args.get(2).and_then(method_type_of) else {
            return Ok(None);
        };
        let known = JDK_LOOKUPS
            .iter()
            .any(|(owner, member, member_descriptor, is_static)| {
                *owner == class
                    && *member == name
                    && *member_descriptor == descriptor
                    && *is_static == wanted_static
            });
        if !known {
            // `jdk_reflect_proxy.rs` answers the JDK's `NoSuchMethodException`.
            return Ok(None);
        }
        let kind = if wanted_static {
            Kind::Static
        } else {
            Kind::Virtual
        };
        Ok(Some(
            Handle::Direct {
                kind,
                owner: class,
                name: name.clone(),
                descriptor,
            }
            .value(),
        ))
    }

    fn find_jdk_constructor(&mut self, args: &[Value]) -> Result<Option<Value>, NativeError> {
        let Some(class) = args.first().and_then(crate::runtime::host_class::class_name) else {
            return Ok(None);
        };
        let class = class.replace('.', "/");
        if self.has_class(&class) {
            return Ok(None);
        }
        let Some(descriptor) = args.get(1).and_then(method_type_of) else {
            return Ok(None);
        };
        let known = JDK_LOOKUPS
            .iter()
            .any(|(owner, member, member_descriptor, _)| {
                *owner == class && *member == "<init>" && *member_descriptor == descriptor
            });
        Ok(known.then(|| {
            Handle::Direct {
                kind: Kind::Constructor,
                owner: class,
                name: "<init>".into(),
                descriptor,
            }
            .value()
        }))
    }

    /// The `MethodHandles` combinators and invokers.
    fn method_handles_static(
        &mut self,
        r: &MethodRef,
        args: &[Value],
    ) -> Result<Option<Value>, NativeError> {
        let handle = match (r.name.as_str(), r.descriptor.as_str()) {
            (
                "dropArguments",
                "(Ljava/lang/invoke/MethodHandle;I[Ljava/lang/Class;)Ljava/lang/invoke/MethodHandle;"
                | "(Ljava/lang/invoke/MethodHandle;ILjava/util/List;)Ljava/lang/invoke/MethodHandle;",
            ) => {
                let Some(target) = self.handle_arg(args, 0)? else {
                    return Ok(None);
                };
                let Some(Value::Int(position)) = args.get(1) else {
                    return Ok(None);
                };
                let classes = match args.get(2) {
                    Some(value) if self.owned_array(value).is_some() => self.array(value)?.clone(),
                    Some(value @ Value::Ref(Some(_))) => {
                        runtime_collection_values(self.host, value)?
                    }
                    // A shared empty array: what the unmodelled call returned.
                    None | Some(Value::Host(_)) => return Ok(None),
                    Some(_) => return Err(npe()),
                };
                let types = classes
                    .iter()
                    .map(|class| expect_ref(class).map(|class| field_descriptor(class)))
                    .collect::<Option<Vec<_>>>()
                    .ok_or_else(npe)?;
                let parameters = descriptor_arguments(&target.descriptor()?);
                let position = usize::try_from(*position)
                    .ok()
                    .filter(|position| *position <= parameters.len())
                    .ok_or_else(|| NativeError::UncaughtException {
                        class: "java/lang/IllegalArgumentException".into(),
                        message: Some(format!("bad argument count {position}")),
                        frame: None,
                    })?;
                // Nothing to drop: Java answers the target itself.
                if types.is_empty() {
                    return Ok(args.first().cloned());
                }
                Handle::Drop {
                    target: Box::new(target),
                    position,
                    types,
                }
            }
            (
                "insertArguments",
                "(Ljava/lang/invoke/MethodHandle;I[Ljava/lang/Object;)Ljava/lang/invoke/MethodHandle;",
            ) => {
                let Some(target) = self.handle_arg(args, 0)? else {
                    return Ok(None);
                };
                let Some(Value::Int(position)) = args.get(1) else {
                    return Ok(None);
                };
                let values = match args.get(2) {
                    Some(value) if host_array::array_key(value).is_some() => {
                        self.array(value)?.clone()
                    }
                    _ => Vec::new(),
                };
                let parameters = descriptor_arguments(&target.descriptor()?);
                let position = usize::try_from(*position)
                    .ok()
                    .filter(|position| position + values.len() <= parameters.len())
                    .ok_or_else(|| NativeError::UncaughtException {
                        class: "java/lang/IllegalArgumentException".into(),
                        message: Some("too many values to insert".into()),
                        frame: None,
                    })?;
                // Nothing to insert: Java answers the target itself.
                if values.is_empty() {
                    return Ok(args.first().cloned());
                }
                Handle::Insert {
                    target: Box::new(target),
                    position,
                    values,
                }
            }
            (
                "filterReturnValue",
                "(Ljava/lang/invoke/MethodHandle;Ljava/lang/invoke/MethodHandle;)Ljava/lang/invoke/MethodHandle;",
            ) => {
                let (Some(target), Some(filter)) =
                    (self.handle_arg(args, 0)?, self.handle_arg(args, 1)?)
                else {
                    return Ok(None);
                };
                // The filter takes the target's result: none for a void
                // target, otherwise exactly that type.
                let result = return_desc(&target.descriptor()?).to_owned();
                let filter_parameters = descriptor_arguments(&filter.descriptor()?);
                let fits = if result == "V" {
                    filter_parameters.is_empty()
                } else {
                    filter_parameters == [result.clone()]
                };
                if !fits {
                    return Err(NativeError::UncaughtException {
                        class: "java/lang/IllegalArgumentException".into(),
                        message: Some(format!(
                            "target and filter types do not match: {}, {}",
                            method_type_text(&target.descriptor()?),
                            method_type_text(&filter.descriptor()?)
                        )),
                        frame: None,
                    });
                }
                Handle::FilterReturn {
                    target: Box::new(target),
                    filter: Box::new(filter),
                }
            }
            (
                "catchException",
                "(Ljava/lang/invoke/MethodHandle;Ljava/lang/Class;Ljava/lang/invoke/MethodHandle;)Ljava/lang/invoke/MethodHandle;",
            ) => {
                let (Some(target), Some(handler)) =
                    (self.handle_arg(args, 0)?, self.handle_arg(args, 2)?)
                else {
                    return Ok(None);
                };
                let Some(exception) = args.get(1).and_then(crate::runtime::host_class::class_name) else {
                    return Err(npe());
                };
                Handle::Catch {
                    target: Box::new(target),
                    exception: exception.replace('.', "/"),
                    handler: Box::new(handler),
                }
            }
            (
                "invoker" | "exactInvoker",
                "(Ljava/lang/invoke/MethodType;)Ljava/lang/invoke/MethodHandle;",
            ) => {
                let Some(descriptor) = args.first().and_then(method_type_of) else {
                    return Err(npe());
                };
                Handle::Invoker {
                    descriptor,
                    exact: r.name == "exactInvoker",
                }
            }
            _ => return Ok(None),
        };
        Ok(Some(handle.value()))
    }

    /// `LambdaMetafactory.metafactory(lookup, name, factoryType, samType,
    /// implementation, instantiatedType)`: a constant call site whose target
    /// spins the lambda.
    fn metafactory(&mut self, args: &[Value]) -> Result<Option<Value>, NativeError> {
        let Some(factory) = args.get(2).and_then(method_type_of) else {
            return Ok(None);
        };
        let Some(implementation) = self.handle_arg(args, 4)? else {
            return Ok(None);
        };
        let target = Handle::LambdaFactory {
            descriptor: factory,
            implementation: Box::new(implementation),
        };
        // The call site keeps the target handle's key, so `dynamicInvoker`/
        // `getTarget` hand an equal handle back.
        Ok(Some(Value::Host(HostRef::new(
            HostKind::CallSite,
            target.to_json().to_string(),
        ))))
    }

    fn method_type_call(
        &mut self,
        r: &MethodRef,
        receiver: &Value,
        args: &[Value],
    ) -> Option<Value> {
        let descriptor = method_type_of(receiver)?;
        let parameters = descriptor_arguments(&descriptor);
        Some(match (r.name.as_str(), r.descriptor.as_str()) {
            ("parameterType", "(I)Ljava/lang/Class;") => {
                let Some(Value::Int(index)) = args.first() else {
                    return None;
                };
                let field = usize::try_from(*index)
                    .ok()
                    .and_then(|index| parameters.get(index))?;
                Value::Ref(Some(class_marker(field)))
            }
            ("returnType", "()Ljava/lang/Class;") => {
                Value::Ref(Some(class_marker(return_desc(&descriptor))))
            }
            ("parameterCount", "()I") => Value::Int(parameters.len() as i32),
            ("toMethodDescriptorString", "()Ljava/lang/String;") => {
                Value::Ref(Some(descriptor.clone()))
            }
            ("changeReturnType", "(Ljava/lang/Class;)Ljava/lang/invoke/MethodType;") => {
                let class = args.first().and_then(crate::runtime::host_class::class_name)?;
                method_type_value(&method_descriptor(&parameters, &field_descriptor(class)))
            }
            _ => return None,
        })
    }

    /// Calls on a handle: `type`, `invoke`, `invokeExact`, `asType`, and, for
    /// this module's handles, `bindTo` and `invokeWithArguments` (the legacy
    /// markers keep `jdk_reflect_proxy.rs`'s answers for those).
    fn handle_call(
        &mut self,
        r: &MethodRef,
        receiver: Option<&Value>,
        args: &[Value],
    ) -> Result<Option<Value>, NativeError> {
        let Some(receiver) = receiver else {
            return Ok(None);
        };
        let Some(handle) = self.handle_of(receiver)? else {
            return Ok(None);
        };
        let descriptor = handle.descriptor()?;
        match (r.name.as_str(), r.descriptor.as_str()) {
            ("type", "()Ljava/lang/invoke/MethodType;") => Ok(Some(method_type_value(&descriptor))),
            ("asType", "(Ljava/lang/invoke/MethodType;)Ljava/lang/invoke/MethodHandle;") => {
                let Some(wanted) = args.first().and_then(method_type_of) else {
                    return Err(npe());
                };
                if arg_count(&wanted) != arg_count(&descriptor) {
                    return Err(wrong_method_type(format!(
                        "cannot convert MethodHandle{} to {}",
                        method_type_text(&descriptor),
                        method_type_text(&wanted)
                    )));
                }
                Ok(Some(if wanted == descriptor {
                    receiver.clone()
                } else {
                    Handle::AsType {
                        target: Box::new(handle),
                        descriptor: wanted,
                    }
                    .value()
                }))
            }
            ("bindTo", "(Ljava/lang/Object;)Ljava/lang/invoke/MethodHandle;") => {
                let parameters = descriptor_arguments(&descriptor);
                if parameters.first().is_none_or(|first| is_primitive(first)) {
                    return Err(NativeError::UncaughtException {
                        class: "java/lang/IllegalArgumentException".into(),
                        message: Some(format!(
                            "no leading reference parameter: {}",
                            args.first().map(value_text).unwrap_or_default()
                        )),
                        frame: None,
                    });
                }
                Ok(Some(
                    Handle::Insert {
                        target: Box::new(handle),
                        position: 0,
                        values: vec![args.first().cloned().unwrap_or(Value::Ref(None))],
                    }
                    .value(),
                ))
            }
            ("invokeWithArguments", "([Ljava/lang/Object;)Ljava/lang/Object;") => {
                let values = match args.first() {
                    Some(value) if host_array::array_key(value).is_some() => {
                        self.array(value)?.clone()
                    }
                    _ => Vec::new(),
                };
                let call_site = method_descriptor(
                    &vec!["Ljava/lang/Object;".to_owned(); values.len()],
                    "Ljava/lang/Object;",
                );
                self.invoke_adapted(&handle, &values, &call_site).map(Some)
            }
            ("invokeExact", call_site) => {
                if call_site != descriptor {
                    return Err(wrong_method_type(format!(
                        "handle's method type {} but found {}",
                        method_type_text(&descriptor),
                        method_type_text(call_site)
                    )));
                }
                self.invoke_handle_tree(&handle, args, 0).map(Some)
            }
            ("invoke", call_site) => self.invoke_adapted(&handle, args, call_site).map(Some),
            _ => Ok(None),
        }
    }

    /// `MethodHandle.invoke`: the arguments of `call_site` adapted to the
    /// handle's type, and its result adapted back.
    fn invoke_adapted(
        &mut self,
        handle: &Handle,
        args: &[Value],
        call_site: &str,
    ) -> Result<Value, NativeError> {
        let descriptor = handle.descriptor()?;
        let from = descriptor_arguments(call_site);
        let to = descriptor_arguments(&descriptor);
        if from.len() != to.len() || args.len() != to.len() {
            return Err(wrong_method_type(format!(
                "cannot convert MethodHandle{} to {}",
                method_type_text(&descriptor),
                method_type_text(call_site)
            )));
        }
        let mut adapted = Vec::with_capacity(args.len());
        for ((value, from), to) in args.iter().zip(&from).zip(&to) {
            adapted.push(self.convert(value, from, to)?);
        }
        let result = self.invoke_handle_tree(handle, &adapted, 0)?;
        self.convert(&result, return_desc(&descriptor), return_desc(call_site))
    }

    /// `asType`'s conversion of one value from type `from` to type `to`:
    /// boxing, unboxing, primitive widening; a reference keeps its marker
    /// (the cast is the caller's `checkcast`).
    fn convert(&mut self, value: &Value, from: &str, to: &str) -> Result<Value, NativeError> {
        if to == "V" {
            return Ok(Value::Void);
        }
        if from == "V" {
            return Ok(Value::default_for(&format!("(){to}")));
        }
        match (is_primitive(from), is_primitive(to)) {
            (true, false) => self.box_primitive(from, value),
            (false, true) => {
                if matches!(value, Value::Ref(None)) {
                    return Err(npe());
                }
                Ok(
                    coerce_arguments(&format!("({to})V"), std::slice::from_ref(value))
                        .pop()
                        .unwrap_or_else(|| value.clone()),
                )
            }
            (true, true) => Ok(match (value, to) {
                (Value::Int(int), "J") => Value::Long(i64::from(*int)),
                (Value::Int(int), "F" | "D") => Value::Float(f64::from(*int)),
                (Value::Long(long), "F" | "D") => Value::Float(*long as f64),
                (other, _) => other.clone(),
            }),
            (false, false) => Ok(value.clone()),
        }
    }

    /// Run a handle on arguments of exactly its type; the result is of its
    /// type's return.
    fn invoke_handle_tree(
        &mut self,
        handle: &Handle,
        args: &[Value],
        depth: usize,
    ) -> Result<Value, NativeError> {
        if depth > MAX_HANDLE_DEPTH {
            return Err(malformed("nested past the combinator bound"));
        }
        let expected = arg_count(&handle.descriptor()?);
        if args.len() != expected {
            return Err(wrong_method_type(format!(
                "{} takes {expected} arguments, got {}",
                method_type_text(&handle.descriptor()?),
                args.len()
            )));
        }
        match handle {
            Handle::Direct {
                kind,
                owner,
                name,
                descriptor,
            } => self.invoke_direct(*kind, owner, name, descriptor, args),
            Handle::Insert {
                target,
                position,
                values,
            } => {
                let mut full = args.to_vec();
                for (offset, value) in values.iter().enumerate() {
                    full.insert(position + offset, value.clone());
                }
                self.invoke_handle_tree(target, &full, depth + 1)
            }
            Handle::Drop {
                target,
                position,
                types,
            } => {
                let mut kept = args.to_vec();
                kept.drain(*position..position + types.len());
                self.invoke_handle_tree(target, &kept, depth + 1)
            }
            Handle::AsType { target, descriptor } => {
                let inner = target.descriptor()?;
                let from = descriptor_arguments(descriptor);
                let to = descriptor_arguments(&inner);
                let mut adapted = Vec::with_capacity(args.len());
                for ((value, from), to) in args.iter().zip(&from).zip(&to) {
                    adapted.push(self.convert(value, from, to)?);
                }
                let result = self.invoke_handle_tree(target, &adapted, depth + 1)?;
                self.convert(&result, return_desc(&inner), return_desc(descriptor))
            }
            Handle::FilterReturn { target, filter } => {
                let result = self.invoke_handle_tree(target, args, depth + 1)?;
                if matches!(result, Value::Void) {
                    self.invoke_handle_tree(filter, &[], depth + 1)
                } else {
                    self.invoke_handle_tree(filter, &[result], depth + 1)
                }
            }
            Handle::Catch {
                target,
                exception,
                handler,
            } => match self.invoke_handle_tree(target, args, depth + 1) {
                Err(NativeError::UncaughtException {
                    class,
                    message,
                    frame,
                }) => {
                    if !exception_is_subtype(self.classes, &self.linked, &class, exception) {
                        return Err(NativeError::UncaughtException {
                            class,
                            message,
                            frame,
                        });
                    }
                    // The handler takes the throw, then a prefix of the
                    // target's arguments.
                    let thrown = self.caught_throwable(&class, message, frame.as_deref());
                    let takes = arg_count(&handler.descriptor()?).saturating_sub(1);
                    let mut values = vec![thrown];
                    values.extend(args.iter().take(takes).cloned());
                    self.invoke_handle_tree(handler, &values, depth + 1)
                }
                other => other,
            },
            Handle::Invoker { descriptor, exact } => {
                let Some((first, rest)) = args.split_first() else {
                    return Err(npe());
                };
                if matches!(first, Value::Ref(None)) {
                    return Err(npe());
                }
                let Some(inner) = self.handle_of(first)? else {
                    return Err(malformed("invoker's first argument is not a handle"));
                };
                if *exact {
                    let inner_type = inner.descriptor()?;
                    if &inner_type != descriptor {
                        return Err(wrong_method_type(format!(
                            "handle's method type {} but found {}",
                            method_type_text(&inner_type),
                            method_type_text(descriptor)
                        )));
                    }
                    self.invoke_handle_tree(&inner, rest, depth + 1)
                } else {
                    self.invoke_adapted(&inner, rest, descriptor)
                }
            }
            Handle::LambdaFactory {
                descriptor: _,
                implementation,
            } => Ok(self.spin_lambda(implementation, args)?),
        }
    }

    /// The lambda a metafactory call site answers for `captured`: the same
    /// `@indy` callback object an `invokedynamic` site makes, so calling its
    /// single method goes through the interpreter's callback path. A direct
    /// implementation is its target; any other handle is reached through
    /// `MethodHandle.invokeExact` on the handle itself, captured first.
    fn spin_lambda(
        &mut self,
        implementation: &Handle,
        captured: &[Value],
    ) -> Result<Value, NativeError> {
        let (target, captures) = match implementation {
            Handle::Direct {
                owner,
                name,
                descriptor,
                ..
            } => (format!("{owner}#{name}{descriptor}"), captured.to_vec()),
            other => {
                let mut captures = vec![other.value()];
                captures.extend_from_slice(captured);
                (
                    format!("{HANDLE_CLASS}#invokeExact{}", other.descriptor()?),
                    captures,
                )
            }
        };
        Ok(Value::Ref(Some(format!(
            "@indy:{METAFACTORY_SITE}:0:{target}:{}",
            bounded_captured_markers(&captures)
        ))))
    }

    /// Run one member on arguments of the handle's type (the receiver first
    /// for an instance member).
    fn invoke_direct(
        &mut self,
        kind: Kind,
        owner: &str,
        name: &str,
        descriptor: &str,
        args: &[Value],
    ) -> Result<Value, NativeError> {
        let member = MethodRef {
            owner: owner.to_owned(),
            name: name.to_owned(),
            descriptor: descriptor.to_owned(),
        };
        let jar_owned = self.has_class(owner);
        match kind {
            Kind::Constructor => {
                let values = coerce_arguments(descriptor, args);
                let object = self.new_object(owner);
                if jar_owned {
                    self.invoke(owner, "<init>", descriptor, Some(object.clone()), &values)?;
                } else if string_constructors::count(self, &member)?.is_some() {
                    // `String::new`: counted like `invokespecial` (`string_constructors.rs`).
                } else {
                    self.host.invoke_instance(&member, &object, &values)?;
                }
                Ok(object)
            }
            Kind::Static => {
                let values = coerce_arguments(descriptor, args);
                if jar_owned {
                    return self.invoke(owner, name, descriptor, None, &values);
                }
                if let Some(value) =
                    self.invoke_library_model(&member, None, &values, "method-handle", name)?
                {
                    return Ok(value);
                }
                self.host.invoke(&member, &values)
            }
            Kind::Virtual | Kind::Special => {
                let Some((receiver, rest)) = args.split_first() else {
                    return Err(npe());
                };
                if matches!(receiver, Value::Ref(None)) {
                    return Err(npe());
                }
                let values = coerce_arguments(descriptor, rest);
                // A virtual call on a `Proxy` goes to its invocation handler,
                // as the JVM routes it (`jdk_reflect_proxy.rs`).
                if kind == Kind::Virtual
                    && let Some(proxy) =
                        expect_ref(receiver).filter(|marker| jdk_reflect_proxy::is_proxy(marker))
                {
                    let proxy = proxy.clone();
                    let answer = self.invoke_on_proxy(&proxy, &member, &values)?;
                    return Ok(answer);
                }
                // A virtual handle selects the receiver's override, as
                // `invokevirtual` would; a special one runs `owner`'s body.
                let receiver_class = crate::runtime::jar_object::object_class(receiver).map(str::to_owned);
                let resolved = match (kind, receiver_class) {
                    (Kind::Virtual, Some(class)) => {
                        match self.member_owner(&class, name, descriptor) {
                            Some((resolved, true)) => Some(resolved),
                            _ => None,
                        }
                    }
                    _ => None,
                };
                if let Some(resolved) = resolved {
                    return self.invoke(
                        &resolved,
                        name,
                        descriptor,
                        Some(receiver.clone()),
                        &values,
                    );
                }
                if jar_owned {
                    return self.invoke(owner, name, descriptor, Some(receiver.clone()), &values);
                }
                if let Some(value) = self.invoke_library_model(
                    &member,
                    Some(receiver),
                    &values,
                    "method-handle",
                    name,
                )? {
                    return Ok(value);
                }
                self.host.invoke_instance(&member, receiver, &values)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handle_markers_round_trip_with_bound_values() {
        let handle = Handle::Catch {
            target: Box::new(Handle::Insert {
                target: Box::new(Handle::Direct {
                    kind: Kind::Static,
                    owner: "demo/A".into(),
                    name: "f".into(),
                    descriptor: "(Ljava/lang/Object;I)Ljava/lang/String;".into(),
                }),
                position: 0,
                values: vec![Value::Ref(Some("demo/Thing#3,x".into()))],
            }),
            exception: "java/lang/Throwable".into(),
            handler: Box::new(Handle::Invoker {
                descriptor: "(Ljava/lang/Throwable;)Ljava/lang/String;".into(),
                exact: true,
            }),
        };
        let value = handle.value();
        let json = handle_json(&value, HostKind::MethodHandle).expect("a handle is a MethodHandle");
        let parsed = parse_handle(json).unwrap();
        assert_eq!(parsed, handle);
        assert_eq!(parsed.descriptor().unwrap(), "(I)Ljava/lang/String;");
    }

    /// A member name is jar text (`findStatic(cls, name, type)`): it goes in
    /// a handle's key as a JSON string and reads back as itself, whatever it
    /// holds, and distinct names never share a key (#156: keys never parse
    /// jar text).
    #[test]
    fn names_with_every_separator_round_trip_as_their_own_names() {
        const ALPHABET: &[&str] = &[
            "\"", "\\", "{", "}", "[", "]", ",", ":", "#", "(", ")", ";", "/", "mh", "k", "direct",
            "null", "é", "😀", "\u{0}", "\n", "a",
        ];
        let mut state: u64 = 0x2545_f491_4f6c_dd1d;
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        let mut seen = std::collections::BTreeMap::new();
        for _ in 0..3000 {
            let length = next() % 8;
            let name: String = (0..length)
                .map(|_| ALPHABET[(next() % ALPHABET.len() as u64) as usize])
                .collect();
            for kind in ["static", "virtual", "special", "new"] {
                let value = direct_handle(kind, "demo/Owner", &name, "(I)V").unwrap();
                let json = handle_json(&value, HostKind::MethodHandle).unwrap();
                match parse_handle(json).unwrap() {
                    Handle::Direct {
                        name: back, owner, ..
                    } => {
                        assert_eq!(
                            (back.as_str(), owner.as_str()),
                            (name.as_str(), "demo/Owner")
                        );
                    }
                    other => panic!("{other:?}"),
                }
                if let Some(previous) = seen.insert(json.to_owned(), (kind, name.clone())) {
                    assert_eq!(previous, (kind, name.clone()));
                }
            }
        }
    }

    #[test]
    fn combinator_types_follow_the_jdk() {
        // `java` 27: dropArguments(Integer.valueOf, 1, Object.class).type()
        // is (String,Object)Integer.
        let value_of = Handle::Direct {
            kind: Kind::Static,
            owner: "java/lang/Integer".into(),
            name: "valueOf".into(),
            descriptor: "(Ljava/lang/String;)Ljava/lang/Integer;".into(),
        };
        let dropped = Handle::Drop {
            target: Box::new(value_of),
            position: 1,
            types: vec!["Ljava/lang/Object;".into()],
        };
        assert_eq!(
            method_type_text(&dropped.descriptor().unwrap()),
            "(String,Object)Integer"
        );
        let constructor = Handle::Direct {
            kind: Kind::Constructor,
            owner: "java/math/BigInteger".into(),
            name: "<init>".into(),
            descriptor: "(Ljava/lang/String;)V".into(),
        };
        assert_eq!(
            constructor.descriptor().unwrap(),
            "(Ljava/lang/String;)Ljava/math/BigInteger;"
        );
        let invoker = Handle::Invoker {
            descriptor: "(Ljava/lang/String;)I".into(),
            exact: false,
        };
        assert_eq!(
            invoker.descriptor().unwrap(),
            "(Ljava/lang/invoke/MethodHandle;Ljava/lang/String;)I"
        );
    }
}

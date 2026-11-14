//! `java.lang.reflect.Proxy` over interfaces the running jar (or a library
//! linked beside it) declares, and the `MethodHandle` subset a proxy's
//! handler uses to run an interface's default methods.
//!
//! Config libraries hand out their settings as proxies: DazzleConf builds one
//! `Proxy.newProxyInstance(loader, new Class[] {MainConfig.class}, handler)`
//! per config section, and every `config.someKey()` the mod calls later is a
//! call on the proxy. The JVM routes such a call to
//! `handler.invoke(proxy, method, args)` with a `Method` for the interface
//! method and the arguments boxed into an `Object[]` (null when there are
//! none); the handler's answer is unboxed to the method's return type. That
//! is what happens here: the `Method` is the same handle (`host_reflect.rs`)
//! the reflection surface hands out, so `getName`, `getDeclaringClass`,
//! `getReturnType` and `isDefault` answer from the class file.
//!
//! `Object`'s `equals`, `hashCode` and `toString` on a proxy also go to the
//! handler, with `java.lang.Object` as the declaring class, as on the JVM.
//!
//! Method handles: `MethodHandles.lookup()`/`publicLookup()`/
//! `privateLookupIn` (`Lookup` handles, `host_invoke.rs`), and
//! `Lookup.unreflect`/`unreflectSpecial`/`findStatic`/`findVirtual`/
//! `findConstructor` over jar classes, which hand out `method_handles.rs`'s
//! direct handles: those answer `bindTo`, `invokeWithArguments` and the rest.
//! A special handle runs the named class's own body (a default method on the
//! proxy), never a virtual override; a virtual one on a proxy goes to its
//! handler.

use super::*;

/// The class of every proxy object; markers are `jdk/proxy/$Proxy#<n>`.
const PROXY_CLASS: &str = "jdk/proxy/$Proxy";
/// Interfaces one proxy may implement; `newProxyInstance` itself refuses
/// more than 65535, a real config section names one.
const MAX_PROXY_INTERFACES: usize = 64;

/// The field descriptor of a class marker: a primitive's letter, an array
/// descriptor as is, `L...;` for anything else.
fn class_type_descriptor(name: &str) -> String {
    match name {
        "boolean" => "Z".into(),
        "byte" => "B".into(),
        "char" => "C".into(),
        "short" => "S".into(),
        "int" => "I".into(),
        "long" => "J".into(),
        "float" => "F".into(),
        "double" => "D".into(),
        "void" => "V".into(),
        array if array.starts_with('[') => array.to_owned(),
        other => format!("L{};", other.replace('.', "/")),
    }
}

fn proxy_field(marker: &str, name: &str) -> String {
    format!("{marker}|{PROXY_CLASS}#{name}")
}

pub(super) fn is_proxy(marker: &str) -> bool {
    marker
        .strip_prefix(PROXY_CLASS)
        .is_some_and(|rest| rest.starts_with('#'))
}

fn npe(message: Option<&str>) -> NativeError {
    NativeError::UncaughtException {
        class: "java/lang/NullPointerException".into(),
        message: message.map(str::to_owned),
        frame: None,
    }
}

impl<H: Host> Runtime<'_, H> {
    /// The proxy and method-handle calls this module answers, or `None`.
    pub(super) fn invoke_reflect_proxy(
        &mut self,
        r: &MethodRef,
        receiver: Option<&Value>,
        args: &[Value],
    ) -> Result<Option<Value>, NativeError> {
        if let Some(marker) = receiver.and_then(expect_ref)
            && is_proxy(marker)
            && r.name != "<init>"
        {
            let marker = marker.clone();
            return self.invoke_on_proxy(&marker, r, args).map(Some);
        }
        match (r.owner.as_str(), r.name.as_str(), r.descriptor.as_str()) {
            (
                "java/lang/reflect/Proxy",
                "newProxyInstance",
                "(Ljava/lang/ClassLoader;[Ljava/lang/Class;Ljava/lang/reflect/InvocationHandler;)Ljava/lang/Object;",
            ) => self.new_proxy(args).map(Some),
            (
                "java/lang/reflect/Proxy",
                "getInvocationHandler",
                "(Ljava/lang/Object;)Ljava/lang/reflect/InvocationHandler;",
            ) => {
                let target = args.first().and_then(expect_ref).cloned();
                match target {
                    Some(marker) if is_proxy(&marker) => Ok(Some(
                        self.fields
                            .get(&proxy_field(&marker, "h"))
                            .cloned()
                            .unwrap_or(Value::Ref(None)),
                    )),
                    _ => Err(NativeError::UncaughtException {
                        class: "java/lang/IllegalArgumentException".into(),
                        message: Some("not a proxy instance".into()),
                        frame: None,
                    }),
                }
            }
            ("java/lang/reflect/Proxy", "isProxyClass", "(Ljava/lang/Class;)Z") => {
                Ok(Some(Value::Int(i32::from(
                    args.first()
                        .and_then(crate::host_class::class_name)
                        .is_some_and(|class| class == PROXY_CLASS),
                ))))
            }
            _ => self.invoke_method_handles(r, receiver, args),
        }
    }

    /// `Proxy.newProxyInstance`: a fresh proxy object remembering its
    /// interfaces and handler. Every interface must be one the class model
    /// holds, since only those can say which methods the proxy has.
    fn new_proxy(&mut self, args: &[Value]) -> Result<Value, NativeError> {
        let interfaces = match args.get(1) {
            Some(value @ Value::Host(_)) => self.array(value)?.clone(),
            _ => return Err(npe(Some("interfaces"))),
        };
        let handler = args.get(2).cloned().unwrap_or(Value::Ref(None));
        if matches!(handler, Value::Ref(None)) {
            return Err(npe(Some("h")));
        }
        if interfaces.len() > MAX_PROXY_INTERFACES {
            return Err(NativeError::Metadata(format!(
                "Proxy.newProxyInstance names {} interfaces; the bounded proxy takes at most {MAX_PROXY_INTERFACES}",
                interfaces.len()
            )));
        }
        let mut names = Vec::with_capacity(interfaces.len());
        for interface in &interfaces {
            let Some(name) = expect_ref(interface) else {
                return Err(npe(None));
            };
            let name = name.replace('.', "/");
            let is_interface = self
                .class(&name)
                .and_then(crate::jdk_model::class_access_flags)
                .is_some_and(|flags| flags & 0x0200 != 0);
            if !is_interface && self.class(&name).is_some() {
                // The JVM refuses a class here.
                return Err(NativeError::UncaughtException {
                    class: "java/lang/IllegalArgumentException".into(),
                    message: Some(format!("{} is not an interface", name.replace('/', "."))),
                    frame: None,
                });
            }
            // An interface outside the class model (a JDK or game interface:
            // Architectury's `createLoop` proxies `Runnable`) has no method
            // table here, but the proxy needs none to exist: a call on it
            // names its own method, which `invoke_on_proxy` hands the handler.
            names.push(Value::Ref(Some(name)));
        }
        let proxy = self.new_object(PROXY_CLASS);
        let marker = expect_ref(&proxy).cloned().unwrap_or_default();
        let list = self.new_array_value("[Ljava/lang/Class;", names);
        self.fields.insert(proxy_field(&marker, "interfaces"), list);
        self.fields.insert(proxy_field(&marker, "h"), handler);
        Ok(proxy)
    }

    /// The interfaces a proxy implements, in the order it was built with.
    fn proxy_interfaces(&self, marker: &str) -> Vec<String> {
        let Some(list) = self
            .fields
            .get(&proxy_field(marker, "interfaces"))
            .and_then(host_array::array_key)
        else {
            return Vec::new();
        };
        self.arrays
            .get(list)
            .map(|values| {
                values
                    .iter()
                    .filter_map(|value| expect_ref(value).cloned())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// `proxy instanceof wanted`: one of its interfaces is (or extends)
    /// `wanted`, or `wanted` is `Proxy` or `Object`.
    pub(super) fn proxy_instance_of(&self, marker: &str, wanted: &str) -> bool {
        if !is_proxy(marker) {
            return false;
        }
        matches!(
            wanted,
            "java/lang/Object" | "java/lang/reflect/Proxy" | "java/io/Serializable"
        ) || self
            .proxy_interfaces(marker)
            .iter()
            .any(|interface| interface == wanted || self.interface_extends(interface, wanted))
    }

    /// The interface that declares `name descriptor` for a proxy: the first
    /// of its interfaces (or their superinterfaces) with that method.
    fn proxy_declaring_interface(
        &self,
        marker: &str,
        name: &str,
        descriptor: &str,
    ) -> Option<String> {
        let mut queue = std::collections::VecDeque::from(self.proxy_interfaces(marker));
        let mut seen = std::collections::BTreeSet::new();
        while let Some(interface) = queue.pop_front() {
            if seen.len() >= MAX_SUPERCLASS_WALK || !seen.insert(interface.clone()) {
                continue;
            }
            let Some(file) = self.class(&interface) else {
                continue;
            };
            if file
                .method(name, descriptor)
                .is_some_and(|method| !method.is_static())
            {
                return Some(interface);
            }
            queue.extend(file.interface_names());
        }
        None
    }

    /// A call on a proxy: `handler.invoke(proxy, method, args)`.
    pub(super) fn invoke_on_proxy(
        &mut self,
        marker: &str,
        r: &MethodRef,
        args: &[Value],
    ) -> Result<Value, NativeError> {
        let object_method = matches!(
            (r.name.as_str(), r.descriptor.as_str()),
            ("equals", "(Ljava/lang/Object;)Z")
                | ("hashCode", "()I")
                | ("toString", "()Ljava/lang/String;")
        );
        let declaring = if object_method {
            "java/lang/Object".to_owned()
        } else {
            match self.proxy_declaring_interface(marker, &r.name, &r.descriptor) {
                Some(interface) => interface,
                // A call through an interface the class model does not hold
                // is that interface's own method.
                None if self.class(&r.owner).is_none()
                    && self.proxy_interfaces(marker).contains(&r.owner) =>
                {
                    r.owner.clone()
                }
                None if r.owner == "java/lang/Object" && r.name == "getClass" => {
                    return Ok(crate::host_class::class_value(PROXY_CLASS));
                }
                None => {
                    return Err(NativeError::MissingMethod(format!(
                        "proxy method {}#{}{}",
                        r.owner, r.name, r.descriptor
                    )));
                }
            }
        };
        let method = self.reflected_method(&declaring, &r.name, Some(&r.descriptor));
        // The JVM passes null, not an empty array, for a no-argument method.
        let parameters = descriptor_arguments(&r.descriptor);
        let arguments = if parameters.is_empty() {
            Value::Ref(None)
        } else {
            let mut boxed = Vec::with_capacity(args.len());
            for (parameter, value) in parameters.iter().zip(args) {
                boxed.push(self.box_primitive(parameter, value)?);
            }
            self.new_array_value("[Ljava/lang/Object;", boxed)
        };
        let handler = self
            .fields
            .get(&proxy_field(marker, "h"))
            .cloned()
            .unwrap_or(Value::Ref(None));
        // The proxy object itself (`jar_object.rs`).
        let call = [crate::jar_object::stored_value(marker), method, arguments];
        let answer = self.call_invocation_handler(&handler, &call)?;
        let result = return_desc(&r.descriptor);
        Ok(match result {
            "V" => Value::Void,
            "Z" | "B" | "C" | "S" | "I" | "J" | "F" | "D" => {
                if matches!(answer, Value::Ref(None)) {
                    // Unboxing the handler's null for a primitive return.
                    return Err(npe(None));
                }
                unbox_to(result, &answer)
            }
            _ => answer,
        })
    }

    /// Run `InvocationHandler.invoke` on a handler: a lambda through the
    /// callback path, a jar object through its own (or inherited) body.
    fn call_invocation_handler(
        &mut self,
        handler: &Value,
        call: &[Value],
    ) -> Result<Value, NativeError> {
        const INVOKE: &str =
            "(Ljava/lang/Object;Ljava/lang/reflect/Method;[Ljava/lang/Object;)Ljava/lang/Object;";
        let class = crate::jar_object::object_class(handler).map(str::to_owned);
        if let Some(class) = class
            && let Some((owner, true)) = self.member_owner(&class, "invoke", INVOKE)
        {
            return self.invoke(&owner, "invoke", INVOKE, Some(handler.clone()), call);
        }
        if let Some(value) = self.invoke_callback(handler, call)? {
            return Ok(value);
        }
        self.host.invoke_instance(
            &MethodRef {
                owner: "java/lang/reflect/InvocationHandler".into(),
                name: "invoke".into(),
                descriptor: INVOKE.into(),
            },
            handler,
            call,
        )
    }

    /// `MethodHandles.lookup()`, `privateLookupIn`, `Lookup.unreflect`/
    /// `unreflectSpecial`, the `find*` lookups over jar classes, and
    /// `Module.addReads` (which the JDK 11 default-method path calls first).
    fn invoke_method_handles(
        &mut self,
        r: &MethodRef,
        receiver: Option<&Value>,
        args: &[Value],
    ) -> Result<Option<Value>, NativeError> {
        match (r.owner.as_str(), r.name.as_str(), r.descriptor.as_str()) {
            // `Lookup` handles (`host_invoke.rs`): the caller of `lookup()` is
            // not recorded, nor the modes `in` keeps.
            ("java/lang/invoke/MethodHandles", "lookup", _) => Ok(Some(host_invoke::lookup(None))),
            ("java/lang/invoke/MethodHandles$Lookup", "in", _) => {
                Ok(Some(host_invoke::narrowed_lookup()))
            }
            ("java/lang/invoke/MethodHandles", "publicLookup", _) => {
                Ok(Some(host_invoke::public_lookup()))
            }
            // JDK 25's checks in order: a primitive or array target, then a
            // caller without full privilege, throw; a target the runtime
            // holds gets its full-privilege lookup; one outside the jars
            // (the JDK, the game) depends on the launch's modules: counted.
            (
                "java/lang/invoke/MethodHandles",
                "privateLookupIn",
                "(Ljava/lang/Class;Ljava/lang/invoke/MethodHandles$Lookup;)Ljava/lang/invoke/MethodHandles$Lookup;",
            ) => {
                let (Some(target), Some(caller)) = (args.first(), args.get(1)) else {
                    return Ok(None);
                };
                // Java 25's helpful messages, the caller read first.
                if matches!(caller, Value::Ref(None)) {
                    return Err(npe(Some(
                        "Cannot read field \"allowedModes\" because \"caller\" is null",
                    )));
                }
                if matches!(target, Value::Ref(None)) {
                    return Err(npe(Some(
                        "Cannot invoke \"java.lang.Class.isPrimitive()\" because \"targetClass\" is null",
                    )));
                }
                let Some(class) =
                    crate::host_class::class_name(target).map(|class| class.replace('.', "/"))
                else {
                    return self.host.unknown_operand_call(r).map(Some);
                };
                let illegal = |message: String| NativeError::UncaughtException {
                    class: "java/lang/IllegalArgumentException".into(),
                    message: Some(message),
                    frame: None,
                };
                if class_is_primitive(&Value::Ref(Some(class.clone()))) {
                    return Err(illegal(format!("{class} is a primitive class")));
                }
                if class.starts_with('[') {
                    return Err(illegal(format!(
                        "class {} is an array class",
                        class.replace('/', ".")
                    )));
                }
                match host_invoke::lookup_mode(caller) {
                    Some("public") => Err(NativeError::UncaughtException {
                        class: "java/lang/IllegalAccessException".into(),
                        message: Some("caller does not have PRIVATE and MODULE lookup mode".into()),
                        frame: None,
                    }),
                    Some("full") if self.class(&class).is_some() => {
                        Ok(Some(host_invoke::lookup(Some(&class))))
                    }
                    _ => self.host.unknown_operand_call(r).map(Some),
                }
            }
            ("java/lang/invoke/MethodHandles$Lookup", "unreflect" | "unreflectSpecial", _) => {
                let Some(method) = args.first().filter(|method| **method != Value::Ref(None))
                else {
                    return Err(npe(None));
                };
                // A direct handle (`method_handles.rs`) on a method the
                // runtime read from a class file; a lookup it could not
                // settle is counted.
                let Some(host_reflect::Member {
                    class,
                    name,
                    descriptor: Some(descriptor),
                    access: Some(access),
                    ..
                }) = host_reflect::method_of(method)
                else {
                    return Ok(None);
                };
                if r.name == "unreflectSpecial" {
                    // The receiver is typed as the special caller.
                    let Some(caller) = args
                        .get(1)
                        .and_then(expect_ref)
                        .map(|caller| caller.replace('.', "/"))
                    else {
                        return Err(npe(None));
                    };
                    return Ok(Some(method_handles::special_handle(
                        &class,
                        &name,
                        &descriptor,
                        &caller,
                    )));
                }
                let kind = if access & 0x0008 != 0 {
                    "static"
                } else {
                    "virtual"
                };
                Ok(method_handles::direct_handle(
                    kind,
                    &class,
                    &name,
                    &descriptor,
                ))
            }
            // A lookup by name over the class model. A static or virtual
            // method the model holds becomes a handle; anything else is the
            // JDK's `NoSuchMethodException` (a caller probing `valueOf` on a
            // JDK type takes its fallback), since the bounded runtime cannot
            // bind a JDK method to a handle.
            (
                "java/lang/invoke/MethodHandles$Lookup",
                "findStatic" | "findVirtual",
                "(Ljava/lang/Class;Ljava/lang/String;Ljava/lang/invoke/MethodType;)Ljava/lang/invoke/MethodHandle;",
            ) => {
                let class = args
                    .first()
                    .and_then(crate::host_class::class_name)
                    .map(str::to_owned);
                let name = args.get(1).and_then(expect_ref).cloned();
                let (Some(class), Some(name)) = (class, name) else {
                    return Err(npe(None));
                };
                let class = class.replace('.', "/");
                let wanted_static = r.name == "findStatic";
                let descriptor = args
                    .get(2)
                    .and_then(host_invoke::method_type_of)
                    .map(str::to_owned);
                let found = self.class(&class).and_then(|file| {
                    file.methods
                        .iter()
                        .find(|method| {
                            method.name == name
                                && method.is_static() == wanted_static
                                && descriptor
                                    .as_deref()
                                    .is_none_or(|descriptor| method.descriptor == descriptor)
                        })
                        .map(|method| method.descriptor.clone())
                });
                let kind = if wanted_static { "static" } else { "virtual" };
                match found {
                    Some(descriptor) => Ok(method_handles::direct_handle(
                        kind,
                        &class,
                        &name,
                        &descriptor,
                    )),
                    None => Err(NativeError::UncaughtException {
                        class: "java/lang/NoSuchMethodException".into(),
                        message: Some(format!(
                            "no such method: {}.{name}",
                            class.replace('/', ".")
                        )),
                        frame: None,
                    }),
                }
            }
            (
                "java/lang/invoke/MethodHandles$Lookup",
                "findConstructor",
                "(Ljava/lang/Class;Ljava/lang/invoke/MethodType;)Ljava/lang/invoke/MethodHandle;",
            ) => {
                let Some(class) = args
                    .first()
                    .and_then(crate::host_class::class_name)
                    .map(|c| c.replace('.', "/"))
                else {
                    return Err(npe(None));
                };
                // A constructor's type returns `void`; the descriptor matches
                // `<init>` exactly.
                let descriptor = args
                    .get(1)
                    .and_then(host_invoke::method_type_of)
                    .map(str::to_owned);
                let found = descriptor.as_deref().is_some_and(|descriptor| {
                    self.class(&class)
                        .is_some_and(|file| file.method("<init>", descriptor).is_some())
                });
                match (found, descriptor) {
                    (true, Some(descriptor)) => Ok(method_handles::direct_handle(
                        "new",
                        &class,
                        "<init>",
                        &descriptor,
                    )),
                    _ => Err(NativeError::UncaughtException {
                        class: "java/lang/NoSuchMethodException".into(),
                        message: Some(format!("no such constructor: {}", class.replace('/', "."))),
                        frame: None,
                    }),
                }
            }
            // `MethodType.methodType(rtype, ptypes...)` as its descriptor, so a
            // lookup can match a method exactly.
            ("java/lang/invoke/MethodType", "methodType", descriptor)
                if descriptor.ends_with(")Ljava/lang/invoke/MethodType;") =>
            {
                let mut types = Vec::new();
                let declared = descriptor_arguments(descriptor);
                for (index, value) in args.iter().enumerate() {
                    // A null `Class[]` (`methodType(r, p0, (Class[]) null)`).
                    if matches!(value, Value::Ref(None))
                        && declared
                            .get(index)
                            .is_some_and(|parameter| parameter.starts_with('['))
                    {
                        return Err(npe(Some(
                            "Cannot read the array length because \"ptypes\" is null",
                        )));
                    }
                    match value {
                        array if self.owned_array(array).is_some() => {
                            types.extend(self.array(value)?.clone());
                        }
                        // `methodType(rtype, List<Class<?>>)`: its elements.
                        value if self.host_collection_kind(value).is_some() => {
                            types.extend(self.collection_elements(value)?);
                        }
                        // `methodType(rtype, MethodType)` and friends.
                        value if host_invoke::method_type_of(value).is_some() => {
                            return Ok(None);
                        }
                        other => types.push(other.clone()),
                    }
                }
                let Some((result, parameters)) = types.split_first() else {
                    return Ok(None);
                };
                // Java 25 interns the type by its hash first: a null return
                // type, then a null parameter, then a `void` one.
                let Some(result) = crate::host_class::class_name(result) else {
                    return Err(npe(Some(
                        "Cannot invoke \"Object.hashCode()\" because \"this.rtype\" is null",
                    )));
                };
                if parameters
                    .iter()
                    .any(|parameter| crate::host_class::class_name(parameter).is_none())
                {
                    return Err(npe(Some(
                        "Cannot invoke \"Object.hashCode()\" because \"ptype\" is null",
                    )));
                }
                let mut descriptor = String::from("(");
                for parameter in parameters {
                    let Some(name) = crate::host_class::class_name(parameter) else {
                        return Err(npe(None));
                    };
                    if name == "void" {
                        return Err(NativeError::UncaughtException {
                            class: "java/lang/IllegalArgumentException".into(),
                            message: Some("parameter type cannot be void".into()),
                            frame: None,
                        });
                    }
                    descriptor.push_str(&class_type_descriptor(name));
                }
                descriptor.push(')');
                descriptor.push_str(&class_type_descriptor(result));
                Ok(Some(host_invoke::method_type(&descriptor)))
            }
            ("java/lang/Module", "addReads", "(Ljava/lang/Module;)Ljava/lang/Module;") => {
                Ok(receiver.cloned())
            }
            _ => Ok(None),
        }
    }
}

/// One boxed handler answer as the primitive `descriptor` names.
fn unbox_to(descriptor: &str, value: &Value) -> Value {
    match descriptor {
        "J" => boxed_long(value).map_or_else(|| value.clone(), Value::Long),
        "F" | "D" => boxed_float(value).map_or_else(|| value.clone(), Value::Float),
        "Z" => {
            boxed_boolean(value).map_or_else(|| value.clone(), |flag| Value::Int(i32::from(flag)))
        }
        _ => boxed_integer(value).map_or_else(|| value.clone(), Value::Int),
    }
}

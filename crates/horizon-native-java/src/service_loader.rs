//! `java.util.ServiceLoader` providers, built the way JDK 27's `ServiceLoader`
//! builds them (read from its bytecode: `LazyClassPathLookupIterator`,
//! `loadProvider`, `getConstructor`, `ProviderImpl.newInstance`).
//!
//! Before this module the loader handed out each provider as its bare class
//! name (its `Provider` was text split at the first `/`, so a packaged
//! service gave a wrong class name), and no provider was ever constructed,
//! so its constructor and field initialisers never ran. The loader, its
//! iterators and providers are host handles now (`host_services.rs`).
//!
//! What the JDK does, and this module with it:
//!
//! - **Lookup**, per provider name in `META-INF/services` order. On the class
//!   path (Fabric's Knot) the provider must be a subtype of the service
//!   (`<cn> not a subtype`) with a public no-arg constructor
//!   (`<cn> Unable to get public no-arg constructor`). In a named module
//!   (NeoForge loads each mod as an automatic module) the class must also be
//!   public (`class <cn> is not public`). Every failure is a
//!   `ServiceConfigurationError` whose message starts `<service>: `.
//! - **Construction** runs the public no-arg constructor. Anything it throws
//!   becomes `Provider <cn> could not be instantiated`. On the class path a
//!   non-public class passes the lookup and fails here, with an
//!   `IllegalAccessException` cause.
//! - **`iterator()`** is lazy: a provider is looked up and constructed on
//!   `next()`, and a failure consumes that provider (the next `next()` moves
//!   on). Instances are cached per loader, so a second iterator or
//!   `findFirst()` returns the same objects, and `reload()` clears the cache
//!   and makes older iterators throw `ConcurrentModificationException`.
//! - **`stream()`** gives one `Provider` per provider, looked up when used.
//!   Each `get()` constructs a new instance (the JDK does not cache them).
//!
//! Counted, not guessed: a provider class the runtime does not hold (it may be
//! on the game's class path), a provider in an explicit module (whose
//! `provides` and static `provider()` rule this module does not read), and a
//! subtype check against a service the runtime cannot walk. Each of these
//! hands out the old bare class-name marker after counting.
//!
//! Named departures:
//! - the error's `getCause()` is `null`. The runtime's thrown exceptions carry
//!   a class and a message only, but the message is the JDK's;
//! - a `stream()` element whose lookup fails throws the error when it is
//!   used (`type()`, `get()`), not when the stream reaches it, so a terminal
//!   operation that never uses it (`count()`) does not throw, and the
//!   providers after it are still streamed;
//! - on the module path the JDK keeps a duplicate line of a services file,
//!   while the seeded table (and the class path) does not.

use super::*;

const SERVICE_ERROR: &str = "java/util/ServiceConfigurationError";
const ACC_PUBLIC: u16 = 0x0001;
const ACC_INTERFACE: u16 = 0x0200;

/// The side table a loader or iterator keeps in the runtime's field map,
/// under its handle's key (`host_services.rs`).
fn slot(key: &str, name: &str) -> String {
    format!("{key}|java/util/ServiceLoader#{name}")
}

fn service_error(service: &str, message: &str) -> NativeError {
    NativeError::UncaughtException {
        class: SERVICE_ERROR.into(),
        message: Some(format!("{}: {message}", service.replace('/', "."))),
        frame: None,
    }
}

/// What a provider lookup found.
enum Lookup {
    /// A provider the runtime can construct exactly.
    Exact,
    /// A provider it cannot (counted); an object of its class stands for it.
    Opaque,
}

impl<H: Host> Runtime<'_, H> {
    /// The `ServiceLoader` surface, for a direct call or a method reference.
    pub(super) fn invoke_service_loader(
        &mut self,
        r: &MethodRef,
        receiver: Option<&Value>,
        args: &[Value],
    ) -> Result<Option<Value>, NativeError> {
        let state = receiver
            .and_then(host_services::state_key)
            .map(str::to_owned);
        let is =
            |kind: HostKind| matches!(receiver, Some(Value::Host(handle)) if handle.kind() == kind);
        match (r.owner.as_str(), r.name.as_str(), r.descriptor.as_str()) {
            // The host validates the service (and refuses a JDK or game one it
            // cannot see); each load is then its own loader object.
            ("java/util/ServiceLoader", "load" | "loadInstalled", descriptor)
                if descriptor.starts_with("(Ljava/lang/Class;") =>
            {
                let loader = self.host.invoke(r, args)?;
                let Some((service, _)) = host_services::loader_parts(&loader) else {
                    return Ok(Some(loader));
                };
                // Each load is its own loader object.
                let loader = host_services::loader(&service, self.next_object);
                self.next_object = self.next_object.saturating_add(1);
                Ok(Some(loader))
            }
            ("java/util/ServiceLoader" | "java/lang/Iterable", _, _)
                if is(HostKind::ServiceLoader) =>
            {
                let Some(loader) = receiver.cloned() else {
                    return Ok(None);
                };
                self.loader_call(r, &loader, args)
            }
            ("java/util/Iterator", "hasNext", "()Z") if is(HostKind::ServiceIterator) => {
                let iterator = state.unwrap_or_default();
                self.iterator_has_next(&iterator)
                    .map(|more| Some(Value::Int(i32::from(more))))
            }
            ("java/util/Iterator", "next", "()Ljava/lang/Object;")
                if is(HostKind::ServiceIterator) =>
            {
                let iterator = state.unwrap_or_default();
                self.iterator_next(&iterator).map(Some)
            }
            ("java/util/ServiceLoader$Provider", name, descriptor) => {
                let Some((service, provider)) = receiver.and_then(host_services::provider_parts)
                else {
                    return Ok(None);
                };
                match (name, descriptor) {
                    ("type", "()Ljava/lang/Class;") => {
                        self.lookup(&service, &provider)?;
                        Ok(Some(crate::host_class::class_value(&provider)))
                    }
                    ("get", "()Ljava/lang/Object;") => {
                        self.provider_instance(&service, &provider).map(Some)
                    }
                    _ => Ok(None),
                }
            }
            _ => Ok(None),
        }
    }

    fn loader_call(
        &mut self,
        r: &MethodRef,
        handle: &Value,
        args: &[Value],
    ) -> Result<Option<Value>, NativeError> {
        let (service, _) = host_services::loader_parts(handle).unwrap_or_default();
        let loader = host_services::state_key(handle)
            .unwrap_or_default()
            .to_owned();
        let loader = loader.as_str();
        match (r.name.as_str(), r.descriptor.as_str()) {
            ("iterator", "()Ljava/util/Iterator;") => Ok(Some(self.new_service_iterator(handle))),
            ("findFirst", "()Ljava/util/Optional;") => {
                let iterator = self.new_service_iterator(handle);
                let iterator = host_services::state_key(&iterator)
                    .unwrap_or_default()
                    .to_owned();
                if !self.iterator_has_next(&iterator)? {
                    return Ok(Some(empty_optional()));
                }
                let first = self.iterator_next(&iterator)?;
                Ok(Some(optional_wrap(first)))
            }
            ("forEach", "(Ljava/util/function/Consumer;)V") => {
                let action = StdHost::argument(r, args, 0)?.clone();
                let iterator = self.new_service_iterator(handle);
                let iterator = host_services::state_key(&iterator)
                    .unwrap_or_default()
                    .to_owned();
                while self.iterator_has_next(&iterator)? {
                    let value = self.iterator_next(&iterator)?;
                    if self
                        .invoke_callback(&action, std::slice::from_ref(&value))?
                        .is_none()
                    {
                        let accept = MethodRef {
                            owner: "java/util/function/Consumer".into(),
                            name: "accept".into(),
                            descriptor: "(Ljava/lang/Object;)V".into(),
                        };
                        self.host.invoke_instance(&accept, &action, &[value])?;
                    }
                }
                Ok(Some(Value::Void))
            }
            ("stream", "()Ljava/util/stream/Stream;") => {
                // Each element looks its provider up when it is used, so a
                // `findFirst()` never reaches a later provider's failure. The
                // loader keeps one `Provider` per provider (`host_services.rs`).
                let providers = self
                    .service_names(&service)
                    .into_iter()
                    .map(|provider| host_services::provider(&service, &provider))
                    .collect();
                // `ProviderSpliterator` is not `SIZED`.
                let stream = self.new_stream("java/util/ServiceLoader", "stream", providers);
                Ok(Some(stream))
            }
            ("reload", "()V") => {
                let reloads = self.int_slot(loader, "reloads");
                for name in ["lookup", "count"] {
                    self.fields.remove(&slot(loader, name));
                }
                self.fields
                    .insert(slot(loader, "reloads"), Value::Int(reloads + 1));
                Ok(Some(Value::Void))
            }
            _ => Ok(None),
        }
    }

    /// The provider class names for `service`, in `META-INF/services` order.
    fn service_names(&self, service: &str) -> Vec<String> {
        self.host
            .service_providers(service)
            .into_iter()
            .map(|provider| provider.replace('.', "/"))
            .collect()
    }

    fn int_slot(&self, marker: &str, name: &str) -> i32 {
        match self.fields.get(&slot(marker, name)) {
            Some(Value::Int(value)) => *value,
            _ => 0,
        }
    }

    /// `ServiceLoader.iterator()`: a cursor over the loader's cached instances,
    /// then its shared lookup.
    fn new_service_iterator(&mut self, loader: &Value) -> Value {
        let handle = host_services::iterator(self.next_object);
        self.next_object = self.next_object.saturating_add(1);
        let iterator = host_services::state_key(&handle)
            .unwrap_or_default()
            .to_owned();
        let key = host_services::state_key(loader)
            .unwrap_or_default()
            .to_owned();
        let reloads = self.int_slot(&key, "reloads");
        self.fields
            .insert(slot(&iterator, "loader"), loader.clone());
        self.fields
            .insert(slot(&iterator, "reloads"), Value::Int(reloads));
        handle
    }

    /// The iterator's loader (its state key and its service), after
    /// `checkReloadCount`.
    fn iterator_loader(&self, iterator: &str) -> Result<(String, String), NativeError> {
        let Some(handle) = self.fields.get(&slot(iterator, "loader")) else {
            return Err(NativeError::Metadata(format!(
                "unknown ServiceLoader iterator {iterator}"
            )));
        };
        let (Some(loader), Some((service, _))) = (
            host_services::state_key(handle),
            host_services::loader_parts(handle),
        ) else {
            return Err(NativeError::Metadata(format!(
                "unknown ServiceLoader iterator {iterator}"
            )));
        };
        let loader = loader.to_owned();
        if self.int_slot(iterator, "reloads") != self.int_slot(&loader, "reloads") {
            return Err(NativeError::UncaughtException {
                class: "java/util/ConcurrentModificationException".into(),
                message: None,
                frame: None,
            });
        }
        Ok((loader, service))
    }

    fn iterator_has_next(&mut self, iterator: &str) -> Result<bool, NativeError> {
        let (loader, service) = self.iterator_loader(iterator)?;
        if self.int_slot(iterator, "index") < self.int_slot(&loader, "count") {
            return Ok(true);
        }
        let lookup = self.int_slot(&loader, "lookup");
        Ok((lookup as usize) < self.service_names(&service).len())
    }

    fn iterator_next(&mut self, iterator: &str) -> Result<Value, NativeError> {
        let (loader, service) = self.iterator_loader(iterator)?;
        let index = self.int_slot(iterator, "index");
        let count = self.int_slot(&loader, "count");
        let value = if index < count {
            self.fields
                .get(&slot(&loader, &format!("instance{index}")))
                .cloned()
                .unwrap_or(Value::Ref(None))
        } else {
            let lookup = self.int_slot(&loader, "lookup");
            let Some(provider) = self
                .host
                .service_providers(&service)
                .into_iter()
                .nth(lookup as usize)
            else {
                return Err(NativeError::UncaughtException {
                    class: "java/util/NoSuchElementException".into(),
                    message: None,
                    frame: None,
                });
            };
            // The lookup iterator moves past this provider whether or not it
            // can be built.
            self.fields
                .insert(slot(&loader, "lookup"), Value::Int(lookup + 1));
            let value = self.provider_instance(&service, &provider)?;
            self.fields
                .insert(slot(&loader, &format!("instance{count}")), value.clone());
            self.fields
                .insert(slot(&loader, "count"), Value::Int(count + 1));
            value
        };
        self.fields
            .insert(slot(iterator, "index"), Value::Int(index + 1));
        Ok(value)
    }

    /// Whether this run is on Fabric's class path (Knot) rather than in a
    /// module layer (NeoForge).
    fn on_class_path(&self) -> bool {
        self.host
            .platform_class_present("net/fabricmc/loader/api/FabricLoader")
    }

    /// The JDK's provider lookup: the checks before construction.
    fn lookup(&mut self, service: &str, provider: &str) -> Result<Lookup, NativeError> {
        let class_path = self.on_class_path();
        let access = self.class(provider).map(checkcast::class_access);
        let Some(access) = access else {
            // Not in a jar the runtime holds: `Provider ... not found` on a
            // JVM only if no other class path has it either.
            self.count_guess(provider, "<service-provider>", "()Ljava/lang/Object;")?;
            return Ok(Lookup::Opaque);
        };
        let access = access.unwrap_or(ACC_PUBLIC);
        if !class_path {
            if self.host.jar_module_descriptor() == Some(true) {
                // An explicit module lists providers in `module-info` and may
                // build them with a static `provider()` method.
                self.count_guess(provider, "<service-provider>", "()Ljava/lang/Object;")?;
                return Ok(Lookup::Opaque);
            }
            if access & ACC_PUBLIC == 0 {
                let kind = if access & ACC_INTERFACE != 0 {
                    "interface"
                } else {
                    "class"
                };
                return Err(service_error(
                    service,
                    &format!("{kind} {} is not public", provider.replace('/', ".")),
                ));
            }
        }
        match self.class_known_instance_of(provider, service) {
            Some(true) => {}
            Some(false) if self.class(service).is_some() => {
                return Err(service_error(
                    service,
                    &format!("{} not a subtype", provider.replace('/', ".")),
                ));
            }
            // A service the runtime does not hold: its supertypes are not
            // walkable, so the check is counted and the provider kept.
            _ => self.count_guess(service, "<instanceof>", "(Ljava/lang/Object;)Z")?,
        }
        let public_constructor = self.class(provider).is_some_and(|file| {
            file.methods.iter().any(|method| {
                method.name == "<init>"
                    && method.descriptor == "()V"
                    && method.access & ACC_PUBLIC != 0
            })
        });
        if !public_constructor {
            return Err(service_error(
                service,
                &format!(
                    "{} Unable to get public no-arg constructor",
                    provider.replace('/', ".")
                ),
            ));
        }
        Ok(Lookup::Exact)
    }

    fn count_guess(
        &mut self,
        owner: &str,
        name: &str,
        descriptor: &str,
    ) -> Result<(), NativeError> {
        let call = MethodRef {
            owner: owner.to_owned(),
            name: name.into(),
            descriptor: descriptor.into(),
        };
        self.host.invoke(&call, &[]).map(|_| ())
    }

    /// Look the provider up and build one instance, as `Provider.get` does.
    fn provider_instance(&mut self, service: &str, provider: &str) -> Result<Value, NativeError> {
        // A provider the runtime cannot build (counted in `lookup`) stands
        // as an object of its class whose constructor did not run: never its
        // name, which is a `String` (#156 P13c).
        if let Lookup::Opaque = self.lookup(service, provider)? {
            return Ok(self.new_object(provider));
        }
        let cannot = || {
            service_error(
                service,
                &format!(
                    "Provider {} could not be instantiated",
                    provider.replace('/', ".")
                ),
            )
        };
        let public_class = self
            .class(provider)
            .and_then(checkcast::class_access)
            .is_none_or(|access| access & ACC_PUBLIC != 0);
        if !public_class {
            // Class path only (a module refused it in the lookup): the
            // reflective call from `java.base` cannot reach a non-public class
            // (`IllegalAccessException`).
            return Err(cannot());
        }
        // `Constructor.newInstance` initialises the class, then runs `<init>`;
        // whatever either throws is the error's cause.
        let built = self.ensure_initialized(provider).and_then(|()| {
            let instance = self.new_object(provider);
            self.invoke(provider, "<init>", "()V", Some(instance.clone()), &[])
                .map(|_| instance)
        });
        match built {
            Err(NativeError::UncaughtException { .. }) => Err(cannot()),
            other => other,
        }
    }
}

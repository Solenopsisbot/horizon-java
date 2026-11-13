//! `ServiceLoader`, its iterators and its `Provider`s as host handles (#156
//! P9). The loader model (lookup, construction, caching, `reload`) is
//! `service_loader.rs`, unchanged; this module is the objects' keys and
//! rows.
//!
//! These were `service-loader/<service>;<n>`, `service-iterator/<n>` and
//! `service-provider/<service>;<provider>` text. Now (Java 25 checked in
//! `tests/fixture_host_services.rs`):
//!
//! * loader: key [`HostRef::of_values`] of the service class name and the
//!   load's serial. Every `ServiceLoader.load` is a new loader: canonical by
//!   serial, `Object`'s `equals`, `toString` `java.util.ServiceLoader[<service>]`.
//! * iterator: key the serial; one object per `iterator()` call.
//! * provider: key the service, the provider class and its loader's serial.
//!   A loader hands out the same `Provider` object for a provider every time
//!   (`l.stream().findFirst().get() == l.stream().findFirst().get()` on Java
//!   25), so the key names one object: canonical. `ProviderImpl.equals`
//!   compares the service and provider classes, so providers of two loaders
//!   are equal but not `==`; `hashCode` hashes the two `Class` objects
//!   (identity hashes): counted.
//!
//! The classes are `ServiceLoader` (final), an anonymous iterator and
//! `ServiceLoader$ProviderImpl`; all three are [`ClassRule::Unknown`] here,
//! so `getClass()` is counted, never answered from a guessed class.
//!
//! [`ClassRule::Unknown`]: crate::host_ref::ClassRule

use crate::host_executors::identity_equals;
use crate::host_ref::{HostKind, HostRef, Methods, Nested, instance_of_class};
use crate::{NativeError, Value};

const LOADER_TYPES: &[&str] = &["java/util/ServiceLoader", "java/lang/Iterable"];
const ITERATOR_TYPES: &[&str] = &["java/util/Iterator"];
const PROVIDER_TYPES: &[&str] = &[
    "java/util/ServiceLoader$Provider",
    "java/util/function/Supplier",
];

pub(crate) fn loader_types(_: &str) -> &'static [&'static str] {
    LOADER_TYPES
}
pub(crate) fn iterator_types(_: &str) -> &'static [&'static str] {
    ITERATOR_TYPES
}
pub(crate) fn provider_types(_: &str) -> &'static [&'static str] {
    PROVIDER_TYPES
}

/// A `ServiceLoader` for `service` (internal name), load number `serial`.
pub(crate) fn loader(service: &str, serial: usize) -> Value {
    Value::Host(HostRef::of_values(
        HostKind::ServiceLoader,
        &[
            Value::Ref(Some(service.to_owned())),
            Value::Int(serial as i32),
        ],
    ))
}

/// A loader handle's service and serial.
pub(crate) fn loader_parts(value: &Value) -> Option<(String, i32)> {
    let Value::Host(handle) = value else {
        return None;
    };
    if handle.kind() != HostKind::ServiceLoader {
        return None;
    }
    match handle.values()?.as_slice() {
        [Value::Ref(Some(service)), Value::Int(serial)] => Some((service.clone(), *serial)),
        _ => None,
    }
}

/// The text a loader or iterator keeps its state under in the runtime's
/// field table: its handle's key, which names it alone.
pub(crate) fn state_key(value: &Value) -> Option<&str> {
    match value {
        Value::Host(handle)
            if matches!(
                handle.kind(),
                HostKind::ServiceLoader | HostKind::ServiceIterator
            ) =>
        {
            Some(handle.key())
        }
        _ => None,
    }
}

/// Iterator number `serial`.
pub(crate) fn iterator(serial: usize) -> Value {
    Value::Host(HostRef::new(HostKind::ServiceIterator, serial.to_string()))
}

/// The `Provider` for `provider` (internal name) of `service`, from the loader
/// with serial `loader`.
pub(crate) fn provider(service: &str, provider: &str, loader: i32) -> Value {
    Value::Host(HostRef::of_values(
        HostKind::ServiceProvider,
        &[
            Value::Ref(Some(service.to_owned())),
            Value::Ref(Some(provider.to_owned())),
            Value::Int(loader),
        ],
    ))
}

/// A provider handle's service and provider class.
pub(crate) fn provider_parts(value: &Value) -> Option<(String, String)> {
    let Value::Host(handle) = value else {
        return None;
    };
    provider_handle_parts(handle)
}

fn provider_handle_parts(handle: &HostRef) -> Option<(String, String)> {
    if handle.kind() != HostKind::ServiceProvider {
        return None;
    }
    match handle.values()?.as_slice() {
        [Value::Ref(Some(service)), Value::Ref(Some(provider)), _] => {
            Some((service.clone(), provider.clone()))
        }
        _ => None,
    }
}

/// `"java.util.ServiceLoader[" + service.getName() + "]"`.
fn loader_text(handle: &HostRef, _: &mut dyn Nested) -> Result<Option<String>, NativeError> {
    Ok(loader_parts(&Value::Host(handle.clone()))
        .map(|(service, _)| format!("java.util.ServiceLoader[{}]", service.replace('/', "."))))
}

pub(crate) const LOADER_METHODS: Methods = Methods {
    to_string: loader_text,
    hash_code: |_, _| Ok(None),
    equals: identity_equals,
    instance_of: instance_of_class,
    crosses_jars: |_, _| false,
    compare_to: None,
};

/// `ProviderImpl.equals`: another `ProviderImpl` with the same service and
/// provider classes.
fn provider_equals(
    handle: &HostRef,
    other: &Value,
    _: &mut dyn Nested,
) -> Result<Option<bool>, NativeError> {
    Ok(Some(match other {
        Value::Host(other) => {
            provider_handle_parts(other).is_some()
                && provider_handle_parts(other) == provider_handle_parts(handle)
        }
        _ => false,
    }))
}

pub(crate) const PROVIDER_METHODS: Methods = Methods {
    // `Object.toString`: the class name and an identity hash.
    to_string: |_, _| Ok(None),
    // `Objects.hash(service, type)`: two `Class` identity hashes.
    hash_code: |_, _| Ok(None),
    equals: provider_equals,
    instance_of: instance_of_class,
    crosses_jars: |_, _| false,
    compare_to: None,
};

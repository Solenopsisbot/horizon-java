//! Mod-defined registries: `ResourceKey.createRegistryKey` and Fabric's
//! `FabricRegistryBuilder`.
//!
//! A library that adds its own registry builds the key, hands it to
//! `FabricRegistryBuilder.create*(key)...buildAndRegister()`, and binds the
//! result before anything reads it (Collector's Album's card types). The
//! registry the builder answers is a `Registry` handle keyed by the id
//! (`host_registry.rs`), the one later `Registry.register` calls name.
//!
//! A builder is a handle too (`HostKind::RegistryBuilder`, #177 review B1):
//! its key holds the registry `buildAndRegister()` answers, as the runtime
//! made it, so a jar `String` spelled like the builder's old text
//! (`fabric-registry-builder/1minecraft:item`) is no builder. Its cast to
//! `FabricRegistryBuilder` throws (`host_registry::retired_marker`), and a
//! builder call on anything but a handle is counted.

use super::*;

pub(crate) const BUILDER: &str = "net/fabricmc/fabric/api/event/registry/FabricRegistryBuilder";

/// The `<namespace>:<path>` an `Identifier`, a registry key or a `Registry`
/// names.
fn registry_id(value: &Value) -> Option<String> {
    crate::runtime::host_ids::named_id(value)
        .or_else(|| crate::runtime::host_registry::registry_id(value).map(str::to_owned))
}

/// A builder whose `buildAndRegister()` answers `registry`. Two builders are
/// two objects (`FabricRegistryBuilder` keeps `Object.equals`): never
/// canonical, so `==` is known only for the same handle (a fluent setter's
/// `this`).
pub(crate) fn builder(registry: Value) -> Value {
    Value::Host(HostRef::of_values(HostKind::RegistryBuilder, &[registry]))
}

/// The registry a builder handle will answer; `None` for anything else.
fn built(receiver: &Value) -> Option<Value> {
    match receiver {
        Value::Host(handle) if handle.kind() == HostKind::RegistryBuilder => {
            handle.values()?.into_iter().next()
        }
        _ => None,
    }
}

pub(crate) fn static_call(r: &MethodRef, args: &[Value]) -> Option<Result<Value, NativeError>> {
    match (r.owner.as_str(), r.name.as_str()) {
        ("net/minecraft/resources/ResourceKey", "createRegistryKey")
            if r.descriptor
                == "(Lnet/minecraft/resources/Identifier;)Lnet/minecraft/resources/ResourceKey;" =>
        {
            let id = registry_id(args.first()?)?;
            Some(Ok(crate::runtime::host_ids::registry_key_of(&id)))
        }
        // `create` builds a `MappedRegistry`, `createDefaulted` a
        // `DefaultedMappedRegistry` (fabric-registry-sync 8.0.4), and `from`
        // wraps the registry it is given, which `buildAndRegister` answers
        // (@69–72: `getfield registry; areturn`).
        (BUILDER, "create" | "createSimple" | "createDefaulted" | "from")
            if r.descriptor.ends_with(&format!(")L{BUILDER};")) =>
        {
            let arg = args.first()?;
            let registry = match r.name.as_str() {
                // A registry the jar built itself (`new MappedRegistry`) is
                // `library_dispatch.rs`'s.
                "from" => crate::runtime::host_registry::registry_id(arg).map(|_| arg.clone())?,
                "createDefaulted" => crate::runtime::host_registry::built_registry(
                    crate::runtime::host_registry::DEFAULTED_MAPPED,
                    &registry_id(arg)?,
                ),
                _ => crate::runtime::host_registry::built_registry(
                    crate::runtime::host_registry::MAPPED,
                    &registry_id(arg)?,
                ),
            };
            Some(Ok(builder(registry)))
        }
        _ => None,
    }
}

pub(crate) fn instance_call(
    host: &mut StdHost,
    r: &MethodRef,
    receiver: &Value,
) -> Option<Result<Value, NativeError>> {
    if r.owner != BUILDER {
        return None;
    }
    // A builder only the runtime made: anything else (a String spelled like
    // the old text) is counted.
    let Some(registry) = built(receiver) else {
        return Some(host.unknown_host_call(r));
    };
    match r.name.as_str() {
        // Fluent setters answer the builder itself.
        _ if r.descriptor.ends_with(&format!(")L{BUILDER};")) => Some(Ok(receiver.clone())),
        "buildAndRegister" => Some(Ok(registry)),
        _ => None,
    }
}

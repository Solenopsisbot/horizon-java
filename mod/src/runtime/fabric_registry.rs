//! Mod-defined registries: `ResourceKey.createRegistryKey` and Fabric's
//! `FabricRegistryBuilder`.
//!
//! A library that adds its own registry builds the key, hands it to
//! `FabricRegistryBuilder.create*(key)...buildAndRegister()`, and binds the
//! result before anything reads it (Collector's Album's card types). The host
//! models a registry and its key as the same `registry/<id>` marker (see
//! `BuiltInRegistries`), so the key, the builder and the built registry all
//! carry that id, and the registry the builder answers is the one later
//! `Registry.register` calls name.

use super::*;

const BUILDER: &str = "net/fabricmc/fabric/api/event/registry/FabricRegistryBuilder";
const BUILDER_PREFIX: &str = "fabric-registry-builder/";

/// The `<namespace>:<path>` an `Identifier`, a registry key or a `Registry`
/// names.
fn registry_id(value: &Value) -> Option<String> {
    crate::runtime::host_ids::named_id(value)
        .or_else(|| crate::runtime::host_registry::registry_id(value).map(str::to_owned))
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
        // wraps the registry it is given. The builder's marker keeps that
        // digit before the (validated) id.
        (BUILDER, "create" | "createSimple" | "createDefaulted" | "from")
            if r.descriptor.ends_with(&format!(")L{BUILDER};")) =>
        {
            let arg = args.first()?;
            let id = registry_id(arg)?;
            let implementation = match r.name.as_str() {
                "createDefaulted" => crate::runtime::host_registry::DEFAULTED_MAPPED,
                "from" => crate::runtime::host_registry::registry_implementation(arg)
                    .unwrap_or(crate::runtime::host_registry::ONLY_REGISTRY),
                _ => crate::runtime::host_registry::MAPPED,
            };
            Some(Ok(Value::Ref(Some(format!(
                "{BUILDER_PREFIX}{implementation}{id}"
            )))))
        }
        _ => None,
    }
}

pub(crate) fn instance_call(r: &MethodRef, receiver: &Value) -> Option<Result<Value, NativeError>> {
    if r.owner != BUILDER {
        return None;
    }
    let built = expect_ref(receiver)?.strip_prefix(BUILDER_PREFIX)?;
    let implementation = built.chars().next()?;
    let id = built.get(1..)?.to_owned();
    match r.name.as_str() {
        // Fluent setters answer the builder itself.
        _ if r.descriptor.ends_with(&format!(")L{BUILDER};")) => Some(Ok(receiver.clone())),
        "buildAndRegister" => Some(Ok(crate::runtime::host_registry::built_registry(
            implementation,
            &id,
        ))),
        _ => None,
    }
}

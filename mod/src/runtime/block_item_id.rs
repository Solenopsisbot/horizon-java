//! Vanilla 26.3's `net.minecraft.references.BlockItemId`.
//!
//! The record pairs a block key and an item key; `create` builds both with
//! `ResourceKey.create(Registries.BLOCK/ITEM, id)` and mods register a block
//! under `blockItemId.block()` (Farmer's Delight's `ModBlockItemIds`). The
//! host keeps the two identifiers in the handle (`host_ids.rs`) and answers each accessor
//! through its own `ResourceKey.create`, so the key is exactly the one a direct
//! `ResourceKey.create` call would have produced, never a nameless null.

use super::*;

const OWNER: &str = "net/minecraft/references/BlockItemId";
const RESOURCE_KEY_CREATE: &str = "(Lnet/minecraft/resources/ResourceKey;Lnet/minecraft/resources/Identifier;)Lnet/minecraft/resources/ResourceKey;";

impl StdHost {
    /// `BlockItemId.create(...)`; `None` for every other static call.
    pub(super) fn block_item_id_static(
        &mut self,
        r: &MethodRef,
        args: &[Value],
    ) -> Option<Result<Value, NativeError>> {
        if r.owner != OWNER || r.name != "create" {
            return None;
        }
        // `Identifier.withDefaultNamespace(name)`, which validates the path
        // (`BlockItemId.create("a:b", ..)` throws, as on 26.3).
        let default_namespace = |host: &mut Self, value: &Value| {
            let with_default = MethodRef {
                owner: "net/minecraft/resources/Identifier".into(),
                name: "withDefaultNamespace".into(),
                descriptor: "(Ljava/lang/String;)Lnet/minecraft/resources/Identifier;".into(),
            };
            host.invoke(&with_default, std::slice::from_ref(value))
        };
        Some(match (r.descriptor.as_str(), args) {
            (
                "(Lnet/minecraft/resources/Identifier;Lnet/minecraft/resources/Identifier;)Lnet/minecraft/references/BlockItemId;",
                [block, item],
            ) => Ok(host_ids::block_item_id(block, item)),
            (
                "(Ljava/lang/String;Ljava/lang/String;)Lnet/minecraft/references/BlockItemId;",
                [block, item],
            ) => default_namespace(self, block).and_then(|block| {
                let item = default_namespace(self, item)?;
                Ok(host_ids::block_item_id(&block, &item))
            }),
            ("(Ljava/lang/String;)Lnet/minecraft/references/BlockItemId;", [name]) => {
                default_namespace(self, name).map(|id| host_ids::block_item_id(&id, &id))
            }
            _ => return None,
        })
    }

    /// `block()`/`item()` on a modelled id: the key `ResourceKey.create`
    /// builds for that registry and identifier.
    pub(super) fn block_item_id_instance(
        &mut self,
        r: &MethodRef,
        receiver: &Value,
    ) -> Option<Result<Value, NativeError>> {
        if r.owner != OWNER || r.descriptor != "()Lnet/minecraft/resources/ResourceKey;" {
            return None;
        }
        let (block, item) = host_ids::block_item_id_parts(receiver)?;
        let (registry, id) = match r.name.as_str() {
            "block" => ("BLOCK", block),
            "item" => ("ITEM", item),
            _ => return None,
        };
        let registry = MethodRef {
            owner: "net/minecraft/core/registries/Registries".into(),
            name: registry.into(),
            descriptor: "Lnet/minecraft/resources/ResourceKey;".into(),
        };
        let create = MethodRef {
            owner: "net/minecraft/resources/ResourceKey".into(),
            name: "create".into(),
            descriptor: RESOURCE_KEY_CREATE.into(),
        };
        Some(
            self.get_static(&registry)
                .and_then(|registry| self.invoke(&create, &[registry, id])),
        )
    }
}

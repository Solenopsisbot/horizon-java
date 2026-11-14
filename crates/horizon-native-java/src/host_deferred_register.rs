//! A NeoForge/Forge `DeferredRegister` as a host handle (#182 P1).
//!
//! `DeferredRegister.create(Registries.ITEM, "demo")` was the text
//! `deferred-register/registry/minecraft:item/demo`: two registers for one
//! registry and namespace were one `String`, so `r1 == r2` was `true` and the
//! two shared one entry list. NeoForge 26.3.0.101's `create` is
//! `new DeferredRegister(key, namespace); areturn`, a new object per call, so
//! each register is now a handle of its own ([`HostKind::DeferredRegister`]).
//!
//! The key is `<class>@<serial>@<text>`: the class `create` made
//! (`DeferredRegister`, `DeferredRegister$DataComponents`, Forge's), one serial
//! per call, and the old text, which still names the register in the
//! registration rows the game reads (`RegistryEntry::registry`) and in the
//! attachment bridge (`neoforge_attachments.rs`). Only the runtime makes such a
//! key, so it is read back by position: class and serial hold no `@`, and the
//! text is the rest. Its holders (`HolderRow::register`) and its `getEntries`
//! view (`StdHost::register_entries`) are keyed by the whole key, so each
//! register sees only what was registered through it.
//!
//! `DeferredRegister` overrides none of `Object`'s methods: `equals` is
//! identity, `hashCode` and `toString` an identity hash (counted).

use std::sync::atomic::{AtomicU64, Ordering};

use crate::Value;
use crate::host_ref::{ClassHierarchy, HostKind, HostRef, Methods};

/// NeoForge's register class.
pub(crate) const NEOFORGE: &str = "net/neoforged/neoforge/registries/DeferredRegister";
/// `DeferredRegister.createDataComponents`'s subclass.
pub(crate) const DATA_COMPONENTS: &str =
    "net/neoforged/neoforge/registries/DeferredRegister$DataComponents";
/// Forge's register class.
pub(crate) const FORGE: &str = "net/minecraftforge/registries/DeferredRegister";

/// One serial per register made: a new object per `create`.
static NEXT: AtomicU64 = AtomicU64::new(0);

/// A new register of `class`, named `text` in the registration rows.
pub(crate) fn new_register(class: &str, text: &str) -> Value {
    let serial = NEXT.fetch_add(1, Ordering::Relaxed);
    Value::Host(HostRef::new(
        HostKind::DeferredRegister,
        format!("{class}@{serial}@{text}"),
    ))
}

/// A register's key and its text; `None` for any other value (a `String`
/// spelled like the old text included).
pub(crate) fn parts(value: &Value) -> Option<(&str, &str)> {
    let Value::Host(handle) = value else {
        return None;
    };
    if handle.kind() != HostKind::DeferredRegister {
        return None;
    }
    let key = handle.key();
    let text = key.splitn(3, '@').nth(2)?;
    Some((key, text))
}

/// The class a register's key names.
pub(crate) fn register_class(key: &str) -> &str {
    key.split('@').next().unwrap_or(NEOFORGE)
}

/// `instanceof` a register: its class and `DataComponents`' superclass are
/// known; `DeferredRegister` extends `Object` and implements nothing, and a
/// Mixin adds no superclass, so another class is `false`. An interface a
/// loaded Mixin may add is unknown (counted).
fn instance_of(handle: &HostRef, wanted: &str, hierarchy: &dyn ClassHierarchy) -> Option<bool> {
    let class = register_class(handle.key());
    if wanted == "java/lang/Object"
        || wanted == class
        || (class == DATA_COMPONENTS && wanted == NEOFORGE)
        || hierarchy.is_instance(class, wanted) == Some(true)
    {
        return Some(true);
    }
    (hierarchy.is_interface(wanted) == Some(false)).then_some(false)
}

pub(crate) const REGISTER_METHODS: Methods = Methods {
    to_string: |_, _| Ok(None),
    hash_code: |_, _| Ok(None),
    equals: |handle, other, _| {
        Ok(Some(
            matches!(other, Value::Host(other) if handle.same_object(other) == Some(true)),
        ))
    },
    instance_of,
    crosses_jars: |_, _| false,
    compare_to: None,
};

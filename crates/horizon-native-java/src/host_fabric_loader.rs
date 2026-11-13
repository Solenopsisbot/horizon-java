//! Fabric Loader's objects as host handles (#156 P6, first half): a
//! `ModContainer`, its `ModMetadata` and `ModOrigin`, and the metadata's
//! `Person`, `ContactInformation` and `CustomValue`s. Their behaviour stays
//! in `fabric_mods.rs`, `mod_origin.rs` and `lib.rs`; this module is their
//! keys and rows.
//!
//! These were text a jar `String` could spell: `mod-container/<id>/<version>`
//! (whose `toString` was that text), `metadata/<id>/<version>`,
//! `mod-origin/<kind>/<argument>`, and `fabric-person/<id><pointer>`,
//! `fabric-contact/…`, `fabric-cv/…` (a mod id and a JSON pointer joined by
//! `/`, split back at the first `/`).
//!
//! What Fabric Loader 0.19.5 does (bytecode, and the fixture's oracle on the
//! real classes): `getModContainer(id)` is `modMap.get(id)` and `getAllMods`
//! lists the same objects, so there is one `ModContainerImpl` per id; its
//! `getMetadata`, `getOrigin`, and the metadata's `getAuthors`, `getContact`
//! and `getCustomValue` hand back the objects they hold. So every handle here
//! is one object per key: canonical. None of these classes overrides
//! `equals`/`hashCode` (identity, and an identity hash: counted);
//! `ModContainerImpl.toString` is `String.format("%s %s", id, version)` and
//! `ModOriginImpl.toString` the origin's text, the rest `Object`'s (counted).
//!
//! Keys hold jar text (a mod id, a JSON pointer) only through
//! [`HostRef::of_values`]:
//!
//! * container: the id and its metadata's version (`null` when the host
//!   does not know it, then `toString` is counted);
//! * metadata: the id;
//! * origin: its kind (`path`, `nested`, `unknown`) and argument;
//! * person, contact, custom value: the id and the JSON pointer into its
//!   `fabric.mod.json`.
//!
//! The classes are implementations (`ModContainerImpl`, `V1ModMetadata`,
//! `BuiltinModMetadata`, `SimplePerson`, `ContactInformationImpl`,
//! `CustomValueImpl$…`, `ModOriginImpl`): [`ClassRule::Unknown`], so
//! `getClass()` is counted and never answered without the rest of the
//! `Class` surface.
//!
//! [`ClassRule::Unknown`]: crate::host_ref::ClassRule

use crate::host_ref::{HostKind, HostRef, Methods, Nested, instance_of_class};
use crate::{NativeError, Value};

const CONTAINER_TYPES: &[&str] = &[
    "net/fabricmc/loader/impl/ModContainerImpl",
    "net/fabricmc/loader/ModContainer",
    "net/fabricmc/loader/api/ModContainer",
];
const METADATA_TYPES: &[&str] = &["net/fabricmc/loader/api/metadata/ModMetadata"];
const ORIGIN_TYPES: &[&str] = &["net/fabricmc/loader/api/metadata/ModOrigin"];
const PERSON_TYPES: &[&str] = &["net/fabricmc/loader/api/metadata/Person"];
const CONTACT_TYPES: &[&str] = &["net/fabricmc/loader/api/metadata/ContactInformation"];
const CUSTOM_VALUE_TYPES: &[&str] = &["net/fabricmc/loader/api/metadata/CustomValue"];

pub(crate) fn container_types(_: &str) -> &'static [&'static str] {
    CONTAINER_TYPES
}
pub(crate) fn metadata_types(_: &str) -> &'static [&'static str] {
    METADATA_TYPES
}
pub(crate) fn origin_types(_: &str) -> &'static [&'static str] {
    ORIGIN_TYPES
}
pub(crate) fn person_types(_: &str) -> &'static [&'static str] {
    PERSON_TYPES
}
pub(crate) fn contact_types(_: &str) -> &'static [&'static str] {
    CONTACT_TYPES
}
pub(crate) fn custom_value_types(_: &str) -> &'static [&'static str] {
    CUSTOM_VALUE_TYPES
}

fn text(value: &str) -> Value {
    Value::Ref(Some(value.to_owned()))
}

fn strings(value: &Value, kind: HostKind) -> Option<Vec<Option<String>>> {
    match value {
        Value::Host(handle) if handle.kind() == kind => handle
            .values()?
            .into_iter()
            .map(|value| match value {
                Value::Ref(text) => Some(text),
                _ => None,
            })
            .collect(),
        _ => None,
    }
}

/// The container for mod `id`, whose metadata answers `version`.
pub(crate) fn container(id: &str, version: Option<&str>) -> Value {
    Value::Host(HostRef::of_values(
        HostKind::FabricModContainer,
        &[text(id), Value::Ref(version.map(str::to_owned))],
    ))
}

/// A container handle's mod id.
pub(crate) fn container_id(value: &Value) -> Option<String> {
    strings(value, HostKind::FabricModContainer)?
        .into_iter()
        .next()?
}

/// The metadata of mod `id` (`ModContainer.getMetadata`).
pub(crate) fn metadata(id: &str) -> Value {
    Value::Host(HostRef::of_values(HostKind::FabricModMetadata, &[text(id)]))
}

/// A metadata handle's mod id.
pub(crate) fn metadata_id(value: &Value) -> Option<String> {
    strings(value, HostKind::FabricModMetadata)?
        .into_iter()
        .next()?
}

/// A `ModOrigin` of `kind` (`path`, `nested`, `unknown`) over `argument`.
pub(crate) fn origin(kind: &str, argument: &str) -> Value {
    Value::Host(HostRef::of_values(
        HostKind::FabricModOrigin,
        &[text(kind), text(argument)],
    ))
}

/// An origin handle's kind and argument.
pub(crate) fn origin_parts(value: &Value) -> Option<(String, String)> {
    match strings(value, HostKind::FabricModOrigin)?.as_slice() {
        [Some(kind), Some(argument)] => Some((kind.clone(), argument.clone())),
        _ => None,
    }
}

/// What a JSON-pointer handle points into.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Pointed {
    Person,
    Contact,
    CustomValue,
}

impl Pointed {
    fn kind(self) -> HostKind {
        match self {
            Pointed::Person => HostKind::FabricPerson,
            Pointed::Contact => HostKind::FabricContact,
            Pointed::CustomValue => HostKind::FabricCustomValue,
        }
    }
}

/// The `Person`/`ContactInformation`/`CustomValue` at `pointer` in mod
/// `id`'s `fabric.mod.json`.
pub(crate) fn pointed(what: Pointed, id: &str, pointer: &str) -> Value {
    Value::Host(HostRef::of_values(what.kind(), &[text(id), text(pointer)]))
}

/// A pointer handle's mod id and pointer.
pub(crate) fn pointed_parts(value: &Value, what: Pointed) -> Option<(String, String)> {
    match strings(value, what.kind())?.as_slice() {
        [Some(id), Some(pointer)] => Some((id.clone(), pointer.clone())),
        _ => None,
    }
}

/// `Object.equals`: identity, which the canonical key decides.
fn identity_equals(
    handle: &HostRef,
    other: &Value,
    _: &mut dyn Nested,
) -> Result<Option<bool>, NativeError> {
    Ok(match other {
        Value::Host(other) => handle.same_object(other),
        _ => Some(false),
    })
}

const fn methods(
    to_string: fn(&HostRef, &mut dyn Nested) -> Result<Option<String>, NativeError>,
) -> Methods {
    Methods {
        to_string,
        hash_code: |_, _| Ok(None),
        equals: identity_equals,
        instance_of: instance_of_class,
        // A jar's own loader objects.
        crosses_jars: |_, _| false,
        compare_to: None,
    }
}

/// `ModContainerImpl.toString`: `String.format("%s %s", id, version)`.
fn container_text(handle: &HostRef, _: &mut dyn Nested) -> Result<Option<String>, NativeError> {
    Ok(
        match strings(&Value::Host(handle.clone()), HostKind::FabricModContainer).as_deref() {
            Some([Some(id), Some(version)]) => Some(format!("{id} {version}")),
            _ => None,
        },
    )
}

/// `ModOriginImpl.toString`: a `PATH` origin's root path (the virtual mod
/// directory, `mod_origin.rs`), a `NESTED` one's `parent:sublocation`, else
/// `unknown`.
fn origin_text(handle: &HostRef, _: &mut dyn Nested) -> Result<Option<String>, NativeError> {
    Ok(
        origin_parts(&Value::Host(handle.clone())).map(|(kind, argument)| match kind.as_str() {
            "path" => crate::mod_origin::root_path(&argument),
            "nested" => argument.replace('|', ":"),
            _ => "unknown".into(),
        }),
    )
}

pub(crate) const CONTAINER_METHODS: Methods = methods(container_text);
pub(crate) const ORIGIN_METHODS: Methods = methods(origin_text);
/// `Object.toString` (a class name and an identity hash): counted.
pub(crate) const OBJECT_METHODS: Methods = methods(|_, _| Ok(None));

#[cfg(test)]
mod tests {
    use super::*;

    /// A mod id and a JSON pointer (jar text) with every separator a layer
    /// uses read back as themselves, and two different pairs never make one
    /// key: the old `fabric-cv/<id><pointer>` text split at the first `/`, so
    /// an id with a `/` named another mod's value ("keys never parse jar
    /// text", #156).
    #[test]
    fn ids_and_pointers_round_trip_and_never_alias() {
        let alphabet = ["a", "/", ",", "\\", ":", "|", "~1", "r", "hfcv:", ""];
        let texts: Vec<String> = alphabet
            .iter()
            .flat_map(|a| alphabet.iter().map(move |b| format!("{a}{b}")))
            .collect();
        let mut seen = std::collections::BTreeMap::new();
        for (index, id) in texts.iter().enumerate() {
            let pointer = &texts[(index * 7 + 3) % texts.len()];
            for what in [Pointed::Person, Pointed::Contact, Pointed::CustomValue] {
                let value = pointed(what, id, pointer);
                assert_eq!(
                    pointed_parts(&value, what),
                    Some((id.clone(), pointer.clone()))
                );
            }
            assert_eq!(
                container_id(&container(id, Some(pointer))),
                Some(id.clone())
            );
            assert_eq!(metadata_id(&metadata(id)), Some(id.clone()));
            assert_eq!(
                origin_parts(&origin(id, pointer)),
                Some((id.clone(), pointer.clone()))
            );
            let Value::Host(handle) = pointed(Pointed::CustomValue, id, pointer) else {
                panic!("{id:?}")
            };
            let parts = (id.clone(), pointer.clone());
            if let Some(other) = seen.insert(handle.key().to_owned(), parts.clone()) {
                assert_eq!(other, parts);
            }
        }
    }
}

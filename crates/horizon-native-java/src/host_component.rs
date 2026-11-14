//! Chat components as host handles (#156 P5a): what `Component.literal`/`translatable`, a
//! `MutableComponent`'s `append`/`withStyle`/`copy` and the façade's `getDisplayName`s answer.
//! They were the text `component/literal/<text>`, `component/translatable/<key>` and
//! `component/json/<json>`, which a String could spell: `"component/literal/hi"` printed, sent and
//! compared as a component.
//!
//! **Key.** A component is its vanilla component JSON (`facade_text.rs`: `text`/`translate`+`with`,
//! style flags, `extra`), keyed by that JSON written with every object's keys sorted, so one
//! content is one key whatever order it was built in (and whatever `serde_json` features the game's
//! build unifies). JSON is read back with a JSON parser: jar text in it is escaped, never split.
//!
//! **Class.** NeoForge's and vanilla 26.3's `MutableComponent` (`javap -p`): `final`, a
//! package-private constructor, `implements Component` (`extends Message, FormattedText`). Every
//! component the model makes is one, so the class is exact (`host_ref::ClassSide`). It is a game
//! class a Mixin may target: an interface a loaded Mixin plan adds is `true`, another interface is
//! counted, a class other than its own and `Object` is a proved `false`.
//!
//! **Object methods** (`MutableComponent`, 26.3 bytecode): `equals` compares contents, style and
//! siblings, so two components with one key are equal. Different keys are a known "not equal" only
//! between two bare texts or two bare keys (the JSON's other forms, colours and translation
//! arguments, may print one value two ways); otherwise counted. `hashCode` combines the contents',
//! style's and siblings' hashes: counted. `toString` is the contents' text, then
//! `[style=…, siblings=…]` when there is either: exact for a bare text (`literal{hi}`) or a bare key
//! (`translation{key='k', args=[]}`), counted otherwise.
//!
//! **Identity.** Every factory call makes a new object (`Component.literal("a")` twice is two), so
//! the kind is never canonical: `==` on two handles is counted unless one handle flows to both sides.
//!
//! **Departure D1 (kept from the marker model, not new):** `append` and `withStyle` change the
//! receiver in place and return it (`MutableComponent.append` @1–12: `siblings.add(c); return
//! this`). The model answers a new component instead and leaves the receiver as it was, so a jar
//! that reads the receiver after changing it, or compares it with the result, gets main's answer,
//! not Java's. A per-allocation model with the component's state in a collected host table would
//! reach Java's; it is left for a follow-up.

use serde_json::{Map, Value as Json};

use crate::host_ref::{
    ClassHierarchy, ClassSide, HostKind, HostRef, Methods, Nested, exact_class_rules_out,
};
use crate::{NativeError, Value};

pub(crate) const MUTABLE: &str = "net/minecraft/network/chat/MutableComponent";
const COMPONENT: &str = "net/minecraft/network/chat/Component";
const MESSAGE: &str = "com/mojang/brigadier/Message";
const FORMATTED_TEXT: &str = "net/minecraft/network/chat/FormattedText";
const OBJECT: &str = "java/lang/Object";

/// `json` as text with every object's keys sorted: one content, one key.
fn canonical(json: &Json) -> String {
    match json {
        Json::Object(object) => {
            let mut keys: Vec<&String> = object.keys().collect();
            keys.sort();
            let fields: Vec<String> = keys
                .into_iter()
                .map(|key| {
                    format!(
                        "{}:{}",
                        Json::String(key.clone()),
                        canonical(&object[key.as_str()])
                    )
                })
                .collect();
            format!("{{{}}}", fields.join(","))
        }
        Json::Array(items) => format!(
            "[{}]",
            items.iter().map(canonical).collect::<Vec<_>>().join(",")
        ),
        other => other.to_string(),
    }
}

/// A new component of `json` (a fresh object, as every factory makes).
pub(crate) fn component(json: &Json) -> Value {
    Value::Host(HostRef::new(HostKind::Component, canonical(json)))
}

/// `Component.literal(text)`.
pub(crate) fn literal(text: &str) -> Value {
    component(&Json::Object(Map::from_iter([(
        "text".to_owned(),
        Json::String(text.to_owned()),
    )])))
}

/// `Component.translatable(key)`.
pub(crate) fn translatable(key: &str) -> Value {
    component(&Json::Object(Map::from_iter([(
        "translate".to_owned(),
        Json::String(key.to_owned()),
    )])))
}

/// The JSON of a component handle, `None` for any other value.
pub(crate) fn json_of(value: &Value) -> Option<Json> {
    match value {
        Value::Host(handle) if handle.kind() == HostKind::Component => {
            serde_json::from_str(handle.key()).ok()
        }
        _ => None,
    }
}

/// Whether `value` is a component handle.
pub(crate) fn is_component(value: &Value) -> bool {
    matches!(value, Value::Host(handle) if handle.kind() == HostKind::Component)
}

/// The one string field a bare component holds (`{"text": …}` or `{"translate": …}`).
fn bare(json: &Json, field: &str) -> Option<String> {
    match json {
        Json::Object(object) if object.len() == 1 => match object.get(field) {
            Some(Json::String(text)) => Some(text.clone()),
            _ => None,
        },
        _ => None,
    }
}

/// The text of a bare literal component (no style, no siblings).
pub(crate) fn bare_literal(value: &Value) -> Option<String> {
    bare(&json_of(value)?, "text")
}

/// The key of a bare translatable component (no arguments, style or siblings).
pub(crate) fn bare_key(value: &Value) -> Option<String> {
    bare(&json_of(value)?, "translate")
}

/// `MutableComponent.toString` where it follows from the JSON: `literal{text}` and
/// `translation{key='k', args=[]}` with no style and no siblings.
fn component_to_string(
    handle: &HostRef,
    _: &mut dyn Nested,
) -> Result<Option<String>, NativeError> {
    let Some(json) = serde_json::from_str::<Json>(handle.key()).ok() else {
        return Ok(None);
    };
    Ok(bare(&json, "text")
        .map(|text| format!("literal{{{text}}}"))
        .or_else(|| bare(&json, "translate").map(|key| format!("translation{{key='{key}', args=[]}}"))))
}

fn unknown_hash(_: &HostRef, _: &mut dyn Nested) -> Result<Option<i32>, NativeError> {
    Ok(None)
}

/// `MutableComponent.equals`: contents, style and siblings.
fn component_equals(
    handle: &HostRef,
    other: &Value,
    _: &mut dyn Nested,
) -> Result<Option<bool>, NativeError> {
    let Value::Host(other) = other else {
        return Ok(Some(false));
    };
    if other.kind() != HostKind::Component {
        return Ok(Some(false));
    }
    if handle == other {
        return Ok(Some(true));
    }
    let (Ok(a), Ok(b)) = (
        serde_json::from_str::<Json>(handle.key()),
        serde_json::from_str::<Json>(other.key()),
    ) else {
        return Ok(None);
    };
    let simple = |json: &Json| bare(json, "text").is_some() || bare(json, "translate").is_some();
    Ok((simple(&a) && simple(&b)).then_some(false))
}

/// `instanceof` for an object of exactly `MutableComponent`.
fn class_instance_of(class: &str, wanted: &str, hierarchy: &dyn ClassHierarchy) -> Option<bool> {
    if [OBJECT, MUTABLE, COMPONENT, MESSAGE, FORMATTED_TEXT].contains(&wanted)
        || hierarchy.is_instance(class, wanted) == Some(true)
    {
        return Some(true);
    }
    (hierarchy.is_interface(wanted) == Some(false) || exact_class_rules_out(class, wanted))
        .then_some(false)
}

/// A component is a game object of the jar's runtime (its IMC crossing is unchanged: the markers
/// did not cross either).
fn stays_in_its_jar(_: &HostRef, _: &dyn Fn(&Value) -> bool) -> bool {
    false
}

pub(crate) const COMPONENT_METHODS: Methods = Methods {
    to_string: component_to_string,
    hash_code: unknown_hash,
    equals: component_equals,
    instance_of: |_, wanted, hierarchy| class_instance_of(MUTABLE, wanted, hierarchy),
    crosses_jars: stays_in_its_jar,
    compare_to: None,
};

pub(crate) fn component_class(_: &str) -> &str {
    MUTABLE
}

pub(crate) const COMPONENT_CLASS_SIDE: ClassSide = ClassSide {
    owns: |class| class == MUTABLE,
    instance_of: class_instance_of,
    superclass: |_| Some(OBJECT.to_owned()),
    is_interface: |_| Some(false),
    classes: || vec![MUTABLE.to_owned()],
};

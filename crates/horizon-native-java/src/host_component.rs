//! Chat components as host handles (#156 P5a): what `Component.literal`/`translatable`, a
//! `MutableComponent`'s `append`/`withStyle`/`copy` and the façade's `getDisplayName`s answer.
//! They were the text `component/literal/<text>`, `component/translatable/<key>` and
//! `component/json/<json>`, which a String could spell: `"component/literal/hi"` printed, sent and
//! compared as a component.
//!
//! **Key.** `MutableComponent#<serial>:<json>`: a serial minted once per allocation,
//! process-wide, then the JSON the component was made with (vanilla's: `text`/`translate`+`with`,
//! style flags, `extra`), every object's keys sorted. The marker before the first `:` is the
//! runtime's own text; the JSON is read back with a JSON parser, so jar text in it is escaped,
//! never split.
//!
//! **State.** A `MutableComponent` changes in place: `append` adds a sibling and `withStyle`/
//! `withColor`/`setStyle` replace the style, each returning `this` (`MutableComponent.append`
//! @1–12: `siblings.add(c); return this`). A component changed once has a row in its host's
//! [`States`] (`StdHost`'s table, kept and collected with the resident heap like a mutable
//! collection's elements, `heap.rs`): its contents and style as JSON, and its appended siblings
//! and component arguments as the objects they are, so a sibling changed later shows in its
//! parent. Every reader (`getString`, `equals`, `toString`, a command's feedback, the client
//! surface) resolves the state when it reads, as Java does when it sends or prints; a host
//! without the table (a test host) counts the change instead of answering it.
//!
//! **Class.** NeoForge's and vanilla 26.3's `MutableComponent` (`javap -p`): `final`, a
//! package-private constructor, `implements Component` (`extends Message, FormattedText`). Every
//! component the model makes is one, so the class is exact (`host_ref::ClassSide`). It is a game
//! class a Mixin may target: an interface a loaded Mixin plan adds is `true`, another interface is
//! counted, a class other than its own and `Object` is a proved `false`.
//!
//! **Object methods** (`MutableComponent`, 26.3 bytecode): `equals` compares contents, style and
//! siblings, as they are now. A component equals itself; two trees of plain texts and bare keys
//! (no style or arguments, any siblings) are equal exactly when their JSON is. Anything else is
//! counted: the JSON prints some different values one way (a translation argument `1` and `1L`)
//! and some equal ones two ways (a colour by name or by value).
//! `hashCode` combines the contents', style's and siblings' hashes: counted. `toString` is the
//! contents' text, then `[style=…, siblings=…]` when there is either: exact for a bare text
//! (`literal{hi}`) or a bare key (`translation{key='k', args=[]}`), counted otherwise.
//!
//! **Identity.** Every factory call makes a new object (`Component.literal("a")` twice is two, and
//! `copy` another), and each gets its own serial: one key is one object (canonical), so `==` is
//! exact.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};

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

static NEXT_SERIAL: AtomicU64 = AtomicU64::new(0);

/// A component's longest resolved JSON: a callback appending in a loop cannot grow one without
/// end (the bound the marker model had).
pub(crate) const MAX_JSON_BYTES: usize = 64 * 1024;
/// How deep siblings and arguments resolve: deeper (or a cycle, `a.append(a)`, which Java
/// answers with a `StackOverflowError`) is unknown.
const MAX_DEPTH: usize = 64;

/// What a component holds once it has been changed in place: its contents and style as vanilla
/// JSON, then the siblings appended since and the translation arguments, as the objects they
/// are (a sibling changed later shows in its parent, as in Java).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ComponentState {
    /// Contents and style; its own `extra` (a component decoded from JSON) comes first.
    pub(crate) base: Map<String, Json>,
    /// Siblings appended in place (`MutableComponent.append`), each a component handle.
    pub(crate) siblings: Vec<Value>,
    /// The translation arguments as values, when one is a component (`with` resolves them).
    pub(crate) args: Option<Vec<Value>>,
}

/// The host's table of changed components, keyed by [`marker`].
pub type States = BTreeMap<String, ComponentState>;

/// A new component of `json` (a fresh object, as every factory makes).
pub(crate) fn component(json: &Json) -> Value {
    let serial = NEXT_SERIAL.fetch_add(1, Ordering::Relaxed);
    Value::Host(HostRef::new(
        HostKind::Component,
        format!("{MUTABLE}#{serial}:{}", canonical(json)),
    ))
}

/// The handle of `value` when it is a component.
fn handle_of(value: &Value) -> Option<&HostRef> {
    match value {
        Value::Host(handle) if handle.kind() == HostKind::Component => Some(handle),
        _ => None,
    }
}

/// The marker naming one component's allocation, `MutableComponent#<serial>`: its row in the
/// host's [`States`], and what the resident heap's collector finds in the key
/// (`heap.rs`). The serial is the runtime's digits, so the key splits at its first `:`.
pub(crate) fn marker(handle: &HostRef) -> &str {
    handle
        .key()
        .split_once(':')
        .map_or(handle.key(), |(marker, _)| marker)
}

/// The JSON a component was made with (after its marker).
fn key_json(key: &str) -> Option<Json> {
    let (_, json) = key.split_once(':')?;
    serde_json::from_str(json).ok()
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

/// Whether `value` is a component handle.
pub(crate) fn is_component(value: &Value) -> bool {
    handle_of(value).is_some()
}

/// A component's state now: its row in `states`, or what it was made with.
pub(crate) fn state_of(value: &Value, states: Option<&States>) -> Option<ComponentState> {
    let handle = handle_of(value)?;
    if let Some(state) = states.and_then(|states| states.get(marker(handle))) {
        return Some(state.clone());
    }
    let base = match key_json(handle.key())? {
        Json::Object(object) => object,
        Json::String(text) => Map::from_iter([("text".to_owned(), Json::String(text))]),
        other => Map::from_iter([("text".to_owned(), Json::String(other.to_string()))]),
    };
    Some(ComponentState {
        base,
        ..ComponentState::default()
    })
}

/// The vanilla JSON `value` stands for now, its siblings and component arguments as they are
/// now; `None` for any other value, or one nested past [`MAX_DEPTH`]. `argument` writes a
/// translation argument that is not a component.
pub(crate) fn json_in(
    value: &Value,
    states: Option<&States>,
    argument: &dyn Fn(&Value) -> Json,
) -> Option<Json> {
    fn resolve(
        value: &Value,
        states: Option<&States>,
        argument: &dyn Fn(&Value) -> Json,
        depth: usize,
    ) -> Option<Json> {
        if depth > MAX_DEPTH {
            return None;
        }
        let state = state_of(value, states)?;
        let mut object = state.base;
        if let Some(args) = &state.args {
            let mut with = Vec::with_capacity(args.len());
            for arg in args {
                with.push(if is_component(arg) {
                    resolve(arg, states, argument, depth + 1)?
                } else {
                    argument(arg)
                });
            }
            object.insert("with".into(), Json::Array(with));
        }
        if !state.siblings.is_empty() {
            let mut extra = match object.remove("extra") {
                Some(Json::Array(extra)) => extra,
                _ => Vec::new(),
            };
            for sibling in &state.siblings {
                extra.push(resolve(sibling, states, argument, depth + 1)?);
            }
            object.insert("extra".into(), Json::Array(extra));
        }
        Some(Json::Object(object))
    }
    resolve(value, states, argument, 0)
}

/// Change the component `receiver` in place: `change` its state, then keep it in `states`. The
/// error names a component grown past [`MAX_JSON_BYTES`] or nested past [`MAX_DEPTH`] (the
/// change is undone).
pub(crate) fn change(
    states: &mut States,
    receiver: &Value,
    argument: &dyn Fn(&Value) -> Json,
    change: impl FnOnce(&mut ComponentState),
) -> Result<(), NativeError> {
    let (Some(handle), Some(mut state)) = (handle_of(receiver), state_of(receiver, Some(states)))
    else {
        return Err(NativeError::Metadata("not a chat component".into()));
    };
    let marker = marker(handle).to_owned();
    let before = states.insert(marker.clone(), {
        change(&mut state);
        state
    });
    let size = json_in(receiver, Some(states), argument).map(|json| json.to_string().len());
    if size.is_none_or(|size| size > MAX_JSON_BYTES) {
        match before {
            Some(before) => states.insert(marker, before),
            None => states.remove(&marker),
        };
        return Err(NativeError::Metadata(format!(
            "a chat component grew past {MAX_JSON_BYTES} bytes or {MAX_DEPTH} levels"
        )));
    }
    Ok(())
}

/// `MutableComponent.copy()` (`new MutableComponent(contents, new ArrayList<>(siblings),
/// style)`) or, with `plain`, `plainCopy()` (the contents alone): a new object whose siblings
/// and arguments are the receiver's own objects.
pub(crate) fn copy(states: Option<&mut States>, receiver: &Value, plain: bool) -> Option<Value> {
    let mut state = state_of(receiver, states.as_deref())?;
    if plain {
        state
            .base
            .retain(|key, _| matches!(key.as_str(), "text" | "translate" | "with" | "fallback"));
        state.siblings.clear();
    }
    let made = component(&Json::Object(state.base.clone()));
    if let (Some(states), Some(handle)) = (states, handle_of(&made))
        && (!state.siblings.is_empty() || state.args.is_some())
    {
        states.insert(marker(handle).to_owned(), state);
    }
    Some(made)
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

/// Whether `json` is a component of literal texts and bare translation keys alone: each node an
/// object with one `text` or `translate` string and, optionally, an `extra` array of such nodes
/// (no style, arguments, fallback or other contents). Two such trees are equal in Java
/// (`MutableComponent.equals`: contents, an empty style, siblings in order) exactly when their
/// JSON is.
fn plain_tree(json: &Json) -> bool {
    let Json::Object(object) = json else {
        return false;
    };
    let contents = ["text", "translate"]
        .iter()
        .filter(|key| matches!(object.get(**key), Some(Json::String(_))))
        .count();
    contents == 1
        && object
            .keys()
            .all(|key| matches!(key.as_str(), "text" | "translate" | "extra"))
        && match object.get("extra") {
            None => true,
            Some(Json::Array(extra)) => extra.iter().all(plain_tree),
            Some(_) => false,
        }
}

/// The text of a bare literal component's JSON (no style, no siblings).
pub(crate) fn bare_literal(json: &Json) -> Option<String> {
    bare(json, "text")
}

/// The key of a bare translatable component's JSON (no arguments, style or siblings).
pub(crate) fn bare_key(json: &Json) -> Option<String> {
    bare(json, "translate")
}

/// `MutableComponent.toString` where it follows from the JSON: `literal{text}` and
/// `translation{key='k', args=[]}` with no style and no siblings.
fn component_to_string(
    handle: &HostRef,
    nested: &mut dyn Nested,
) -> Result<Option<String>, NativeError> {
    let Some(json) = nested.component_json(&Value::Host(handle.clone())) else {
        return Ok(None);
    };
    Ok(bare(&json, "text")
        .map(|text| format!("literal{{{text}}}"))
        .or_else(|| {
            bare(&json, "translate").map(|key| format!("translation{{key='{key}', args=[]}}"))
        }))
}

fn unknown_hash(_: &HostRef, _: &mut dyn Nested) -> Result<Option<i32>, NativeError> {
    Ok(None)
}

/// `MutableComponent.equals`: contents, style and siblings.
fn component_equals(
    handle: &HostRef,
    other: &Value,
    nested: &mut dyn Nested,
) -> Result<Option<bool>, NativeError> {
    let Some(other_handle) = handle_of(other) else {
        return Ok(Some(false));
    };
    if handle == other_handle {
        return Ok(Some(true));
    }
    let (Some(a), Some(b)) = (
        nested.component_json(&Value::Host(handle.clone())),
        nested.component_json(other),
    ) else {
        return Ok(None);
    };
    Ok((plain_tree(&a) && plain_tree(&b)).then_some(a == b))
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

#[cfg(test)]
#[path = "host_component_tests.rs"]
mod tests;

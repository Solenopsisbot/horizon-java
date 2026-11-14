//! Chat components that keep their style and translation arguments.
//!
//! The interpreter's first component model was text only: a literal's text
//! and a translatable's key. That is
//! enough to read a mod's name, but command feedback lost what made it
//! readable: `Component.translatable("commands.demo.set", value)` arrived as
//! the bare key with its `%s` unfilled, and `withStyle(ChatFormatting.RED)`
//! arrived white.
//!
//! A component is a host handle (`host_component.rs`) whose key is vanilla's
//! own component JSON
//! (`ComponentSerialization`: `text`/`translate`+`with`, `color`, `bold`,
//! `italic`, `underlined`, `strikethrough`, `obfuscated`, `extra`), so the game
//! hands it to the same decoder a server's chat packet goes through and a
//! vanilla client gets it unchanged.
//!
//! The calls answered here run in the interpreter
//! ([`Runtime::invoke_text`]) because two of them take a varargs array the
//! interpreter owns (`translatable(key, Object...)`,
//! `withStyle(ChatFormatting...)`):
//!
//! - `Component.translatable(key, args...)`, `translatableWithFallback(key,
//!   fallback, args...)`, `translatableEscape(key, args...)`: arguments as
//!   vanilla serialises them: a component as itself, a `String`, number or
//!   boolean as a JSON primitive, anything else as its text.
//! - `withStyle(ChatFormatting)`, `withStyle(ChatFormatting...)`,
//!   `withColor(int)`: vanilla's `Style.applyFormat` (a colour replaces the
//!   colour, a decoration sets its flag, `RESET` clears the style).
//! - `append(Component|String)`: a sibling in `extra`, which inherits the
//!   parent's style as in vanilla.
//! - `copy()`, `plainCopy()` (the root's contents only, unstyled) and
//!   `getString()` (the text with translation keys as they are: the
//!   interpreter has no language table, as before).
//!
//! A style object (`Style.EMPTY.withColor(...)`, `TextColor`) is not
//! modelled: those calls still reach the host's surface and refuse there.

use serde_json::{Map, Value as Json, json};

use crate::runtime::{Boxed, Host, MethodRef, NativeError, Runtime, Value, expect_ref, format_java_float};

const COMPONENT: &str = "net/minecraft/network/chat/Component";
const MUTABLE: &str = "net/minecraft/network/chat/MutableComponent";
const FORMATTING_PREFIX: &str = "enum/net/minecraft/ChatFormatting/";
const RETURNS_MUTABLE: &str = ")Lnet/minecraft/network/chat/MutableComponent;";

/// A component's JSON nests at most this deep and carries at most this many
/// bytes: a callback appending in a loop cannot grow a marker without end.
const MAX_JSON_BYTES: usize = 64 * 1024;

/// The vanilla JSON a component value stands for, or `None` when `value` is
/// not a component.
pub(crate) fn to_json(value: &Value) -> Option<Json> {
    crate::runtime::host_component::json_of(value)
}

/// A new component of `json`.
pub(crate) fn from_json(json: &Json) -> Value {
    crate::runtime::host_component::component(json)
}

/// `getString()`: every text in order, a translation as its key (the
/// interpreter has no language table).
pub(crate) fn plain(json: &Json) -> String {
    fn walk(json: &Json, out: &mut String) {
        match json {
            Json::String(text) => out.push_str(text),
            Json::Object(object) => {
                if let Some(Json::String(text)) = object.get("text") {
                    out.push_str(text);
                } else if let Some(Json::String(key)) = object.get("translate") {
                    out.push_str(key);
                }
                if let Some(Json::Array(extra)) = object.get("extra") {
                    extra.iter().for_each(|part| walk(part, out));
                }
            }
            _ => {}
        }
    }
    let mut out = String::new();
    walk(json, &mut out);
    out
}

/// One translation argument as vanilla's `TranslatableContents` keeps it:
/// a component as itself, a primitive as a JSON primitive, anything else
/// as its text.
fn argument(value: &Value) -> Json {
    if let Some(component) = to_json(value) {
        return component;
    }
    match value {
        Value::Int(v) => json!(v),
        Value::Long(v) => json!(v),
        Value::Float(v) => serde_json::Number::from_f64(*v)
            .map_or_else(|| json!(format_java_float(*v)), Json::Number),
        Value::Ref(None) => json!("null"),
        Value::Boxed(Boxed::Boolean(b)) => json!(b),
        Value::Boxed(boxed @ Boxed::Character(_)) => json!(boxed.text()),
        Value::Boxed(Boxed::Float(bits)) => {
            serde_json::Number::from_f64(f64::from(f32::from_bits(*bits))).map_or_else(
                || json!(format_java_float(f64::from(f32::from_bits(*bits)))),
                Json::Number,
            )
        }
        Value::Boxed(boxed) => json!(boxed.int()),
        Value::Ref(Some(text)) => json!(text),
        Value::Host(_) => json!(crate::runtime::value_text(value)),
        Value::Void => json!(""),
    }
}

/// `Style.applyFormat`: the JSON style keys one `ChatFormatting` sets.
fn apply_format(object: &mut Map<String, Json>, format: &str) -> Result<(), String> {
    const STYLE_KEYS: [&str; 6] = [
        "color",
        "bold",
        "italic",
        "underlined",
        "strikethrough",
        "obfuscated",
    ];
    let decoration = match format {
        "BOLD" => "bold",
        "ITALIC" => "italic",
        "UNDERLINE" => "underlined",
        "STRIKETHROUGH" => "strikethrough",
        "OBFUSCATED" => "obfuscated",
        "RESET" => {
            STYLE_KEYS.iter().for_each(|key| {
                object.remove(*key);
            });
            return Ok(());
        }
        colour => {
            const COLOURS: [&str; 16] = [
                "BLACK",
                "DARK_BLUE",
                "DARK_GREEN",
                "DARK_AQUA",
                "DARK_RED",
                "DARK_PURPLE",
                "GOLD",
                "GRAY",
                "DARK_GRAY",
                "BLUE",
                "GREEN",
                "AQUA",
                "RED",
                "LIGHT_PURPLE",
                "YELLOW",
                "WHITE",
            ];
            if !COLOURS.contains(&colour) {
                return Err(format!("ChatFormatting.{colour} is not a vanilla format"));
            }
            object.insert("color".into(), json!(colour.to_ascii_lowercase()));
            return Ok(());
        }
    };
    object.insert(decoration.into(), json!(true));
    Ok(())
}

fn formatting(value: &Value) -> Option<&str> {
    expect_ref(value)?.strip_prefix(FORMATTING_PREFIX)
}

fn bounded(json: Json) -> Result<Value, NativeError> {
    let value = from_json(&json);
    if matches!(&value, Value::Host(handle) if handle.key().len() > MAX_JSON_BYTES) {
        return Err(NativeError::Metadata(format!(
            "a chat component grew past {MAX_JSON_BYTES} bytes"
        )));
    }
    Ok(value)
}

/// A component as an object whose style keys can be set: a bare string
/// (a sibling `append(String)` added) becomes `{"text": ...}`.
fn object_of(json: Json) -> Map<String, Json> {
    match json {
        Json::Object(object) => object,
        Json::String(text) => Map::from_iter([("text".to_owned(), Json::String(text))]),
        other => Map::from_iter([("text".to_owned(), Json::String(other.to_string()))]),
    }
}

impl<H: Host> Runtime<'_, H> {
    /// The component calls this module answers; `None` for every other call
    /// (the host's text-only surface answers those).
    pub(crate) fn invoke_text(
        &mut self,
        r: &MethodRef,
        receiver: Option<&Value>,
        args: &[Value],
    ) -> Result<Option<Value>, NativeError> {
        if r.owner != COMPONENT && r.owner != MUTABLE {
            return Ok(None);
        }
        let name = r.name.as_str();
        let descriptor = r.descriptor.as_str();
        let Some(receiver) = receiver else {
            // Static factories with arguments.
            let (key, fallback, array) = match (name, args) {
                ("translatable" | "translatableEscape", [key, array])
                    if descriptor
                        == format!("(Ljava/lang/String;[Ljava/lang/Object;{RETURNS_MUTABLE}") =>
                {
                    (key, None, array)
                }
                ("translatableWithFallback", [key, fallback, array])
                    if descriptor
                        == format!(
                            "(Ljava/lang/String;Ljava/lang/String;[Ljava/lang/Object;{RETURNS_MUTABLE}"
                        ) =>
                {
                    (key, Some(fallback), array)
                }
                _ => return Ok(None),
            };
            let Some(key) = expect_ref(key) else {
                return Ok(None);
            };
            let with: Vec<Json> = self.array(array)?.iter().map(argument).collect();
            let mut object = Map::from_iter([("translate".to_owned(), json!(key))]);
            if let Some(fallback) = fallback.and_then(expect_ref) {
                object.insert("fallback".into(), json!(fallback));
            }
            if !with.is_empty() {
                object.insert("with".into(), Json::Array(with));
            }
            return bounded(Json::Object(object)).map(Some);
        };
        let Some(own) = to_json(receiver) else {
            return Ok(None);
        };
        let value = match (name, descriptor) {
            ("getString", "()Ljava/lang/String;") => Value::Ref(Some(plain(&own))),
            // A new object with the same contents (`MutableComponent.copy`).
            ("copy", "()Lnet/minecraft/network/chat/MutableComponent;") => from_json(&own),
            // The contents alone: no style, no siblings.
            ("plainCopy", "()Lnet/minecraft/network/chat/MutableComponent;") => {
                let mut object = object_of(own);
                object.retain(|key, _| {
                    matches!(key.as_str(), "text" | "translate" | "with" | "fallback")
                });
                bounded(Json::Object(object))?
            }
            (
                "withStyle",
                "(Lnet/minecraft/ChatFormatting;)Lnet/minecraft/network/chat/MutableComponent;",
            ) => {
                let Some(format) = args.first().and_then(formatting) else {
                    return Ok(None);
                };
                let mut object = object_of(own);
                apply_format(&mut object, format).map_err(NativeError::Metadata)?;
                bounded(Json::Object(object))?
            }
            (
                "withStyle",
                "([Lnet/minecraft/ChatFormatting;)Lnet/minecraft/network/chat/MutableComponent;",
            ) => {
                let Some(array) = args.first() else {
                    return Ok(None);
                };
                let formats = self.array(array)?.clone();
                let mut object = object_of(own);
                for format in &formats {
                    let Some(format) = formatting(format) else {
                        return Ok(None);
                    };
                    apply_format(&mut object, format).map_err(NativeError::Metadata)?;
                }
                bounded(Json::Object(object))?
            }
            ("withColor", "(I)Lnet/minecraft/network/chat/MutableComponent;") => {
                let Some(Value::Int(rgb)) = args.first() else {
                    return Ok(None);
                };
                let mut object = object_of(own);
                object.insert("color".into(), json!(format!("#{:06X}", rgb & 0xFF_FFFF)));
                bounded(Json::Object(object))?
            }
            (
                "append",
                "(Lnet/minecraft/network/chat/Component;)Lnet/minecraft/network/chat/MutableComponent;",
            )
            | ("append", "(Ljava/lang/String;)Lnet/minecraft/network/chat/MutableComponent;") => {
                let Some(sibling) = args.first() else {
                    return Ok(None);
                };
                let sibling = if descriptor.contains("Ljava/lang/String;") {
                    json!({ "text": expect_ref(sibling).cloned().unwrap_or_default() })
                } else {
                    match to_json(sibling) {
                        Some(sibling) => sibling,
                        None => return Ok(None),
                    }
                };
                let mut object = object_of(own);
                match object.get_mut("extra") {
                    Some(Json::Array(extra)) => extra.push(sibling),
                    _ => {
                        object.insert("extra".into(), Json::Array(vec![sibling]));
                    }
                }
                bounded(Json::Object(object))?
            }
            _ => return Ok(None),
        };
        if matches!(name, "withStyle" | "withColor" | "append") {
            // Departure D1 (`host_component.rs`): Java changes the receiver
            // and returns it; the model answers a new component and leaves the
            // receiver's handle as it was. A later read of the receiver, or
            // `==` between it and the answer, would differ from Java, so the
            // call is counted while its answer stands (`Host::count_departure`).
            self.host.count_departure(r);
        }
        Ok(Some(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_component_is_its_json_and_no_string_is_one() {
        let literal = from_json(&json!({"text": "hi"}));
        assert_eq!(
            crate::runtime::host_component::bare_literal(&literal).as_deref(),
            Some("hi")
        );
        let key = from_json(&json!({"translate": "demo.key"}));
        assert_eq!(
            crate::runtime::host_component::bare_key(&key).as_deref(),
            Some("demo.key")
        );
        // The old markers' spellings are Strings, not components.
        for text in [
            "component/literal/hi",
            "component/translatable/demo.key",
            "component/json/{\"text\":\"hi\"}",
        ] {
            assert_eq!(to_json(&Value::Ref(Some(text.into()))), None, "{text}");
        }
        let styled = from_json(&json!({"text": "hi", "color": "red"}));
        assert_eq!(
            to_json(&styled),
            Some(json!({"text": "hi", "color": "red"}))
        );
    }

    #[test]
    fn formats_apply_like_style_apply_format() {
        let mut object = Map::new();
        apply_format(&mut object, "BOLD").unwrap();
        apply_format(&mut object, "GOLD").unwrap();
        // A colour after a decoration keeps the decoration (applyFormat, not
        // the legacy `§` rule).
        assert_eq!(
            Json::Object(object.clone()),
            json!({"bold": true, "color": "gold"})
        );
        apply_format(&mut object, "RESET").unwrap();
        assert_eq!(Json::Object(object), json!({}));
    }

    #[test]
    fn arguments_serialise_as_vanilla_keeps_them() {
        assert_eq!(argument(&Value::Int(7)), json!(7));
        assert_eq!(argument(&Value::Boxed(Boxed::Integer(3))), json!(3));
        // A String spelled like the old box marker is text.
        assert_eq!(
            argument(&Value::Ref(Some("integer:3".into()))),
            json!("integer:3")
        );
        assert_eq!(argument(&Value::Ref(Some("Alex".into()))), json!("Alex"));
        assert_eq!(
            argument(&crate::runtime::host_component::literal("Alex")),
            json!({"text": "Alex"})
        );
        // A String spelled like the old component marker is a String argument.
        assert_eq!(
            argument(&Value::Ref(Some("component/literal/Alex".into()))),
            json!("component/literal/Alex")
        );
        assert_eq!(
            plain(&json!({"text": "a", "extra": ["b", {"translate": "k"}]})),
            "abk"
        );
    }
}

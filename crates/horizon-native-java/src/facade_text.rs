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

use crate::host_component::{ComponentState, MAX_JSON_BYTES, States};
use crate::{Boxed, Host, MethodRef, NativeError, Runtime, Value, expect_ref, format_java_float};

const COMPONENT: &str = "net/minecraft/network/chat/Component";
const MUTABLE: &str = "net/minecraft/network/chat/MutableComponent";
const FORMATTING_PREFIX: &str = "enum/net/minecraft/ChatFormatting/";
const RETURNS_MUTABLE: &str = ")Lnet/minecraft/network/chat/MutableComponent;";

/// The vanilla JSON a component holds now, its changed siblings and
/// arguments resolved through `states` (`host_component.rs`), or `None` when
/// `value` is not a component.
pub(crate) fn json_now(value: &Value, states: Option<&States>) -> Option<Json> {
    crate::host_component::json_in(value, states, &argument)
}

/// A new component of `json`.
pub(crate) fn from_json(json: &Json) -> Value {
    crate::host_component::component(json)
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

/// One translation argument that is not a component, as vanilla's
/// `TranslatableContents` serialises it: a primitive as a JSON primitive,
/// anything else as its text (a component resolves through its host).
pub(crate) fn argument(value: &Value) -> Json {
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
        Value::Host(_) => json!(crate::value_text(value)),
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
    if json.to_string().len() > MAX_JSON_BYTES {
        return Err(NativeError::Metadata(format!(
            "a chat component grew past {MAX_JSON_BYTES} bytes"
        )));
    }
    Ok(from_json(&json))
}

impl<H: Host> Runtime<'_, H> {
    /// The JSON `value` holds now, if it is a component.
    fn component_now(&self, value: &Value) -> Option<Json> {
        json_now(value, self.host.component_states())
    }

    /// Change the component `receiver` in place and answer it (`return
    /// this`); a host that keeps no component table counts the call.
    fn change_component(
        &mut self,
        r: &MethodRef,
        receiver: &Value,
        change: impl FnOnce(&mut ComponentState),
    ) -> Result<Option<Value>, NativeError> {
        let Some(states) = self.host.component_states_mut() else {
            return self.host.unknown_operand_call(r).map(Some);
        };
        crate::host_component::change(states, receiver, &argument, change)?;
        Ok(Some(receiver.clone()))
    }

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
            let values = self.array(array)?.clone();
            let with: Vec<Json> = values
                .iter()
                .map(|value| self.component_now(value).unwrap_or_else(|| argument(value)))
                .collect();
            let mut object = Map::from_iter([("translate".to_owned(), json!(key))]);
            if let Some(fallback) = fallback.and_then(expect_ref) {
                object.insert("fallback".into(), json!(fallback));
            }
            if !with.is_empty() {
                object.insert("with".into(), Json::Array(with));
            }
            let made = bounded(Json::Object(object.clone()))?;
            // `TranslatableContents` keeps its arguments: a component one
            // changed later shows in this one.
            if values.iter().any(crate::host_component::is_component)
                && let (Some(states), Value::Host(handle)) =
                    (self.host.component_states_mut(), &made)
            {
                states.insert(
                    crate::host_component::marker(handle).to_owned(),
                    ComponentState {
                        base: object,
                        siblings: Vec::new(),
                        args: Some(values),
                    },
                );
            }
            return Ok(Some(made));
        };
        if !crate::host_component::is_component(receiver) {
            return Ok(None);
        }
        match (name, descriptor) {
            ("getString", "()Ljava/lang/String;") => match self.component_now(receiver) {
                Some(json) => Ok(Some(Value::Ref(Some(plain(&json))))),
                // Nested past what resolves (Java: a cycle overflows).
                None => self.host.unknown_operand_call(r).map(Some),
            },
            // A new object: the same contents, style, siblings and arguments.
            ("copy", "()Lnet/minecraft/network/chat/MutableComponent;") => Ok(
                crate::host_component::copy(self.host.component_states_mut(), receiver, false),
            ),
            // The contents alone: no style, no siblings.
            ("plainCopy", "()Lnet/minecraft/network/chat/MutableComponent;") => Ok(
                crate::host_component::copy(self.host.component_states_mut(), receiver, true),
            ),
            (
                "withStyle",
                "(Lnet/minecraft/ChatFormatting;)Lnet/minecraft/network/chat/MutableComponent;",
            ) => {
                let Some(format) = args.first().and_then(formatting).map(str::to_owned) else {
                    return Ok(None);
                };
                apply_format(&mut Map::new(), &format).map_err(NativeError::Metadata)?;
                self.change_component(r, receiver, |state| {
                    let _ = apply_format(&mut state.base, &format);
                })
            }
            (
                "withStyle",
                "([Lnet/minecraft/ChatFormatting;)Lnet/minecraft/network/chat/MutableComponent;",
            ) => {
                let Some(array) = args.first() else {
                    return Ok(None);
                };
                let mut formats = Vec::new();
                for format in self.array(array)?.clone() {
                    let Some(format) = formatting(&format) else {
                        return Ok(None);
                    };
                    apply_format(&mut Map::new(), format).map_err(NativeError::Metadata)?;
                    formats.push(format.to_owned());
                }
                self.change_component(r, receiver, |state| {
                    for format in &formats {
                        let _ = apply_format(&mut state.base, format);
                    }
                })
            }
            ("withColor", "(I)Lnet/minecraft/network/chat/MutableComponent;") => {
                let Some(Value::Int(rgb)) = args.first() else {
                    return Ok(None);
                };
                let colour = json!(format!("#{:06X}", rgb & 0xFF_FFFF));
                self.change_component(r, receiver, |state| {
                    state.base.insert("color".into(), colour);
                })
            }
            (
                "append",
                "(Lnet/minecraft/network/chat/Component;)Lnet/minecraft/network/chat/MutableComponent;",
            )
            | ("append", "(Ljava/lang/String;)Lnet/minecraft/network/chat/MutableComponent;") => {
                let Some(sibling) = args.first() else {
                    return Ok(None);
                };
                // `append(String)` is `append(Component.literal(text))`.
                let sibling = if descriptor.contains("Ljava/lang/String;") {
                    crate::host_component::literal(expect_ref(sibling).map_or("", String::as_str))
                } else if crate::host_component::is_component(sibling) {
                    sibling.clone()
                } else {
                    return Ok(None);
                };
                self.change_component(r, receiver, |state| state.siblings.push(sibling))
            }
            _ => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_component_is_its_json_and_no_string_is_one() {
        let literal = json_now(&from_json(&json!({"text": "hi"})), None).unwrap();
        assert_eq!(
            crate::host_component::bare_literal(&literal).as_deref(),
            Some("hi")
        );
        let key = json_now(&from_json(&json!({"translate": "demo.key"})), None).unwrap();
        assert_eq!(
            crate::host_component::bare_key(&key).as_deref(),
            Some("demo.key")
        );
        // The old markers' spellings are Strings, not components.
        for text in [
            "component/literal/hi",
            "component/translatable/demo.key",
            "component/json/{\"text\":\"hi\"}",
        ] {
            assert_eq!(
                json_now(&Value::Ref(Some(text.into())), None),
                None,
                "{text}"
            );
        }
        let styled = from_json(&json!({"text": "hi", "color": "red"}));
        assert_eq!(
            json_now(&styled, None),
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

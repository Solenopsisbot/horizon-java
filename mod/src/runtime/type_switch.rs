//! `SwitchBootstraps.typeSwitch`, the call site of a pattern-matching
//! `switch` (`switch (value) { case Enum e -> ...; case Boolean b -> ... }`).
//!
//! The site answers the index of the first label at or after the restart
//! index that the value matches, `-1` for `null`, and the label count when
//! none matches, which the following `tableswitch` turns into the `default`
//! arm. Labels are the bootstrap's static arguments: a class (an `instanceof`
//! test), a `String` or an `Integer` (an equality test). A site with any other
//! label (an `EnumDesc` constant) is left on the interpreter's generic path.
//! ShulkerBoxTooltip builds its config readers through one such switch.

use super::*;

/// One `typeSwitch` label.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum SwitchLabel {
    Class(String),
    String(String),
    Int(i32),
}

impl ClassFile {
    /// The labels of an `invokedynamic` site bootstrapped by
    /// `SwitchBootstraps.typeSwitch`, or `None` for any other site.
    pub(crate) fn type_switch_labels(&self, index: u16) -> Option<Vec<SwitchLabel>> {
        let bootstrap = match self.pool.get(index as usize) {
            Some(Cp::InvokeDynamic { bootstrap, .. }) => {
                self.bootstrap_methods.get(*bootstrap as usize)?
            }
            _ => return None,
        };
        let handle = match self.pool.get(bootstrap.method_ref as usize) {
            Some(Cp::MethodHandle { reference_index }) => *reference_index,
            _ => return None,
        };
        let target = self.method_ref(handle).ok()?;
        if target.owner != "java/lang/runtime/SwitchBootstraps" || target.name != "typeSwitch" {
            return None;
        }
        bootstrap
            .arguments
            .iter()
            .map(|argument| match self.pool.get(*argument as usize)? {
                Cp::Class(_) => class_name(&self.pool, *argument)
                    .ok()
                    .map(SwitchLabel::Class),
                Cp::String(text) => match self.pool.get(*text as usize)? {
                    Cp::Utf8(text) => Some(SwitchLabel::String(text.clone())),
                    _ => None,
                },
                Cp::Integer(value) => Some(SwitchLabel::Int(*value)),
                _ => None,
            })
            .collect()
    }
}

impl<H: Host> Runtime<'_, H> {
    /// The index a `typeSwitch` site answers for `value`, starting at
    /// `restart`.
    pub(crate) fn type_switch_index(
        &mut self,
        labels: &[SwitchLabel],
        value: &Value,
        restart: i32,
    ) -> Result<i32, NativeError> {
        if *value == Value::Ref(None) {
            return Ok(-1);
        }
        let start = usize::try_from(restart).unwrap_or(0);
        for (index, label) in labels.iter().enumerate().skip(start) {
            let matched = match (label, value) {
                // An object as the `instanceof` opcode decides it: its class's
                // chain, and a "no" counted only where the opcode counts one
                // (a class outside what the runtime holds and Java 25's table).
                (SwitchLabel::Class(class), Value::Host(handle))
                    if crate::runtime::jar_object::is_object_handle(handle) =>
                {
                    self.switch_instance_of(value, class)
                        || self.instance_of_is_guess(value, class)
                            && self.handle_instance_of(handle, class)?
                }
                // A host handle by its kind's row (`host_ref.rs`), as the
                // `instanceof` opcode decides it.
                (SwitchLabel::Class(class), Value::Host(handle)) => {
                    class == "java/lang/Object" || self.handle_instance_of(handle, class)?
                }
                (SwitchLabel::Class(class), _) => self.switch_instance_of(value, class),
                (SwitchLabel::String(text), _) => {
                    expect_ref(value).is_some_and(|marker| marker == text)
                }
                (SwitchLabel::Int(expected), _) => boxed_int(value) == Some(*expected),
            };
            if matched {
                return Ok(index as i32);
            }
        }
        Ok(labels.len() as i32)
    }

    /// `value instanceof class` for a switch label: jar objects through their
    /// hierarchy, boxed and string values through their JDK class, and game
    /// enum constants as `Enum`s.
    fn switch_instance_of(&self, value: &Value, class: &str) -> bool {
        if class == "java/lang/Object" {
            return true;
        }
        // A handle is what its kind's row says (`host_ref.rs`): an array is
        // an `Object[]` by JLS array subtyping, not by a class chain.
        // An object's "yes" is its class's chain; any other answer goes on
        // to the paths below.
        if let Value::Host(handle) = value {
            let row = handle.instance_of(class, self) == Some(true);
            if row || !crate::runtime::jar_object::is_object_handle(handle) {
                return row;
            }
        }
        if let Some(own) = class_of_value(value)
            && (own == class || self.class_is_instance(&own, class))
        {
            return true;
        }
        let Some(marker) = expect_ref(value) else {
            return false;
        };
        if class == "java/lang/Enum" && marker.starts_with("enum/") {
            return true;
        }
        // A string is a bare marker: no object, boxed, collection, array,
        // callback, enum or game prefix claims it. A `#` claims it only as an
        // object's `class#n` (`[I#0` is a string: arrays are handles).
        if class == "java/lang/String" || class == "java/lang/CharSequence" {
            let claimed = class_of_value(value).is_some()
                || marker.contains('#') && !value_class::is_plain_string(marker)
                || ["@", "enum/", "game/"]
                    .iter()
                    .any(|prefix| marker.starts_with(prefix));
            return !claimed;
        }
        self.object_is_instance_of(marker, class)
    }
}

/// An `int` a switch value carries, boxed or not.
fn boxed_int(value: &Value) -> Option<i32> {
    match value {
        Value::Int(value) | Value::Boxed(crate::runtime::Boxed::Integer(value)) => Some(*value),
        _ => None,
    }
}

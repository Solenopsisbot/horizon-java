//! `ArrayStoreException`: a reference stored into an array must be an
//! instance of the array's component type, as JDK 25's HotSpot checks it.
//!
//! Every array handle carries its exact class (`host_array.rs`), so an
//! `Object[]`-typed store into a `String[]` can now be checked. Before, the
//! store went through silently, and `toArray(new String[0])`/`copyOf(...,
//! String[].class)` handed back a "`String[]`" holding an `Integer`.
//!
//! Messages (checked on JDK 25.0.5):
//!
//! * `aastore`: the stored value's class name (`java.lang.Integer`, `[J`);
//! * `System.arraycopy` inside `ArrayList.toArray(T[])` and `Arrays.copyOf(..,
//!   newType)`: when the destination component is a subtype of the source
//!   component, `arraycopy: element type mismatch: can not cast one of the
//!   elements of java.lang.Object[] to the type of the destination array,
//!   java.lang.String`, after copying the elements before the first misfit;
//!   when the two components are unrelated, `arraycopy: type mismatch: can
//!   not copy java.lang.Integer[] into java.lang.String[]`.
//!
//! A store the runtime cannot decide is treated as `checkcast` treats the
//! same cast: against a class the runtime holds it is counted as `<array
//! class>#<aastore>(Ljava/lang/Object;)V` (and stored); against a JDK or game
//! component (a host model's `OpenOption` constant into an `OpenOption[]`)
//! the value is stored as before, uncounted.

use super::*;

fn array_store_exception(message: String) -> NativeError {
    NativeError::UncaughtException {
        class: "java/lang/ArrayStoreException".into(),
        message: Some(message),
        frame: None,
    }
}

/// A class's binary name as Java prints it (`java.lang.Integer`, `[J`,
/// `[Ljava.lang.Object;`).
fn binary_name(class: &str) -> String {
    class.replace('/', ".")
}

/// The element type as `arraycopy`'s messages print it: a class's binary
/// name, or an array class's descriptor (`[J`).
fn element_name(component: &str) -> String {
    binary_name(component)
}

/// Every type an object of a final `java.lang` value class is (JDK 25's
/// `getSuperclass`/`getInterfaces` closure): complete, so a store of one of
/// these into any other component type is a known misfit.
fn final_value_supertypes(class: &str) -> Option<&'static [&'static str]> {
    const NUMBER: &[&str] = &[
        "java/lang/Number",
        "java/lang/Comparable",
        "java/io/Serializable",
        "java/lang/constant/Constable",
        "java/lang/constant/ConstantDesc",
    ];
    const SMALL_NUMBER: &[&str] = &[
        "java/lang/Number",
        "java/lang/Comparable",
        "java/io/Serializable",
        "java/lang/constant/Constable",
    ];
    const VALUE: &[&str] = &[
        "java/lang/Comparable",
        "java/io/Serializable",
        "java/lang/constant/Constable",
    ];
    Some(match class {
        "java/lang/String" => crate::runtime::value_class::STRING_TYPES,
        "java/lang/Integer" | "java/lang/Long" | "java/lang/Float" | "java/lang/Double" => NUMBER,
        "java/lang/Short" | "java/lang/Byte" => SMALL_NUMBER,
        "java/lang/Boolean" | "java/lang/Character" => VALUE,
        _ => return None,
    })
}

/// The last store verdict for a jar object's class against a component.
pub(crate) struct StoreVerdict {
    class: String,
    component: String,
    fits: Option<bool>,
}

/// The final `java.lang` classes an array component can be: an array of one
/// holds only that exact class.
const FINAL_COMPONENTS: &[&str] = &[
    "java/lang/String",
    "java/lang/Integer",
    "java/lang/Long",
    "java/lang/Short",
    "java/lang/Byte",
    "java/lang/Character",
    "java/lang/Boolean",
    "java/lang/Float",
    "java/lang/Double",
    "java/lang/Class",
];

/// The boxes: no text value is ever one (boxes are `Value::Boxed`).
fn is_box(class: &str) -> bool {
    FINAL_COMPONENTS[1..9].contains(&class)
}

/// Whether an object of the final value class `class` is a `component`.
fn value_class_fits(class: &str, component: &str) -> bool {
    class == component
        || final_value_supertypes(class).is_some_and(|supertypes| supertypes.contains(&component))
}

impl<H: Host> Runtime<'_, H> {
    /// Whether `value` may be stored into an array of `component` (an
    /// internal name, or a descriptor for an array component): `None` when
    /// the runtime cannot tell. Runs on every `aastore`, so the common
    /// answers come first and nothing is allocated on the way.
    ///
    /// Known: a handle (its kind's row); any other value against an array
    /// component (only handles are arrays); a `String` or box (its whole
    /// type); an object whose class the runtime knows (a jar object, a JDK
    /// object a jar constructed, an enum constant) against a final
    /// component (only that class fits), a component it is, or a component
    /// the runtime can rule out (a jar class whose chain ends at `Object`,
    /// `Object` itself); a lambda against a final component.
    pub(crate) fn store_fits(&self, component: &str, value: &Value) -> Option<bool> {
        if *value == Value::Ref(None) || component == "java/lang/Object" {
            return Some(true);
        }
        let text = match value {
            Value::Host(handle) if !crate::runtime::jar_object::is_object_handle(handle) => {
                return handle.instance_of(component, self);
            }
            _ if component.starts_with('[') => return Some(false),
            // An object: by its class's chain, below (`jar_object.rs`).
            Value::Host(handle) => handle.key(),
            Value::Ref(Some(text)) => text.as_str(),
            Value::Boxed(boxed) => return Some(value_class_fits(boxed.class(), component)),
            other => return Some(value_class_fits(&class_of_value(other)?, component)),
        };
        // A store loop: the same jar object's class into the same component
        // as last time (only a held class's verdict is kept).
        if let Some(class) = crate::runtime::jar_object::packaged_object_class(value) {
            if class == component {
                return Some(true);
            }
            if let Some(cached) = self.store_verdict.borrow().as_ref()
                && cached.class == class
                && cached.component == component
            {
                return cached.fits;
            }
        }
        if let Some(class) = self.marker_class(value, text) {
            if class == component {
                return Some(true);
            }
            if FINAL_COMPONENTS.contains(&component) {
                return Some(false);
            }
            if self.class(class).is_some() {
                // A jar object: its chain, trusted for a "no" only when it
                // ends at `Object` (a JDK superclass's interfaces are not all
                // known here). The answer depends on the two classes only.
                let fits = match self.known_instance_of(value, component) {
                    Some(false) if !self.chain_ends_at_object(class) => None,
                    fits => fits,
                };
                *self.store_verdict.borrow_mut() = Some(StoreVerdict {
                    class: class.to_owned(),
                    component: component.to_owned(),
                    fits,
                });
                return fits;
            }
            // A JDK object: `Object` implements nothing; any other is only
            // known where Java 25's tables say "yes".
            return match class {
                "java/lang/Object" => Some(false),
                _ => crate::runtime::jdk_model::class_instance_of(class, component).then_some(true),
            };
        }
        if text.starts_with("@indy:") {
            // A lambda's hidden class is no `String`, box or `Class`.
            if FINAL_COMPONENTS.contains(&component) {
                return Some(false);
            }
            self.class(component)?;
            return self.known_instance_of(value, component);
        }
        // A `String`, a `Class` (text until #156 P13: `int.class` is "int")
        // or a host marker whose class is unknown. A `String`'s components
        // fit, or this is undecided either way.
        if is_box(component) {
            return Some(false);
        }
        // Only a `Ref` is text; an object is no `String`.
        let string = matches!(value, Value::Ref(_)) && value_class::is_plain_string(text);
        if value_class_fits("java/lang/String", component) {
            // A `String` fits; any other text is undecided.
            return string.then_some(true);
        }
        if string && !self.text_may_be_class(text) {
            return Some(false);
        }
        self.class(component)?;
        self.known_instance_of(value, component)
    }

    /// The class of a marker the runtime knows exactly: a jar object or a
    /// JDK object a jar constructed (`class#n`), or a host enum constant
    /// (`enum/<class>/<NAME>`, the class `getClass()` answers for it).
    fn marker_class<'t>(&self, value: &'t Value, text: &'t str) -> Option<&'t str> {
        if let Some(rest) = text.strip_prefix("enum/") {
            return rest.rsplit_once('/').map(|(class, _)| class);
        }
        let class = crate::runtime::jar_object::packaged_object_class(value)?;
        (self.class(class).is_some() || class.starts_with("java/")).then_some(class)
    }

    /// Whether a text value may be a `Class` (a class name, a primitive's,
    /// an array descriptor), since `Class` objects are still text.
    fn text_may_be_class(&self, text: &str) -> bool {
        class_marker_like(text, self.classes, &self.linked)
    }

    /// Whether every superclass of the held class `class` is held, up to
    /// `java/lang/Object`: then its whole type is the runtime's to decide.
    fn chain_ends_at_object(&self, class: &str) -> bool {
        let mut current = class;
        for _ in 0..MAX_SUPERCLASS_WALK {
            let Some(file) = self.class(current) else {
                return current == "java/lang/Object";
            };
            if file.super_name.is_empty() || file.super_name == current {
                return false;
            }
            current = &file.super_name;
        }
        false
    }

    /// Whether an undecided store into an array of `component` is counted:
    /// as `checkcast` counts the same cast, only against a class the runtime
    /// holds. A JDK or game component keeps the store as before (the host
    /// models build their values, such as an `OpenOption` constant, by
    /// marker).
    fn store_counted(&self, component: &str) -> bool {
        self.class(component.trim_start_matches('[')).is_some()
            || component
                .trim_start_matches('[')
                .strip_prefix('L')
                .and_then(|name| name.strip_suffix(';'))
                .is_some_and(|name| self.class(name).is_some())
    }

    /// The exact class of a stored value, for an `ArrayStoreException`.
    fn stored_class(&self, value: &Value) -> Option<String> {
        match value {
            // A shared empty array's exact class is unknown.
            Value::Host(_) if host_array::array_key(value).is_some() => {
                host_array::exact_class(value).map(str::to_owned)
            }
            Value::Host(handle) => handle.class().map(str::to_owned),
            Value::Ref(Some(text)) => match self.marker_class(value, text) {
                Some(class) => Some(class.to_owned()),
                // A text is a `String` unless it may be a `Class`.
                None if value_class::is_plain_string(text) && !self.text_may_be_class(text) => {
                    Some("java/lang/String".to_owned())
                }
                None => None,
            },
            other => self.exact_class(other),
        }
    }

    /// Count a store whose type check the runtime cannot decide.
    fn count_store(&mut self, array_class: &str) -> Result<(), NativeError> {
        self.host
            .unknown_operand_call(&MethodRef {
                owner: array_class.to_owned(),
                name: "<aastore>".into(),
                descriptor: "(Ljava/lang/Object;)V".into(),
            })
            .map(|_| ())
    }

    /// `aastore`'s type check of `value` into `array` (after its bounds
    /// check): `Ok` to store (also after counting an undecided check), or
    /// Java's `ArrayStoreException`.
    pub(crate) fn check_array_store(
        &mut self,
        array: &Value,
        value: &Value,
    ) -> Result<(), NativeError> {
        let Some(class) = host_array::exact_class(array) else {
            return Ok(());
        };
        let Some(component) = host_array::component_of(class) else {
            return Ok(());
        };
        match self.store_fits(component, value) {
            Some(true) => Ok(()),
            Some(false) => match self.stored_class(value) {
                Some(stored) => Err(array_store_exception(binary_name(&stored))),
                None => self.count_store(class),
            },
            None if self.store_counted(component) => self.count_store(class),
            None => Ok(()),
        }
    }

    /// The first of `values` that cannot be stored into an array of
    /// `component`: `Ok(None)` when all fit, counting each undecided one.
    pub(crate) fn first_misfit(
        &mut self,
        array_class: &str,
        component: &str,
        values: &[Value],
    ) -> Result<Option<usize>, NativeError> {
        for (index, value) in values.iter().enumerate() {
            match self.store_fits(component, value) {
                Some(true) => {}
                Some(false) => return Ok(Some(index)),
                None if self.store_counted(component) => self.count_store(array_class)?,
                None => {}
            }
        }
        Ok(None)
    }

    /// `System.arraycopy`'s exception for an element of a `source` array (of
    /// `source` component) that does not fit `destination`'s component;
    /// `None` when the components' relation is unknown.
    pub(crate) fn arraycopy_store_exception(
        &self,
        source: &str,
        destination: &str,
    ) -> Option<NativeError> {
        Some(array_store_exception(
            if host_array::class_assignable(destination, source, self)? {
                format!(
                    "arraycopy: element type mismatch: can not cast one of the elements of {}[] \
                     to the type of the destination array, {}",
                    element_name(source),
                    element_name(destination)
                )
            } else {
                format!(
                    "arraycopy: type mismatch: can not copy {}[] into {}[]",
                    element_name(source),
                    element_name(destination)
                )
            },
        ))
    }
}

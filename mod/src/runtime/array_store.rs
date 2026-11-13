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

/// `java.lang.Class` and every type it is (JDK 25).
const CLASS_TYPES: &[&str] = &[
    "java/lang/Class",
    "java/io/Serializable",
    "java/lang/constant/Constable",
    "java/lang/invoke/TypeDescriptor",
    "java/lang/invoke/TypeDescriptor$OfField",
    "java/lang/reflect/AnnotatedElement",
    "java/lang/reflect/GenericDeclaration",
    "java/lang/reflect/Type",
];

impl<H: Host> Runtime<'_, H> {
    /// Whether `value` may be stored into an array of `component` (an
    /// internal name, or a descriptor for an array component): `None` when
    /// the runtime cannot tell.
    ///
    /// Known: a handle (its kind's row), any other value against an array
    /// component (only handles are arrays), a `String` or box (its whole
    /// type is known), and a jar object against a component whose chain the
    /// runtime holds.
    pub(crate) fn store_fits(&self, component: &str, value: &Value) -> Option<bool> {
        if *value == Value::Ref(None) || component == "java/lang/Object" {
            return Some(true);
        }
        if let Value::Host(handle) = value {
            return handle.instance_of(component, self);
        }
        if component.starts_with('[') {
            return Some(false);
        }
        let Some(class) = self.exact_class(value) else {
            // A lambda or another marker: `checkcast`'s verdict against a
            // class the runtime holds (a lambda is its functional interface).
            self.class(component)?;
            return self.known_instance_of(value, component);
        };
        if let Some(supertypes) = final_value_supertypes(&class) {
            let fits = class == component || supertypes.contains(&component);
            // A `Class` is still text until #156 P13 (`int.class` is `"int"`):
            // a text that does not fit as a `String` may be a `Class` that does.
            if !fits && matches!(value, Value::Ref(Some(_))) && CLASS_TYPES.contains(&component) {
                return None;
            }
            return Some(fits);
        }
        // A jar object: `checkcast`'s own verdict.
        self.class(component)?;
        self.known_instance_of(value, component)
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
        let (Some(class), Some(component)) = (
            host_array::exact_class(array),
            value_class::array_component(array),
        ) else {
            return Ok(());
        };
        match self.store_fits(&component, value) {
            Some(true) => Ok(()),
            Some(false) => match self.stored_class(value) {
                Some(stored) => Err(array_store_exception(binary_name(&stored))),
                None => self.count_store(class),
            },
            None if self.store_counted(&component) => {
                let class = class.to_owned();
                self.count_store(&class)
            }
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

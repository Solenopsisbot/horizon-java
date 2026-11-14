//! `Object`'s members on a JDK object a jar built with `new` (#217).
//!
//! A jar's `new BigDecimal("1.0")` is a jar object (`java/math/BigDecimal#0`) of a class the
//! runtime holds no bytecode for. A host model answers the members it models (`File`, a map
//! entry, `StringBuilder.toString`...). What no model answered used to fall back to `Object`'s
//! defaults: `equals` by identity (`List.of(new BigDecimal("1.0")).contains(new
//! BigDecimal("1.0"))` was `false`; Java says `true`) and an explicit `toString()` printing the
//! object's key. Those answers are Java's only for a class that keeps `Object`'s member.
//!
//! Java 25's table (`jdk_supertypes_25.txt`, `jdk_closure.rs`) records, for every public class of
//! every package the system modules export (all a jar can build or extend), which of
//! `equals`/`hashCode`/`toString`/`compareTo` it declares or inherits from a JDK superclass below
//! `Object`. Where no model answered:
//!
//! * `x.equals(x)` on the very same object is `true`: every JDK `equals` is reflexive.
//! * Otherwise `equals` is identity only for a class the table shows keeping `Object.equals`
//!   (`new Object()`, `Random`, `StringBuilder`, `AtomicInteger`, a jar's `extends Handler`);
//!   for any other, and for a class the table does not hold (an incubator module's), the JDK
//!   skeleton's body or the host's model of the class's `equals` answers, or the call is
//!   counted.
//! * `toString` is counted for every jar object: `Object.toString` prints an identity hash no
//!   run reproduces, and an override is a body the host does not have.
//! * `hashCode` was already counted (`object_methods.rs`'s `host_value_hash` has no hash for a
//!   jar object).
//! * natural order (`TreeSet`, `Collections.sort`) of a class the table shows is no
//!   `Comparable` throws Java's `ClassCastException`; a `Comparable` one, or an unlisted one,
//!   is counted.
//!
//! A jar class that extends a JDK class and declares none of these runs the JDK class's
//! members: its chain is followed to the class it leaves the jar at.

use crate::{Host, Runtime, Value};

/// One of `Object`'s members a JDK class may override, or `Comparable.compareTo`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Member {
    Equals,
    CompareTo,
}

impl Member {
    fn name(self) -> &'static str {
        match self {
            Member::Equals => "equals",
            Member::CompareTo => "compareTo",
        }
    }

    /// Its letter in the table's `object` rows.
    fn letter(self) -> char {
        match self {
            Member::Equals => 'e',
            Member::CompareTo => 'c',
        }
    }
}

/// Whether JDK class `class` runs a JDK body of `member` rather than `Object`'s (for
/// `compareTo`: whether it is `Comparable`), as Java 25 reports it. `None` for a class the
/// table does not list.
pub(crate) fn overrides(class: &str, member: Member) -> Option<bool> {
    match crate::jdk_closure::class(class) {
        Some(row) => (!row.is_interface).then(|| row.overrides.contains(&member.name())),
        // Every other public class of an exported package (`java.util.logging.Handler`).
        None => crate::jdk_closure::object_members(class)
            .map(|letters| letters.contains(member.letter())),
    }
}

/// Superclass walks are bounded like every other chain walk.
const MAX_CHAIN: usize = 64;

impl<H: Host> Runtime<'_, H> {
    /// The JDK class a jar object's superclass chain leaves the jar at: its own class for a
    /// `new BigDecimal(..)`, otherwise the first superclass the runtime does not interpret
    /// (no bytecode, or a class the host or a library model answers for). `None` for a value
    /// that is no jar object, or whose chain leaves the jar at a class outside the JDK's
    /// packages (a game class, a modelled library).
    pub(crate) fn jdk_base_class(&self, value: &Value) -> Option<String> {
        let mut current = crate::jar_object::object_class(value)?.to_owned();
        for _ in 0..MAX_CHAIN {
            let interpreted = !self.host.models_class(&current)
                && !crate::jvm_dispatch::library_modelled(&current);
            match self.class(&current).filter(|_| interpreted) {
                Some(file) if !file.super_name.is_empty() => current = file.super_name.clone(),
                Some(_) => return None,
                None => {
                    return crate::jdk_closure::in_jdk_package(&current).then_some(current);
                }
            }
        }
        None
    }
}

#[cfg(test)]
#[path = "jdk_object_methods_tests.rs"]
mod tests;

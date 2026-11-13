//! The JDK's spliterators over jar collections, and streams over any
//! spliterator, as JDK 25 runs them.
//!
//! A stream over a jar's data starts at a `Spliterator`: `Collection.stream()`
//! is `StreamSupport.stream(spliterator(), false)`, Guava's
//! `ImmutableCollection.spliterator()` is
//! `Spliterators.spliterator(this, flags)`, fastutil hands out spliterators
//! of its own, and service code extends `Spliterators.AbstractSpliterator`.
//! The runtime had no model of any of them: `Spliterators.spliterator` over
//! a jar collection was counted, and `StreamSupport.stream` drained a jar
//! spliterator eagerly when the stream was made.
//!
//! This module writes the JDK classes from their specification (each body's
//! calls checked against JDK 25.0.5's behaviour with a logging probe, never
//! copied):
//!
//! * **`IteratorSpliterator`** (`Spliterators.spliterator(Collection, int)`,
//!   `(Iterator, long, int)`, `spliteratorUnknownSize(Iterator, int)`, and the
//!   `Collection`/`Set`/`SortedSet`/`Iterable` defaults): it asks the
//!   collection for `iterator()` and `size()` only at the first traversal or
//!   size query, `forEachRemaining` hands the action to the iterator's own
//!   `forEachRemaining`, and `estimateSize` stays the first `size()`.
//! * **`AbstractList.RandomAccessSpliterator`** (`List.spliterator()` of a
//!   `RandomAccess` list): `size()` once at the first use, `get(i)` per
//!   element, an `AbstractList`'s `modCount` checked after each advance.
//! * **`Spliterators.AbstractSpliterator`**'s state (`est`, the flags) for a
//!   jar subclass, and the `Spliterator` defaults (`forEachRemaining` as a
//!   `tryAdvance` loop, `getExactSizeIfKnown`, `hasCharacteristics`,
//!   `getComparator`) for any jar spliterator.
//! * **`StreamSupport.stream(spliterator, parallel)`**: reads
//!   `characteristics()` (and `getComparator()` when `SORTED`) when the stream
//!   is made, as `StreamOpFlag.fromCharacteristics` does; the elements are
//!   pulled lazily by the terminal (`stream_pipeline.rs`).
//!
//! `trySplit` on a modelled spliterator is counted: a sequential stream never
//! splits, and the JDK's batch sizes are not modelled.

use super::*;
use crate::runtime::host_ref::{HostKind, HostRef, Methods};
use crate::runtime::jdk_call::{Site, throw};

/// A modelled spliterator is a `HostKind::Spliterator` handle keyed
/// `iter:<n>` (`Spliterators.IteratorSpliterator`), `sorted:<n>`
/// (`SortedSet.spliterator()`'s anonymous `SortedSet$1`, an
/// `IteratorSpliterator` subclass) or `ra:<n>`
/// (`AbstractList.RandomAccessSpliterator`); the key carries the class.
pub(crate) fn spliterator_class(key: &str) -> &str {
    if key.starts_with("ra:") {
        "java/util/AbstractList$RandomAccessSpliterator"
    } else if key.starts_with("sorted:") {
        "java/util/SortedSet$1"
    } else {
        ITERATOR_SPLITERATOR
    }
}

const ITERATOR_SPLITERATOR: &str = "java/util/Spliterators$IteratorSpliterator";

/// The modelled spliterators' `Object` members: identity `equals`, the
/// identity hash in `toString`/`hashCode` (counted). The classes implement
/// `Spliterator` and nothing else; `SortedSet$1` extends
/// `IteratorSpliterator` (JDK 25.0.5's `javap`).
pub(crate) const JDK_SPLITERATOR: Methods = Methods {
    to_string: |_, _| Ok(None),
    hash_code: |_, _| Ok(None),
    equals: |handle, other, _| {
        Ok(match other {
            Value::Host(other) => handle.same_object(other),
            _ => Some(false),
        })
    },
    instance_of: |handle, wanted, _| {
        Some(
            matches!(wanted, "java/lang/Object" | "java/util/Spliterator")
                || handle.class() == Some(wanted)
                || (wanted == ITERATOR_SPLITERATOR && handle.key().starts_with("sorted:")),
        )
    },
    crosses_jars: |_, _| false,
    compare_to: None,
};

/// The key of a modelled spliterator handle.
fn split_key(value: &Value) -> Option<&str> {
    match value {
        Value::Host(handle) if handle.kind() == HostKind::Spliterator => Some(handle.key()),
        _ => None,
    }
}

/// `Spliterator`'s characteristic bits.
pub(crate) const DISTINCT: i32 = 0x1;
pub(crate) const SORTED: i32 = 0x4;
pub(crate) const ORDERED: i32 = 0x10;
pub(crate) const SIZED: i32 = 0x40;
const CONCURRENT: i32 = 0x1000;
pub(crate) const SUBSIZED: i32 = 0x4000;

const SPLITERATOR: &str = "java/util/Spliterator";
const ABSTRACT_SPLITERATOR: &str = "java/util/Spliterators$AbstractSpliterator";
const ITERATOR: &str = "java/util/Iterator";
const CONSUMER: &str = "java/util/function/Consumer";
const ACCEPT: &str = "(Ljava/lang/Object;)V";

/// One modelled JDK spliterator.
#[derive(Clone, Debug)]
pub(crate) enum Split {
    /// `Spliterators.IteratorSpliterator`: over a collection (`iterator()`
    /// and `size()` asked lazily) or over an iterator it was given.
    Iter {
        collection: Option<Value>,
        it: Option<Value>,
        est: i64,
        flags: i32,
        /// `SortedSet.spliterator()`'s `getComparator()` asks the set.
        sorted_set: Option<Value>,
    },
    /// `AbstractList.RandomAccessSpliterator`.
    RandomAccess {
        list: Value,
        index: i32,
        /// `-1` until the first use reads `size()`.
        fence: i32,
        /// The `AbstractList`'s `modCount` when made, if it is one.
        mod_count: Option<i32>,
    },
}

/// The modelled spliterators of one runtime.
#[derive(Default)]
pub(crate) struct Splits {
    models: BTreeMap<String, Split>,
    next: usize,
}

/// `IteratorSpliterator`'s flags: `SIZED | SUBSIZED` unless `CONCURRENT`.
pub(crate) fn sized_flags(flags: i32) -> i32 {
    if flags & CONCURRENT == 0 {
        flags | SIZED | SUBSIZED
    } else {
        flags
    }
}

fn require_action(action: &Value) -> Result<(), NativeError> {
    if *action == Value::Ref(None) {
        return Err(throw("java/lang/NullPointerException", None));
    }
    Ok(())
}

fn truth(value: &Value) -> bool {
    matches!(value, Value::Int(flag) if *flag != 0)
}

fn int_value(value: &Value) -> i32 {
    match value {
        Value::Int(v) => *v,
        Value::Long(v) => *v as i32,
        _ => 0,
    }
}

fn long_value(value: &Value) -> i64 {
    match value {
        Value::Long(v) => *v,
        Value::Int(v) => i64::from(*v),
        _ => 0,
    }
}

impl<H: Host> Runtime<'_, H> {
    /// A new modelled spliterator.
    pub(crate) fn new_split(&mut self, split: Split) -> Value {
        let splits = &mut self.pipelines.splits;
        let tag = match split {
            Split::Iter {
                sorted_set: Some(_),
                ..
            } => "sorted",
            Split::Iter { .. } => "iter",
            Split::RandomAccess { .. } => "ra",
        };
        let key = format!("{tag}:{}", splits.next);
        splits.next += 1;
        splits.models.insert(key.clone(), split);
        Value::Host(HostRef::new(HostKind::Spliterator, key))
    }

    fn split(&self, marker: &str) -> Result<Split, NativeError> {
        self.pipelines
            .splits
            .models
            .get(marker)
            .cloned()
            .ok_or_else(|| NativeError::Metadata(format!("unknown spliterator {marker}")))
    }

    fn put_split(&mut self, marker: &str, split: Split) {
        self.pipelines
            .splits
            .models
            .insert(marker.to_owned(), split);
    }

    /// The JDK spliterator bodies: `Spliterators`' factories,
    /// `StreamSupport.stream`, members of a modelled spliterator, and
    /// `AbstractSpliterator`'s state and the `Spliterator` defaults on a jar
    /// spliterator. Runs right after the jar's own bodies and the collection
    /// skeletons. `Ok(None)` leaves the call to the rest of the chain.
    pub(crate) fn invoke_jdk_spliterator(
        &mut self,
        r: &MethodRef,
        opcode: u8,
        receiver: Option<&Value>,
        args: &[Value],
        class: &str,
        method: &str,
    ) -> Result<Option<Value>, NativeError> {
        let arg = |index: usize| args.get(index).cloned().unwrap_or(Value::Ref(None));
        match (r.owner.as_str(), r.name.as_str(), r.descriptor.as_str()) {
            (
                "java/util/Spliterators",
                "spliterator",
                "(Ljava/util/Collection;I)Ljava/util/Spliterator;",
            ) => {
                let collection = arg(0);
                require_action(&collection)?;
                return Ok(Some(self.new_split(Split::Iter {
                    collection: Some(collection),
                    it: None,
                    est: 0,
                    flags: sized_flags(int_value(&arg(1))),
                    sorted_set: None,
                })));
            }
            (
                "java/util/Spliterators",
                "spliterator",
                "(Ljava/util/Iterator;JI)Ljava/util/Spliterator;",
            ) => {
                let it = arg(0);
                require_action(&it)?;
                return Ok(Some(self.new_split(Split::Iter {
                    collection: None,
                    it: Some(it),
                    est: long_value(&arg(1)),
                    flags: sized_flags(int_value(&arg(2))),
                    sorted_set: None,
                })));
            }
            (
                "java/util/Spliterators",
                "spliteratorUnknownSize",
                "(Ljava/util/Iterator;I)Ljava/util/Spliterator;",
            ) => {
                let it = arg(0);
                require_action(&it)?;
                return Ok(Some(self.new_split(Split::Iter {
                    collection: None,
                    it: Some(it),
                    est: i64::MAX,
                    flags: int_value(&arg(1)) & !(SIZED | SUBSIZED),
                    sorted_set: None,
                })));
            }
            (
                "java/util/stream/StreamSupport",
                "stream",
                "(Ljava/util/Spliterator;Z)Ljava/util/stream/Stream;",
            ) => {
                let parallel = matches!(arg(1), Value::Int(flag) if flag != 0);
                return self
                    .stream_of_spliterator(&arg(0), parallel, class, method)
                    .map(Some);
            }
            _ => {}
        }
        let Some(this) = receiver else {
            return Ok(None);
        };
        if let Some(key) = split_key(this) {
            let key = key.to_owned();
            return self.model_member(&key, r, args);
        }
        let Some(object_class) = expect_ref(this)
            .and_then(|marker| jvm_dispatch::constructed_object_class(marker))
            .map(str::to_owned)
        else {
            return Ok(None);
        };
        let abstract_split = self.class_is_instance(&object_class, ABSTRACT_SPLITERATOR);
        // `super(est, additionalCharacteristics)`.
        if abstract_split
            && opcode == 0xb7
            && r.owner == ABSTRACT_SPLITERATOR
            && r.name == "<init>"
            && r.descriptor == "(JI)V"
        {
            let flags = int_value(&arg(1));
            let flags = if flags & SIZED != 0 {
                flags | SUBSIZED
            } else {
                flags
            };
            self.fields.insert(
                format!("{}|abstract-spliterator#est", expect_ref(this).unwrap()),
                arg(0),
            );
            self.fields.insert(
                format!("{}|abstract-spliterator#flags", expect_ref(this).unwrap()),
                Value::Int(flags),
            );
            return Ok(Some(Value::Void));
        }
        if !matches!(opcode, 0xb6 | 0xb9)
            || !(abstract_split || self.class_is_instance(&object_class, SPLITERATOR))
        {
            return Ok(None);
        }
        let marker = expect_ref(this).cloned().unwrap_or_default();
        let field = |runtime: &Self, name: &str| {
            runtime
                .fields
                .get(&format!("{marker}|abstract-spliterator#{name}"))
                .cloned()
        };
        Ok(Some(match (r.name.as_str(), r.descriptor.as_str()) {
            ("estimateSize", "()J") if abstract_split => {
                Value::Long(field(self, "est").map_or(0, |v| long_value(&v)))
            }
            ("characteristics", "()I") if abstract_split => {
                Value::Int(field(self, "flags").map_or(0, |v| int_value(&v)))
            }
            ("trySplit", "()Ljava/util/Spliterator;") if abstract_split => {
                self.host.unknown_operand_call(r)?
            }
            // `Spliterator`'s defaults.
            ("forEachRemaining", "(Ljava/util/function/Consumer;)V") => {
                let action = arg(0);
                while truth(&self.call_virtual(
                    Site::Interface,
                    SPLITERATOR,
                    "tryAdvance",
                    "(Ljava/util/function/Consumer;)Z",
                    this,
                    std::slice::from_ref(&action),
                )?) {}
                Value::Void
            }
            ("getExactSizeIfKnown", "()J") => Value::Long(self.exact_size_if_known(this)?),
            ("hasCharacteristics", "(I)Z") => {
                let wanted = int_value(&arg(0));
                let flags = self.characteristics_of(this)?;
                Value::Int(i32::from(flags & wanted == wanted))
            }
            ("getComparator", "()Ljava/util/Comparator;") => {
                return Err(throw("java/lang/IllegalStateException", None));
            }
            _ => return Ok(None),
        }))
    }

    /// `spliterator.characteristics()`.
    pub(crate) fn characteristics_of(&mut self, spliterator: &Value) -> Result<i32, NativeError> {
        let flags = self.call_virtual(
            Site::Interface,
            SPLITERATOR,
            "characteristics",
            "()I",
            spliterator,
            &[],
        )?;
        Ok(int_value(&flags))
    }

    /// `spliterator.getExactSizeIfKnown()`.
    pub(crate) fn exact_size_if_known(&mut self, spliterator: &Value) -> Result<i64, NativeError> {
        if split_key(spliterator).is_some() {
            let size = self.call_virtual(
                Site::Interface,
                SPLITERATOR,
                "getExactSizeIfKnown",
                "()J",
                spliterator,
                &[],
            )?;
            return Ok(long_value(&size));
        }
        // The default body: `(characteristics() & SIZED) == 0 ? -1 :
        // estimateSize()`, unless the jar overrides it.
        let r = MethodRef {
            owner: SPLITERATOR.into(),
            name: "getExactSizeIfKnown".into(),
            descriptor: "()J".into(),
        };
        if let Some(owner) = self.jvm_selected_jar_body(&r, 0xb9, Some(spliterator))? {
            let size = self.invoke(
                &owner,
                &r.name,
                &r.descriptor,
                Some(spliterator.clone()),
                &[],
            )?;
            return Ok(long_value(&size));
        }
        if self.characteristics_of(spliterator)? & SIZED == 0 {
            return Ok(-1);
        }
        let size = self.call_virtual(
            Site::Interface,
            SPLITERATOR,
            "estimateSize",
            "()J",
            spliterator,
            &[],
        )?;
        Ok(long_value(&size))
    }

    /// A member of a modelled spliterator.
    fn model_member(
        &mut self,
        marker: &str,
        r: &MethodRef,
        args: &[Value],
    ) -> Result<Option<Value>, NativeError> {
        let arg = || args.first().cloned().unwrap_or(Value::Ref(None));
        let flags = match self.split(marker)? {
            Split::Iter { flags, .. } => flags,
            Split::RandomAccess { .. } => ORDERED | SIZED | SUBSIZED,
        };
        Ok(Some(match (r.name.as_str(), r.descriptor.as_str()) {
            ("tryAdvance", "(Ljava/util/function/Consumer;)Z") => {
                let action = arg();
                require_action(&action)?;
                Value::Int(i32::from(self.model_advance(marker, &action)?))
            }
            ("forEachRemaining", "(Ljava/util/function/Consumer;)V") => {
                let action = arg();
                require_action(&action)?;
                self.model_for_each(marker, &action)?;
                Value::Void
            }
            ("estimateSize", "()J") => Value::Long(self.model_estimate(marker)?),
            ("getExactSizeIfKnown", "()J") => Value::Long(if flags & SIZED == 0 {
                -1
            } else {
                self.model_estimate(marker)?
            }),
            ("characteristics", "()I") => Value::Int(flags),
            ("hasCharacteristics", "(I)Z") => {
                let wanted = int_value(&arg());
                Value::Int(i32::from(flags & wanted == wanted))
            }
            ("getComparator", "()Ljava/util/Comparator;") => {
                if flags & SORTED == 0 {
                    return Err(throw("java/lang/IllegalStateException", None));
                }
                match self.split(marker)? {
                    Split::Iter {
                        sorted_set: Some(set),
                        ..
                    } => self.call_virtual(
                        Site::Interface,
                        "java/util/SortedSet",
                        "comparator",
                        "()Ljava/util/Comparator;",
                        &set,
                        &[],
                    )?,
                    _ => Value::Ref(None),
                }
            }
            ("trySplit", "()Ljava/util/Spliterator;") => self.host.unknown_operand_call(r)?,
            _ => return Ok(None),
        }))
    }

    /// `IteratorSpliterator`'s lazy start: the collection's `iterator()`,
    /// then its `size()` as the estimate.
    fn model_start(&mut self, marker: &str) -> Result<(), NativeError> {
        if let Split::Iter {
            collection: Some(collection),
            it: None,
            ..
        } = self.split(marker)?
        {
            let it = self.call_virtual(
                Site::Interface,
                "java/util/Collection",
                "iterator",
                "()Ljava/util/Iterator;",
                &collection,
                &[],
            )?;
            self.set_iter(marker, Some(it), None);
            let size = self.call_virtual(
                Site::Interface,
                "java/util/Collection",
                "size",
                "()I",
                &collection,
                &[],
            )?;
            self.set_iter(marker, None, Some(i64::from(int_value(&size))));
        }
        Ok(())
    }

    fn set_iter(&mut self, marker: &str, it: Option<Value>, est: Option<i64>) {
        if let Ok(Split::Iter {
            collection,
            it: old_it,
            est: old_est,
            flags,
            sorted_set,
        }) = self.split(marker)
        {
            self.put_split(
                marker,
                Split::Iter {
                    collection,
                    it: it.or(old_it),
                    est: est.unwrap_or(old_est),
                    flags,
                    sorted_set,
                },
            );
        }
    }

    /// `RandomAccessSpliterator.getFence()`: `size()` at the first use.
    fn model_fence(&mut self, marker: &str) -> Result<(i32, i32), NativeError> {
        let Split::RandomAccess {
            list,
            index,
            fence,
            mod_count,
        } = self.split(marker)?
        else {
            return Ok((0, 0));
        };
        if fence >= 0 {
            return Ok((index, fence));
        }
        let size =
            self.call_virtual(Site::Interface, "java/util/List", "size", "()I", &list, &[])?;
        let fence = int_value(&size);
        self.put_split(
            marker,
            Split::RandomAccess {
                list,
                index,
                fence,
                mod_count,
            },
        );
        Ok((index, fence))
    }

    fn set_index(&mut self, marker: &str, index: i32) {
        if let Ok(Split::RandomAccess {
            list,
            fence,
            mod_count,
            ..
        }) = self.split(marker)
        {
            self.put_split(
                marker,
                Split::RandomAccess {
                    list,
                    index,
                    fence,
                    mod_count,
                },
            );
        }
    }

    /// `RandomAccessSpliterator.get(list, i)`: an `IndexOutOfBoundsException`
    /// becomes a `ConcurrentModificationException`.
    fn random_access_get(&mut self, list: &Value, i: i32) -> Result<Value, NativeError> {
        match self.call_virtual(
            Site::Interface,
            "java/util/List",
            "get",
            "(I)Ljava/lang/Object;",
            list,
            &[Value::Int(i)],
        ) {
            Err(error) if self.catches(&error, "java/lang/IndexOutOfBoundsException") => {
                Err(throw("java/util/ConcurrentModificationException", None))
            }
            other => other,
        }
    }

    /// `checkAbstractListModCount`.
    fn check_abstract_list_mod_count(&mut self, marker: &str) -> Result<(), NativeError> {
        if let Split::RandomAccess {
            list,
            mod_count: Some(expected),
            ..
        } = self.split(marker)?
            && self.abstract_list_mod_count(&list)? != Some(expected)
        {
            return Err(throw("java/util/ConcurrentModificationException", None));
        }
        Ok(())
    }

    fn model_advance(&mut self, marker: &str, action: &Value) -> Result<bool, NativeError> {
        match self.split(marker)? {
            Split::Iter { .. } => {
                self.model_start(marker)?;
                let Split::Iter { it: Some(it), .. } = self.split(marker)? else {
                    return Ok(false);
                };
                if !truth(&self.call_virtual(
                    Site::Interface,
                    ITERATOR,
                    "hasNext",
                    "()Z",
                    &it,
                    &[],
                )?) {
                    return Ok(false);
                }
                let value = self.call_virtual(
                    Site::Interface,
                    ITERATOR,
                    "next",
                    "()Ljava/lang/Object;",
                    &it,
                    &[],
                )?;
                self.call_virtual(
                    Site::Interface,
                    CONSUMER,
                    "accept",
                    ACCEPT,
                    action,
                    &[value],
                )?;
                Ok(true)
            }
            Split::RandomAccess { list, .. } => {
                let (index, fence) = self.model_fence(marker)?;
                if index >= fence {
                    return Ok(false);
                }
                self.set_index(marker, index + 1);
                let value = self.random_access_get(&list, index)?;
                self.call_virtual(
                    Site::Interface,
                    CONSUMER,
                    "accept",
                    ACCEPT,
                    action,
                    &[value],
                )?;
                self.check_abstract_list_mod_count(marker)?;
                Ok(true)
            }
        }
    }

    fn model_for_each(&mut self, marker: &str, action: &Value) -> Result<(), NativeError> {
        match self.split(marker)? {
            Split::Iter { .. } => {
                self.model_start(marker)?;
                let Split::Iter { it: Some(it), .. } = self.split(marker)? else {
                    return Ok(());
                };
                self.call_virtual(
                    Site::Interface,
                    ITERATOR,
                    "forEachRemaining",
                    "(Ljava/util/function/Consumer;)V",
                    &it,
                    std::slice::from_ref(action),
                )?;
                Ok(())
            }
            Split::RandomAccess { list, .. } => {
                let (index, fence) = self.model_fence(marker)?;
                self.set_index(marker, fence);
                for i in index..fence {
                    let value = self.random_access_get(&list, i)?;
                    self.call_virtual(
                        Site::Interface,
                        CONSUMER,
                        "accept",
                        ACCEPT,
                        action,
                        &[value],
                    )?;
                }
                self.check_abstract_list_mod_count(marker)
            }
        }
    }

    fn model_estimate(&mut self, marker: &str) -> Result<i64, NativeError> {
        match self.split(marker)? {
            Split::Iter { .. } => {
                self.model_start(marker)?;
                match self.split(marker)? {
                    Split::Iter { est, .. } => Ok(est),
                    Split::RandomAccess { .. } => Ok(0),
                }
            }
            Split::RandomAccess { .. } => {
                let (index, fence) = self.model_fence(marker)?;
                Ok(i64::from(fence - index))
            }
        }
    }
}

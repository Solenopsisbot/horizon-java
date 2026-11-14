//! The `Iterable`, `Collection`, `Iterator` default methods and the
//! collection interfaces' `spliterator()` defaults as JDK 25 runs them. A
//! default's calls on `this` are `invokeinterface`, as in the JDK's
//! interface bodies.

use super::*;
use crate::runtime::jdk_spliterators::{DISTINCT, ORDERED, SORTED, Split, sized_flags};

/// The defaults this file implements: `(interface, name, descriptor)`.
pub(super) const BODIES: &[(&str, &str, &str)] = &[
    (
        "java/lang/Iterable",
        "forEach",
        "(Ljava/util/function/Consumer;)V",
    ),
    (COLLECTION, "removeIf", "(Ljava/util/function/Predicate;)Z"),
    (
        COLLECTION,
        "toArray",
        "(Ljava/util/function/IntFunction;)[Ljava/lang/Object;",
    ),
    ("java/lang/Iterable", "spliterator", SPLITERATOR),
    (COLLECTION, "spliterator", SPLITERATOR),
    ("java/util/List", "spliterator", SPLITERATOR),
    ("java/util/Set", "spliterator", SPLITERATOR),
    ("java/util/SortedSet", "spliterator", SPLITERATOR),
    (COLLECTION, "stream", "()Ljava/util/stream/Stream;"),
    (COLLECTION, "parallelStream", "()Ljava/util/stream/Stream;"),
    (ITERATOR, "remove", "()V"),
    (
        ITERATOR,
        "forEachRemaining",
        "(Ljava/util/function/Consumer;)V",
    ),
];

/// `spliterator()`'s descriptor.
const SPLITERATOR: &str = "()Ljava/util/Spliterator;";

/// `Consumer.accept(Object)`.
const ACCEPT: (&str, &str, &str) = (
    "java/util/function/Consumer",
    "accept",
    "(Ljava/lang/Object;)V",
);

impl<H: Host> Runtime<'_, H> {
    /// A JDK interface's default method.
    pub(super) fn interface_default_body(
        &mut self,
        owner: &str,
        name: &str,
        descriptor: &str,
        this: &Value,
        args: &[Value],
    ) -> Result<Value, NativeError> {
        let arg = |index: usize| args.get(index).cloned().unwrap_or(Value::Ref(None));
        match (owner, name) {
            ("java/lang/Iterable", "forEach") => {
                let action = arg(0);
                require_non_null(&action)?;
                let it = self.iterator_of(Site::Interface, "java/lang/Iterable", this)?;
                self.each_remaining(&it, &action)?;
                Ok(Value::Void)
            }
            (COLLECTION, "removeIf") => {
                let filter = arg(0);
                require_non_null(&filter)?;
                let each = self.iterator_of(Site::Interface, COLLECTION, this)?;
                let mut removed = false;
                while self.has_next(&each)? {
                    let e = self.next(&each)?;
                    let selected = self.call_functional(
                        "java/util/function/Predicate",
                        "test",
                        "(Ljava/lang/Object;)Z",
                        &filter,
                        &[e],
                    )?;
                    if truth(&selected) {
                        self.remove_current(&each)?;
                        removed = true;
                    }
                }
                Ok(boolean(removed))
            }
            (COLLECTION, "toArray") => {
                let array = self.call_functional(
                    "java/util/function/IntFunction",
                    "apply",
                    "(I)Ljava/lang/Object;",
                    &arg(0),
                    &[Value::Int(0)],
                )?;
                self.call_virtual(
                    Site::Interface,
                    COLLECTION,
                    "toArray",
                    "([Ljava/lang/Object;)[Ljava/lang/Object;",
                    this,
                    &[array],
                )
            }
            (COLLECTION, "stream" | "parallelStream") => {
                // `StreamSupport.stream(spliterator(), parallel)`: the
                // class's own `spliterator()` if it has one, else the
                // interface default below; the elements are pulled when the
                // terminal operation runs (`stream_pipeline.rs`).
                let spliterator = self.call_virtual(
                    Site::Interface,
                    COLLECTION,
                    "spliterator",
                    SPLITERATOR,
                    this,
                    &[],
                )?;
                let parallel = name == "parallelStream";
                self.stream_of_spliterator(&spliterator, parallel, COLLECTION, name)
            }
            // `Spliterators.spliteratorUnknownSize(iterator(), 0)`: the
            // iterator is asked for now.
            ("java/lang/Iterable", "spliterator") => {
                let it = self.iterator_of(Site::Interface, "java/lang/Iterable", this)?;
                Ok(self.new_split(Split::Iter {
                    collection: None,
                    it: Some(it),
                    est: i64::MAX,
                    flags: 0,
                    sorted_set: None,
                }))
            }
            (_, "spliterator") => {
                // `List.spliterator()` of a `RandomAccess` list is
                // `AbstractList.RandomAccessSpliterator`, which records an
                // `AbstractList`'s `modCount` when made.
                if owner == "java/util/List"
                    && self.java_instance_of(this, "java/util/RandomAccess")?
                {
                    let mod_count = self.abstract_list_mod_count(this)?;
                    return Ok(self.new_split(Split::RandomAccess {
                        list: this.clone(),
                        index: 0,
                        fence: -1,
                        mod_count,
                    }));
                }
                // `Spliterators.spliterator(this, flags)` (an
                // `IteratorSpliterator`): `iterator()` and `size()` are
                // asked at the first traversal.
                let flags = match owner {
                    "java/util/List" => ORDERED,
                    "java/util/Set" => DISTINCT,
                    "java/util/SortedSet" => DISTINCT | SORTED | ORDERED,
                    _ => 0,
                };
                Ok(self.new_split(Split::Iter {
                    collection: Some(this.clone()),
                    it: None,
                    est: 0,
                    flags: sized_flags(flags),
                    sorted_set: (owner == "java/util/SortedSet").then(|| this.clone()),
                }))
            }
            (ITERATOR, "remove") => Err(throw(
                "java/lang/UnsupportedOperationException",
                Some("remove".into()),
            )),
            (ITERATOR, "forEachRemaining") => {
                let action = arg(0);
                require_non_null(&action)?;
                self.each_remaining(this, &action)?;
                Ok(Value::Void)
            }
            _ => Err(NativeError::Metadata(format!(
                "no JDK default for {owner}#{name}{descriptor}"
            ))),
        }
    }

    /// `while (it.hasNext()) action.accept(it.next());`
    fn each_remaining(&mut self, it: &Value, action: &Value) -> Result<(), NativeError> {
        while self.has_next(it)? {
            let e = self.next(it)?;
            self.call_functional(ACCEPT.0, ACCEPT.1, ACCEPT.2, action, &[e])?;
        }
        Ok(())
    }
}

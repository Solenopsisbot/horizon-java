//! `java.util.stream` pipelines evaluated lazily, one element at a time, as
//! JDK 25's sequential streams are.
//!
//! The runtime's first stream model was eager: every stage built the list of
//! its outputs, so `Stream.of(a, b).peek(p).map(m).forEach(e)` ran
//! `p(a) p(b) m(a) m(b) e(a) e(b)` where the JDK runs `p(a) m(a) e(a) p(b)
//! m(b) e(b)`, and `findFirst`, `anyMatch` and `limit` ran every upstream
//! lambda instead of stopping. Any lambda with a side effect (a log line, a
//! counter, a registration) saw a different program.
//!
//! **Model.** A source stream is still a list of values (`Runtime::streams`).
//! Every intermediate stage now only records a [`Link`] to its upstream
//! stream; nothing runs until a terminal operation. The terminal walks the
//! links back to the source and pushes each source element through the
//! stages into a terminal sink, exactly like `AbstractPipeline.copyInto`:
//!
//! * a pipeline that can stop early (a `limit`, or a `find*`/`*Match`
//!   terminal) asks `cancellationRequested` before every element, as
//!   `copyIntoWithCancel` does; `limit` cancels once it has passed its count,
//!   a `find`/`match` once its answer is known;
//! * `sorted` is a barrier: it buffers every element and pushes the sorted
//!   list downstream when the source ends (`SortedOps`' `end`), never asking
//!   upstream to stop, and stops its own push once downstream cancels;
//! * `flatMap` pushes each inner stream's elements straight downstream and
//!   stops inside the inner stream once downstream cancels (JDK 10+);
//! * `count()` of a pipeline whose size is known without traversal (a sized
//!   source through `map`, `peek`, `sorted`, `skip`, `limit`: JDK 25's
//!   `SIZED` flag, with JDK 18's size-adjusting slices) answers that size and
//!   runs no lambda at all;
//! * `Stream.concat` is lazy too: its two streams' pipelines run, in turn,
//!   only when the concatenation's terminal pulls them.
//!
//! The terminals that run callbacks per element (`forEach`, `forEachOrdered`,
//! `reduce` in all three forms, `max`/`min`, `collect(supplier, accumulator,
//! combiner)`, `collect(Collectors.toMap(...))`, the matches and
//! `findFirst`/`findAny`) run inside the push, so their calls interleave
//! with the stages' exactly. Sequential `reduce` and `collect` never call
//! the combiner. The terminals that run no jar code per element (`toList`,
//! `toArray`, `iterator`, `toList()`/`toSet()`/`joining` collectors, the
//! primitive sums) get the pipeline's output list from one full push and
//! then run as before.
//!
//! **A stream over a host collection** (`ArrayList`, `LinkedList`,
//! `HashSet`, ...: the fail-fast ones) binds late, as their spliterators do:
//! it reads the collection when the terminal starts, not when `stream()` was
//! called, and captures its `modCount` then (`mod_count.rs`). It checks that
//! `modCount` as the JDK's spliterators do, after each element when the
//! pipeline pulls (`tryAdvance`) and after the last when it pushes
//! (`forEachRemaining`): a structural change throws
//! `ConcurrentModificationException`, even one that restored the contents.
//! What the JDK's traversal visits after a change depends on the
//! collection's internals (a `HashSet` visits an element added behind the
//! cursor, an `ArrayList` reads a `set` element from its live array), so a
//! change of the contents other than appending to a list is counted.
//!
//! `distinct()` keeps JDK 25's `HashSet` (`hash_bulk.rs`), so the elements'
//! own `equals`/`hashCode` decide it; `sorted()` is TimSort (`tim_sort.rs`),
//! so a comparator sees the JDK's comparisons in the JDK's order.
//!
//! A stream is used once: linking a stage to it, or running a terminal on
//! it, a second time throws the JDK's `IllegalStateException` ("stream has
//! already been operated upon or closed").
//!
//! **Counted, not answered** (each counts an unknown host call and runs on):
//!
//! * a parallel pipeline (`parallel()`, `parallelStream()`), which runs
//!   sequentially (`Stream#parallel`): the JDK's parallel element order is
//!   not reproducible;
//! * `iterator()`/`spliterator()` of a pipeline that runs code or binds to a
//!   host collection (`Stream#iterator`): the JDK pulls it lazily as the
//!   iterator advances, this hands out the pushed list;
//! * a stream over a map's `keySet()`/`values()`/`entrySet()` copy whose map
//!   changed before the traversal ended (`<Map>#spliterator`): the JDK's view
//!   sees the change, the copy does not.

use crate::host_ref::{HostKind, HostRef, Methods};
use crate::{MethodRef, NativeError, Value, expect_ref};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

/// `Stream.builder()`'s object (`Streams.StreamBuilderImpl`) in the kind
/// table (`host_ref.rs`): a handle keyed by the runtime's builder number.
/// `Object`'s `equals` is identity; its `toString`/`hashCode` print and are
/// the identity hash, which is not modelled (`None`: the caller counts).
pub(crate) const STREAM_BUILDER: Methods = Methods {
    to_string: |_, _| Ok(None),
    hash_code: |_, _| Ok(None),
    equals: |handle, other, _| {
        Ok(match other {
            Value::Host(other) => handle.same_object(other),
            _ => Some(false),
        })
    },
    // `StreamBuilderImpl extends AbstractStreamBuilderImpl implements
    // Stream.Builder` (a `Consumer`); `AbstractStreamBuilderImpl implements
    // Spliterator` (JDK 25.0.5's `javap`).
    instance_of: |_, wanted, _| {
        Some(matches!(
            wanted,
            "java/lang/Object"
                | "java/util/stream/Streams$StreamBuilderImpl"
                | "java/util/stream/Streams$AbstractStreamBuilderImpl"
                | "java/util/stream/Stream$Builder"
                | "java/util/function/Consumer"
                | "java/util/Spliterator"
        ))
    },
    crosses_jars: |_, _| false,
    compare_to: None,
};

/// The builder number a `Stream.Builder` handle carries.
pub(crate) fn builder_key(value: &Value) -> Option<&str> {
    match value {
        Value::Host(handle) if handle.kind() == HostKind::StreamBuilder => Some(handle.key()),
        _ => None,
    }
}

/// Elements one push may collect, or one builder may hold, before the
/// runtime refuses the stream as a runaway (a `flatMap` fan-out); far above
/// any list a mod builds at load.
const MAX_ELEMENTS: usize = 1 << 20;

/// One intermediate stage.
#[derive(Clone, Debug)]
enum Stage {
    /// `map`/`mapToX`: the function and its interface.
    Map(Value, String),
    Filter(Value, String),
    Peek(Value, String),
    /// `flatMap`: the function answers a stream (or `null`).
    FlatMap(Value, String),
    Skip(i64),
    Limit(i64),
    Distinct,
    /// `sorted`: the comparator, `null` for natural order.
    Sorted(Value),
    /// `boxed`, `unordered`: the elements pass unchanged.
    Pass,
    /// `asLongStream`/`asDoubleStream` (`J`/`D`): a widening conversion;
    /// `IntStream.boxed()` (`I`): `Integer.valueOf`.
    Widen(char),
}

/// A stream that is not a source: its upstream and the stage after it.
#[derive(Clone, Debug)]
struct Link {
    upstream: String,
    stage: Stage,
}

/// Where a pipeline's elements come from.
#[derive(Clone, Debug)]
enum Source {
    /// A value list the runtime already holds (`Runtime::streams`).
    Values(String),
    /// `Stream.concat(a, b)`.
    Concat(String, String),
}

/// What a running stage keeps between elements.
#[derive(Clone, Debug)]
enum State {
    None,
    /// `skip`/`limit`: how many elements are left to drop or pass.
    Left(i64),
    /// `distinct`: `DistinctOps`' `HashSet` of the elements seen (a
    /// primitive stream's `distinct` boxes them into the same set), and the
    /// counted elements whose hash the set cannot know, decided by value.
    Seen(Value, Vec<Value>),
    /// `sorted`: the elements buffered until the source ends.
    Buffer(Vec<Value>),
    /// `distinct` on a pipeline known `SORTED`: `DistinctOps` compares each
    /// element with the last one passed (`equals`, no hash): whether a `null`
    /// was passed, and the last element.
    Adjacent(bool, Option<Value>),
}

/// The terminal sink of a push.
#[derive(Clone, Debug)]
enum Terminal {
    /// Keep every element (a terminal that then runs on the list).
    Collect(Vec<Value>),
    /// `forEach`: the consumer and its interface.
    Each(Value, String),
    /// `findFirst`/`findAny`.
    Find(Option<Value>),
    /// `anyMatch` (`Some(true)` stops), `allMatch`/`noneMatch` (`Some(false)`
    /// on the deciding element): the predicate, its interface, the kind and
    /// the answer once decided.
    Match {
        predicate: Value,
        interface: String,
        kind: Match,
        decided: Option<bool>,
    },
    Count(i64),
    /// `reduce(op)`: the operator, its interface and the partial result.
    Reduce(Value, String, Option<Value>),
    /// `reduce(identity, op[, combiner])`: the accumulator, its interface
    /// and the running value.
    Fold(Value, String, Value),
    /// `max`/`min(comparator)`: `BinaryOperator.maxBy`/`minBy`.
    Extreme {
        comparator: Value,
        max: bool,
        best: Option<Value>,
    },
    /// `collect(supplier, accumulator, combiner)`: the accumulator and the
    /// container the supplier made.
    Container(Value, Value),
    /// `collect(Collectors.toMap(...))`.
    ToMap(Box<crate::jdk_model::ToMap>),
    /// The downstream of a `flatMap` or `concat`: the outer push and the
    /// stage its elements go to.
    Forward(u64, usize),
    /// A `concat` side with stages under a short-circuiting push: JDK 25's
    /// `WrappingSpliterator`. Each pulled source element runs through the
    /// side's stages into this buffer (which never asks the side to stop),
    /// and the outer push takes the buffered elements one at a time,
    /// asking its own cancellation before each (`drain_wrap`).
    Wrap(u64, usize, Vec<Value>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Match {
    Any,
    All,
    None,
}

/// One running push.
struct Drive {
    stages: Arc<Vec<Stage>>,
    states: Vec<State>,
    terminal: Terminal,
    /// Whether the pipeline can stop early (`StreamOpFlag.SHORT_CIRCUIT`):
    /// its source is then pulled one element at a time (`tryAdvance`), else
    /// pushed in bulk (`forEachRemaining`).
    short: bool,
}

/// The host collection a value-list source streams (`Collection.stream()`),
/// and its class.
#[derive(Clone, Debug)]
struct Origin {
    collection: Value,
    class: String,
}

/// A stream over a map view's copy: what the map was when it was made.
#[derive(Clone, Debug)]
struct ViewOrigin {
    map: Value,
    stamp: crate::mod_count::ModStamp,
    entries: Vec<String>,
}

/// The JDK collections whose spliterators bind late and check `modCount`.
const FAIL_FAST: &[&str] = &[
    "java/util/ArrayList",
    "java/util/Vector",
    "java/util/Stack",
    "java/util/LinkedList",
    "java/util/ArrayDeque",
    "java/util/PriorityQueue",
    "java/util/HashSet",
    "java/util/LinkedHashSet",
    "java/util/TreeSet",
];

/// The lists whose bulk traversal visits only the elements it bound to when
/// elements are appended behind it (an array or node walk bounded by the
/// size it read first).
const APPEND_SAFE: &[&str] = &[
    "java/util/ArrayList",
    "java/util/Vector",
    "java/util/Stack",
    "java/util/LinkedList",
];

/// The runtime's lazy stream state.
#[derive(Default)]
pub(crate) struct Pipelines {
    links: BTreeMap<String, Link>,
    concats: BTreeMap<String, (String, String)>,
    /// Sources whose pipeline was made parallel (`sourceStage.parallel`).
    parallel: BTreeSet<String>,
    /// `Stream.Builder`s: their elements, `None` once built.
    builders: BTreeMap<String, Option<Vec<Value>>>,
    next_builder: usize,
    drives: BTreeMap<u64, Drive>,
    next_drive: u64,
    /// Value-list sources over a fail-fast host collection.
    origins: BTreeMap<String, Origin>,
    /// Value-list sources over a map's `keySet()`/`values()`/`entrySet()`
    /// copy: the map, its `modCount` and its entries when the stream was
    /// made.
    view_origins: BTreeMap<String, ViewOrigin>,
    /// Streams a stage or terminal has used (`linkedOrConsumed`).
    used: BTreeSet<String>,
}

/// The marker of a new stream (a source or a stage). Streams are still
/// marker text (#156's P15 moves them onto handles); the per-process random
/// tag keeps a jar `String` constant from spelling a live stream, so a forged
/// one is the `String` it is (`is_live_stream`).
pub(crate) fn stream_marker(class: &str, method: &str, n: usize) -> String {
    static TAG: std::sync::OnceLock<u64> = std::sync::OnceLock::new();
    let tag = TAG.get_or_init(|| {
        use std::hash::{BuildHasher, Hasher};
        std::collections::hash_map::RandomState::new()
            .build_hasher()
            .finish()
    });
    format!("stream/{class}/{method}/{n}~{tag:016x}")
}

/// Whether `text` has a stream marker's shape (`stream/...~<16 hex>`). A
/// path that cannot ask the runtime whether a stream is live (the host's
/// collection text) treats such a text as a stream: it is never printed,
/// since a printed marker is a forgeable capability.
pub(crate) fn is_stream_marker(text: &str) -> bool {
    text.starts_with("stream/")
        && text.rsplit_once('~').is_some_and(|(_, tag)| {
            tag.len() == 16 && tag.bytes().all(|byte| byte.is_ascii_hexdigit())
        })
}

/// What a stream prints where its text is counted: Java's is the stage
/// class and identity hash (`ReferencePipeline$Head@1b6d3586`), which no run
/// reproduces; never the marker.
pub(crate) const STREAM_TEXT: &str = "null";

/// The stream interfaces a `String` never is.
pub(crate) fn is_stream_interface(class: &str) -> bool {
    matches!(
        class,
        "java/util/stream/Stream"
            | "java/util/stream/IntStream"
            | "java/util/stream/LongStream"
            | "java/util/stream/DoubleStream"
            | "java/util/stream/BaseStream"
    )
}

/// The single abstract method of a `java.util.function` interface.
fn sam(interface: &str) -> Option<(&'static str, &'static str)> {
    Some(match interface.strip_prefix("java/util/function/")? {
        "Function" | "UnaryOperator" => ("apply", "(Ljava/lang/Object;)Ljava/lang/Object;"),
        "BiFunction" | "BinaryOperator" => (
            "apply",
            "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
        ),
        "Predicate" => ("test", "(Ljava/lang/Object;)Z"),
        "Consumer" => ("accept", "(Ljava/lang/Object;)V"),
        "BiConsumer" => ("accept", "(Ljava/lang/Object;Ljava/lang/Object;)V"),
        "Supplier" => ("get", "()Ljava/lang/Object;"),
        "ToIntFunction" => ("applyAsInt", "(Ljava/lang/Object;)I"),
        "ToLongFunction" => ("applyAsLong", "(Ljava/lang/Object;)J"),
        "ToDoubleFunction" => ("applyAsDouble", "(Ljava/lang/Object;)D"),
        "IntFunction" => ("apply", "(I)Ljava/lang/Object;"),
        "LongFunction" => ("apply", "(J)Ljava/lang/Object;"),
        "DoubleFunction" => ("apply", "(D)Ljava/lang/Object;"),
        "IntUnaryOperator" => ("applyAsInt", "(I)I"),
        "LongUnaryOperator" => ("applyAsLong", "(J)J"),
        "DoubleUnaryOperator" => ("applyAsDouble", "(D)D"),
        "IntPredicate" => ("test", "(I)Z"),
        "LongPredicate" => ("test", "(J)Z"),
        "DoublePredicate" => ("test", "(D)Z"),
        "IntConsumer" => ("accept", "(I)V"),
        "LongConsumer" => ("accept", "(J)V"),
        "DoubleConsumer" => ("accept", "(D)V"),
        "IntToLongFunction" => ("applyAsLong", "(I)J"),
        "IntToDoubleFunction" => ("applyAsDouble", "(I)D"),
        "LongToIntFunction" => ("applyAsInt", "(J)I"),
        "LongToDoubleFunction" => ("applyAsDouble", "(J)D"),
        "DoubleToIntFunction" => ("applyAsInt", "(D)I"),
        "DoubleToLongFunction" => ("applyAsLong", "(D)J"),
        "IntBinaryOperator" => ("applyAsInt", "(II)I"),
        "LongBinaryOperator" => ("applyAsLong", "(JJ)J"),
        "DoubleBinaryOperator" => ("applyAsDouble", "(DD)D"),
        _ => return None,
    })
}

/// The class of a descriptor's parameter `index`, when it is a class type.
fn parameter_class(descriptor: &str, index: usize) -> Option<&str> {
    let mut rest = descriptor.strip_prefix('(')?;
    let mut at = 0;
    while !rest.starts_with(')') {
        let array = rest.len() - rest.trim_start_matches('[').len();
        let body = &rest[array..];
        let (class, len) = if let Some(class) = body.strip_prefix('L') {
            let end = class.find(';')?;
            (Some(&class[..end]), array + end + 2)
        } else {
            (None, array + 1)
        };
        if at == index {
            return if array == 0 { class } else { None };
        }
        rest = &rest[len..];
        at += 1;
    }
    None
}

fn npe() -> NativeError {
    NativeError::UncaughtException {
        class: "java/lang/NullPointerException".into(),
        message: None,
        frame: None,
    }
}

fn concurrent_modification() -> NativeError {
    NativeError::UncaughtException {
        class: "java/util/ConcurrentModificationException".into(),
        message: None,
        frame: None,
    }
}

fn illegal_state(message: Option<&str>) -> NativeError {
    NativeError::UncaughtException {
        class: "java/lang/IllegalStateException".into(),
        message: message.map(str::to_owned),
        frame: None,
    }
}

fn truth(value: &Value) -> bool {
    matches!(value, Value::Int(flag) if *flag != 0)
}

/// Whether a stream member is a terminal that consumes the stream's output
/// (and so may run its pipeline).
pub(crate) fn takes_output(name: &str) -> bool {
    matches!(
        name,
        "toList"
            | "toArray"
            | "collect"
            | "iterator"
            | "spliterator"
            | "sum"
            | "average"
            | "max"
            | "min"
            | "summaryStatistics"
            | "findFirst"
            | "findAny"
            | "reduce"
            | "count"
            | "forEach"
            | "forEachOrdered"
            | "anyMatch"
            | "allMatch"
            | "noneMatch"
            | "isEmpty"
    )
}

fn is_stream_owner(owner: &str) -> bool {
    matches!(
        owner,
        "java/util/stream/Stream"
            | "java/util/stream/IntStream"
            | "java/util/stream/LongStream"
            | "java/util/stream/DoubleStream"
            | "java/util/stream/BaseStream"
    )
}

impl<H: crate::Host> crate::Runtime<'_, H> {
    /// An unmodifiable `ListN` holding `values` (`Stream.toList()`,
    /// `Collectors.toUnmodifiableList()`).
    pub(crate) fn unmodifiable_list(&mut self, values: Vec<Value>) -> Value {
        let fallback = crate::collection_value("list", &values);
        self.host.unmodifiable_list(values).unwrap_or(fallback)
    }

    /// A new `ArrayList` of its own holding `values` (`Collectors.toList()`).
    pub(crate) fn new_array_list(&mut self, values: Vec<Value>) -> Value {
        match self.host.empty_mutable_collection("list") {
            Some(list) if self.host.mutable_collection_values(&list).is_some() => {
                self.host.replace_mutable_collection(&list, values);
                list
            }
            _ => crate::collection_value("list", &values),
        }
    }

    /// `linkedOrConsumed`: a stream takes one stage or one terminal.
    fn consume(&mut self, stream: &str) -> Result<(), NativeError> {
        if !self.pipelines.used.insert(stream.to_owned()) {
            return Err(illegal_state(Some(
                "stream has already been operated upon or closed",
            )));
        }
        Ok(())
    }

    /// Count `Stream#name` once: a call this model cannot answer as the JDK.
    fn count_stream(&mut self, name: &str, descriptor: &str) -> Result<(), NativeError> {
        self.host
            .unknown_operand_call(&MethodRef {
                owner: "java/util/stream/Stream".into(),
                name: name.into(),
                descriptor: descriptor.into(),
            })
            .map(drop)
    }

    fn lazy_marker(&mut self, class: &str, method: &str) -> String {
        let marker = stream_marker(class, method, self.next_stream);
        self.next_stream += 1;
        marker
    }

    /// Whether `value` is a stream this runtime made (a source or a stage).
    fn is_stream(&self, value: &Value) -> Option<String> {
        let marker = expect_ref(value)?;
        self.is_live_stream(marker).then(|| marker.clone())
    }

    /// Whether `marker` names a live stream of this runtime. A `stream/...`
    /// text that does not is a jar `String` spelled like one.
    pub(crate) fn is_live_stream(&self, marker: &str) -> bool {
        self.streams.contains_key(marker)
            || self.pipelines.links.contains_key(marker)
            || self.pipelines.concats.contains_key(marker)
    }

    /// `text instanceof wanted` for a `stream/...` text that is no live
    /// stream: the `String` it is.
    pub(crate) fn forged_stream_instance_of(&self, text: &str, wanted: &str) -> bool {
        text.starts_with("stream/")
            && !self.is_live_stream(text)
            && matches!(
                wanted,
                "java/lang/String"
                    | "java/lang/Object"
                    | "java/lang/CharSequence"
                    | "java/lang/Comparable"
                    | "java/io/Serializable"
                    | "java/lang/constant/Constable"
                    | "java/lang/constant/ConstantDesc"
            )
    }

    /// `getClass()` on a `stream/...` text. A live stream's class is a
    /// pipeline stage's own (`ReferencePipeline$Head`, `SortedOps$OfRef`...),
    /// which is not modelled: counted. Any other such text is a `String`.
    fn stream_get_class(
        &mut self,
        r: &MethodRef,
        receiver: Option<&Value>,
    ) -> Result<Option<Value>, NativeError> {
        if r.name != "getClass" || r.descriptor != "()Ljava/lang/Class;" {
            return Ok(None);
        }
        let Some(text) = receiver
            .and_then(expect_ref)
            .filter(|text| text.starts_with("stream/"))
        else {
            return Ok(None);
        };
        if self.is_live_stream(text) {
            self.host.unknown_operand_call(r)?;
            return Ok(Some(Value::Ref(None)));
        }
        Ok(Some(Value::Ref(Some("java/lang/String".into()))))
    }

    /// Mark a new stream parallel (`Collection.parallelStream()`).
    pub(crate) fn mark_parallel_stream(&mut self, stream: &Value) {
        if let Some(marker) = expect_ref(stream) {
            let root = self.root(marker);
            self.pipelines.parallel.insert(root);
        }
    }

    /// Mark a value-list source as `collection.stream()`: when the collection
    /// is a fail-fast host collection, the stream binds late and checks for
    /// modification.
    pub(crate) fn mark_stream_origin(&mut self, stream: &Value, collection: &Value) {
        let (Some(marker), Some(source)) = (expect_ref(stream), expect_ref(collection)) else {
            return;
        };
        // A map view's copy: the JDK's view binds late and fails fast on its
        // map, which the copy cannot; a write to the map before the
        // traversal ends is counted.
        if let Some(map) = self.host.view_copy_source(source) {
            let map = crate::jar_object::stored_value(&map);
            let origin = ViewOrigin {
                stamp: self.capture_mod_count(&map),
                entries: crate::runtime_collection_values(self.host, &map).unwrap_or_default(),
                map,
            };
            self.pipelines.view_origins.insert(marker.clone(), origin);
            return;
        }
        let class = crate::jar_object::marker_class(source).unwrap_or("");
        if FAIL_FAST.contains(&class) && self.host.mutable_collection_values(collection).is_some() {
            self.pipelines.origins.insert(
                marker.clone(),
                Origin {
                    collection: collection.clone(),
                    class: class.to_owned(),
                },
            );
        }
    }

    /// The host collection's elements now, as its encoded items.
    fn origin_items(&self, origin: &Origin) -> Result<Vec<String>, NativeError> {
        crate::runtime_collection_values(self.host, &origin.collection)
    }

    /// Bind every host-collection source of `stream` to the collection's
    /// current elements (the spliterator's first use reads them).
    fn bind_sources(&mut self, stream: &str) -> Result<(), NativeError> {
        match self.chain(stream)?.0 {
            Source::Values(marker) => {
                if let Some(origin) = self.pipelines.origins.get(&marker).cloned() {
                    let values = self
                        .origin_items(&origin)?
                        .iter()
                        .map(|item| crate::parse_collection_value(item))
                        .collect::<Result<Vec<_>, _>>()?;
                    self.streams.insert(marker, values);
                }
                Ok(())
            }
            Source::Concat(a, b) => {
                self.bind_sources(&a)?;
                self.bind_sources(&b)
            }
        }
    }

    /// Whether `collection.stream()`'s spliterator reports SIZED: a JDK
    /// collection's does, a concurrent one's does not, except the
    /// copy-on-write ones (`Spliterators.spliterator(array, ...)`). A map
    /// view copy is its map's (`ConcurrentHashMap.keySet()` is not SIZED).
    pub(crate) fn collection_stream_sized(&self, collection: &Value) -> bool {
        let Some(marker) = expect_ref(collection) else {
            return false;
        };
        let source = self
            .host
            .view_copy_source(marker)
            .unwrap_or_else(|| marker.clone());
        let class = crate::jar_object::marker_class_or_text(&source);
        !class.starts_with("java/util/concurrent/")
            || matches!(
                class,
                "java/util/concurrent/CopyOnWriteArrayList"
                    | "java/util/concurrent/CopyOnWriteArraySet"
            )
    }

    /// The source a stream's links lead back to, and the stages from the
    /// source down to the stream.
    fn chain(&self, stream: &str) -> Result<(Source, Vec<Stage>), NativeError> {
        let mut stages = Vec::new();
        let mut at = stream.to_owned();
        loop {
            if let Some(link) = self.pipelines.links.get(&at) {
                stages.push(link.stage.clone());
                at = link.upstream.clone();
                continue;
            }
            stages.reverse();
            if let Some((a, b)) = self.pipelines.concats.get(&at) {
                return Ok((Source::Concat(a.clone(), b.clone()), stages));
            }
            if self.streams.contains_key(&at) {
                return Ok((Source::Values(at), stages));
            }
            return Err(NativeError::Metadata(format!(
                "stream {at} is not owned by this invocation"
            )));
        }
    }

    /// The marker of the source a stream's links lead back to.
    fn root(&self, stream: &str) -> String {
        let mut at = stream;
        while let Some(link) = self.pipelines.links.get(at) {
            at = &link.upstream;
        }
        at.to_owned()
    }

    /// The pipeline's exact size when the JDK knows it without traversal
    /// (`StreamOpFlag.SIZED` and the slices' size adjustment).
    fn known_size(&self, stream: &str) -> Option<i64> {
        let (source, stages) = self.chain(stream).ok()?;
        let mut size = match source {
            // A source is SIZED only where its JDK spliterator reports it
            // (`Runtime::mark_sized`): a collection that is not concurrent,
            // an array, `Stream.of`, a range, a builder.
            Source::Values(marker) => {
                if !self.stream_is_sized(&marker) {
                    return None;
                }
                self.streams.get(&marker)?.len() as i64
            }
            Source::Concat(a, b) => self.known_size(&a)?.checked_add(self.known_size(&b)?)?,
        };
        for stage in &stages {
            size = match stage {
                Stage::Map(..) | Stage::Peek(..) | Stage::Sorted(_) | Stage::Pass => size,
                Stage::Widen(_) => size,
                Stage::Skip(n) => (size - n).max(0),
                Stage::Limit(n) => size.min(*n),
                Stage::Filter(..) | Stage::FlatMap(..) | Stage::Distinct => return None,
            };
        }
        Some(size)
    }

    /// Call `callback` as its functional `interface`'s method, dispatched the
    /// way a JDK body's `invokeinterface` would be.
    pub(crate) fn call_sam(
        &mut self,
        interface: &str,
        callback: &Value,
        args: &[Value],
    ) -> Result<Value, NativeError> {
        match sam(interface) {
            Some((name, descriptor)) => self.call_virtual(
                crate::jdk_call::Site::Interface,
                interface,
                name,
                descriptor,
                callback,
                args,
            ),
            None => self.apply_callback(callback, args),
        }
    }

    /// The lazy half of the stream surface: `Stream.concat`, the builders,
    /// every intermediate stage, and the terminals that run per element.
    /// `Ok(None)` leaves the call to the value-list model.
    pub(crate) fn invoke_lazy_stream(
        &mut self,
        r: &MethodRef,
        receiver: Option<&Value>,
        args: &[Value],
        class: &str,
        method: &str,
    ) -> Result<Option<Value>, NativeError> {
        if let Some(value) = self.stream_get_class(r, receiver)? {
            return Ok(Some(value));
        }
        if let Some(value) = self.stream_builder_call(r, receiver, args)? {
            return Ok(Some(value));
        }
        if r.owner == "java/util/stream/Stream"
            && r.name == "concat"
            && r.descriptor
                == "(Ljava/util/stream/Stream;Ljava/util/stream/Stream;)Ljava/util/stream/Stream;"
        {
            let [a, b] = args else { return Ok(None) };
            if *a == Value::Ref(None) || *b == Value::Ref(None) {
                return Err(npe());
            }
            let (Some(a), Some(b)) = (self.is_stream(a), self.is_stream(b)) else {
                return Ok(None);
            };
            // `a.spliterator()`, `b.spliterator()`: both are consumed.
            self.consume(&a)?;
            self.consume(&b)?;
            let marker = self.lazy_marker(class, method);
            // `Stream.concat` is parallel when either side is.
            if self.pipelines.parallel.contains(&self.root(&a))
                || self.pipelines.parallel.contains(&self.root(&b))
            {
                self.pipelines.parallel.insert(marker.clone());
            }
            self.pipelines.concats.insert(marker.clone(), (a, b));
            return Ok(Some(Value::Ref(Some(marker))));
        }
        if !is_stream_owner(&r.owner) {
            return Ok(None);
        }
        let Some(stream) = receiver.and_then(|value| self.is_stream(value)) else {
            return Ok(None);
        };
        let arg = |index: usize| args.get(index).cloned().unwrap_or(Value::Ref(None));
        let interface = |index: usize| {
            parameter_class(&r.descriptor, index)
                .unwrap_or_default()
                .to_owned()
        };
        let returns = r.descriptor.rsplit_once(')').map_or("", |(_, ret)| ret);
        let to_stream = returns.starts_with("Ljava/util/stream/") && returns.ends_with("Stream;");
        let stage = match (r.name.as_str(), args.len()) {
            ("parallel" | "sequential", 0) if to_stream => {
                // `AbstractPipeline.parallel()` flips the source stage's flag
                // and answers the same stream.
                let root = self.root(&stream);
                if r.name == "parallel" {
                    self.pipelines.parallel.insert(root);
                } else {
                    self.pipelines.parallel.remove(&root);
                }
                return Ok(Some(Value::Ref(Some(stream))));
            }
            ("isParallel", 0) => {
                let parallel = self.pipelines.parallel.contains(&self.root(&stream));
                return Ok(Some(Value::Int(i32::from(parallel))));
            }
            ("map" | "mapToObj" | "mapToInt" | "mapToLong" | "mapToDouble", 1) if to_stream => {
                Stage::Map(arg(0), interface(0))
            }
            ("filter", 1) if to_stream => Stage::Filter(arg(0), interface(0)),
            ("peek", 1) if to_stream => Stage::Peek(arg(0), interface(0)),
            ("flatMap", 1) if to_stream => Stage::FlatMap(arg(0), interface(0)),
            ("skip" | "limit", 1) if to_stream => {
                let Value::Long(n) = arg(0) else {
                    return Ok(None);
                };
                if n < 0 {
                    return Err(NativeError::UncaughtException {
                        class: "java/lang/IllegalArgumentException".into(),
                        message: Some(n.to_string()),
                        frame: None,
                    });
                }
                if r.name == "skip" {
                    Stage::Skip(n)
                } else {
                    Stage::Limit(n)
                }
            }
            ("distinct", 0) if to_stream => Stage::Distinct,
            ("sorted", 0) if to_stream => Stage::Sorted(Value::Ref(None)),
            ("sorted", 1) if to_stream => Stage::Sorted(arg(0)),
            // `IntStream.boxed()` is `mapToObj(Integer::valueOf)`; a `long`
            // or `double` is its own box in the runtime's value model.
            ("boxed", 0) if r.owner == "java/util/stream/IntStream" => Stage::Widen('I'),
            ("boxed" | "unordered", 0) if to_stream => Stage::Pass,
            ("asLongStream", 0) if to_stream => Stage::Widen('J'),
            ("asDoubleStream", 0) if to_stream => Stage::Widen('D'),
            _ => return self.lazy_terminal(r, &stream, args),
        };
        // The JDK's stage constructors `requireNonNull` their argument
        // (`sorted(null)` included: only the no-argument form is natural).
        if matches!(
            &stage,
            Stage::Map(f, _) | Stage::Filter(f, _) | Stage::Peek(f, _) | Stage::FlatMap(f, _)
                if *f == Value::Ref(None)
        ) || (r.name == "sorted" && args.first() == Some(&Value::Ref(None)))
        {
            return Err(npe());
        }
        self.consume(&stream)?;
        let marker = self.lazy_marker(class, method);
        self.pipelines.links.insert(
            marker.clone(),
            Link {
                upstream: stream,
                stage,
            },
        );
        Ok(Some(Value::Ref(Some(marker))))
    }

    /// A terminal that runs its callbacks per element, or `Ok(None)`.
    fn lazy_terminal(
        &mut self,
        r: &MethodRef,
        stream: &str,
        args: &[Value],
    ) -> Result<Option<Value>, NativeError> {
        let arg = |index: usize| args.get(index).cloned().unwrap_or(Value::Ref(None));
        let interface = |index: usize| {
            parameter_class(&r.descriptor, index)
                .unwrap_or_default()
                .to_owned()
        };
        let reference = r.owner == "java/util/stream/Stream";
        let terminal = match (r.name.as_str(), r.descriptor.as_str()) {
            ("forEach" | "forEachOrdered", _) if args.len() == 1 => {
                Terminal::Each(arg(0), interface(0))
            }
            ("anyMatch" | "allMatch" | "noneMatch", _) if args.len() == 1 => Terminal::Match {
                predicate: arg(0),
                interface: interface(0),
                kind: match r.name.as_str() {
                    "anyMatch" => Match::Any,
                    "allMatch" => Match::All,
                    _ => Match::None,
                },
                decided: None,
            },
            ("count", "()J") => {
                self.consume(stream)?;
                self.bind_sources(stream)?;
                if let Some(size) = self.known_size(stream) {
                    let root = self.root(stream);
                    self.check_view_origin(&root)?;
                    self.count_parallel(stream)?;
                    return Ok(Some(Value::Long(size)));
                }
                Terminal::Count(0)
            }
            ("findFirst" | "findAny", "()Ljava/util/Optional;") => Terminal::Find(None),
            ("reduce", "(Ljava/util/function/BinaryOperator;)Ljava/util/Optional;") => {
                Terminal::Reduce(arg(0), interface(0), None)
            }
            ("reduce", _) if args.len() == 2 && interface(1).starts_with("java/util/function/") => {
                Terminal::Fold(arg(1), interface(1), arg(0))
            }
            ("reduce", _) if reference && args.len() == 3 => {
                Terminal::Fold(arg(1), interface(1), arg(0))
            }
            ("max" | "min", "(Ljava/util/Comparator;)Ljava/util/Optional;") => Terminal::Extreme {
                comparator: arg(0),
                max: r.name == "max",
                best: None,
            },
            ("collect", _) if reference && args.len() == 3 => Terminal::Container(arg(1), arg(0)),
            // `Collectors.toMap`: its key and value functions run per element,
            // interleaved with the stages.
            ("collect", "(Ljava/util/stream/Collector;)Ljava/lang/Object;") => {
                let collector = expect_ref(&arg(0)).cloned().unwrap_or_default();
                let Some(state) = self.begin_to_map(&collector)? else {
                    return Ok(None);
                };
                Terminal::ToMap(Box::new(state))
            }
            _ => return Ok(None),
        };
        let callbacks: &[Value] = match &terminal {
            Terminal::Each(f, _) | Terminal::Reduce(f, _, _) | Terminal::Fold(f, _, _) => {
                std::slice::from_ref(f)
            }
            Terminal::Match { predicate, .. } => std::slice::from_ref(predicate),
            Terminal::Extreme { comparator, .. } => std::slice::from_ref(comparator),
            Terminal::Container(..) => args,
            _ => &[],
        };
        if callbacks.contains(&Value::Ref(None)) {
            return Err(npe());
        }
        if !matches!(terminal, Terminal::Count(_)) {
            self.consume(stream)?;
        }
        let terminal = match terminal {
            // `ReduceOps.makeRef(supplier, ...)`: the supplier runs in the
            // sink's `begin`, before the first element.
            Terminal::Container(accumulator, supplier) => {
                let container = self.call_sam("java/util/function/Supplier", &supplier, &[])?;
                Terminal::Container(accumulator, container)
            }
            other => other,
        };
        self.count_parallel(stream)?;
        let result = match self.drive(stream, terminal, None)? {
            Terminal::Each(..) => Value::Void,
            Terminal::Match { kind, decided, .. } => {
                Value::Int(i32::from(decided.unwrap_or(kind != Match::Any)))
            }
            Terminal::Count(n) => Value::Long(n),
            Terminal::Find(found) | Terminal::Reduce(_, _, found) => match found {
                Some(Value::Ref(None)) => return Err(npe()),
                Some(value) => crate::optional_wrap(value),
                None => crate::empty_optional(),
            },
            Terminal::Extreme { best, .. } => match best {
                Some(Value::Ref(None)) => return Err(npe()),
                Some(value) => crate::optional_wrap(value),
                None => crate::empty_optional(),
            },
            Terminal::Fold(_, _, value) | Terminal::Container(_, value) => value,
            Terminal::ToMap(state) => state.finish(),
            Terminal::Collect(_) | Terminal::Forward(..) | Terminal::Wrap(..) => Value::Void,
        };
        Ok(Some(result))
    }

    /// Count a terminal on a parallel pipeline: it runs sequentially here.
    fn count_parallel(&mut self, stream: &str) -> Result<(), NativeError> {
        if self.pipelines.parallel.contains(&self.root(stream)) {
            self.host.unknown_operand_call(&MethodRef {
                owner: "java/util/stream/Stream".into(),
                name: "parallel".into(),
                descriptor: "()Ljava/util/stream/BaseStream;".into(),
            })?;
        }
        Ok(())
    }

    /// The elements a stream yields for the value-list terminal `terminal`:
    /// a source's list, or a stage's output from one full push.
    pub(crate) fn stream_output(
        &mut self,
        stream: &Value,
        terminal: &str,
    ) -> Result<Vec<Value>, NativeError> {
        let marker = expect_ref(stream)
            .filter(|marker| marker.starts_with("stream/"))
            .ok_or_else(|| NativeError::Metadata("expected a native stream marker".into()))?
            .clone();
        self.consume(&marker)?;
        // The JDK's stream iterator pulls lazily as it advances; a pipeline
        // that runs code, or reads a host collection, would see its calls
        // move.
        if matches!(terminal, "iterator" | "spliterator")
            && (!self.streams.contains_key(&marker) || self.pipelines.origins.contains_key(&marker))
        {
            let descriptor = if terminal == "iterator" {
                "()Ljava/util/Iterator;"
            } else {
                "()Ljava/util/Spliterator;"
            };
            self.count_stream(terminal, descriptor)?;
        }
        self.bind_sources(&marker)?;
        if let Some(values) = self.streams.get(&marker).cloned() {
            self.check_view_origin(&marker)?;
            return Ok(values);
        }
        self.count_parallel(&marker)?;
        match self.drive(&marker, Terminal::Collect(Vec::new()), None)? {
            Terminal::Collect(values) => Ok(values),
            _ => Ok(Vec::new()),
        }
    }

    /// Push every element of `stream` through its stages into `terminal`,
    /// and answer the terminal's final state. `outer` is the push a
    /// `flatMap`/`concat` forwards into, whose cancellation this one obeys.
    fn drive(
        &mut self,
        stream: &str,
        terminal: Terminal,
        outer: Option<(u64, usize)>,
    ) -> Result<Terminal, NativeError> {
        let (source, stages) = self.chain(stream)?;
        let mut states = Vec::with_capacity(stages.len());
        // `StreamOpFlag.SORTED` as it stands before each stage: a `TreeSet`
        // source, a natural-order `sorted()`; a mapping or a comparator sort
        // clears it.
        let mut sorted = self.source_sorted(&source);
        // `StreamOpFlag.DISTINCT`: a set's spliterator, or a `distinct()`;
        // a mapping clears it.
        let mut distinct = self.source_distinct(&source);
        for stage in &stages {
            let (sorted_here, distinct_here) = (sorted, distinct);
            distinct = match stage {
                Stage::Distinct => true,
                Stage::Map(..) | Stage::FlatMap(..) | Stage::Pass | Stage::Widen(_) => false,
                _ => distinct,
            };
            sorted = match stage {
                Stage::Sorted(comparator) => *comparator == Value::Ref(None),
                Stage::Filter(..)
                | Stage::Peek(..)
                | Stage::Skip(_)
                | Stage::Limit(_)
                | Stage::Distinct => sorted,
                Stage::Map(..) | Stage::FlatMap(..) | Stage::Pass | Stage::Widen(_) => false,
            };
            states.push(match stage {
                Stage::Skip(n) | Stage::Limit(n) => State::Left(*n),
                // `DistinctOps` on a `DISTINCT` pipeline is the downstream
                // sink itself.
                Stage::Distinct if distinct_here => State::None,
                Stage::Distinct if sorted_here => State::Adjacent(false, None),
                // `DistinctOps`' sink makes its `HashSet` in `begin`.
                Stage::Distinct => State::Seen(self.new_distinct_set()?, Vec::new()),
                Stage::Sorted(_) => State::Buffer(Vec::new()),
                _ => State::None,
            });
        }
        let terminal = match (outer, terminal) {
            (_, wrap @ Terminal::Wrap(..)) => wrap,
            (Some((id, at)), _) => Terminal::Forward(id, at),
            (None, terminal) => terminal,
        };
        let short = stages.iter().any(|stage| matches!(stage, Stage::Limit(_)))
            || match &terminal {
                // `WrappingSpliterator` pulls one element at a time.
                Terminal::Find(_) | Terminal::Match { .. } | Terminal::Wrap(..) => true,
                Terminal::Forward(outer, _) => self
                    .pipelines
                    .drives
                    .get(outer)
                    .is_some_and(|drive| drive.short),
                _ => false,
            };
        if let Source::Values(marker) = &source
            && self.pipelines.origins.contains_key(marker)
        {
            self.bind_sources(marker)?;
        }
        let id = self.pipelines.next_drive;
        self.pipelines.next_drive += 1;
        self.pipelines.drives.insert(
            id,
            Drive {
                stages: Arc::new(stages),
                states,
                terminal,
                short,
            },
        );
        let result = self
            .feed(&source, id)
            .and_then(|()| self.flush(id, 0))
            .and_then(|()| self.drain_wrap(id));
        let drive = self.pipelines.drives.remove(&id);
        result?;
        drive
            .map(|drive| drive.terminal)
            .ok_or_else(|| NativeError::Metadata("stream push lost its state".into()))
    }

    /// Feed a source's elements into push `id` at its first stage, asking
    /// for cancellation before each one.
    fn feed(&mut self, source: &Source, id: u64) -> Result<(), NativeError> {
        match source {
            Source::Values(marker) => {
                let values = self.streams.get(marker).cloned().unwrap_or_default();
                let origin = self.pipelines.origins.get(marker).cloned();
                let Some(origin) = origin else {
                    for value in values {
                        if self.stop_pulling(id)? {
                            break;
                        }
                        self.accept(id, 0, value)?;
                    }
                    return self.check_view_origin(marker);
                };
                self.feed_collection(&origin, values, id)
            }
            Source::Concat(a, b) => {
                let short = self
                    .pipelines
                    .drives
                    .get(&id)
                    .is_some_and(|drive| drive.short);
                for side in [a, b] {
                    if self.stop_pulling(id)? {
                        break;
                    }
                    // `ConcatSpliterator` over each side's `spliterator()`: a
                    // side with stages is a `WrappingSpliterator`, which a
                    // short-circuiting push pulls through a buffer; a bare
                    // source is pulled directly.
                    if short && !self.chain(side)?.1.is_empty() {
                        self.drive(side, Terminal::Wrap(id, 0, Vec::new()), None)?;
                    } else {
                        self.drive(side, Terminal::Collect(Vec::new()), Some((id, 0)))?;
                    }
                }
                Ok(())
            }
        }
    }

    /// A stream over a map view's copy whose map was written to since the
    /// stream was made: the JDK's view would have seen the change (and its
    /// iterator failed fast), the copy did not. Counted.
    fn check_view_origin(&mut self, marker: &str) -> Result<(), NativeError> {
        let Some(origin) = self.pipelines.view_origins.get(marker).cloned() else {
            return Ok(());
        };
        let entries = crate::runtime_collection_values(self.host, &origin.map)?;
        if self.capture_mod_count(&origin.map) != origin.stamp || entries != origin.entries {
            let map = expect_ref(&origin.map).map_or("", String::as_str);
            let class = crate::jar_object::marker_class_or_text(map);
            self.host.unknown_operand_call(&MethodRef {
                owner: class.to_owned(),
                name: "spliterator".into(),
                descriptor: "()Ljava/util/Spliterator;".into(),
            })?;
        }
        Ok(())
    }

    /// Feed a host collection's bound elements, checking its `modCount` as
    /// its fail-fast spliterator does: after each element when pulled
    /// (`tryAdvance`), after the last when pushed in bulk
    /// (`forEachRemaining`). `expectedModCount` is captured here, at the
    /// spliterator's first use.
    fn feed_collection(
        &mut self,
        origin: &Origin,
        values: Vec<Value>,
        id: u64,
    ) -> Result<(), NativeError> {
        use crate::mod_count::ModVerdict;
        let bound = self.origin_items(origin)?;
        let expected = self.capture_mod_count(&origin.collection);
        let short = self
            .pipelines
            .drives
            .get(&id)
            .is_some_and(|drive| drive.short);
        let counted = MethodRef {
            owner: origin.class.clone(),
            name: "spliterator".into(),
            descriptor: "()Ljava/util/Spliterator;".into(),
        };
        let list = APPEND_SAFE.contains(&origin.class.as_str());
        // `ArrayDeque` has no `modCount`: its spliterator checks the slots at
        // its ends, which depends on the array's layout. Any change is
        // counted.
        let tracked = self.has_mod_count(&origin.collection);
        let verdict_now = |this: &mut Self, now: &[String]| {
            if tracked {
                this.capture_mod_count(&origin.collection).verdict(expected)
            } else if now == bound.as_slice() {
                ModVerdict::Same
            } else {
                ModVerdict::Unknown
            }
        };
        let mut uncertain = false;
        // A structural change during a bulk traversal: the JDK's list walk
        // may now read a stale array.
        let mut structural = false;
        for (index, bound_value) in values.into_iter().enumerate() {
            if self.stop_pulling(id)? {
                break;
            }
            // A list's spliterator reads each element from the live list
            // (`elementData[i]`, a node's `item`), so a `set` made during the
            // traversal is seen, as long as nothing structural happened.
            let value = match self.origin_items(origin)?.get(index) {
                Some(item) if list && !structural => crate::parse_collection_value(item)?,
                _ => bound_value,
            };
            self.accept(id, 0, value)?;
            let now = self.origin_items(origin)?;
            let verdict = verdict_now(self, &now);
            let unsure = if short {
                // `tryAdvance` checks `modCount` right after the element.
                match verdict {
                    ModVerdict::Changed => return Err(concurrent_modification()),
                    ModVerdict::Unknown => true,
                    // Other contents with no structural change: a list's
                    // `set`, read live above.
                    ModVerdict::Same => now != bound && !list,
                }
            } else {
                // In bulk the traversal goes on over the bound elements when a
                // list was appended to, or written to and back; any other
                // structural change makes what it visits next depend on the
                // collection's internals (a `HashSet` visits an element added
                // behind its cursor, an `ArrayList` shifts its array).
                structural |= verdict == ModVerdict::Changed;
                let appended = now.len() > bound.len() && now.starts_with(&bound);
                match verdict {
                    ModVerdict::Unknown => true,
                    ModVerdict::Same => now != bound && !list,
                    ModVerdict::Changed => !(list && (now == bound || appended)),
                }
            };
            if unsure && !uncertain {
                uncertain = true;
                self.host.unknown_operand_call(&counted)?;
            }
        }
        // `forEachRemaining`'s check after the last element.
        if !short {
            let now = self.origin_items(origin)?;
            match verdict_now(self, &now) {
                ModVerdict::Changed => return Err(concurrent_modification()),
                ModVerdict::Unknown if !uncertain => {
                    self.host.unknown_operand_call(&counted)?;
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Before the source hands push `id` its next element: pass on what a
    /// `Wrap` terminal buffered from the previous one, then whether to stop
    /// (the push's own `cancellationRequested()`, or for a `Wrap` side, the
    /// outer push no longer pulling).
    fn stop_pulling(&mut self, id: u64) -> Result<bool, NativeError> {
        self.drain_wrap(id)?;
        let outer = match self.pipelines.drives.get(&id).map(|drive| &drive.terminal) {
            Some(Terminal::Wrap(outer, next, _)) => Some((*outer, *next)),
            _ => None,
        };
        Ok(self.cancelled(id, 0) || outer.is_some_and(|(outer, next)| self.cancelled(outer, next)))
    }

    /// Hand a `Wrap` terminal's buffered elements to the outer push, one at a
    /// time, while it does not cancel; the rest are dropped, as the JDK's
    /// buffer is once the outer push stops calling `tryAdvance`.
    fn drain_wrap(&mut self, id: u64) -> Result<(), NativeError> {
        let Some(Terminal::Wrap(outer, next, buffer)) = self
            .pipelines
            .drives
            .get_mut(&id)
            .map(|drive| &mut drive.terminal)
        else {
            return Ok(());
        };
        let (outer, next, buffer) = (*outer, *next, std::mem::take(buffer));
        for value in buffer {
            if self.cancelled(outer, next) {
                break;
            }
            self.accept(outer, next, value)?;
        }
        Ok(())
    }

    /// Whether a source's spliterator reports `DISTINCT`: a host set's.
    fn source_distinct(&self, source: &Source) -> bool {
        match source {
            Source::Values(marker) => self.pipelines.origins.get(marker).is_some_and(|origin| {
                matches!(
                    origin.class.as_str(),
                    "java/util/HashSet" | "java/util/LinkedHashSet" | "java/util/TreeSet"
                )
            }),
            Source::Concat(..) => false,
        }
    }

    /// Whether a source's spliterator reports `SORTED`: a `TreeSet`'s.
    fn source_sorted(&self, source: &Source) -> bool {
        match source {
            Source::Values(marker) => self
                .pipelines
                .origins
                .get(marker)
                .is_some_and(|origin| origin.class == "java/util/TreeSet"),
            Source::Concat(..) => false,
        }
    }

    /// `cancellationRequested()` of push `id`'s sink at stage `at`.
    fn cancelled(&self, id: u64, at: usize) -> bool {
        let Some(drive) = self.pipelines.drives.get(&id) else {
            return true;
        };
        for (index, stage) in drive.stages.iter().enumerate().skip(at) {
            match (stage, &drive.states[index]) {
                (Stage::Limit(_), State::Left(0)) => return true,
                // `SortedOps`' sinks never cancel upstream: they need every
                // element before they can push one.
                (Stage::Sorted(_), _) => return false,
                _ => {}
            }
        }
        match &drive.terminal {
            Terminal::Find(found) => found.is_some(),
            Terminal::Match { decided, .. } => decided.is_some(),
            Terminal::Forward(outer, next) => self.cancelled(*outer, *next),
            _ => false,
        }
    }

    fn state(&mut self, id: u64, at: usize) -> Option<&mut State> {
        self.pipelines.drives.get_mut(&id)?.states.get_mut(at)
    }

    /// `accept(value)` on push `id`'s sink at stage `at`.
    fn accept(&mut self, id: u64, at: usize, value: Value) -> Result<(), NativeError> {
        let Some(stages) = self
            .pipelines
            .drives
            .get(&id)
            .map(|drive| Arc::clone(&drive.stages))
        else {
            return Err(NativeError::Metadata(
                "stream sink outlived its push".into(),
            ));
        };
        let Some(stage) = stages.get(at) else {
            return self.terminal_accept(id, value);
        };
        let next = at + 1;
        match stage {
            Stage::Map(f, interface) => {
                let mapped = self.call_sam(interface, f, &[value])?;
                self.accept(id, next, mapped)
            }
            Stage::Filter(p, interface) => {
                let keep = self.call_sam(interface, p, std::slice::from_ref(&value))?;
                if truth(&keep) {
                    self.accept(id, next, value)?;
                }
                Ok(())
            }
            Stage::Peek(c, interface) => {
                self.call_sam(interface, c, std::slice::from_ref(&value))?;
                self.accept(id, next, value)
            }
            Stage::FlatMap(f, interface) => {
                let inner = self.call_sam(interface, f, &[value])?;
                if inner == Value::Ref(None) {
                    return Ok(());
                }
                let Some(inner) = self.is_stream(&inner) else {
                    return Err(NativeError::Metadata(format!(
                        "flatMap answered {}, not a stream",
                        crate::describe_value(&inner)
                    )));
                };
                self.consume(&inner)?;
                self.drive(&inner, Terminal::Collect(Vec::new()), Some((id, next)))
                    .map(drop)
            }
            Stage::Skip(_) => {
                if let Some(State::Left(left)) = self.state(id, at)
                    && *left > 0
                {
                    *left -= 1;
                    return Ok(());
                }
                self.accept(id, next, value)
            }
            Stage::Limit(_) => {
                if let Some(State::Left(left)) = self.state(id, at)
                    && *left > 0
                {
                    *left -= 1;
                    return self.accept(id, next, value);
                }
                Ok(())
            }
            Stage::Distinct => {
                if let Some(State::None) = self.state(id, at) {
                    return self.accept(id, next, value);
                }
                if let Some(State::Adjacent(seen_null, last)) = self.state(id, at) {
                    // `if (t == null) { if (!seenNull) pass } else if
                    // (lastSeen == null || !t.equals(lastSeen)) pass`.
                    let (seen_null, last) = (*seen_null, last.clone());
                    let passes = if value == Value::Ref(None) {
                        !seen_null
                    } else {
                        match last {
                            None => true,
                            Some(last) => !self.java_equals(&value, &last)?,
                        }
                    };
                    if passes {
                        if let Some(State::Adjacent(seen_null, last)) = self.state(id, at) {
                            if value == Value::Ref(None) {
                                *seen_null = true;
                                *last = None;
                            } else {
                                *last = Some(value.clone());
                            }
                        }
                        self.accept(id, next, value)?;
                    }
                    return Ok(());
                }
                let Some(State::Seen(set, _)) = self.state(id, at) else {
                    return self.accept(id, next, value);
                };
                let set = set.clone();
                let passes = match self.distinct_accept(&set, &value)? {
                    Some(added) => added,
                    // Counted: kept by value.
                    None => match self.state(id, at) {
                        Some(State::Seen(_, by_value)) if !by_value.contains(&value) => {
                            by_value.push(value.clone());
                            true
                        }
                        _ => false,
                    },
                };
                if passes {
                    self.accept(id, next, value)?;
                }
                Ok(())
            }
            Stage::Sorted(_) => {
                if let Some(State::Buffer(buffer)) = self.state(id, at) {
                    buffer.push(value);
                }
                Ok(())
            }
            Stage::Pass => self.accept(id, next, value),
            Stage::Widen(kind) => {
                let widened = match (kind, value) {
                    // `Integer.valueOf`.
                    ('I', Value::Int(v)) => Value::Boxed(crate::Boxed::Integer(v)),
                    ('J', Value::Int(v)) => Value::Long(i64::from(v)),
                    ('D', Value::Int(v)) => Value::Float(f64::from(v)),
                    ('D', Value::Long(v)) => Value::Float(v as f64),
                    (_, other) => other,
                };
                self.accept(id, next, widened)
            }
        }
    }

    /// The end of the source: each `sorted` from `at` on sorts its buffer
    /// and pushes it downstream, then ends its own downstream.
    fn flush(&mut self, id: u64, at: usize) -> Result<(), NativeError> {
        let Some(stages) = self
            .pipelines
            .drives
            .get(&id)
            .map(|drive| Arc::clone(&drive.stages))
        else {
            return Ok(());
        };
        let Some((index, comparator)) =
            stages
                .iter()
                .enumerate()
                .skip(at)
                .find_map(|(index, stage)| match stage {
                    Stage::Sorted(comparator) => Some((index, comparator.clone())),
                    _ => None,
                })
        else {
            return Ok(());
        };
        let buffer = match self.state(id, index) {
            Some(State::Buffer(buffer)) => std::mem::take(buffer),
            _ => Vec::new(),
        };
        // `list.sort(comparator)`: TimSort, the JDK's comparisons in the
        // JDK's order.
        let sorted = self.sort_values(buffer, &comparator)?;
        for value in sorted {
            if self.cancelled(id, index + 1) {
                break;
            }
            self.accept(id, index + 1, value)?;
        }
        self.flush(id, index + 1)
    }

    /// The terminal sink's `accept`.
    fn terminal_accept(&mut self, id: u64, value: Value) -> Result<(), NativeError> {
        let terminal = self
            .pipelines
            .drives
            .get_mut(&id)
            .map(|drive| std::mem::replace(&mut drive.terminal, Terminal::Count(0)))
            .ok_or_else(|| NativeError::Metadata("stream sink outlived its push".into()))?;
        // The terminal is taken out while its callbacks run (they may start
        // pushes of their own) and put back after, also on a throw.
        let mut terminal = terminal;
        let result = self.terminal_step(&mut terminal, value);
        if let Some(drive) = self.pipelines.drives.get_mut(&id) {
            drive.terminal = terminal;
        }
        result
    }

    fn terminal_step(&mut self, terminal: &mut Terminal, value: Value) -> Result<(), NativeError> {
        match terminal {
            Terminal::Collect(values) => {
                if values.len() >= MAX_ELEMENTS {
                    return Err(NativeError::Metadata(format!(
                        "a stream yielded more than {} elements",
                        MAX_ELEMENTS
                    )));
                }
                values.push(value);
            }
            Terminal::Each(consumer, interface) => {
                let (consumer, interface) = (consumer.clone(), interface.clone());
                self.call_sam(&interface, &consumer, &[value])?;
            }
            Terminal::Find(found) => {
                if found.is_none() {
                    *found = Some(value);
                }
            }
            Terminal::Match {
                predicate,
                interface,
                kind,
                decided,
            } => {
                let (predicate, interface, kind) = (predicate.clone(), interface.clone(), *kind);
                let matched = truth(&self.call_sam(&interface, &predicate, &[value])?);
                // `MatchOps`: stop on the element that decides the answer
                // (a match for `anyMatch`/`noneMatch`, a miss for `allMatch`).
                if matched != (kind == Match::All) {
                    *decided = Some(kind == Match::Any);
                }
            }
            Terminal::Count(n) => *n += 1,
            Terminal::Reduce(op, interface, acc) => {
                let next = match acc.take() {
                    None => value,
                    Some(current) => {
                        let (op, interface) = (op.clone(), interface.clone());
                        self.call_sam(&interface, &op, &[current, value])?
                    }
                };
                *acc = Some(next);
            }
            Terminal::Fold(op, interface, acc) => {
                let (op, interface) = (op.clone(), interface.clone());
                let current = std::mem::replace(acc, Value::Void);
                *acc = self.call_sam(&interface, &op, &[current, value])?;
            }
            Terminal::Extreme {
                comparator,
                max,
                best,
            } => {
                let next = match best.take() {
                    None => value,
                    Some(current) => {
                        let comparator = comparator.clone();
                        let order = self.compare_with(&comparator, &current, &value)?;
                        if (*max && order >= 0) || (!*max && order <= 0) {
                            current
                        } else {
                            value
                        }
                    }
                };
                *best = Some(next);
            }
            Terminal::Container(accumulator, container) => {
                let (accumulator, container) = (accumulator.clone(), container.clone());
                self.call_sam(
                    "java/util/function/BiConsumer",
                    &accumulator,
                    &[container, value],
                )?;
            }
            Terminal::ToMap(state) => self.accept_to_map(state, &value)?,
            Terminal::Forward(outer, next) => {
                let (outer, next) = (*outer, *next);
                self.accept(outer, next, value)?;
            }
            Terminal::Wrap(_, _, buffer) => buffer.push(value),
        }
        Ok(())
    }

    /// `Stream.builder()` and the builder's `add`/`accept`/`build`.
    fn stream_builder_call(
        &mut self,
        r: &MethodRef,
        receiver: Option<&Value>,
        args: &[Value],
    ) -> Result<Option<Value>, NativeError> {
        // `Stream.builder()` only: `IntStream`/`LongStream`/`DoubleStream`
        // builders are other classes (`Streams$IntStreamBuilderImpl`, an
        // `IntConsumer`), not modelled, and stay counted.
        if r.name == "builder"
            && r.owner == "java/util/stream/Stream"
            && r.descriptor == "()Ljava/util/stream/Stream$Builder;"
        {
            let key = self.pipelines.next_builder.to_string();
            self.pipelines.next_builder += 1;
            self.pipelines
                .builders
                .insert(key.clone(), Some(Vec::new()));
            return Ok(Some(Value::Host(HostRef::new(
                HostKind::StreamBuilder,
                key,
            ))));
        }
        let Some(builder) = receiver.filter(|value| builder_key(value).is_some()) else {
            return Ok(None);
        };
        let marker = builder_key(builder).unwrap_or_default().to_owned();
        match r.name.as_str() {
            "add" | "accept" => {
                self.builder_accept(&marker, args)?;
                Ok(Some(if r.name == "add" {
                    // `add` answers `this`: the same handle.
                    builder.clone()
                } else {
                    Value::Void
                }))
            }
            "build" => {
                let values = self
                    .pipelines
                    .builders
                    .get_mut(&marker)
                    .and_then(Option::take)
                    .ok_or_else(|| illegal_state(None))?;
                // `StreamBuilderImpl`'s spliterator is SIZED.
                let stream = self.new_stream("java/util/stream/Stream", "build", values);
                self.mark_sized(&stream);
                Ok(Some(stream))
            }
            _ => Ok(None),
        }
    }

    /// `Stream.Builder.accept`: append, or `IllegalStateException` once built.
    fn builder_accept(&mut self, marker: &str, args: &[Value]) -> Result<(), NativeError> {
        let values = self
            .pipelines
            .builders
            .get_mut(marker)
            .and_then(Option::as_mut)
            .ok_or_else(|| illegal_state(None))?;
        if values.len() >= MAX_ELEMENTS {
            return Err(NativeError::Metadata(format!(
                "a Stream.Builder held more than {} elements",
                MAX_ELEMENTS
            )));
        }
        values.push(args.first().cloned().unwrap_or(Value::Ref(None)));
        Ok(())
    }

    /// A builder handle invoked as its functional interface, a `Consumer`
    /// (`list.forEach(builder)`); `None` for any other value.
    pub(crate) fn stream_callback(
        &mut self,
        callback: &Value,
        args: &[Value],
    ) -> Result<Option<Value>, NativeError> {
        let Some(key) = builder_key(callback).map(str::to_owned) else {
            return Ok(None);
        };
        self.builder_accept(&key, args)?;
        Ok(Some(Value::Void))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parameter_classes_skip_primitives_and_arrays() {
        let descriptor = "(I[Ljava/lang/String;JLjava/util/function/IntUnaryOperator;)V";
        assert_eq!(parameter_class(descriptor, 0), None);
        assert_eq!(parameter_class(descriptor, 1), None);
        assert_eq!(parameter_class(descriptor, 2), None);
        assert_eq!(
            parameter_class(descriptor, 3),
            Some("java/util/function/IntUnaryOperator")
        );
        assert_eq!(parameter_class(descriptor, 4), None);
    }
}

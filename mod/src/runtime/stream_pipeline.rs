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
//! called. A change to the collection during the traversal is checked like
//! the JDK's `modCount` check, after each element when the pipeline pulls
//! (`tryAdvance`) and after the last when it pushes (`forEachRemaining`): a
//! changed size throws `ConcurrentModificationException`. What the JDK's
//! bulk traversal visits after a change depends on the collection's
//! internals (a `HashSet` visits an element added behind the cursor), so any
//! change other than appending to a list is counted; so is a list of the
//! same size with different elements, which only `modCount` could tell from
//! a `set`.
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
//! * `distinct()` over objects (`Stream#distinct`): the runtime compares by
//!   value, where the JDK's `HashSet` runs their `equals`/`hashCode`.

use crate::runtime::{MethodRef, NativeError, Value, expect_ref};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

/// Prefix of the stream callback markers (`stream_callback`).
pub(crate) const CALLBACK_PREFIX: &str = "@fn:stream-";

/// Prefix of a `Stream.Builder` marker. A builder is a `Consumer`, so it is a
/// callback marker: `list.forEach(builder)` reaches it through the callback
/// path.
const BUILDER_PREFIX: &str = "@fn:stream-builder:";

/// Prefix of the `Consumer` a push hands a spliterator: `accept(value)` is
/// the sink's `accept` at one stage of one running push
/// (`@fn:stream-sink:<push>:<stage>`).
const SINK_PREFIX: &str = "@fn:stream-sink:";

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
    /// `StreamSupport.stream(spliterator, parallel)`: the spliterator.
    Spliterator(Value),
}

/// What a running stage keeps between elements.
#[derive(Clone, Debug)]
enum State {
    None,
    /// `skip`/`limit`: how many elements are left to drop or pass.
    Left(i64),
    /// `distinct`: the elements seen.
    Seen(Vec<Value>),
    /// `sorted`: the elements buffered until the source ends.
    Buffer(Vec<Value>),
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
    /// `collect(supplier, accumulator, combiner)` before the push begins: the
    /// accumulator and the supplier.
    Supply(Value, Value),
    /// The same once begun: the accumulator and the container the supplier
    /// made.
    Container(Value, Value),
    /// `collect(Collectors.toMap(...))`.
    ToMap(Box<crate::runtime::jdk_model::ToMap>),
    /// The downstream of a `flatMap` or `concat`: the outer push and the
    /// stage its elements go to.
    Forward(u64, usize),
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
    /// `StreamSupport.stream` sources: the spliterator and the
    /// characteristics it reported when the stream was made.
    spliterators: BTreeMap<String, (Value, i32)>,
    /// The modelled JDK spliterators (`jdk_spliterators.rs`).
    pub(crate) splits: crate::runtime::jdk_spliterators::Splits,
    /// Value-list sources whose JDK spliterator is not `SIZED`.
    not_sized: BTreeSet<String>,
    /// Sources whose pipeline was made parallel (`sourceStage.parallel`).
    parallel: BTreeSet<String>,
    /// `Stream.Builder`s: their elements, `None` once built.
    builders: BTreeMap<String, Option<Vec<Value>>>,
    next_builder: usize,
    /// `Collector.of` collectors: supplier, accumulator, and the finisher
    /// unless the collector is `IDENTITY_FINISH`.
    collectors: BTreeMap<String, (Value, Value, Option<Value>)>,
    drives: BTreeMap<u64, Drive>,
    next_drive: u64,
    /// Value-list sources over a fail-fast host collection.
    origins: BTreeMap<String, Origin>,
    /// Streams a stage or terminal has used (`linkedOrConsumed`).
    used: BTreeSet<String>,
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

impl<H: crate::runtime::Host> crate::runtime::Runtime<'_, H> {
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
        let marker = format!("stream/{class}/{method}/{}", self.next_stream);
        self.next_stream += 1;
        marker
    }

    /// Whether `value` is a stream this runtime made (a source or a stage).
    fn is_stream(&self, value: &Value) -> Option<String> {
        let marker = expect_ref(value)?;
        (self.streams.contains_key(marker)
            || self.pipelines.links.contains_key(marker)
            || self.pipelines.concats.contains_key(marker)
            || self.pipelines.spliterators.contains_key(marker))
        .then(|| marker.clone())
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
        let class = source.split_once('#').map_or("", |(class, _)| class);
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
        crate::runtime::runtime_collection_values(self.host, &origin.collection)
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
                        .map(|item| crate::runtime::parse_collection_value(item))
                        .collect::<Result<Vec<_>, _>>()?;
                    self.streams.insert(marker, values);
                }
                Ok(())
            }
            Source::Concat(a, b) => {
                self.bind_sources(&a)?;
                self.bind_sources(&b)
            }
            Source::Spliterator(_) => Ok(()),
        }
    }

    /// Whether a spliterator source feeds `stream` (directly or through a
    /// concatenation).
    fn spliterator_fed(&self, stream: &str) -> bool {
        match self.chain(stream) {
            Ok((Source::Spliterator(_), _)) => true,
            Ok((Source::Concat(a, b), _)) => self.spliterator_fed(&a) || self.spliterator_fed(&b),
            _ => false,
        }
    }

    /// Mark a value-list source whose JDK spliterator is not `SIZED`, so
    /// `count()` traverses it (`Files.list`, `ServiceLoader.stream`).
    pub(crate) fn mark_unsized_stream(&mut self, stream: &Value) {
        if let Some(marker) = expect_ref(stream) {
            self.pipelines.not_sized.insert(marker.clone());
        }
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
            if let Some((spliterator, _)) = self.pipelines.spliterators.get(&at) {
                return Ok((Source::Spliterator(spliterator.clone()), stages));
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

    /// Whether the pipeline's size is known without traversal: a `SIZED`
    /// source (`StreamOpFlag.SIZED`) through stages that keep or adjust it.
    fn sized(&self, stream: &str) -> bool {
        let Ok((source, stages)) = self.chain(stream) else {
            return false;
        };
        let source_sized = match &source {
            Source::Values(marker) => !self.pipelines.not_sized.contains(marker),
            Source::Concat(a, b) => self.sized(a) && self.sized(b),
            Source::Spliterator(_) => self
                .pipelines
                .spliterators
                .get(&self.root(stream))
                .is_some_and(|(_, flags)| flags & crate::runtime::jdk_spliterators::SIZED != 0),
        };
        source_sized
            && !stages.iter().any(|stage| {
                matches!(
                    stage,
                    Stage::Filter(..) | Stage::FlatMap(..) | Stage::Distinct
                )
            })
    }

    /// The source's `getExactSizeIfKnown()`: a value list's length, a
    /// spliterator's answer (a call into it), a concatenation's sum.
    fn source_size(&mut self, source: &Source) -> Result<i64, NativeError> {
        Ok(match source {
            Source::Values(marker) => self.streams.get(marker).map_or(0, Vec::len) as i64,
            Source::Concat(a, b) => {
                let (a, b) = (a.clone(), b.clone());
                let a = self.exact_size(&a)?.unwrap_or(-1);
                let b = self.exact_size(&b)?.unwrap_or(-1);
                if a < 0 || b < 0 {
                    -1
                } else {
                    a.saturating_add(b)
                }
            }
            Source::Spliterator(spliterator) => self.exact_size_if_known(spliterator)?,
        })
    }

    /// The pipeline's exact output size when the JDK knows it without
    /// traversal (`exactOutputSizeIfKnown`, with JDK 18's size-adjusting
    /// slices): asks the source, runs no stage.
    fn exact_size(&mut self, stream: &str) -> Result<Option<i64>, NativeError> {
        if !self.sized(stream) {
            return Ok(None);
        }
        let (source, stages) = self.chain(stream)?;
        let mut size = self.source_size(&source)?;
        if size < 0 {
            return Ok(None);
        }
        for stage in &stages {
            size = match stage {
                Stage::Skip(n) => (size - n).max(0),
                Stage::Limit(n) => size.min(*n),
                _ => size,
            };
        }
        Ok(Some(size))
    }

    /// `StreamSupport.stream(spliterator, parallel)`: reads the
    /// characteristics now (`StreamOpFlag.fromCharacteristics`, which also
    /// asks a `SORTED` one for its comparator); elements are pulled by the
    /// terminal.
    pub(crate) fn stream_of_spliterator(
        &mut self,
        spliterator: &Value,
        parallel: bool,
        class: &str,
        method: &str,
    ) -> Result<Value, NativeError> {
        if *spliterator == Value::Ref(None) {
            return Err(npe());
        }
        let flags = self.characteristics_of(spliterator)?;
        if flags & crate::runtime::jdk_spliterators::SORTED != 0 {
            self.call_virtual(
                crate::runtime::jdk_call::Site::Interface,
                "java/util/Spliterator",
                "getComparator",
                "()Ljava/util/Comparator;",
                spliterator,
                &[],
            )?;
        }
        let marker = self.lazy_marker(class, method);
        self.pipelines
            .spliterators
            .insert(marker.clone(), (spliterator.clone(), flags));
        if parallel {
            self.pipelines.parallel.insert(marker.clone());
        }
        Ok(Value::Ref(Some(marker)))
    }

    /// Call `callback` as its functional `interface`'s method, dispatched the
    /// way a JDK body's `invokeinterface` would be.
    fn call_sam(
        &mut self,
        interface: &str,
        callback: &Value,
        args: &[Value],
    ) -> Result<Value, NativeError> {
        match sam(interface) {
            Some((name, descriptor)) => self.call_virtual(
                crate::runtime::jdk_call::Site::Interface,
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
        if let Some(value) = self.stream_builder_call(r, receiver, args)? {
            return Ok(Some(value));
        }
        if let Some(value) = self.collector_of(r, args)? {
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
            // `ConcatSpliterator` asks both sides' `estimateSize()` and
            // `characteristics()` now; a jar spliterator here would see those
            // calls, which the lazy push makes later.
            if self.spliterator_fed(&a) || self.spliterator_fed(&b) {
                self.count_stream(
                    "concat",
                    "(Ljava/util/stream/Stream;Ljava/util/stream/Stream;)Ljava/util/stream/Stream;",
                )?;
            }
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
                if let Some(size) = self.exact_size(stream)? {
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
            ("collect", _) if reference && args.len() == 3 => Terminal::Supply(arg(1), arg(0)),
            // A `Collector.of` collector: `supplier` in `begin`, the
            // accumulator per element, the finisher after the push; a
            // `Collectors.toMap` collector: its key and value functions per
            // element. Both interleave with the stages.
            ("collect", "(Ljava/util/stream/Collector;)Ljava/lang/Object;") => {
                let collector = expect_ref(&arg(0)).cloned().unwrap_or_default();
                if let Some((supplier, accumulator, finisher)) =
                    self.pipelines.collectors.get(&collector).cloned()
                {
                    self.consume(stream)?;
                    self.count_parallel(stream)?;
                    let Terminal::Container(_, container) =
                        self.drive(stream, Terminal::Supply(accumulator, supplier), None)?
                    else {
                        return Ok(Some(Value::Ref(None)));
                    };
                    return Ok(Some(match finisher {
                        Some(finisher) => {
                            self.call_sam("java/util/function/Function", &finisher, &[container])?
                        }
                        None => container,
                    }));
                }
                let Some(state) = crate::runtime::jdk_model::ToMap::begin(&collector)? else {
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
            Terminal::Supply(..) => args,
            _ => &[],
        };
        if callbacks.contains(&Value::Ref(None)) {
            return Err(npe());
        }
        if !matches!(terminal, Terminal::Count(_)) {
            self.consume(stream)?;
        }
        self.count_parallel(stream)?;
        let result = match self.drive(stream, terminal, None)? {
            Terminal::Each(..) => Value::Void,
            Terminal::Match { kind, decided, .. } => {
                Value::Int(i32::from(decided.unwrap_or(kind != Match::Any)))
            }
            Terminal::Count(n) => Value::Long(n),
            Terminal::Find(found) | Terminal::Reduce(_, _, found) => match found {
                Some(Value::Ref(None)) => return Err(npe()),
                Some(value) => crate::runtime::optional_wrap(value),
                None => crate::runtime::empty_optional(),
            },
            Terminal::Extreme { best, .. } => match best {
                Some(Value::Ref(None)) => return Err(npe()),
                Some(value) => crate::runtime::optional_wrap(value),
                None => crate::runtime::empty_optional(),
            },
            Terminal::Fold(_, _, value) | Terminal::Container(_, value) => value,
            Terminal::ToMap(state) => state.finish(),
            Terminal::Collect(_) | Terminal::Forward(..) | Terminal::Supply(..) => Value::Void,
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
        if let Some(values) = self.streams.get(&marker) {
            return Ok(values.clone());
        }
        self.count_parallel(&marker)?;
        // `toArray`/`toList` size their array first
        // (`AbstractPipeline.exactOutputSizeIfKnown`), which asks a sized
        // source once more before the push's own `begin` does.
        if matches!(terminal, "toArray" | "toList") {
            self.exact_size(&marker)?;
        }
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
        let states = stages
            .iter()
            .map(|stage| match stage {
                Stage::Skip(n) | Stage::Limit(n) => State::Left(*n),
                Stage::Distinct => State::Seen(Vec::new()),
                Stage::Sorted(_) => State::Buffer(Vec::new()),
                _ => State::None,
            })
            .collect();
        let terminal = match outer {
            Some((id, at)) => Terminal::Forward(id, at),
            None => terminal,
        };
        let short = stages.iter().any(|stage| matches!(stage, Stage::Limit(_)))
            || match &terminal {
                Terminal::Find(_) | Terminal::Match { .. } => true,
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
            .begin(&source, id)
            .and_then(|()| self.feed(&source, id))
            .and_then(|()| self.flush(id, 0));
        let drive = self.pipelines.drives.remove(&id);
        result?;
        drive
            .map(|drive| drive.terminal)
            .ok_or_else(|| NativeError::Metadata("stream push lost its state".into()))
    }

    /// The sink chain's `begin(spliterator.getExactSizeIfKnown())`
    /// (`AbstractPipeline.copyInto`), then the terminal's own `begin`: the
    /// supplier of `collect(supplier, accumulator, combiner)`.
    fn begin(&mut self, source: &Source, id: u64) -> Result<(), NativeError> {
        if let Source::Spliterator(spliterator) = source {
            self.exact_size_if_known(spliterator)?;
        }
        let supply = match self.pipelines.drives.get(&id).map(|drive| &drive.terminal) {
            Some(Terminal::Supply(accumulator, supplier)) => {
                Some((accumulator.clone(), supplier.clone()))
            }
            _ => None,
        };
        if let Some((accumulator, supplier)) = supply {
            let container = self.call_sam("java/util/function/Supplier", &supplier, &[])?;
            if let Some(drive) = self.pipelines.drives.get_mut(&id) {
                drive.terminal = Terminal::Container(accumulator, container);
            }
        }
        Ok(())
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
                        if self.cancelled(id, 0) {
                            break;
                        }
                        self.accept(id, 0, value)?;
                    }
                    return Ok(());
                };
                self.feed_collection(&origin, values, id)
            }
            Source::Concat(a, b) => {
                for side in [a, b] {
                    if self.cancelled(id, 0) {
                        break;
                    }
                    self.drive(side, Terminal::Collect(Vec::new()), Some((id, 0)))?;
                }
                Ok(())
            }
            Source::Spliterator(spliterator) => {
                let sink = Value::Ref(Some(format!("{SINK_PREFIX}{id}:0")));
                let short = self
                    .pipelines
                    .drives
                    .get(&id)
                    .is_some_and(|drive| drive.short);
                if short {
                    // `copyIntoWithCancel`: one `tryAdvance` per element.
                    while !self.cancelled(id, 0) {
                        let advanced = self.call_virtual(
                            crate::runtime::jdk_call::Site::Interface,
                            "java/util/Spliterator",
                            "tryAdvance",
                            "(Ljava/util/function/Consumer;)Z",
                            spliterator,
                            std::slice::from_ref(&sink),
                        )?;
                        if !truth(&advanced) {
                            break;
                        }
                    }
                } else {
                    self.call_virtual(
                        crate::runtime::jdk_call::Site::Interface,
                        "java/util/Spliterator",
                        "forEachRemaining",
                        "(Ljava/util/function/Consumer;)V",
                        spliterator,
                        &[sink],
                    )?;
                }
                Ok(())
            }
        }
    }

    /// Feed a host collection's bound elements, checking it for modification
    /// as its fail-fast spliterator does: after each element when pulled,
    /// after the last when pushed in bulk.
    fn feed_collection(
        &mut self,
        origin: &Origin,
        values: Vec<Value>,
        id: u64,
    ) -> Result<(), NativeError> {
        let bound = self.origin_items(origin)?;
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
        let mut uncertain = false;
        for value in values {
            if self.cancelled(id, 0) {
                break;
            }
            self.accept(id, 0, value)?;
            let now = self.origin_items(origin)?;
            if now == bound {
                continue;
            }
            let resized = now.len() != bound.len();
            if short {
                // `tryAdvance` checks `modCount` right after the element.
                if resized || !list {
                    return Err(concurrent_modification());
                }
                // A list of the same size: a `set`, or a remove and an add.
                if !uncertain {
                    uncertain = true;
                    self.host.unknown_operand_call(&counted)?;
                }
                continue;
            }
            // In bulk the traversal goes on over the bound elements only when
            // a list was appended to; anything else changes what it visits.
            let appended = list && now.len() > bound.len() && now.starts_with(&bound);
            if !appended && !uncertain {
                uncertain = true;
                self.host.unknown_operand_call(&counted)?;
            }
        }
        // `forEachRemaining`'s check after the last element.
        if !short && self.origin_items(origin)?.len() != bound.len() {
            return Err(concurrent_modification());
        }
        Ok(())
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
                        crate::runtime::describe_value(&inner)
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
                // An object's `equals`/`hashCode` are its own; the runtime
                // compares by value.
                if expect_ref(&value)
                    .and_then(|marker| crate::runtime::jvm_dispatch::constructed_object_class(marker))
                    .is_some()
                {
                    self.count_stream("distinct", "()Ljava/util/stream/Stream;")?;
                }
                if let Some(State::Seen(seen)) = self.state(id, at) {
                    if seen.contains(&value) {
                        return Ok(());
                    }
                    seen.push(value.clone());
                }
                self.accept(id, next, value)
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
                    ('I', Value::Int(v)) => Value::Ref(Some(format!("integer:{v}"))),
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
            Terminal::Supply(..) => {}
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
        }
        Ok(())
    }

    /// `Collector.of(supplier, accumulator, combiner[, finisher],
    /// characteristics...)`: a collector `collect` runs per element. The
    /// combiner is never called by a sequential stream.
    fn collector_of(
        &mut self,
        r: &MethodRef,
        args: &[Value],
    ) -> Result<Option<Value>, NativeError> {
        if r.owner != "java/util/stream/Collector" || r.name != "of" {
            return Ok(None);
        }
        let (supplier, accumulator, finisher, characteristics) = match args {
            [supplier, accumulator, combiner, characteristics] => {
                if *combiner == Value::Ref(None) {
                    return Err(npe());
                }
                (supplier, accumulator, None, characteristics)
            }
            [supplier, accumulator, combiner, finisher, characteristics] => {
                if *combiner == Value::Ref(None) || *finisher == Value::Ref(None) {
                    return Err(npe());
                }
                (
                    supplier,
                    accumulator,
                    Some(finisher.clone()),
                    characteristics,
                )
            }
            _ => return Ok(None),
        };
        if [supplier, accumulator, characteristics].contains(&&Value::Ref(None)) {
            return Err(npe());
        }
        let identity = self.array(characteristics).is_ok_and(|values| {
            values
                .iter()
                .any(|value| crate::runtime::value_text(value).ends_with("IDENTITY_FINISH"))
        });
        let marker = format!("collector/of/{}", self.pipelines.collectors.len());
        self.pipelines.collectors.insert(
            marker.clone(),
            (
                supplier.clone(),
                accumulator.clone(),
                finisher.filter(|_| !identity),
            ),
        );
        Ok(Some(Value::Ref(Some(marker))))
    }

    /// `Stream.builder()` and the builder's `add`/`accept`/`build`.
    fn stream_builder_call(
        &mut self,
        r: &MethodRef,
        receiver: Option<&Value>,
        args: &[Value],
    ) -> Result<Option<Value>, NativeError> {
        if r.name == "builder"
            && is_stream_owner(&r.owner)
            && r.descriptor.starts_with("()Ljava/util/stream/")
        {
            let marker = format!("{BUILDER_PREFIX}{}", self.pipelines.next_builder);
            self.pipelines.next_builder += 1;
            self.pipelines
                .builders
                .insert(marker.clone(), Some(Vec::new()));
            return Ok(Some(Value::Ref(Some(marker))));
        }
        let Some(marker) = receiver
            .and_then(expect_ref)
            .filter(|marker| marker.starts_with(BUILDER_PREFIX))
            .cloned()
        else {
            return Ok(None);
        };
        match r.name.as_str() {
            "add" | "accept" => {
                self.builder_accept(&marker, args)?;
                Ok(Some(if r.name == "add" {
                    Value::Ref(Some(marker))
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
                Ok(Some(self.new_stream(
                    "java/util/stream/Stream",
                    "build",
                    values,
                )))
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

    /// A stream callback marker invoked as its functional interface: a
    /// builder used as a `Consumer`, or a push's sink handed to a
    /// spliterator.
    pub(crate) fn stream_callback(
        &mut self,
        marker: &str,
        args: &[Value],
    ) -> Result<Option<Value>, NativeError> {
        if marker.starts_with(BUILDER_PREFIX) {
            self.builder_accept(marker, args)?;
            return Ok(Some(Value::Void));
        }
        let Some(rest) = marker.strip_prefix(SINK_PREFIX) else {
            return Ok(None);
        };
        let (id, at) = rest
            .split_once(':')
            .and_then(|(id, at)| Some((id.parse().ok()?, at.parse().ok()?)))
            .ok_or_else(|| NativeError::Metadata(format!("malformed stream sink {marker}")))?;
        let value = args.first().cloned().unwrap_or(Value::Ref(None));
        self.accept(id, at, value)?;
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

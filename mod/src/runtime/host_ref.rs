//! Host objects as typed handles: `Value::Host(HostRef { kind, key })` (#156).
//!
//! The runtime used to name every host object with text (`optional/…`,
//! `identifier/…`, `list/…`), the same `Value::Ref` a Java `String` is, so a
//! jar string spelled like a marker was that object to every reader that
//! checked the prefix. A [`HostRef`] is a separate `Value` kind: nothing a
//! jar spells can be one.
//!
//! A handle is its family ([`HostKind`]) and the family's identity text
//! (`key`). Everything else about the object is a property of the kind's row
//! in the kind table ([`KINDS`]), read through the handle:
//!
//! * **class**: [`HostRef::class`], from the row's [`ClassRule`]: one fixed
//!   class (`java/util/Optional`), or, for a family (jar objects, collections,
//!   game objects), a function of the key; or not known at all (a lambda the
//!   model stands for), when only the types the object surely is are known
//!   ([`ClassRule::Unknown`]): then `getClass()` and every answer that needs
//!   the exact class are counted, never answered with a declared type;
//! * **identity**: [`HostRef::canonical`], the row's predicate on the key
//!   (below);
//! * **`toString`/`hashCode`/`equals`/`instanceof`**: the row's [`Methods`].
//!   They are the only place a kind's object methods live; the runtime
//!   (`object_methods.rs`, `jar_text.rs`) and the host (`StdHost`) both call
//!   them, each passing a [`Nested`] that answers for the values a handle
//!   holds: the runtime's runs a held jar object's own `hashCode`/`equals`/
//!   `toString` through the interpreter, the host's (which cannot run jar
//!   code) answers `None` for one, and the caller counts it;
//! * **casts**: `checkcast` and `instanceof` read [`HostRef::instance_of`],
//!   so `(String) Optional.empty()` throws Java's `ClassCastException`.
//!
//! # Keys
//!
//! A key is plain text, compared whole. It holds **one way**: two handles
//! with equal keys are equal objects (`equals` is `true`, `hashCode` agrees),
//! so a key is built from the canonical form of the kind's `equals` where the
//! kind can (an `OptionalDouble` keys `Double.doubleToLongBits`: every `NaN`
//! one key, `-0.0` not `0.0`). The converse does not hold: two equal objects
//! may have different keys when a held value is not canonical text (two
//! `List.of(1)` are two allocation markers; two jar objects equal by their
//! own `equals`). So a reader never treats different keys as "not equal":
//! `equals` is the row's method, which asks the held values. What does hold
//! both ways is identity: one object never gets two keys, so different keys
//! are different objects (`if_acmp`, [`HostRef::same_object`]).
//!
//! A key that holds values (an optional's value, a pair, a list's elements)
//! is built by [`HostRef::of_values`] and read by [`HostRef::values`], the one
//! encoder/decoder for nested values: each value's `value_marker`, joined
//! with `,`, separators escaped. A handle's own text form (in a collection, a
//! capture) is [`HostRef::encode`]/[`HostRef::decode`]; the list layer around
//! it (`value_marker`/`split_capture_markers`) escapes and unescapes it as it
//! does a string, so a key reads back byte for byte however deep it nests.
//!
//! # Identity
//!
//! A kind's canonical predicate says whether two handles with equal keys are
//! one object:
//!
//! * **canonical** (`true`): the JDK hands out one object per key (an enum
//!   constant, a `Class`, `Optional.empty()`): `==` is key equality, never
//!   counted;
//! * **not canonical** (`false`): equal keys are equal values but may be
//!   different objects (`Optional.of(5)` twice). An identity operation on two
//!   of them (`==`, `!=`, an `IdentityHashMap` key) is counted, unless the
//!   same handle provably flows to both sides, which the identity analysis
//!   decides (`reference_identity.rs`). Equal keys never answer identity.
//!
//! # Adding a kind (P1–P17)
//!
//! Add a [`HostKind`] variant and its row: a unique tag, the class rule, the
//! canonical predicate, and a `Methods` (the kind's module, like
//! `host_optional.rs`). Nothing else matches on kinds for object methods.

use std::{
    cmp::Ordering,
    collections::HashMap,
    hash::{Hash, Hasher},
    sync::{
        Arc, OnceLock,
        atomic::{AtomicU64, Ordering as AtomicOrdering},
    },
};

use crate::runtime::{NativeError, Value};

/// A host object: its family and the family's identity text.
///
/// Equality, hashing and order are the `(kind, key)` pair's: two handles with
/// equal keys are equal values. `origin` is not part of the value: it names
/// the construction that made this handle, so a clone (the same Java
/// reference flowing on through locals, fields, arrays and the host's
/// collections) shares it, and two constructions never do. Same origin is
/// therefore the same object; different origins prove nothing (a model may
/// wrap one Java object twice), see [`HostRef::same_object`].
#[derive(Clone)]
pub struct HostRef {
    kind: HostKind,
    /// A `String` (not `str`) so that a jar object's key reads back as the
    /// `&String` the text readers take (`expect_ref`, [`Self::key_text`]).
    key: Arc<String>,
    origin: u64,
}

/// The next [`HostRef::origin`]: only ever compared for equality, never shown.
static NEXT_ORIGIN: AtomicU64 = AtomicU64::new(1);

impl PartialEq for HostRef {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind && self.key == other.key
    }
}

impl Eq for HostRef {}

impl Hash for HostRef {
    fn hash<S: Hasher>(&self, state: &mut S) {
        self.kind.hash(state);
        self.key.hash(state);
    }
}

impl PartialOrd for HostRef {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for HostRef {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.kind, &self.key).cmp(&(other.kind, &other.key))
    }
}

impl std::fmt::Debug for HostRef {
    /// The value only: an origin differs run to run and would make any text
    /// that prints a handle unstable.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HostRef")
            .field("kind", &self.kind)
            .field("key", &self.key)
            .finish()
    }
}

/// The families moved onto handles so far; the rest are still `Ref` text
/// (#156's inventory).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum HostKind {
    /// `java.util.Optional`: key `""` when empty, else [`HostRef::of_values`]
    /// of the held value.
    Optional,
    /// `java.util.OptionalInt`: key `""` when empty, else the `int`.
    OptionalInt,
    /// `java.util.OptionalLong`: key `""` when empty, else the `long`.
    OptionalLong,
    /// `java.util.OptionalDouble`: key `""` when empty, else the `double`'s
    /// `Double.doubleToLongBits` (every `NaN` one key, `-0.0` not `0.0`).
    OptionalDouble,
    /// `java.nio.file.Path`: key the path as `UnixPath` keeps it
    /// (`host_path.rs`).
    Path,
    /// A NeoForge `FMLPaths` constant: key its name.
    FmlPath,
    /// A `StandardWatchEventKinds` constant: key its name (`host_watch.rs`).
    WatchKind,
    /// A `WatchService`, `WatchKey` or `WatchEvent` of the file model: key
    /// the number the model minted for it.
    WatchService,
    WatchKey,
    WatchEvent,
    /// `java.lang.reflect.Field`: key the member's facts in the nested-key
    /// encoding, then `#<copy>` for a copy a lookup minted (`host_reflect.rs`).
    ReflectField,
    /// `java.lang.reflect.Method`: key the member's facts (`host_reflect.rs`).
    ReflectMethod,
    /// `java.lang.reflect.Constructor`: key the member's facts
    /// (`host_reflect.rs`).
    ReflectConstructor,
    /// A NeoForge `IEventBus`: key `game`, or `mod` with its id when known
    /// (`host_events.rs`; every key below holds its parts escaped).
    NeoForgeBus,
    /// A Forge EventBus 7 per-event `BUS`: key the event class.
    ForgeBus,
    /// A Fabric `Event`: key an API field's owner and name, a
    /// `createArrayBacked` serial, or a registry's text.
    FabricEvent,
    /// A Fabric event's `invoker()`: key the event's key.
    FabricInvoker,
    /// A NeoForge `InterModComms.IMCMessage`: key its index on the IMC bus.
    ImcMessage,
    /// The supplier of an IMC message another jar sent: key the message's index.
    ImcSupplier,
    /// The `CompletableFuture` `ParallelDispatchEvent.enqueueWork` answers: key the task's serial.
    DeferredWork,
    /// An event a host caller dispatches or posts: key its cancelled flag and
    /// name.
    PostedEvent,
    /// `net.minecraft.resources.Identifier`: key `namespace:path` (`host_ids.rs`).
    Identifier,
    /// `net.minecraft.resources.ResourceKey`: key `registry|location`, or
    /// `?…` when the host cannot name them (`host_ids.rs`).
    ResourceKey,
    /// `net.minecraft.tags.TagKey`: key `registry|location`, or `?…`.
    TagKey,
    /// `net.minecraft.references.BlockItemId`: key the block and item ids'
    /// [`HostRef::of_values`].
    BlockItemId,
    /// `java.lang.invoke.MethodType`: key the method descriptor
    /// (`host_invoke.rs`).
    MethodType,
    /// `java.lang.invoke.MethodHandles$Lookup`: key `[mode, class]`
    /// (`host_invoke.rs`).
    MethodHandlesLookup,
    /// A `java.lang.invoke.MethodHandle`: key its combinator tree as JSON
    /// (`method_handles.rs`).
    MethodHandle,
    /// The `ConstantCallSite` `LambdaMetafactory.metafactory` returns: key its
    /// target handle's JSON (`method_handles.rs`).
    CallSite,
    /// A Java array: key `<array class>#<n>`, its elements in the runtime's
    /// `arrays` table (`host_array.rs`).
    Array,
    /// Vanilla's `ColorCollection` record, its sixteen values in the
    /// runtime's `arrays` table: key `color-collection#<n>`
    /// (`color_collection.rs`).
    ColorCollection,
    /// The empty array the runtime answers for an unmodelled call whose
    /// declared result is an array: key `<declared class>#empty`
    /// (`host_array.rs`). Its runtime class is unknown.
    UnknownArray,
    /// A `Registry` (`BuiltInRegistries.X`, a jar's built registry): key its
    /// id (`host_registry.rs`).
    Registry,
    /// A NeoForge `DeferredHolder` or Forge `RegistryObject`: key the loader,
    /// the registration's serial and the entry's key.
    Holder,
    /// The value a holder's registration registered: key the serial.
    Registered,
    /// A Fabric `FabricRegistryBuilder`: key the [`HostRef::of_values`] of
    /// the registry its `buildAndRegister()` answers (`fabric_registry.rs`).
    RegistryBuilder,
    /// The event a NeoForge/Forge game-event listener receives: key its class
    /// (`host_game_event.rs`).
    GameEvent,
    /// The event a NeoForge loading stage posts: key its class and container.
    LifecycleEvent,
    /// cloth-config's `AutoConfig` holder of a config class (`host_config.rs`).
    AutoConfigHolder,
    /// YetAnotherConfigLib's `ConfigClassHandler` of a config class.
    YaclHandler,
    /// YetAnotherConfigLib's `ConfigClassHandler.Builder` of a config class.
    YaclBuilder,
    /// SuperMartijn642's configlib `IConfigBuilder`.
    ConfiglibBuilder,
    /// A value a configlib builder defined: key its index.
    ConfiglibValue,
    /// Mixin's `CallbackInfo` of one injection point (`host_mixin.rs`).
    CallbackInfo,
    /// The `ClassNode` stand-in a Mixin config plugin's apply hooks receive.
    MixinClassNode,
    /// The `IMixinInfo` stand-in a Mixin config plugin's apply hooks receive.
    MixinInfo,
    /// Mixin's `MixinEnvironment`.
    MixinEnvironment,
    /// A `MixinEnvironment.Side` constant: key its name.
    MixinSide,
    /// An `ExecutorService`: key its allocation marker (`host_executors.rs`).
    Executor,
    /// A `Future` an executor answered: key its allocation marker.
    ExecutorFuture,
    /// `Executors.callable(task[, result])`: key the values it holds.
    ExecutorCallable,
    /// `Executors.defaultThreadFactory()`: key its allocation marker.
    ThreadFactory,
    /// A `ServiceLoader`: key its service and load serial (`host_services.rs`).
    ServiceLoader,
    /// A `ServiceLoader` iterator: key its serial.
    ServiceIterator,
    /// A `ServiceLoader.Provider`: key its service, class and loader serial.
    ServiceProvider,
    /// Guava `Splitter.on`: key its separator (`host_guava.rs`).
    GuavaSplitter,
    /// Guava `Joiner.on`: key its separator.
    GuavaJoiner,
    /// Guava `CacheBuilder`: key its allocation marker.
    GuavaCacheBuilder,
    /// Guava `CacheLoader.from(function)`: key the function.
    GuavaCacheLoader,
    /// The thread callbacks run on (`host_jdk.rs`): key `main`.
    Thread,
    /// A logger: key `of_values([api, name])`.
    Logger,
    /// A `StackWalker`: key `""` for `getInstance()`'s, `options` otherwise.
    StackWalker,
    /// A `StackWalker.StackFrame` / `StackTraceElement` of a jar frame: key
    /// `of_values([class, method])`.
    StackFrame,
    StackTraceElement,
    /// A `java.lang.Module`: key `unnamed` or `java.base`.
    Module,
    /// A Fabric `ModContainer`: key its mod id and version
    /// (`host_fabric_loader.rs`).
    FabricModContainer,
    /// A Fabric `ModMetadata`: key its mod id.
    FabricModMetadata,
    /// A Fabric `ModOrigin`: key its kind and argument.
    FabricModOrigin,
    /// A Fabric metadata `Person`: key the mod id and JSON pointer.
    FabricPerson,
    /// A Fabric `ContactInformation`: key the mod id and JSON pointer, or no
    /// values for the shared `ContactInformation.EMPTY`.
    FabricContact,
    /// A Fabric `CustomValue`: key the mod id and JSON pointer, or `1`/`0`/
    /// `null` alone for the shared `true`/`false`/`null` constants.
    FabricCustomValue,
    /// A NeoForge `AttachmentType`: key a serial minted per allocation
    /// (`host_attachment.rs`).
    AttachmentType,
    /// A NeoForge `AttachmentType$Builder`: key a serial minted per allocation.
    AttachmentBuilder,
    /// A NeoForge `ModContainer`: key its mod id and what its class is known
    /// to be (`host_neoforge_loader.rs`).
    NeoForgeModContainer,
    /// A NeoForge `IModInfo`: key its mod id.
    NeoForgeModInfo,
    /// A `ModFileInfo`: key its file's first mod id.
    NeoForgeModFileInfo,
    /// A `ModConfig.Type` constant: key its name.
    NeoForgeConfigType,
    /// A detected `WorldVersion`: key its name.
    WorldVersion,
    /// The command surface (`host_brigadier.rs`): Brigadier's
    /// `CommandDispatcher` a registration callback receives (no key).
    BrigadierDispatcher,
    /// The `CommandBuildContext` a registration callback receives (no key).
    CommandBuildContext,
    /// A running command's `CommandContext`: key the run's serial.
    CommandContext,
    /// A running command's `CommandSourceStack`: key the run's serial and
    /// the source's index in it.
    CommandSource,
    /// A `CommandNode`: key its arena id, kind and name.
    CommandNode,
    /// A `LiteralArgumentBuilder`/`RequiredArgumentBuilder`: key its id and kind.
    ArgumentBuilder,
    /// An `ArgumentType` a factory made: key its arena id and the factory's
    /// declared class.
    ArgumentType,
    /// A `Permission`, `PermissionCheck`, `PermissionSet` or `requires`
    /// predicate: key what it is and its operator level.
    CommandPermission,
    /// `Stream.builder()`'s `Streams.StreamBuilderImpl`: key the runtime's
    /// builder number (`stream_pipeline.rs`).
    StreamBuilder,
    /// The `Sink` a stream push hands a jar spliterator as its `Consumer`:
    /// key `<push>:<stage>` (`stream_pipeline.rs`).
    StreamSink,
    /// `Collector.of(...)`'s `Collectors.CollectorImpl`: key the runtime's
    /// collector number (`stream_pipeline.rs`).
    Collector,
    /// A modelled JDK spliterator over jar code (`jdk_spliterators.rs`): key
    /// `iter:<n>` or `ra:<n>`, which names its class.
    Spliterator,
    /// Fabric Loader's `FabricLoaderImpl.INSTANCE` (`host_fabric_runtime.rs`).
    FabricLoaderInstance,
    /// The loader's one `MappingResolver`.
    FabricMappingResolver,
    /// An `EnvType` constant: key its name.
    FabricEnvType,
    /// An `EntrypointContainer`: key its kind, class, provider and the
    /// requested type.
    FabricEntrypointContainer,
    /// A `PayloadTypeRegistry`: key its direction.
    FabricPayloadTypes,
    /// A `ResourceLoader`: key its `PackType`.
    FabricResourceLoader,
    /// An object `new` (or a model) allocated: key `class#n`
    /// (`jar_object.rs`), one key per allocation.
    JarObject,
    /// A `java.lang.Class`: key its runtime name in internal form, one object
    /// per name in a run (`host_class.rs`).
    Class,
    /// A chat component (`MutableComponent`): key a serial per allocation, then its vanilla
    /// JSON, keys sorted
    /// (`host_component.rs`).
    Component,
    /// DataFixerUpper's `Pair.of(..)`: key `[serial, first, second]`
    /// (`host_codec.rs`).
    DfuPair,
    /// An unmodelled codec call's answer: key its declared codec type
    /// (`host_codec.rs`).
    OpaqueCodec,
    /// A codec the attachment model builds: key `compound-tag` or
    /// `codec:<n>`/`map:<n>` (`host_codec.rs`, `attachment_codec.rs`).
    ModelledCodec,
    /// A `java.lang.reflect.Type` that is no class (`ParameterizedType`,
    /// `TypeVariable`, `WildcardType`, `GenericArrayType`): key the node
    /// (`host_reflect_type.rs`).
    ReflectType,
    /// An `AnnotatedType`: key the type it annotates.
    AnnotatedType,
    /// Gson's `FormattingStyle`: key `[id, newline, indent, space]`, `id` a
    /// constant's name or a serial per allocation (`host_gson.rs`).
    FormattingStyle,
    /// A `GsonBuilder`'s or `Gson`'s option set, held in its synthetic field:
    /// key the runtime's encoding (`host_gson.rs`, `gson_model.rs`).
    GsonOptions,
    /// Guava `TypeToken.of(..)`'s `SimpleTypeToken`: key `[serial, type]`
    /// (`host_type_token.rs`).
    TypeToken,
    /// Gson's canonical `ParameterizedType`/`GenericArrayType`/`WildcardType`
    /// (`GsonTypes$…Impl`): key `[id, node]` (`host_gson_type.rs`).
    GsonType,
    /// Gson's `TypeToken` from a factory (`get`, `getArray`): key
    /// `[serial, canonical type]` (`host_gson_type.rs`).
    GsonTypeToken,
    /// A collection the host made (an unmodelled call's neutral answer,
    /// `subList`, `Collections.unmodifiableList`, `List.of`, a reversed
    /// view, ...): key `<JDK class>#neutral<n>`, `#immutable<n>` or
    /// `#reversed<n>`, the text it used to be (`jar_object.rs`), one key per
    /// allocation. An object like a
    /// jar's `new ArrayList()` (a [`HostKind::JarObject`]), whose row it
    /// shares.
    Collection,
    /// A `TimeUnit` constant: key its name (`host_jdk_constant.rs`).
    TimeUnit,
    /// A `CodingErrorAction` constant: key its name.
    CodingErrorAction,
    /// A `Character.UnicodeBlock` constant: key its name.
    UnicodeBlock,
    /// A `Locale` constant: key `<language>_<country>`.
    Locale,
    /// A compiled `Pattern`: key `[serial, text]`.
    RegexPattern,
    /// NeoForge's `ArtifactVersion`: key its text.
    ArtifactVersion,
    /// `java.util.UUID`: key its canonical text (`host_jdk_value.rs`).
    Uuid,
    /// `java.time.Duration`: key its nanoseconds (`host_jdk_value.rs`).
    Duration,
    /// `java.net.URI`: key its text (`host_jdk_value.rs`).
    Uri,
    /// A game entity (`ServerPlayer`, a mob, the client's `LocalPlayer`): key
    /// its marker (`game/player/<uuid>`, `game/entity/<id>`), one object per
    /// entity (`game_object.rs`, #156 P16).
    GameEntity,
    /// A level (`ServerLevel`, `ClientLevel`): key its marker, one per dimension.
    GameLevel,
    /// A game service with one object per name (the server, its player list,
    /// game rules and registries, a level's world border, the client's
    /// `Minecraft`, `Gui`...): key its marker.
    GameSingleton,
    /// A registry entry (an `Item`, a `Block`, a `BlockState`, a `GameRule`,
    /// a registry): key its marker, one per entry.
    GameRegistryObject,
    /// A game value (`BlockPos`, `LevelResource`, `Style`...): key its marker,
    /// which spells its fields; `equals` by them, `==` per producer.
    GameValue,
    /// A game object that is its own object each time (a `DamageSource`, an
    /// `ItemStack`, a hit result, a placeholder for an object the hook did not
    /// give): key its marker, never canonical.
    GameTransient,
    /// A collection's `iterator()`: key `iterator/<n>`, the host's table key, or
    /// `iterator/empty` for `Collections.emptyIterator()` (`host_iterator.rs`).
    Iterator,
    /// An `Enumeration` (`ClassLoader.getResources`, `Collections.enumeration`,
    /// `Vector.elements()`): key `enumeration/<n>` or `enumeration/empty`.
    Enumeration,
    /// A list's `listIterator()`: key `list-iterator/<n>`.
    ListIterator,
}

/// How a kind's Java class follows from its key.
#[derive(Clone, Copy)]
pub(crate) enum ClassRule {
    /// Every object of the kind is this class.
    Fixed(&'static str),
    /// A family whose objects are of many classes (jar objects, collections,
    /// game objects, arrays), or a class other values than handles may also be
    /// (`CompletableFuture`): the class is read from the key.
    FromKey(fn(&str) -> &str),
    /// The object's runtime class is not known (a jar's lambda, a proxy, a
    /// loader class the runtime does not model). `declared` answers, for a
    /// key, the types the object surely is (the call site's declared type).
    /// [`HostRef::class`] is `None`, so `getClass()`, a
    /// `ClassCastException`'s message and anything else that needs the exact
    /// class are counted; `instanceof` and casts follow
    /// [`instance_of_class`]: `true` for a declared type's supertype, `false`
    /// only where [`proves_not_instance`] proves it, counted otherwise.
    Unknown {
        declared: fn(&str) -> &'static [&'static str],
    },
}

/// `class`'s generated Java 25 row (`jdk_closure.rs`): whether it is final, and every supertype
/// (the class itself left out).
fn jdk_closed(class: &str) -> Option<(bool, &'static [&'static str])> {
    crate::runtime::jdk_closure::class(class).map(|row| (row.is_final, row.supertypes.as_slice()))
}

/// Whether an object whose exact class is `class` is provably not a
/// `wanted`: `wanted` is a `final` JDK class (Java 25's table) other than `class`,
/// whose only instances are of that very class.
pub(crate) fn exact_class_rules_out(class: &str, wanted: &str) -> bool {
    class != wanted && jdk_closed(wanted).is_some_and(|(is_final, _)| is_final)
}

/// Whether no object can be both a `class` and a `declared`, proved: `class`
/// is `final` and its whole supertype set is known without `declared`.
/// Only the JDK classes of Java 25's table (`jdk_closure.rs`) qualify. A jar class never does:
/// another jar's Mixin (or a class this runtime does not hold) can add an
/// interface to it, which no single jar's runtime sees.
pub(crate) fn proves_not_instance(class: &str, declared: &str) -> bool {
    class != declared
        && jdk_closed(class)
            .is_some_and(|(is_final, supertypes)| is_final && !supertypes.contains(&declared))
}

/// Answers for the values a handle holds, for a kind's [`Methods`]. `None`
/// is "unknown here": the caller counts the call instead of guessing.
pub(crate) trait Nested {
    /// `Objects.hashCode(value)`.
    fn hash(&mut self, value: &Value) -> Result<Option<i32>, NativeError>;
    /// `Objects.equals(a, b)`.
    fn equals(&mut self, a: &Value, b: &Value) -> Result<Option<bool>, NativeError>;
    /// `String.valueOf(value)`.
    fn text(&mut self, value: &Value) -> Result<Option<String>, NativeError>;
    /// The vanilla JSON a chat component holds now (its host's state,
    /// `host_component.rs`); `None` where this context cannot read the
    /// host's table (the caller counts).
    fn component_json(&mut self, value: &Value) -> Option<serde_json::Value> {
        let _ = value;
        None
    }
    /// The `(msb, lsb)` of a jar's `new UUID(msb, lsb)` object
    /// (`host_jdk_value.rs`); `None` where this context cannot read the
    /// host's table or the object is no constructed UUID (the caller counts).
    fn uuid_bits(&mut self, value: &Value) -> Option<(i64, i64)> {
        let _ = value;
        None
    }
    /// The canonical type a jar's `new com.google.gson.reflect.TypeToken<T>() {}` holds
    /// (`gson_type_token.rs`); `None` where this context cannot read jar objects' fields.
    fn gson_token_type(&mut self, value: &Value) -> Option<Value> {
        let _ = value;
        None
    }
    /// The type a jar's `new com.google.common.reflect.TypeToken<T>() {}` captured
    /// (`guava_model.rs`); `None` where this context cannot read jar objects' fields.
    fn guava_token_type(&mut self, value: &Value) -> Option<Value> {
        let _ = value;
        None
    }
}

/// `toString()` of a handle.
pub(crate) type ToStringFn = fn(&HostRef, &mut dyn Nested) -> Result<Option<String>, NativeError>;
/// `hashCode()` of a handle.
pub(crate) type HashCodeFn = fn(&HostRef, &mut dyn Nested) -> Result<Option<i32>, NativeError>;
/// `handle.equals(other)`.
pub(crate) type EqualsFn =
    fn(&HostRef, &Value, &mut dyn Nested) -> Result<Option<bool>, NativeError>;
/// `handle instanceof wanted` (`wanted` an internal class name), asking
/// `hierarchy` about classes; `None` when the hierarchy cannot tell (the
/// caller counts the check).
pub(crate) type InstanceOfFn = fn(&HostRef, &str, &dyn ClassHierarchy) -> Option<bool>;
/// Whether a handle means the same in another jar's runtime (NeoForge IMC):
/// `crosses` answers for a held value.
pub(crate) type CrossesFn = fn(&HostRef, &dyn Fn(&Value) -> bool) -> bool;
/// `handle.compareTo(other)` (`Comparable`); `None` when it is unknown here
/// (the caller counts it).
pub(crate) type CompareToFn =
    fn(&HostRef, &Value, &mut dyn Nested) -> Result<Option<i32>, NativeError>;

/// The class hierarchy a kind's `instance_of` reads: the JDK's supertypes,
/// and in the runtime the jar's own classes (a jar object's superclasses and
/// interfaces, a Mixin's added interface), which a family kind needs.
pub(crate) trait ClassHierarchy {
    /// Whether an object of class `class` is a `wanted` (internal names).
    /// `Some(false)` only when proved; `None` when unknown (a class whose
    /// chain leaves what is known).
    fn is_instance(&self, class: &str, wanted: &str) -> Option<bool>;

    /// Whether `class` is an interface; `None` when unknown.
    fn is_interface(&self, class: &str) -> Option<bool> {
        crate::runtime::jdk_closure::class(class).map(|row| row.is_interface)
    }
}

/// The JDK's own hierarchy, for a caller with no runtime: a JDK class never
/// extends or implements a jar type.
pub(crate) struct JdkHierarchy;

impl ClassHierarchy for JdkHierarchy {
    /// `true` from the JDK models' supertypes; `false` only when proved: a
    /// class of Java 25's table lacks `wanted`, `wanted` is a final JDK class
    /// other than `class`, or a JDK class against a type outside every system
    /// module's packages (the loaders that define JDK classes cannot see a
    /// jar's or a game's). Unknown otherwise.
    fn is_instance(&self, class: &str, wanted: &str) -> Option<bool> {
        if wanted == "java/lang/Object"
            || class == wanted
            || crate::runtime::jdk_model::class_instance_of(class, wanted)
        {
            return Some(true);
        }
        if let Some((_, supertypes)) = jdk_closed(class) {
            return Some(supertypes.contains(&wanted));
        }
        if exact_class_rules_out(class, wanted) {
            return Some(false);
        }
        let jdk = crate::runtime::jdk_closure::in_jdk_package;
        (jdk(class) && !jdk(wanted)).then_some(false)
    }
}

/// A kind's `Object` methods, as the JDK class implements them, and the other
/// per-kind behaviour a shared caller needs.
pub(crate) struct Methods {
    pub(crate) to_string: ToStringFn,
    pub(crate) hash_code: HashCodeFn,
    pub(crate) equals: EqualsFn,
    pub(crate) instance_of: InstanceOfFn,
    pub(crate) crosses_jars: CrossesFn,
    /// Natural order, for a kind whose class is `Comparable`: every caller of
    /// `compareTo` (a direct call, a sort, a `TreeMap`, `sorted()`) reaches
    /// it. `None` for a kind that is not `Comparable`, or not modelled: the
    /// caller counts the comparison.
    pub(crate) compare_to: Option<CompareToFn>,
}

/// Facts about a class *name* a family kind answers for (no handle, no key): the class a handle's
/// `getClass()` returns, an `ldc` of it, or a supertype in its family. A `FromKey` kind whose
/// class names the reflective `Class` surface can reach has one ([`CLASS_SIDES`]); a `Fixed` kind
/// is found by [`HostKind::of_class`], an `Unknown` one never (its declared types are not its
/// class).
///
/// Two rules every row keeps: **a class file the run holds wins** (a jar's own copy of an owned
/// class answers from that file: the runtime asks `Runtime::owned_class_side`, which defers to
/// it), and **one owner per name** (no two rows, nor a row and a fixed class, own one name: checked
/// over every row by a unit test, and by `owning_class` in debug builds).
pub(crate) struct ClassSide {
    /// Whether this kind answers for the class name `class` (its handles' classes and their
    /// family's supertypes). No two kinds own one class.
    pub(crate) owns: fn(&str) -> bool,
    /// `class instanceof wanted` (a subtype check: `Class.isAssignableFrom`, a class walk), the
    /// row's own rule; `None` = counted.
    pub(crate) instance_of: InstanceOfClassFn,
    /// The direct superclass, when known exactly; `None` = counted.
    pub(crate) superclass: fn(&str) -> Option<String>,
    /// Whether `class` is an interface, when known; `None` = counted.
    pub(crate) is_interface: fn(&str) -> Option<bool>,
    /// Every class name `owns` accepts (its family's table), so that "one owner per name" can be
    /// checked over every row (read by host_ref's unit test, not at run time).
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) classes: fn() -> Vec<String>,
}

/// [`ClassSide::instance_of`].
pub(crate) type InstanceOfClassFn = fn(&str, &str, &dyn ClassHierarchy) -> Option<bool>;

/// The kinds with a [`ClassSide`], in `KINDS` order.
const CLASS_SIDES: &[(HostKind, &ClassSide)] = &[
    (
        HostKind::GsonType,
        &crate::runtime::host_gson_type::GSON_TYPE_CLASS_SIDE,
    ),
    (
        HostKind::GsonTypeToken,
        &crate::runtime::host_gson_type::TOKEN_CLASS_SIDE,
    ),
    (HostKind::DfuPair, &crate::runtime::host_codec::PAIR_CLASS_SIDE),
    (
        HostKind::GameEvent,
        &crate::runtime::host_game_event::EVENT_CLASS_SIDE,
    ),
    (
        HostKind::LifecycleEvent,
        &crate::runtime::host_game_event::LIFECYCLE_CLASS_SIDE,
    ),
    (
        HostKind::AttachmentType,
        &crate::runtime::host_attachment::TYPE_CLASS_SIDE,
    ),
    (
        HostKind::AttachmentBuilder,
        &crate::runtime::host_attachment::BUILDER_CLASS_SIDE,
    ),
    (
        HostKind::FabricEnvType,
        &crate::runtime::host_fabric_runtime::ENV_TYPE_CLASS_SIDE,
    ),
    (
        HostKind::Component,
        &crate::runtime::host_component::COMPONENT_CLASS_SIDE,
    ),
    (
        HostKind::FormattingStyle,
        &crate::runtime::host_gson::STYLE_CLASS_SIDE,
    ),
    (
        HostKind::TypeToken,
        &crate::runtime::host_type_token::TOKEN_CLASS_SIDE,
    ),
];

/// One row of the kind table.
struct KindInfo {
    kind: HostKind,
    /// The tag in a handle's text form (`h<tag>:<key>`); unique per kind.
    tag: &'static str,
    class: ClassRule,
    /// Whether two handles with this key are one object.
    canonical: fn(&str) -> bool,
    methods: &'static Methods,
}

/// The canonical predicate of a kind whose empty key is its singleton
/// (`Optional.empty()`) and whose other keys are values.
pub(crate) fn canonical_when_empty(key: &str) -> bool {
    key.is_empty()
}

/// The canonical predicate of a kind with one object per key.
pub(crate) fn always_canonical(_key: &str) -> bool {
    true
}

/// The canonical predicate of a kind whose every object is a fresh value.
pub(crate) fn never_canonical(_key: &str) -> bool {
    false
}

/// [`InstanceOfFn`] for a kind whose class is all it is: the class and its
/// supertypes, as `hierarchy` knows them. For a [`ClassRule::Unknown`] kind:
/// `true` for a supertype of a declared type; `false` only where
/// [`proves_not_instance`] proves a declared type rules `wanted` out; unknown
/// (counted) otherwise.
pub(crate) fn instance_of_class(
    handle: &HostRef,
    wanted: &str,
    hierarchy: &dyn ClassHierarchy,
) -> Option<bool> {
    let declared = match handle.kind.info().class {
        ClassRule::Unknown { declared } => declared(handle.key()),
        _ => return hierarchy.is_instance(handle.class()?, wanted),
    };
    if wanted == "java/lang/Object"
        || declared
            .iter()
            .any(|class| hierarchy.is_instance(class, wanted) == Some(true))
    {
        return Some(true);
    }
    // The object is a `declared`; `wanted` is final and provably not one, so
    // the object cannot be a `wanted`.
    declared
        .iter()
        .any(|class| proves_not_instance(wanted, class))
        .then_some(false)
}

/// Every kind: its tag, its class, its identity, its methods.
const KINDS: &[KindInfo] = &[
    KindInfo {
        kind: HostKind::Optional,
        tag: "opt",
        class: ClassRule::Fixed("java/util/Optional"),
        canonical: canonical_when_empty,
        methods: &crate::runtime::host_optional::OPTIONAL,
    },
    KindInfo {
        kind: HostKind::OptionalInt,
        tag: "opti",
        class: ClassRule::Fixed("java/util/OptionalInt"),
        canonical: canonical_when_empty,
        methods: &crate::runtime::host_optional::OPTIONAL_INT,
    },
    KindInfo {
        kind: HostKind::OptionalLong,
        tag: "optl",
        class: ClassRule::Fixed("java/util/OptionalLong"),
        canonical: canonical_when_empty,
        methods: &crate::runtime::host_optional::OPTIONAL_LONG,
    },
    KindInfo {
        kind: HostKind::OptionalDouble,
        tag: "optd",
        class: ClassRule::Fixed("java/util/OptionalDouble"),
        canonical: canonical_when_empty,
        methods: &crate::runtime::host_optional::OPTIONAL_DOUBLE,
    },
    KindInfo {
        kind: HostKind::Path,
        tag: "path",
        class: ClassRule::Fixed("sun/nio/fs/UnixPath"),
        canonical: never_canonical,
        methods: &crate::runtime::host_path::PATH,
    },
    KindInfo {
        kind: HostKind::FmlPath,
        tag: "fmlp",
        class: ClassRule::Fixed(crate::runtime::host_path::FML_PATHS_CLASS),
        canonical: always_canonical,
        methods: &crate::runtime::host_path::FML_PATHS,
    },
    KindInfo {
        kind: HostKind::WatchKind,
        tag: "wkind",
        class: ClassRule::Fixed("java/nio/file/StandardWatchEventKinds$StdWatchEventKind"),
        canonical: always_canonical,
        methods: &crate::runtime::host_watch::WATCH_KIND,
    },
    KindInfo {
        kind: HostKind::WatchService,
        tag: "wsvc",
        class: ClassRule::Fixed("sun/nio/fs/LinuxWatchService"),
        canonical: always_canonical,
        methods: &crate::runtime::host_watch::WATCH_OBJECT,
    },
    KindInfo {
        kind: HostKind::WatchKey,
        tag: "wkey",
        class: ClassRule::Fixed("sun/nio/fs/LinuxWatchService$LinuxWatchKey"),
        canonical: always_canonical,
        methods: &crate::runtime::host_watch::WATCH_OBJECT,
    },
    KindInfo {
        kind: HostKind::WatchEvent,
        tag: "wevt",
        class: ClassRule::Fixed("sun/nio/fs/AbstractWatchKey$Event"),
        canonical: always_canonical,
        methods: &crate::runtime::host_watch::WATCH_OBJECT,
    },
    KindInfo {
        kind: HostKind::ReflectField,
        tag: "rfld",
        class: ClassRule::Fixed(crate::runtime::host_reflect::FIELD),
        canonical: crate::runtime::host_reflect::field_canonical,
        methods: &crate::runtime::host_reflect::MEMBER_METHODS,
    },
    KindInfo {
        kind: HostKind::ReflectMethod,
        tag: "rmth",
        class: ClassRule::Fixed(crate::runtime::host_reflect::METHOD),
        canonical: never_canonical,
        methods: &crate::runtime::host_reflect::MEMBER_METHODS,
    },
    KindInfo {
        kind: HostKind::ReflectConstructor,
        tag: "rctr",
        class: ClassRule::Fixed(crate::runtime::host_reflect::CONSTRUCTOR),
        canonical: never_canonical,
        methods: &crate::runtime::host_reflect::MEMBER_METHODS,
    },
    KindInfo {
        kind: HostKind::NeoForgeBus,
        tag: "nfbus",
        class: ClassRule::FromKey(crate::runtime::host_events::neoforge_bus_class),
        canonical: crate::runtime::host_events::bus_canonical,
        methods: &crate::runtime::host_events::NEOFORGE_BUS_METHODS,
    },
    KindInfo {
        kind: HostKind::ForgeBus,
        tag: "fgbus",
        class: ClassRule::Unknown {
            declared: crate::runtime::host_events::forge_bus_types,
        },
        canonical: always_canonical,
        methods: &crate::runtime::host_events::DECLARED_TYPE_METHODS,
    },
    KindInfo {
        kind: HostKind::FabricEvent,
        tag: "fevt",
        class: ClassRule::FromKey(crate::runtime::host_events::fabric_event_class),
        canonical: always_canonical,
        methods: &crate::runtime::host_events::FABRIC_EVENT_METHODS,
    },
    KindInfo {
        kind: HostKind::FabricInvoker,
        tag: "finv",
        class: ClassRule::Unknown {
            declared: crate::runtime::host_events::no_types,
        },
        // `register` builds a new invoker: two reads are not provably one.
        canonical: never_canonical,
        methods: &crate::runtime::host_events::DECLARED_TYPE_METHODS,
    },
    KindInfo {
        kind: HostKind::ImcMessage,
        tag: "imcm",
        class: ClassRule::FromKey(crate::runtime::host_events::imc_message_class),
        canonical: always_canonical,
        methods: &crate::runtime::host_events::IMC_MESSAGE_METHODS,
    },
    KindInfo {
        kind: HostKind::ImcSupplier,
        tag: "imcs",
        class: ClassRule::Unknown {
            declared: crate::runtime::host_events::supplier_types,
        },
        canonical: always_canonical,
        methods: &crate::runtime::host_events::DECLARED_TYPE_METHODS,
    },
    KindInfo {
        kind: HostKind::DeferredWork,
        tag: "nfdw",
        class: ClassRule::FromKey(crate::runtime::host_events::future_class),
        canonical: always_canonical,
        methods: &crate::runtime::host_events::DEFERRED_WORK_METHODS,
    },
    KindInfo {
        kind: HostKind::PostedEvent,
        tag: "pev",
        class: ClassRule::Unknown {
            declared: crate::runtime::host_events::posted_event_types,
        },
        canonical: never_canonical,
        methods: &crate::runtime::host_events::DECLARED_TYPE_METHODS,
    },
    KindInfo {
        kind: HostKind::Identifier,
        tag: "id",
        class: ClassRule::Fixed(crate::runtime::host_ids::IDENTIFIER),
        // Not interned: two `Identifier.parse("a:x")` are two objects.
        canonical: never_canonical,
        methods: &crate::runtime::host_ids::IDENTIFIER_METHODS,
    },
    KindInfo {
        kind: HostKind::ResourceKey,
        tag: "rk",
        class: ClassRule::Fixed(crate::runtime::host_ids::RESOURCE_KEY),
        // Interned by `ResourceKey.create` (`host_ids.rs`).
        canonical: always_canonical,
        methods: &crate::runtime::host_ids::RESOURCE_KEY_METHODS,
    },
    KindInfo {
        kind: HostKind::TagKey,
        tag: "tk",
        class: ClassRule::Fixed(crate::runtime::host_ids::TAG_KEY),
        // Interned by `TagKey.create`.
        canonical: always_canonical,
        methods: &crate::runtime::host_ids::TAG_KEY_METHODS,
    },
    KindInfo {
        kind: HostKind::BlockItemId,
        tag: "bii",
        class: ClassRule::Fixed(crate::runtime::host_ids::BLOCK_ITEM_ID),
        canonical: never_canonical,
        methods: &crate::runtime::host_ids::BLOCK_ITEM_ID_METHODS,
    },
    KindInfo {
        kind: HostKind::MethodType,
        tag: "mtype",
        class: ClassRule::Fixed(crate::runtime::host_invoke::METHOD_TYPE),
        // Java interns method types: one object per descriptor.
        canonical: always_canonical,
        methods: &crate::runtime::host_invoke::METHOD_TYPE_METHODS,
    },
    KindInfo {
        kind: HostKind::MethodHandlesLookup,
        tag: "mhlk",
        class: ClassRule::Fixed(crate::runtime::host_invoke::LOOKUP),
        canonical: crate::runtime::host_invoke::lookup_canonical,
        methods: &crate::runtime::host_invoke::LOOKUP_METHODS,
    },
    KindInfo {
        kind: HostKind::MethodHandle,
        tag: "mh",
        // `DirectMethodHandle`, `BoundMethodHandle$Species_LL`, …: a
        // `MethodHandle` of a class the runtime does not know.
        class: ClassRule::Unknown {
            declared: crate::runtime::method_handles::handle_types,
        },
        canonical: never_canonical,
        methods: &crate::runtime::method_handles::HANDLE_METHODS,
    },
    KindInfo {
        kind: HostKind::CallSite,
        tag: "csite",
        // Java 25's metafactory returns a plain `ConstantCallSite`. `FromKey`
        // keeps that exact class off `HostKind::of_class`, since a jar may
        // extend `ConstantCallSite`.
        class: ClassRule::FromKey(crate::runtime::method_handles::call_site_class),
        canonical: never_canonical,
        methods: &crate::runtime::method_handles::CALL_SITE_METHODS,
    },
    KindInfo {
        kind: HostKind::Array,
        tag: "arr",
        class: ClassRule::FromKey(crate::runtime::host_array::class_of_key),
        canonical: crate::runtime::host_array::canonical,
        methods: &crate::runtime::host_array::ARRAY,
    },
    KindInfo {
        kind: HostKind::ColorCollection,
        tag: "colors",
        class: ClassRule::Fixed(crate::runtime::color_collection::OWNER),
        // One key per collection the runtime built.
        canonical: always_canonical,
        methods: &crate::runtime::color_collection::COLOR_COLLECTION,
    },
    KindInfo {
        kind: HostKind::UnknownArray,
        tag: "arru",
        class: ClassRule::Unknown {
            declared: crate::runtime::host_array::surely,
        },
        // One key stands for every array such a call returned.
        canonical: never_canonical,
        methods: &crate::runtime::host_array::ARRAY,
    },
    KindInfo {
        kind: HostKind::Registry,
        tag: "reg",
        // An implementation of the interface: `MappedRegistry`,
        // `DefaultedMappedRegistry`, a loader's own.
        class: ClassRule::Unknown {
            declared: crate::runtime::host_registry::registry_types,
        },
        // One registry per registry key.
        canonical: always_canonical,
        methods: &crate::runtime::host_registry::REGISTRY_METHODS,
    },
    KindInfo {
        kind: HostKind::Holder,
        tag: "hold",
        // `DeferredHolder` has subclasses (`DeferredItem`, `DeferredBlock`).
        class: ClassRule::Unknown {
            declared: crate::runtime::host_registry::holder_types,
        },
        // One holder per `register` call: the serial names one allocation.
        canonical: always_canonical,
        methods: &crate::runtime::host_registry::HOLDER_METHODS,
    },
    KindInfo {
        kind: HostKind::Registered,
        tag: "rgd",
        class: ClassRule::Unknown {
            declared: crate::runtime::host_registry::no_types,
        },
        canonical: always_canonical,
        methods: &crate::runtime::host_registry::REGISTERED_METHODS,
    },
    KindInfo {
        kind: HostKind::RegistryBuilder,
        tag: "frb",
        // A final class (fabric-registry-sync 8.0.4).
        class: ClassRule::Fixed(crate::runtime::fabric_registry::BUILDER),
        // `create(key)` twice is two builders for one registry.
        canonical: never_canonical,
        methods: &crate::runtime::host_registry::BUILDER_METHODS,
    },
    KindInfo {
        kind: HostKind::GameEvent,
        tag: "gev",
        class: ClassRule::FromKey(crate::runtime::host_game_event::game_event_class),
        // A key holds a serial minted once per firing and class: one object.
        canonical: always_canonical,
        methods: &crate::runtime::host_game_event::GAME_EVENT,
    },
    KindInfo {
        kind: HostKind::LifecycleEvent,
        tag: "lev",
        class: ClassRule::FromKey(crate::runtime::host_game_event::lifecycle_class),
        // A stage posts one event per container.
        canonical: always_canonical,
        methods: &crate::runtime::host_game_event::LIFECYCLE_EVENT,
    },
    KindInfo {
        kind: HostKind::AutoConfigHolder,
        tag: "acfg",
        class: ClassRule::Unknown {
            declared: crate::runtime::host_config::auto_config_types,
        },
        // One holder per config class: `AutoConfig` keeps them in a map.
        canonical: always_canonical,
        methods: &crate::runtime::host_config::CONFIG_METHODS,
    },
    KindInfo {
        kind: HostKind::YaclHandler,
        tag: "yhnd",
        class: ClassRule::Unknown {
            declared: crate::runtime::host_config::yacl_handler_types,
        },
        canonical: always_canonical,
        methods: &crate::runtime::host_config::CONFIG_METHODS,
    },
    KindInfo {
        kind: HostKind::YaclBuilder,
        tag: "ybld",
        class: ClassRule::Unknown {
            declared: crate::runtime::host_config::yacl_builder_types,
        },
        canonical: always_canonical,
        methods: &crate::runtime::host_config::CONFIG_METHODS,
    },
    KindInfo {
        kind: HostKind::ConfiglibBuilder,
        tag: "clbld",
        class: ClassRule::Unknown {
            declared: crate::runtime::host_config::configlib_builder_types,
        },
        canonical: always_canonical,
        methods: &crate::runtime::host_config::CONFIG_METHODS,
    },
    KindInfo {
        kind: HostKind::ConfiglibValue,
        tag: "clval",
        class: ClassRule::Unknown {
            declared: crate::runtime::host_config::configlib_value_types,
        },
        canonical: always_canonical,
        methods: &crate::runtime::host_config::CONFIG_METHODS,
    },
    KindInfo {
        kind: HostKind::CallbackInfo,
        tag: "mci",
        class: ClassRule::Unknown {
            declared: crate::runtime::host_mixin::callback_info_types,
        },
        // A new object per injector per call of the target; one key per
        // injector.
        canonical: never_canonical,
        methods: &crate::runtime::host_mixin::CALLBACK_INFO_METHODS,
    },
    KindInfo {
        kind: HostKind::MixinClassNode,
        tag: "mcn",
        class: ClassRule::Unknown {
            declared: crate::runtime::host_mixin::class_node_types,
        },
        canonical: never_canonical,
        methods: &crate::runtime::host_mixin::MIXIN_METHODS,
    },
    KindInfo {
        kind: HostKind::MixinInfo,
        tag: "mmi",
        class: ClassRule::Unknown {
            declared: crate::runtime::host_mixin::mixin_info_types,
        },
        canonical: never_canonical,
        methods: &crate::runtime::host_mixin::MIXIN_METHODS,
    },
    KindInfo {
        kind: HostKind::MixinEnvironment,
        tag: "menv",
        class: ClassRule::Unknown {
            declared: crate::runtime::host_mixin::environment_types,
        },
        // One per phase: the current and the default may differ.
        canonical: never_canonical,
        methods: &crate::runtime::host_mixin::MIXIN_METHODS,
    },
    KindInfo {
        kind: HostKind::MixinSide,
        tag: "mside",
        class: ClassRule::Unknown {
            declared: crate::runtime::host_mixin::side_types,
        },
        // An enum constant.
        canonical: always_canonical,
        methods: &crate::runtime::host_mixin::SIDE_METHODS,
    },
    KindInfo {
        kind: HostKind::Executor,
        tag: "exe",
        class: ClassRule::Unknown {
            declared: crate::runtime::host_executors::executor_types,
        },
        // One per `Executors.newX()` call.
        canonical: always_canonical,
        methods: &crate::runtime::host_executors::OBJECT_METHODS,
    },
    KindInfo {
        kind: HostKind::ExecutorFuture,
        tag: "fut",
        class: ClassRule::Unknown {
            declared: crate::runtime::host_executors::future_types,
        },
        // One per `submit`/`schedule`.
        canonical: always_canonical,
        methods: &crate::runtime::host_executors::OBJECT_METHODS,
    },
    KindInfo {
        kind: HostKind::ExecutorCallable,
        tag: "xcall",
        class: ClassRule::Unknown {
            declared: crate::runtime::host_executors::callable_types,
        },
        // A fresh `RunnableAdapter` per call: equal keys may be two objects.
        canonical: never_canonical,
        methods: &crate::runtime::host_executors::OBJECT_METHODS,
    },
    KindInfo {
        kind: HostKind::ThreadFactory,
        tag: "tfac",
        class: ClassRule::Unknown {
            declared: crate::runtime::host_executors::thread_factory_types,
        },
        // One per `defaultThreadFactory()` call.
        canonical: always_canonical,
        methods: &crate::runtime::host_executors::OBJECT_METHODS,
    },
    KindInfo {
        kind: HostKind::ServiceLoader,
        tag: "svl",
        class: ClassRule::Unknown {
            declared: crate::runtime::host_services::loader_types,
        },
        // One per `ServiceLoader.load`.
        canonical: always_canonical,
        methods: &crate::runtime::host_services::LOADER_METHODS,
    },
    KindInfo {
        kind: HostKind::ServiceIterator,
        tag: "svi",
        class: ClassRule::Unknown {
            declared: crate::runtime::host_services::iterator_types,
        },
        // One per `iterator()`.
        canonical: always_canonical,
        methods: &crate::runtime::host_executors::OBJECT_METHODS,
    },
    KindInfo {
        kind: HostKind::ServiceProvider,
        tag: "svp",
        class: ClassRule::Unknown {
            declared: crate::runtime::host_services::provider_types,
        },
        // Equal keys are equal providers; `reload()` makes new objects.
        canonical: never_canonical,
        methods: &crate::runtime::host_services::PROVIDER_METHODS,
    },
    KindInfo {
        kind: HostKind::GuavaSplitter,
        tag: "gsplit",
        class: ClassRule::Unknown {
            declared: crate::runtime::host_guava::splitter_types,
        },
        // A fresh `Splitter` per `on`.
        canonical: never_canonical,
        methods: &crate::runtime::host_executors::OBJECT_METHODS,
    },
    KindInfo {
        kind: HostKind::GuavaJoiner,
        tag: "gjoin",
        class: ClassRule::Unknown {
            declared: crate::runtime::host_guava::joiner_types,
        },
        // A fresh `Joiner` per `on`.
        canonical: never_canonical,
        methods: &crate::runtime::host_executors::OBJECT_METHODS,
    },
    KindInfo {
        kind: HostKind::GuavaCacheBuilder,
        tag: "gcb",
        class: ClassRule::Unknown {
            declared: crate::runtime::host_guava::cache_builder_types,
        },
        // One per `newBuilder()`; setters answer it.
        canonical: always_canonical,
        methods: &crate::runtime::host_executors::OBJECT_METHODS,
    },
    KindInfo {
        kind: HostKind::GuavaCacheLoader,
        tag: "gcl",
        class: ClassRule::Unknown {
            declared: crate::runtime::host_guava::cache_loader_types,
        },
        // A fresh loader per `from`.
        canonical: never_canonical,
        methods: &crate::runtime::host_executors::OBJECT_METHODS,
    },
    KindInfo {
        kind: HostKind::Thread,
        tag: "thr",
        class: ClassRule::FromKey(|_| "java/lang/Thread"),
        canonical: always_canonical,
        methods: &crate::runtime::host_jdk::THREAD,
    },
    KindInfo {
        kind: HostKind::Logger,
        tag: "log",
        class: ClassRule::Unknown {
            declared: crate::runtime::host_jdk::logger_declared,
        },
        canonical: crate::runtime::host_jdk::logger_canonical,
        methods: &crate::runtime::host_jdk::LOGGER,
    },
    KindInfo {
        kind: HostKind::StackWalker,
        tag: "walker",
        class: ClassRule::Fixed("java/lang/StackWalker"),
        canonical: canonical_when_empty,
        methods: &crate::runtime::host_jdk::STACK_WALKER,
    },
    KindInfo {
        kind: HostKind::StackFrame,
        tag: "frame",
        class: ClassRule::Fixed("java/lang/StackFrameInfo"),
        canonical: never_canonical,
        methods: &crate::runtime::host_jdk::STACK_FRAME,
    },
    KindInfo {
        kind: HostKind::StackTraceElement,
        tag: "ste",
        class: ClassRule::FromKey(|_| "java/lang/StackTraceElement"),
        canonical: never_canonical,
        methods: &crate::runtime::host_jdk::STACK_TRACE_ELEMENT,
    },
    KindInfo {
        kind: HostKind::Module,
        tag: "module",
        class: ClassRule::Fixed("java/lang/Module"),
        canonical: always_canonical,
        methods: &crate::runtime::host_jdk::MODULE,
    },
    KindInfo {
        kind: HostKind::FabricModContainer,
        tag: "fmc",
        class: ClassRule::Unknown {
            declared: crate::runtime::host_fabric_loader::container_types,
        },
        // `modMap.get(id)`: one container per mod.
        canonical: always_canonical,
        methods: &crate::runtime::host_fabric_loader::CONTAINER_METHODS,
    },
    KindInfo {
        kind: HostKind::FabricModMetadata,
        tag: "fmm",
        class: ClassRule::Unknown {
            declared: crate::runtime::host_fabric_loader::metadata_types,
        },
        // The container's own metadata.
        canonical: always_canonical,
        methods: &crate::runtime::host_fabric_loader::OBJECT_METHODS,
    },
    KindInfo {
        kind: HostKind::FabricModOrigin,
        tag: "fmo",
        class: ClassRule::Unknown {
            declared: crate::runtime::host_fabric_loader::origin_types,
        },
        // The container's own origin.
        canonical: always_canonical,
        methods: &crate::runtime::host_fabric_loader::ORIGIN_METHODS,
    },
    KindInfo {
        kind: HostKind::FabricPerson,
        tag: "fper",
        class: ClassRule::Unknown {
            declared: crate::runtime::host_fabric_loader::person_types,
        },
        // The metadata's own `Person`.
        canonical: always_canonical,
        methods: &crate::runtime::host_fabric_loader::OBJECT_METHODS,
    },
    KindInfo {
        kind: HostKind::FabricContact,
        tag: "fcon",
        class: ClassRule::Unknown {
            declared: crate::runtime::host_fabric_loader::contact_types,
        },
        // The metadata's (or author's) own contact, or the one `EMPTY`.
        canonical: always_canonical,
        methods: &crate::runtime::host_fabric_loader::OBJECT_METHODS,
    },
    KindInfo {
        kind: HostKind::FabricCustomValue,
        tag: "fcv",
        class: ClassRule::Unknown {
            declared: crate::runtime::host_fabric_loader::custom_value_types,
        },
        // The metadata's own value at the pointer, or one shared constant.
        canonical: always_canonical,
        methods: &crate::runtime::host_fabric_loader::OBJECT_METHODS,
    },
    KindInfo {
        kind: HostKind::AttachmentType,
        tag: "natt",
        class: ClassRule::FromKey(crate::runtime::host_attachment::type_class),
        // One serial per allocation.
        canonical: always_canonical,
        methods: &crate::runtime::host_attachment::TYPE_METHODS,
    },
    KindInfo {
        kind: HostKind::AttachmentBuilder,
        tag: "nattb",
        class: ClassRule::FromKey(crate::runtime::host_attachment::builder_class),
        canonical: always_canonical,
        methods: &crate::runtime::host_attachment::BUILDER_METHODS,
    },
    KindInfo {
        kind: HostKind::NeoForgeModContainer,
        tag: "nfmc",
        class: ClassRule::Unknown {
            declared: crate::runtime::host_neoforge_loader::container_types,
        },
        // `indexedMods.get(id)`: one container per mod.
        canonical: always_canonical,
        methods: &crate::runtime::host_neoforge_loader::CONTAINER_METHODS,
    },
    KindInfo {
        kind: HostKind::NeoForgeModInfo,
        tag: "nfmi",
        class: ClassRule::Unknown {
            declared: crate::runtime::host_neoforge_loader::mod_info_types,
        },
        // The container's final `modInfo` field.
        canonical: always_canonical,
        methods: &crate::runtime::host_neoforge_loader::OBJECT_METHODS,
    },
    KindInfo {
        kind: HostKind::NeoForgeModFileInfo,
        tag: "nfmf",
        class: ClassRule::Unknown {
            declared: crate::runtime::host_neoforge_loader::mod_file_info_types,
        },
        // `fileById.get(id)`: one per file, keyed by the file.
        canonical: always_canonical,
        methods: &crate::runtime::host_neoforge_loader::OBJECT_METHODS,
    },
    KindInfo {
        kind: HostKind::NeoForgeConfigType,
        tag: "nfct",
        class: ClassRule::Unknown {
            declared: crate::runtime::host_neoforge_loader::config_type_types,
        },
        // An enum constant.
        canonical: always_canonical,
        methods: &crate::runtime::host_neoforge_loader::CONFIG_TYPE_METHODS,
    },
    KindInfo {
        kind: HostKind::WorldVersion,
        tag: "wver",
        class: ClassRule::Unknown {
            declared: crate::runtime::host_neoforge_loader::world_version_types,
        },
        // A new record per detection.
        canonical: never_canonical,
        methods: &crate::runtime::host_neoforge_loader::WORLD_VERSION_METHODS,
    },
    // The command surface (`host_brigadier.rs`). An exact `getClass()` would
    // need a class side for brigadier's classes (#174's `ClassSide`): until
    // one is added, `getClass()` is counted.
    KindInfo {
        kind: HostKind::BrigadierDispatcher,
        tag: "brd",
        class: ClassRule::Unknown {
            declared: crate::runtime::host_brigadier::dispatcher_types,
        },
        // A reload makes another dispatcher.
        canonical: never_canonical,
        methods: &crate::runtime::host_brigadier::DISPATCHER_METHODS,
    },
    KindInfo {
        kind: HostKind::CommandBuildContext,
        tag: "brbc",
        class: ClassRule::Unknown {
            declared: crate::runtime::host_brigadier::build_context_types,
        },
        canonical: never_canonical,
        methods: &crate::runtime::host_brigadier::IDENTITY_METHODS,
    },
    KindInfo {
        kind: HostKind::CommandContext,
        tag: "brcc",
        class: ClassRule::Unknown {
            declared: crate::runtime::host_brigadier::context_types,
        },
        // One per run of a command, whose serial the key holds.
        canonical: always_canonical,
        methods: &crate::runtime::host_brigadier::CONTEXT_METHODS,
    },
    KindInfo {
        kind: HostKind::CommandSource,
        tag: "brsrc",
        class: ClassRule::Unknown {
            declared: crate::runtime::host_brigadier::source_types,
        },
        // One per run of a command, whose serial the key holds.
        canonical: always_canonical,
        methods: &crate::runtime::host_brigadier::IDENTITY_METHODS,
    },
    KindInfo {
        kind: HostKind::CommandNode,
        tag: "brnode",
        class: ClassRule::Unknown {
            declared: crate::runtime::host_brigadier::node_types,
        },
        // The arena never reuses an id.
        canonical: always_canonical,
        methods: &crate::runtime::host_brigadier::NODE_METHODS,
    },
    KindInfo {
        kind: HostKind::ArgumentBuilder,
        tag: "brbld",
        class: ClassRule::Unknown {
            declared: crate::runtime::host_brigadier::builder_types,
        },
        // Builder ids only grow.
        canonical: always_canonical,
        methods: &crate::runtime::host_brigadier::BUILDER_METHODS,
    },
    KindInfo {
        kind: HostKind::ArgumentType,
        tag: "brarg",
        class: ClassRule::Unknown {
            declared: crate::runtime::host_brigadier::argument_type_types,
        },
        // The arena never reuses an id.
        canonical: always_canonical,
        methods: &crate::runtime::host_brigadier::ARGUMENT_TYPE_METHODS,
    },
    KindInfo {
        kind: HostKind::CommandPermission,
        tag: "brperm",
        class: ClassRule::Unknown {
            declared: crate::runtime::host_brigadier::permission_types,
        },
        // Several constants and objects share a level.
        canonical: never_canonical,
        methods: &crate::runtime::host_brigadier::STATE_METHODS,
    },
    KindInfo {
        kind: HostKind::StreamBuilder,
        tag: "sbld",
        class: ClassRule::Fixed("java/util/stream/Streams$StreamBuilderImpl"),
        // The key is a per-runtime number: a handle and its clones are one
        // builder, two equal keys from two runs prove nothing.
        canonical: never_canonical,
        methods: &crate::runtime::stream_pipeline::STREAM_BUILDER,
    },
    KindInfo {
        kind: HostKind::StreamSink,
        tag: "ssnk",
        // The JDK's sink is a stage's own class, which is not modelled: only
        // its declared types are known; `getClass()` and a cast's message are
        // counted.
        class: ClassRule::Unknown {
            declared: crate::runtime::stream_pipeline::sink_types,
        },
        canonical: never_canonical,
        methods: &crate::runtime::host_events::DECLARED_TYPE_METHODS,
    },
    KindInfo {
        kind: HostKind::Collector,
        tag: "coll",
        class: ClassRule::Fixed("java/util/stream/Collectors$CollectorImpl"),
        // Guava's two shared collectors; every other one is its own object.
        canonical: crate::runtime::host_collector::canonical,
        methods: &crate::runtime::stream_pipeline::COLLECTOR_IMPL,
    },
    KindInfo {
        kind: HostKind::Spliterator,
        tag: "splt",
        class: ClassRule::FromKey(crate::runtime::jdk_spliterators::spliterator_class),
        canonical: never_canonical,
        methods: &crate::runtime::jdk_spliterators::JDK_SPLITERATOR,
    },
    KindInfo {
        kind: HostKind::FabricLoaderInstance,
        tag: "fli",
        class: ClassRule::Unknown {
            declared: crate::runtime::host_fabric_runtime::loader_types,
        },
        // `FabricLoaderImpl.INSTANCE`.
        canonical: always_canonical,
        methods: &crate::runtime::host_fabric_runtime::EXACT_OBJECT_METHODS,
    },
    KindInfo {
        kind: HostKind::FabricMappingResolver,
        tag: "fmr",
        class: ClassRule::Unknown {
            declared: crate::runtime::host_fabric_runtime::mapping_resolver_types,
        },
        // Made once and kept.
        canonical: always_canonical,
        methods: &crate::runtime::host_fabric_runtime::EXACT_OBJECT_METHODS,
    },
    KindInfo {
        kind: HostKind::FabricEnvType,
        tag: "fenv",
        // A final enum: its class is exact, with its class side.
        class: ClassRule::FromKey(crate::runtime::host_fabric_runtime::env_type_class),
        // An enum constant.
        canonical: always_canonical,
        methods: &crate::runtime::host_fabric_runtime::ENV_TYPE_METHODS,
    },
    KindInfo {
        kind: HostKind::FabricEntrypointContainer,
        tag: "fepc",
        class: ClassRule::Unknown {
            declared: crate::runtime::host_fabric_runtime::entrypoint_container_types,
        },
        // A new container per entry on every call.
        canonical: never_canonical,
        methods: &crate::runtime::host_fabric_runtime::OBJECT_METHODS,
    },
    KindInfo {
        kind: HostKind::FabricPayloadTypes,
        tag: "fpt",
        class: ClassRule::Unknown {
            declared: crate::runtime::host_fabric_runtime::payload_types_types,
        },
        // One of four static constants.
        canonical: always_canonical,
        methods: &crate::runtime::host_fabric_runtime::OBJECT_METHODS,
    },
    KindInfo {
        kind: HostKind::FabricResourceLoader,
        tag: "frl",
        class: ClassRule::Unknown {
            declared: crate::runtime::host_fabric_runtime::resource_loader_types,
        },
        // One per `PackType` (`IMPL_MAP`).
        canonical: always_canonical,
        methods: &crate::runtime::host_fabric_runtime::OBJECT_METHODS,
    },
    KindInfo {
        kind: HostKind::JarObject,
        tag: "obj",
        class: ClassRule::FromKey(crate::runtime::jar_object::class_of_key),
        // `Runtime::new_object` numbers every allocation once.
        canonical: always_canonical,
        methods: &crate::runtime::jar_object::JAR_OBJECT,
    },
    KindInfo {
        kind: HostKind::Class,
        tag: "cls",
        // `FromKey`, not `Fixed`, until #156 P13c: a fixed class's objects are
        // only its handles (`checkcast` throws for anything else), and until
        // then a `Class` may still be class-marker text.
        class: ClassRule::FromKey(crate::runtime::host_class::class_of_class),
        // One `Class` per name in a run (`class_precedence`, #124).
        canonical: always_canonical,
        methods: &crate::runtime::host_class::CLASS_METHODS,
    },
    KindInfo {
        kind: HostKind::Component,
        tag: "comp",
        class: ClassRule::FromKey(crate::runtime::host_component::component_class),
        // A serial per allocation: one key, one object.
        canonical: always_canonical,
        methods: &crate::runtime::host_component::COMPONENT_METHODS,
    },
    KindInfo {
        kind: HostKind::DfuPair,
        tag: "dpair",
        class: ClassRule::FromKey(crate::runtime::host_codec::pair_class),
        // A serial per allocation: one key, one object.
        canonical: always_canonical,
        methods: &crate::runtime::host_codec::PAIR_METHODS,
    },
    KindInfo {
        kind: HostKind::OpaqueCodec,
        tag: "ocdc",
        class: ClassRule::Unknown {
            declared: crate::runtime::host_codec::opaque_codec_types,
        },
        // A static field's codec is one object; a method result's key is its
        // declared type alone, and two such handles prove nothing.
        canonical: crate::runtime::host_codec::opaque_canonical,
        methods: &crate::runtime::host_codec::OPAQUE_CODEC_METHODS,
    },
    KindInfo {
        kind: HostKind::ModelledCodec,
        tag: "mcdc",
        class: ClassRule::Unknown {
            declared: crate::runtime::host_codec::modelled_codec_types,
        },
        // One node, one object.
        canonical: always_canonical,
        methods: &crate::runtime::host_events::DECLARED_TYPE_METHODS,
    },
    KindInfo {
        kind: HostKind::ReflectType,
        tag: "rtyp",
        class: ClassRule::Unknown {
            declared: crate::runtime::host_reflect_type::type_types,
        },
        // Built afresh by every reflective call.
        canonical: never_canonical,
        methods: &crate::runtime::host_reflect_type::TYPE_METHODS,
    },
    KindInfo {
        kind: HostKind::AnnotatedType,
        tag: "rann",
        class: ClassRule::Unknown {
            declared: crate::runtime::host_reflect_type::annotated_types,
        },
        // Built afresh by every reflective call.
        canonical: never_canonical,
        methods: &crate::runtime::host_reflect_type::ANNOTATED_METHODS,
    },
    KindInfo {
        kind: HostKind::FormattingStyle,
        tag: "gfmt",
        class: ClassRule::FromKey(crate::runtime::host_gson::style_class),
        // A constant's name or a serial per allocation: one key, one object.
        canonical: always_canonical,
        methods: &crate::runtime::host_gson::STYLE_METHODS,
    },
    KindInfo {
        kind: HostKind::GsonOptions,
        tag: "gopt",
        // Not a Java object a jar holds: only `Object` is declared.
        class: ClassRule::Unknown {
            declared: crate::runtime::host_gson::options_types,
        },
        canonical: never_canonical,
        methods: &crate::runtime::host_events::DECLARED_TYPE_METHODS,
    },
    KindInfo {
        kind: HostKind::TypeToken,
        tag: "ttok",
        class: ClassRule::FromKey(crate::runtime::host_type_token::token_class),
        // A serial per allocation: one key, one object.
        canonical: always_canonical,
        methods: &crate::runtime::host_type_token::TOKEN_METHODS,
    },
    KindInfo {
        kind: HostKind::GsonType,
        tag: "gtyp",
        class: ClassRule::FromKey(crate::runtime::host_gson_type::gson_type_class),
        // One id per Gson object: one key, one object.
        canonical: always_canonical,
        methods: &crate::runtime::host_gson_type::GSON_TYPE_METHODS,
    },
    KindInfo {
        kind: HostKind::GsonTypeToken,
        tag: "gtok",
        class: ClassRule::FromKey(crate::runtime::host_gson_type::token_class),
        // A serial per allocation.
        canonical: always_canonical,
        methods: &crate::runtime::host_gson_type::TOKEN_METHODS,
    },
    KindInfo {
        kind: HostKind::Collection,
        tag: "hcoll",
        class: ClassRule::FromKey(crate::runtime::jar_object::collection_class),
        // The host numbers every collection it makes once.
        canonical: always_canonical,
        // An object: the runtime and the host's collection models answer
        // for it by its key, as for a jar's `new ArrayList()`.
        methods: &crate::runtime::jar_object::JAR_OBJECT,
    },
    KindInfo {
        kind: HostKind::TimeUnit,
        tag: "tunit",
        class: ClassRule::Fixed("java/util/concurrent/TimeUnit"),
        // One object per constant.
        canonical: always_canonical,
        methods: &crate::runtime::host_jdk_constant::TIME_UNIT_METHODS,
    },
    KindInfo {
        kind: HostKind::CodingErrorAction,
        tag: "cea",
        class: ClassRule::Fixed("java/nio/charset/CodingErrorAction"),
        // One object per constant.
        canonical: always_canonical,
        methods: &crate::runtime::host_jdk_constant::CODING_ERROR_ACTION_METHODS,
    },
    KindInfo {
        kind: HostKind::UnicodeBlock,
        tag: "ublk",
        class: ClassRule::Fixed("java/lang/Character$UnicodeBlock"),
        // One object per constant.
        canonical: always_canonical,
        methods: &crate::runtime::host_jdk_constant::UNICODE_BLOCK_METHODS,
    },
    KindInfo {
        kind: HostKind::Locale,
        tag: "loc",
        class: ClassRule::Fixed("java/util/Locale"),
        // Java 25 caches locales: `Locale.US`, `Locale.of("en", "US")` and
        // `forLanguageTag("en-US")` are one object. The cache is soft, so
        // only the constants' keys are provably one object.
        canonical: crate::runtime::host_jdk_constant::locale_canonical,
        methods: &crate::runtime::host_jdk_constant::LOCALE_METHODS,
    },
    KindInfo {
        kind: HostKind::RegexPattern,
        tag: "rpat",
        class: ClassRule::Fixed("java/util/regex/Pattern"),
        // A serial per `compile`.
        canonical: always_canonical,
        methods: &crate::runtime::host_jdk_constant::PATTERN_METHODS,
    },
    KindInfo {
        kind: HostKind::ArtifactVersion,
        tag: "artv",
        class: ClassRule::Unknown {
            declared: crate::runtime::host_jdk_constant::artifact_version_types,
        },
        // A mod's version object; another with the same text may be another object.
        canonical: never_canonical,
        methods: &crate::runtime::host_jdk_constant::ARTIFACT_VERSION_METHODS,
    },
    KindInfo {
        kind: HostKind::Uuid,
        tag: "uuid",
        class: ClassRule::Fixed("java/util/UUID"),
        // Each `fromString`/`randomUUID` is a new object.
        canonical: never_canonical,
        methods: &crate::runtime::host_jdk_value::UUID_METHODS,
    },
    KindInfo {
        kind: HostKind::Duration,
        tag: "dur",
        class: ClassRule::Fixed("java/time/Duration"),
        // Every zero duration is `Duration.ZERO`.
        canonical: crate::runtime::host_jdk_value::duration_canonical,
        methods: &crate::runtime::host_jdk_value::DURATION_METHODS,
    },
    KindInfo {
        kind: HostKind::Uri,
        tag: "uri",
        class: ClassRule::Fixed("java/net/URI"),
        canonical: never_canonical,
        methods: &crate::runtime::host_jdk_value::URI_METHODS,
    },
    KindInfo {
        kind: HostKind::GameEntity,
        tag: "gent",
        // The exact class comes with the family's producer (#156 P16b–e).
        class: ClassRule::Unknown {
            declared: crate::runtime::game_object::declared_types,
        },
        // One object per entity: P16b keys incarnations.
        canonical: always_canonical,
        methods: &crate::runtime::game_object::IDENTITY_METHODS,
    },
    KindInfo {
        kind: HostKind::GameLevel,
        tag: "glvl",
        // The exact class comes with the family's producer (#156 P16b–e).
        class: ClassRule::Unknown {
            declared: crate::runtime::game_object::declared_types,
        },
        // One level per dimension.
        canonical: always_canonical,
        methods: &crate::runtime::game_object::IDENTITY_METHODS,
    },
    KindInfo {
        kind: HostKind::GameSingleton,
        tag: "gone",
        // The exact class comes with the family's producer (#156 P16b–e).
        class: ClassRule::Unknown {
            declared: crate::runtime::game_object::declared_types,
        },
        // One object per name.
        canonical: always_canonical,
        methods: &crate::runtime::game_object::IDENTITY_METHODS,
    },
    KindInfo {
        kind: HostKind::GameRegistryObject,
        tag: "greg",
        // The exact class comes with the family's producer (#156 P16b–e).
        class: ClassRule::Unknown {
            declared: crate::runtime::game_object::declared_types,
        },
        // One object per registry entry.
        canonical: always_canonical,
        methods: &crate::runtime::game_object::IDENTITY_METHODS,
    },
    KindInfo {
        kind: HostKind::GameValue,
        tag: "gval",
        // The exact class comes with the family's producer (#156 P16b–e).
        class: ClassRule::Unknown {
            declared: crate::runtime::game_object::declared_types,
        },
        // Each `new BlockPos(..)` is its own object; equal by fields.
        canonical: never_canonical,
        methods: &crate::runtime::game_object::VALUE_METHODS,
    },
    KindInfo {
        kind: HostKind::GameTransient,
        tag: "gtmp",
        // The exact class comes with the family's producer (#156 P16b–e).
        class: ClassRule::Unknown {
            declared: crate::runtime::game_object::declared_types,
        },
        // Its own object each time the game makes one.
        canonical: never_canonical,
        methods: &crate::runtime::game_object::IDENTITY_METHODS,
    },
    KindInfo {
        kind: HostKind::Iterator,
        tag: "itr",
        // The collection's own iterator class (`ArrayList$Itr`...), not named here.
        class: ClassRule::Unknown {
            declared: crate::runtime::host_iterator::iterator_types,
        },
        // A per-allocation key, or the one empty iterator.
        canonical: always_canonical,
        methods: &crate::runtime::host_iterator::ITERATOR_METHODS,
    },
    KindInfo {
        kind: HostKind::Enumeration,
        tag: "enm",
        class: ClassRule::Unknown {
            declared: crate::runtime::host_iterator::enumeration_types,
        },
        canonical: always_canonical,
        methods: &crate::runtime::host_iterator::ITERATOR_METHODS,
    },
    KindInfo {
        kind: HostKind::ListIterator,
        tag: "litr",
        class: ClassRule::Unknown {
            declared: crate::runtime::host_iterator::list_iterator_types,
        },
        canonical: always_canonical,
        methods: &crate::runtime::host_iterator::ITERATOR_METHODS,
    },
];

impl HostKind {
    /// The kind's row: `KINDS` is in declaration order, so the discriminant
    /// indexes it (a unit test checks every row's place).
    fn info(self) -> &'static KindInfo {
        &KINDS[self as usize]
    }

    fn from_tag(tag: &str) -> Option<Self> {
        static BY_TAG: OnceLock<HashMap<&'static str, HostKind>> = OnceLock::new();
        BY_TAG
            .get_or_init(|| KINDS.iter().map(|info| (info.tag, info.kind)).collect())
            .get(tag)
            .copied()
    }

    /// `class instanceof wanted` for a class name a kind owns
    /// ([`Self::owning_class`]: a handle's `getClass()` handed to
    /// `Class.isAssignableFrom`, a class walk): a fixed class's row, as for a
    /// handle of it (a fixed class's supertypes do not depend on the key), or
    /// a family's [`ClassSide`]. `None` when no kind owns `class`;
    /// `Some(None)` when the row cannot tell.
    pub(crate) fn class_instance_of(
        class: &str,
        wanted: &str,
        hierarchy: &dyn ClassHierarchy,
    ) -> Option<Option<bool>> {
        let kind = Self::owning_class(class)?;
        Some(match kind.class_side() {
            Some(side) => (side.instance_of)(class, wanted, hierarchy),
            None => HostRef::new(kind, "").instance_of(wanted, hierarchy),
        })
    }

    /// The kind that answers for the class name `class`: a fixed-class kind
    /// whose class it is, else the family kind whose [`ClassSide`] owns it.
    /// Every `Class` call on a name a kind owns answers from the kind or is
    /// counted; it never falls to "a class the runtime does not hold".
    pub(crate) fn owning_class(class: &str) -> Option<Self> {
        Self::of_class(class).or_else(|| {
            debug_assert!(
                CLASS_SIDES
                    .iter()
                    .filter(|(_, side)| (side.owns)(class))
                    .count()
                    <= 1,
                "two kinds own {class}"
            );
            CLASS_SIDES
                .iter()
                .find(|(_, side)| (side.owns)(class))
                .map(|(kind, _)| *kind)
        })
    }

    /// The kind's [`ClassSide`], when it has one.
    pub(crate) fn class_side(self) -> Option<&'static ClassSide> {
        CLASS_SIDES
            .iter()
            .find(|(kind, _)| *kind == self)
            .map(|(_, side)| *side)
    }

    /// The kind of a fixed-class kind whose class is `class` (asked on every
    /// `checkcast`, so a map, not a scan).
    pub(crate) fn of_class(class: &str) -> Option<Self> {
        static BY_CLASS: OnceLock<HashMap<&'static str, HostKind>> = OnceLock::new();
        BY_CLASS
            .get_or_init(|| {
                KINDS
                    .iter()
                    .filter_map(|info| match info.class {
                        ClassRule::Fixed(class) => Some((class, info.kind)),
                        ClassRule::FromKey(_) | ClassRule::Unknown { .. } => None,
                    })
                    .collect()
            })
            .get(class)
            .copied()
    }
}

impl HostRef {
    /// A handle with `key` as it is (a number, a name). A key that holds
    /// values is built with [`Self::of_values`].
    pub(crate) fn new(kind: HostKind, key: impl Into<String>) -> Self {
        Self {
            kind,
            key: Arc::new(key.into()),
            origin: NEXT_ORIGIN.fetch_add(1, AtomicOrdering::Relaxed),
        }
    }

    /// A handle whose key holds `values`: each value's marker, separators
    /// escaped, joined with `,`. No values is the empty key.
    pub(crate) fn of_values(kind: HostKind, values: &[Value]) -> Self {
        let key = values
            .iter()
            .map(crate::runtime::value_marker)
            .collect::<Vec<_>>()
            .join(",");
        Self::new(kind, key)
    }

    /// The values [`Self::of_values`] put in the key, `None` when one does
    /// not parse.
    pub(crate) fn values(&self) -> Option<Vec<Value>> {
        if self.key.is_empty() {
            return Some(Vec::new());
        }
        crate::runtime::split_capture_markers(&self.key)
            .ok()?
            .iter()
            .map(|marker| crate::runtime::parse_value_marker(marker).ok())
            .collect()
    }

    pub fn kind(&self) -> HostKind {
        self.kind
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    /// [`Self::key`] as the `&String` a text reader takes.
    pub(crate) fn key_text(&self) -> &String {
        &self.key
    }

    /// The class this object is (`getClass()`), internal form.
    /// `None` when the runtime class is not known ([`ClassRule::Unknown`]):
    /// the caller counts what needs it.
    pub fn class(&self) -> Option<&str> {
        match self.kind.info().class {
            ClassRule::Fixed(class) => Some(class),
            ClassRule::FromKey(class_of) => Some(class_of(&self.key)),
            ClassRule::Unknown { .. } => None,
        }
    }

    /// Whether two handles equal to this one are the same object.
    pub fn canonical(&self) -> bool {
        (self.kind.info().canonical)(&self.key)
    }

    /// `self == other` as the JVM compares references, where it is known:
    /// different values are different objects; equal values of a canonical
    /// kind, or a handle and its own clone (one construction flowing to both
    /// sides), are one object. `None` for two equal handles of a kind that
    /// is not canonical from different constructions: maybe two objects,
    /// maybe one Java object a model wrapped twice. The caller counts it.
    pub(crate) fn same_object(&self, other: &HostRef) -> Option<bool> {
        if self != other {
            return Some(false);
        }
        (self.canonical() || self.origin == other.origin).then_some(true)
    }

    fn methods(&self) -> &'static Methods {
        self.kind.info().methods
    }

    /// `toString()`; `None` when `nested` cannot print a held value.
    pub(crate) fn to_string(&self, nested: &mut dyn Nested) -> Result<Option<String>, NativeError> {
        (self.methods().to_string)(self, nested)
    }

    /// `hashCode()`; `None` when `nested` cannot hash a held value.
    pub(crate) fn hash_code(&self, nested: &mut dyn Nested) -> Result<Option<i32>, NativeError> {
        (self.methods().hash_code)(self, nested)
    }

    /// `equals(other)`; `None` when `nested` cannot compare held values.
    pub(crate) fn equals(
        &self,
        other: &Value,
        nested: &mut dyn Nested,
    ) -> Result<Option<bool>, NativeError> {
        (self.methods().equals)(self, other, nested)
    }

    /// `this instanceof wanted`, and whether `checkcast wanted` passes;
    /// `None` when `hierarchy` cannot tell.
    pub(crate) fn instance_of(&self, wanted: &str, hierarchy: &dyn ClassHierarchy) -> Option<bool> {
        (self.methods().instance_of)(self, wanted, hierarchy)
    }

    /// `compareTo(other)`; `None` when the kind has no natural order here or
    /// cannot compare these two.
    pub(crate) fn compare_to(
        &self,
        other: &Value,
        nested: &mut dyn Nested,
    ) -> Result<Option<i32>, NativeError> {
        match self.methods().compare_to {
            Some(compare) => compare(self, other, nested),
            None => Ok(None),
        }
    }

    /// Whether this handle means the same in another jar's runtime, with
    /// `crosses` answering for the values it holds.
    pub(crate) fn crosses_jars(&self, crosses: &dyn Fn(&Value) -> bool) -> bool {
        (self.methods().crosses_jars)(self, crosses)
    }

    /// The handle's text form, `h<tag>:<key>`, the key as it is. A list of
    /// markers (`value_marker`) escapes it as it escapes a string.
    pub(crate) fn encode(&self) -> String {
        format!("h{}:{}", self.kind.info().tag, self.key)
    }

    /// Read [`Self::encode`] back.
    pub(crate) fn decode(text: &str) -> Option<Self> {
        let (tag, key) = text.strip_prefix('h')?.split_once(':')?;
        Some(Self::new(HostKind::from_tag(tag)?, key))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_has_one_row_with_a_unique_tag_and_class() {
        for (index, info) in KINDS.iter().enumerate() {
            // `HostKind::info` indexes the table by discriminant.
            assert_eq!(info.kind as usize, index, "{:?}", info.kind);
            assert_eq!(HostKind::from_tag(info.tag), Some(info.kind));
            assert!(
                KINDS[index + 1..].iter().all(|other| other.tag != info.tag
                    && other.kind != info.kind
                    && !matches!(
                        (info.class, other.class),
                        (ClassRule::Fixed(a), ClassRule::Fixed(b)) if a == b
                    )),
                "{:?}",
                info.kind
            );
            let handle = HostRef::new(info.kind, "a,b\\c:d");
            assert_eq!(HostRef::decode(&handle.encode()), Some(handle.clone()));
            if let ClassRule::Fixed(class) = info.class {
                assert_eq!(HostKind::of_class(class), Some(info.kind));
            }
        }
    }

    /// A kind whose runtime class is unknown is never found through a type it
    /// declares: `Supplier` is not an IMC supplier's class, so a class check
    /// on `Supplier` (`Class.isAssignableFrom`, a class walk) is not answered
    /// as that kind's.
    #[test]
    fn unknown_class_kinds_are_not_found_by_their_declared_types() {
        for info in KINDS {
            let ClassRule::Unknown { declared } = info.class else {
                continue;
            };
            let handle = HostRef::new(info.kind, "");
            assert_eq!(handle.class(), None, "{:?}", info.kind);
            for key in ["", "i1,rx", "i0,rx"] {
                for class in declared(key) {
                    assert_ne!(HostKind::of_class(class), Some(info.kind), "{class}");
                }
            }
        }
        for class in [
            "java/util/function/Supplier",
            "net/neoforged/bus/api/Event",
            "net/neoforged/bus/api/ICancellableEvent",
            "net/minecraftforge/eventbus/api/bus/EventBus",
            "java/lang/Object",
        ] {
            assert_eq!(HostKind::of_class(class), None, "{class}");
        }
        // No kind's row answers for a declared type as its class. bus-8's `Event` is a NeoForge
        // event class of its own (`host_game_event.rs`): it answers its own supertypes, never an
        // interface it does not declare.
        for class in [
            "java/util/function/Supplier",
            "net/neoforged/bus/api/ICancellableEvent",
            "net/minecraftforge/eventbus/api/bus/EventBus",
            "java/lang/Object",
        ] {
            assert_eq!(
                HostKind::class_instance_of(class, "java/lang/Object", &JdkHierarchy),
                None,
                "{class}"
            );
        }
        assert_ne!(
            HostKind::class_instance_of(
                "net/neoforged/bus/api/Event",
                "net/neoforged/bus/api/ICancellableEvent",
                &JdkHierarchy
            ),
            Some(Some(true))
        );
    }

    /// One owner per class name, over every class-side row (not a list of
    /// kinds): each name a row enumerates is owned by that row alone, and by
    /// no fixed-class kind.
    #[test]
    fn every_class_name_has_one_owner() {
        for (kind, side) in CLASS_SIDES {
            assert!(kind.class_side().is_some(), "{kind:?}");
            let classes = (side.classes)();
            assert!(!classes.is_empty(), "{kind:?} enumerates no class");
            for class in classes {
                assert!((side.owns)(&class), "{kind:?} does not own its own {class}");
                let owners: Vec<HostKind> = CLASS_SIDES
                    .iter()
                    .filter(|(_, other)| (other.owns)(&class))
                    .map(|(kind, _)| *kind)
                    .collect();
                assert_eq!(owners, [*kind], "{class}");
                assert_eq!(HostKind::of_class(&class), None, "{class}");
            }
        }
    }

    /// A kind's `instance_of` asks the hierarchy it is handed, so a family
    /// kind (P12 jar objects) can answer from the jar's own classes, and an
    /// unknown answer stays unknown for the caller to count.
    #[test]
    fn instance_of_reads_the_hierarchy_it_is_given() {
        struct Stub;
        impl ClassHierarchy for Stub {
            fn is_instance(&self, class: &str, wanted: &str) -> Option<bool> {
                match (class, wanted) {
                    ("java/util/Optional", "demo/Added") => Some(true),
                    ("java/util/Optional", "demo/Unknown") => None,
                    _ => Some(false),
                }
            }
        }
        let handle = HostRef::new(HostKind::Optional, "");
        assert_eq!(handle.instance_of("demo/Added", &Stub), Some(true));
        assert_eq!(handle.instance_of("demo/Unknown", &Stub), None);
        assert_eq!(handle.instance_of("java/lang/Object", &Stub), Some(false));
        // The JDK's own hierarchy: an `Optional` is an `Object` and nothing else.
        assert_eq!(
            handle.instance_of("java/lang/Object", &JdkHierarchy),
            Some(true)
        );
        assert_eq!(
            handle.instance_of("java/io/Serializable", &JdkHierarchy),
            Some(false)
        );
    }

    /// A small deterministic generator (xorshift): the property tests need
    /// no extra crate.
    struct Gen(u64);

    impl Gen {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }

        /// A string over the characters every layer treats specially (the
        /// separators and escapes, the markers' prefixes and tags), plus
        /// non-ASCII text and a NUL.
        fn text(&mut self) -> String {
            const ALPHABET: &[&str] = &[
                ",", "\\", ":", "h", "r", "i", "l", "f", "B", "5", "-", "opt", "null", "void", "/",
                "#", " ", "é", "ü", "😀", "\0", "\\,", ",\\",
            ];
            let length = self.next() % 9;
            (0..length)
                .map(|_| ALPHABET[(self.next() % ALPHABET.len() as u64) as usize])
                .collect()
        }

        /// A value: a string, a number, `null`, or an optional nesting one.
        fn value(&mut self, depth: u32) -> Value {
            match self.next() % 6 {
                0 if depth < 3 => Value::Host(HostRef::of_values(
                    HostKind::Optional,
                    &[self.value(depth + 1)],
                )),
                1 => Value::Int(self.next() as i32),
                2 => Value::Host(HostRef::new(HostKind::OptionalInt, "")),
                _ => Value::Ref(Some(self.text())),
            }
        }
    }

    /// The one nested-key encoder/decoder round-trips every value, compared
    /// by value (not just its kind), through every layer a handle passes:
    /// its key, its own text form, a list of markers (a capture, an immutable
    /// collection), a collection element, and an optional of an optional.
    #[test]
    fn nested_keys_round_trip_by_value_for_arbitrary_strings() {
        let mut fixed = vec![
            "a,b",
            "x\\",
            "x\\,y",
            "k,1",
            "z,z",
            "é,\\ü",
            "",
            ",",
            "\\",
            "\\\\,",
            "hopt:ra",
            "hopt:r",
            "null",
            "r",
            "😀,😀",
        ]
        .into_iter()
        .map(|text| Value::Ref(Some(text.to_owned())))
        .collect::<Vec<_>>();
        let mut generator = Gen(0x9e37_79b9_7f4a_7c15);
        fixed.extend((0..3000).map(|_| generator.value(0)));
        for (index, held) in fixed.iter().enumerate() {
            let other = &fixed[(index * 7 + 3) % fixed.len()];
            let pair = HostRef::of_values(HostKind::Optional, &[held.clone(), other.clone()]);
            assert_eq!(
                pair.values(),
                Some(vec![held.clone(), other.clone()]),
                "{held:?}"
            );
            let optional = HostRef::of_values(HostKind::Optional, std::slice::from_ref(held));
            assert_eq!(optional.values(), Some(vec![held.clone()]), "{held:?}");
            let handle = Value::Host(optional.clone());
            // Its own text form.
            assert_eq!(
                HostRef::decode(&optional.encode()).as_ref(),
                Some(&optional)
            );
            // A list of markers: split, then each part parsed.
            let list = [handle.clone(), held.clone()]
                .iter()
                .map(crate::runtime::value_marker)
                .collect::<Vec<_>>()
                .join(",");
            let parts = crate::runtime::split_capture_markers(&list)
                .unwrap()
                .iter()
                .map(|part| crate::runtime::parse_value_marker(part).ok())
                .collect::<Vec<_>>();
            assert_eq!(
                parts,
                vec![Some(handle.clone()), Some(held.clone())],
                "{held:?}"
            );
            // A collection element.
            assert_eq!(
                crate::runtime::parse_value_marker(&crate::runtime::collection_item_marker(&handle)).ok(),
                Some(handle.clone()),
                "{held:?}"
            );
            // An optional of the optional, read back two levels down.
            let outer = HostRef::of_values(HostKind::Optional, std::slice::from_ref(&handle));
            let inner = match outer.values().as_deref() {
                Some([Value::Host(inner)]) => inner.clone(),
                other => panic!("{other:?}"),
            };
            assert_eq!(inner.values(), Some(vec![held.clone()]), "{held:?}");
        }
    }
}

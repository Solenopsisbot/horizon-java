//! Unmodifiable collections: `List.of`/`Set.of`/`Map.of`, `copyOf`, the
//! `Collections.empty*`/`singleton*` factories, Guava's `ImmutableList.of`/
//! `copyOf`, and `Collections.unmodifiable*` views.
//!
//! These used to answer the content-addressed `list/a,b` value markers, which
//! the host also uses for mutable results (`Collectors.toList()` is an
//! `ArrayList`), so a mutator could not tell them apart and `List.of("a")
//! .add("b")` succeeded. `ImmutableList.copyOf(source)` answered `source`
//! itself, so the "copy" grew with it.
//!
//! Each such collection is now an object of its own (`<class>#immutable<n>`)
//! whose elements live in the host's collection table like any `new
//! ArrayList`, and which is marked unmodifiable: every mutator raises
//! `UnsupportedOperationException` (no message, as the JDK and Guava throw
//! it). A `Collections.unmodifiable*` result is a view: it reads its source's
//! current elements. `equals` compares contents, as `AbstractList`/
//! `AbstractSet`/`AbstractMap` do, for every collection the host keeps.

use crate::{MethodRef, NativeError, StdHost, Value, expect_ref};

/// The members that change a collection, on any collection interface.
const MUTATORS: &[&str] = &[
    "add",
    "addAll",
    "addFirst",
    "addLast",
    "remove",
    "removeAll",
    "removeIf",
    "removeFirst",
    "removeLast",
    "retainAll",
    "clear",
    "set",
    "sort",
    "replaceAll",
    "put",
    "putAll",
    "putIfAbsent",
    "putFirst",
    "putLast",
    "compute",
    "computeIfAbsent",
    "computeIfPresent",
    "merge",
    "replace",
];

/// Whether `name` is one of the members above that change a collection.
pub(crate) fn is_mutator(name: &str) -> bool {
    MUTATORS.contains(&name)
}

fn throw(class: &str, message: Option<String>) -> NativeError {
    NativeError::UncaughtException {
        class: class.into(),
        message,
        frame: None,
    }
}

/// The class Java 25 makes an unmodifiable collection of `kind` holding
/// `size` elements (a map's: entries) as, for the factory `owner.name`
/// (#156 P14c: a host-made collection is an object of that class).
///
/// - `List.of`/`copyOf`: `List12` for one or two elements, else `ListN` (the
///   shared empty one too); `Set.of`/`copyOf` likewise `Set12`/`SetN`;
///   `Map.of`/`copyOf` `Map1` for one entry, else `MapN`.
/// - `Collections.empty*`/`singleton*`: `Collections$EmptyList`, ...,
///   `Collections$SingletonMap`.
/// - Guava 33.6.0's `ImmutableList.of`/`copyOf` and `ImmutableSet.copyOf`:
///   `SingletonImmutableList`/`Set` for one element, else
///   `RegularImmutableList`/`Set`.
/// - Anything else (`Stream.toList()`, whose class is always `ListN`):
///   `ListN`/`SetN`/`MapN`.
fn made_class(owner: &str, name: &str, kind: &str, size: usize) -> &'static str {
    match (owner, name, kind) {
        ("java/util/Collections", "emptyList", _) => "java/util/Collections$EmptyList",
        ("java/util/Collections", "emptySet", _) => "java/util/Collections$EmptySet",
        ("java/util/Collections", "emptyMap", _) => "java/util/Collections$EmptyMap",
        ("java/util/Collections", "singletonList", _) => "java/util/Collections$SingletonList",
        ("java/util/Collections", "singleton", _) => "java/util/Collections$SingletonSet",
        ("java/util/Collections", "singletonMap", _) => "java/util/Collections$SingletonMap",
        ("com/google/common/collect/ImmutableList", _, _) if size == 1 => {
            "com/google/common/collect/SingletonImmutableList"
        }
        ("com/google/common/collect/ImmutableList", _, _) => {
            "com/google/common/collect/RegularImmutableList"
        }
        ("com/google/common/collect/ImmutableSet", _, _) if size == 1 => {
            "com/google/common/collect/SingletonImmutableSet"
        }
        ("com/google/common/collect/ImmutableSet", _, _) => {
            "com/google/common/collect/RegularImmutableSet"
        }
        ("java/util/List", "of" | "copyOf", _) if matches!(size, 1 | 2) => {
            "java/util/ImmutableCollections$List12"
        }
        ("java/util/Set", "of" | "copyOf", _) if matches!(size, 1 | 2) => {
            "java/util/ImmutableCollections$Set12"
        }
        ("java/util/Map", "of" | "copyOf", _) if size == 1 => "java/util/ImmutableCollections$Map1",
        (_, _, "set") => "java/util/ImmutableCollections$SetN",
        (_, _, "map") => "java/util/ImmutableCollections$MapN",
        _ => "java/util/ImmutableCollections$ListN",
    }
}

/// Whether the factory `owner.name` answers one shared object when empty:
/// the JDK's `ImmutableCollections.EMPTY_*` and `Collections.EMPTY_*`, and
/// Guava's `RegularImmutableList.EMPTY`/`RegularImmutableSet.EMPTY`. Not
/// `Stream.toList()`, which makes a new `ListN` each time.
fn shares_empty(owner: &str, name: &str) -> bool {
    matches!(
        (owner, name),
        (
            "java/util/List" | "java/util/Set" | "java/util/Map",
            "of" | "copyOf"
        ) | (
            "java/util/Collections",
            "emptyList" | "emptySet" | "emptyMap"
        ) | (
            "com/google/common/collect/ImmutableList" | "com/google/common/collect/ImmutableSet",
            "of" | "copyOf"
        )
    )
}

/// The class `Collections.unmodifiable{List,Set,Map}(source)` makes: a
/// `List` view is `UnmodifiableRandomAccessList` over a `RandomAccess`
/// source and `UnmodifiableList` over any other (a `LinkedList`).
fn view_class(kind: &str, source_class: Option<&str>) -> &'static str {
    match kind {
        "list"
            if source_class.is_some_and(|class| {
                crate::jdk_model::class_instance_of(class, "java/util/List")
                    && !crate::jdk_model::class_instance_of(class, "java/util/RandomAccess")
            }) =>
        {
            "java/util/Collections$UnmodifiableList"
        }
        "list" => "java/util/Collections$UnmodifiableRandomAccessList",
        "set" => "java/util/Collections$UnmodifiableSet",
        _ => "java/util/Collections$UnmodifiableMap",
    }
}

/// The supertypes `instanceof` sees for one of the classes above, the
/// family's interface first (`java_tostring` prints through it). Java 25's
/// `jdk_closure` table holds every JDK one's whole set; Guava's are its
/// interfaces and Guava's own bases, from the same closure run over Guava
/// 33.6.0.
pub(crate) fn supertypes(class: &str) -> &'static [&'static str] {
    const LIST: &[&str] = &[
        "java/util/List",
        "java/util/SequencedCollection",
        "java/util/Collection",
        "java/lang/Iterable",
        "java/util/RandomAccess",
        "java/io/Serializable",
    ];
    // `UnmodifiableList`: a view over a list that is no `RandomAccess`.
    const SEQUENTIAL_LIST: &[&str] = &[
        "java/util/List",
        "java/util/SequencedCollection",
        "java/util/Collection",
        "java/lang/Iterable",
        "java/io/Serializable",
    ];
    const GUAVA_LIST: &[&str] = &[
        "java/util/List",
        "java/util/SequencedCollection",
        "java/util/Collection",
        "java/lang/Iterable",
        "java/util/RandomAccess",
        "java/io/Serializable",
        "java/util/AbstractCollection",
        "com/google/common/collect/ImmutableCollection",
        "com/google/common/collect/ImmutableList",
    ];
    const GUAVA_SET: &[&str] = &[
        "java/util/Set",
        "java/util/Collection",
        "java/lang/Iterable",
        "java/io/Serializable",
        "java/util/AbstractCollection",
        "com/google/common/collect/ImmutableCollection",
        "com/google/common/collect/ImmutableSet",
    ];
    const GUAVA_REGULAR_SET: &[&str] = &[
        "java/util/Set",
        "java/util/Collection",
        "java/lang/Iterable",
        "java/io/Serializable",
        "java/util/AbstractCollection",
        "com/google/common/collect/ImmutableCollection",
        "com/google/common/collect/ImmutableSet",
        "com/google/common/collect/ImmutableSet$CachingAsList",
    ];
    const SET: &[&str] = &[
        "java/util/Set",
        "java/util/Collection",
        "java/lang/Iterable",
        "java/io/Serializable",
    ];
    const MAP: &[&str] = &["java/util/Map", "java/io/Serializable"];
    match class {
        "java/util/ImmutableCollections$ListN"
        | "java/util/ImmutableCollections$List12"
        | "java/util/Collections$UnmodifiableRandomAccessList"
        | "java/util/Collections$EmptyList"
        | "java/util/Collections$SingletonList" => LIST,
        "java/util/Collections$UnmodifiableList" => SEQUENTIAL_LIST,
        "java/util/ImmutableCollections$SetN"
        | "java/util/ImmutableCollections$Set12"
        | "java/util/Collections$UnmodifiableSet"
        | "java/util/Collections$EmptySet"
        | "java/util/Collections$SingletonSet" => SET,
        "java/util/ImmutableCollections$MapN"
        | "java/util/ImmutableCollections$Map1"
        | "java/util/Collections$UnmodifiableMap"
        | "java/util/Collections$EmptyMap"
        | "java/util/Collections$SingletonMap" => MAP,
        "com/google/common/collect/RegularImmutableList"
        | "com/google/common/collect/SingletonImmutableList" => GUAVA_LIST,
        "com/google/common/collect/SingletonImmutableSet" => GUAVA_SET,
        "com/google/common/collect/RegularImmutableSet" => GUAVA_REGULAR_SET,
        _ => crate::sequenced::supertypes(class),
    }
}

impl StdHost {
    /// A new unmodifiable collection of the JDK (or Guava) class `class`
    /// holding `values`: an object of its own (#156 P14c), keyed
    /// `<class>#immutable<n>`.
    fn unmodifiable_as(&mut self, class: &str, values: Vec<Value>) -> Value {
        let marker = format!("{class}#immutable{}", self.unmodifiable_collections.len());
        self.mutable_collections.insert(marker.clone(), values);
        self.unmodifiable_collections.insert(marker.clone(), None);
        crate::jar_object::collection(marker)
    }

    /// What the factory `owner.name` answers for `values`: the class Java
    /// makes ([`made_class`]), and for an empty one a factory shares
    /// ([`shares_empty`]) that one object, made once per class.
    fn unmodifiable_from(
        &mut self,
        owner: &str,
        name: &str,
        kind: &str,
        values: Vec<Value>,
    ) -> Value {
        let size = if kind == "map" {
            values.len() / 2
        } else {
            values.len()
        };
        let class = made_class(owner, name, kind, size);
        if size == 0 && shares_empty(owner, name) {
            if let Some(shared) = self.shared_empties.get(class) {
                return crate::jar_object::collection(shared.clone());
            }
            let made = self.unmodifiable_as(class, values);
            if let Some(key) = expect_ref(&made) {
                self.shared_empties.insert(class, key.clone());
            }
            return made;
        }
        self.unmodifiable_as(class, values)
    }

    /// An unmodifiable `kind` for the stream terminals
    /// (`Host::unmodifiable_list`): `Stream.toList()`'s `ListN`, new each
    /// time.
    pub(crate) fn unmodifiable_values(&mut self, kind: &str, values: Vec<Value>) -> Value {
        self.unmodifiable_from("java/util/stream/Stream", "toList", kind, values)
    }

    /// The elements a collection argument holds, parsed.
    fn elements(&self, value: &Value) -> Result<Vec<Value>, NativeError> {
        crate::host_collection_values(self, value)
    }

    /// The unmodifiable factories, or `None` for any other static call.
    pub(crate) fn unmodifiable_factory(
        &mut self,
        r: &MethodRef,
        args: &[Value],
    ) -> Option<Result<Value, NativeError>> {
        let varargs = r.descriptor.starts_with("([");
        let kind = match (r.owner.as_str(), r.name.as_str()) {
            ("java/util/List", "of") | ("com/google/common/collect/ImmutableList", "of")
                if !varargs =>
            {
                "list"
            }
            ("java/util/Set", "of") if !varargs => "set",
            ("java/util/Map", "of") => "map",
            ("java/util/List" | "java/util/Set" | "java/util/Map", "copyOf") => {
                let copy = match self.collection_copy_of(r, args)? {
                    Ok(copy) => copy,
                    Err(error) => return Some(Err(error)),
                };
                let kind = r
                    .owner
                    .rsplit('/')
                    .next()
                    .unwrap_or_default()
                    .to_lowercase();
                let list_copy = r.owner == "java/util/List";
                let owner = r.owner.clone();
                return Some(self.elements(&copy).map(|values| {
                    let made = self.unmodifiable_from(&owner, "copyOf", &kind, values);
                    if list_copy {
                        self.mark_jdk_list_of(&made);
                    }
                    made
                }));
            }
            (
                "com/google/common/collect/ImmutableList"
                | "com/google/common/collect/ImmutableSet",
                "copyOf",
            ) if r.descriptor.starts_with("(Ljava/util/Collection;")
                || r.descriptor.starts_with("(Ljava/lang/Iterable;") =>
            {
                let set = r.owner.ends_with("ImmutableSet");
                let owner = r.owner.clone();
                return Some(self.elements(args.first()?).map(|mut values| {
                    if set {
                        let mut seen = std::collections::BTreeSet::new();
                        values.retain(|value| seen.insert(crate::value_marker(value)));
                    }
                    let kind = if set { "set" } else { "list" };
                    self.unmodifiable_from(&owner, "copyOf", kind, values)
                }));
            }
            ("java/util/Collections", "emptyList" | "singletonList") => "list",
            ("java/util/Collections", "emptySet" | "singleton") => "set",
            ("java/util/Collections", "emptyMap" | "singletonMap") => "map",
            ("java/util/Collections", name) if name.starts_with("unmodifiable") => {
                let kind = match name {
                    "unmodifiableList" => "list",
                    "unmodifiableSet" => "set",
                    "unmodifiableMap" => "map",
                    _ => return None,
                };
                let source_value = args.first()?;
                let source = expect_ref(source_value)?.clone();
                let source_class = crate::jar_object::object_class(source_value);
                let marker = format!(
                    "{}#immutable{}",
                    view_class(kind, source_class),
                    self.unmodifiable_collections.len()
                );
                self.unmodifiable_collections
                    .insert(marker.clone(), Some(source));
                return Some(Ok(crate::jar_object::collection(marker)));
            }
            _ => return None,
        };
        // `List.of`/`Set.of`/`Map.of` refuse null elements and duplicates.
        let of = r.name == "of";
        if of && args.iter().any(|value| matches!(value, Value::Ref(None))) {
            return Some(Err(throw("java/lang/NullPointerException", None)));
        }
        let mut seen = std::collections::BTreeMap::new();
        let step = if kind == "map" { 2 } else { 1 };
        for key in args.iter().step_by(step) {
            // Equal text is a duplicate, but for two handles only the proved
            // same object is (`HostRef::same_object`): equal keys may be two
            // objects, whose `equals` the runtime asked (and counted) already.
            let duplicate = match seen.insert(crate::value_marker(key), key) {
                Some(Value::Host(first)) => match key {
                    Value::Host(second) => first.same_object(second) == Some(true),
                    _ => true,
                },
                Some(_) => true,
                None => false,
            };
            if of && kind != "list" && duplicate {
                let what = if kind == "map" { "key" } else { "element" };
                let text = crate::value_text(key);
                return Some(Err(throw(
                    "java/lang/IllegalArgumentException",
                    Some(format!("duplicate {what}: {text}")),
                )));
            }
        }
        let made = self.unmodifiable_from(&r.owner, &r.name, kind, args.to_vec());
        if r.name.starts_with("empty")
            && let Some(marker) = expect_ref(&made)
        {
            self.empty_collections.insert(marker.clone());
        }
        if r.owner == "java/util/List" {
            self.mark_jdk_list_of(&made);
        }
        Some(Ok(made))
    }

    /// Remember a JDK `List12`/`ListN` (`List.of`, `List.copyOf`): its
    /// `indexOf`/`lastIndexOf`/`contains` start with
    /// `Objects.requireNonNull(o)` (`list_equality.rs`).
    fn mark_jdk_list_of(&mut self, list: &Value) {
        if let Some(marker) = expect_ref(list) {
            self.jdk_list_of.insert(marker.clone());
        }
    }

    /// The elements a view reads: its source's current ones.
    pub(crate) fn view_values(&self, marker: &str) -> Option<Vec<Value>> {
        let source = self.unmodifiable_collections.get(marker)?.as_ref()?;
        let source = crate::jar_object::stored_value(source);
        self.mutable_values(&source)
            .or_else(|| crate::collection_values(&source).ok())
    }

    /// A collection member this module answers first: a mutator on an
    /// unmodifiable collection throws, and `equals` compares contents.
    pub(crate) fn unmodifiable_member(
        &mut self,
        r: &MethodRef,
        marker: &str,
        args: &[Value],
    ) -> Option<Result<Value, NativeError>> {
        if MUTATORS.contains(&r.name.as_str())
            && self.empty_collections.contains(marker)
            && let Some(answer) = empty_collection_member(r, args)
        {
            return Some(answer);
        }
        if MUTATORS.contains(&r.name.as_str()) && self.unmodifiable_collections.contains_key(marker)
        {
            return Some(Err(throw("java/lang/UnsupportedOperationException", None)));
        }
        if (r.name.as_str(), r.descriptor.as_str()) != ("equals", "(Ljava/lang/Object;)Z") {
            return None;
        }
        let receiver = crate::jar_object::stored_value(marker);
        let mine = self.elements(&receiver).ok()?;
        let Ok(theirs) = self.elements(args.first()?) else {
            return Some(Ok(Value::Int(0)));
        };
        let is_list = |owner: &str| crate::is_list_owner(owner) || owner == "java/util/Collection";
        let equal = if is_list(&r.owner) {
            mine == theirs
        } else {
            // A set or a map: the same members (map entries as pairs) in any
            // order.
            let step = if crate::is_map_owner(&r.owner) { 2 } else { 1 };
            let members = |values: &[Value]| {
                let mut members = values
                    .chunks(step)
                    .map(|chunk| chunk.iter().map(crate::value_marker).collect::<Vec<_>>())
                    .collect::<Vec<_>>();
                members.sort();
                members
            };
            members(&mine) == members(&theirs)
        };
        Some(Ok(Value::Int(i32::from(equal))))
    }
}

/// `Collections.emptyList()`/`emptySet()`/`emptyMap()` mutators that JDK 25
/// does not refuse: `replaceAll`/`removeIf` only check their argument
/// (`removeIf` answers `false`), `sort` does nothing, and the members
/// inherited from `AbstractCollection`/`AbstractList`/`AbstractMap` that
/// walk the (empty) collection (`remove`, `removeAll`, `retainAll`,
/// `clear`, adding nothing) find nothing to do. `None`: the member refuses.
fn empty_collection_member(r: &MethodRef, args: &[Value]) -> Option<Result<Value, NativeError>> {
    let nothing = || -> Value {
        if r.descriptor.ends_with(")Z") {
            Value::Int(0)
        } else if r.descriptor.ends_with(")V") {
            Value::Void
        } else {
            Value::Ref(None)
        }
    };
    let null_argument = matches!(args.first(), Some(Value::Ref(None)));
    match r.name.as_str() {
        "replaceAll" | "removeIf" | "removeAll" | "retainAll" if null_argument => {
            Some(Err(throw("java/lang/NullPointerException", None)))
        }
        "replaceAll" | "removeIf" | "sort" | "clear" | "removeAll" | "retainAll" => {
            Some(Ok(nothing()))
        }
        // `remove(Object)`: a walk that finds nothing. `List.remove(int)`
        // is `AbstractList`'s refusal.
        "remove" if r.descriptor.starts_with("(Ljava/lang/Object;") => Some(Ok(nothing())),
        _ => None,
    }
}

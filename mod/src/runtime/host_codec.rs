//! DataFixerUpper's `Pair` and the game's codecs as host handles (#156 P5c). They were the text
//! `dfu-pair/<first>,<second>`, `<Codec type>#codec` (an unmodelled codec call's answer) and
//! `nf-codec/<n>` (a codec node the attachment model builds), which a String could spell: a
//! `"dfu-pair/…"` String answered `getFirst`, and an `"nf-codec/0"` String was a codec a
//! NeoForge attachment serialized through.
//!
//! **`Pair`** (DataFixerUpper 10.0.21, `javap -c`): `Pair.of(f, s)` is `new Pair(f, s)`, so its
//! class is exactly `Pair` (a public, non-final class with a public constructor: an instance of
//! a jar's subclass is a jar object, never this handle). `Pair` implements `App`. The key
//! (`HostRef::of_values`, which escapes) is `[serial, first, second]`, a serial per allocation:
//! one key, one object. `equals` is `o instanceof Pair` and `Objects.equals` on both fields,
//! `hashCode` Guava's `Objects.hashCode(first, second)`, `toString` `"(" + first + ", " + second
//! + ")"`: each exact where its held values' answers are (`Nested`), counted otherwise. A jar
//! object compared with a pair may be a `Pair` subclass: counted.
//!
//! **Opaque codecs.** What an unmodelled call declared to answer a `Codec`, `MapCodec`,
//! `StreamCodec`, `PrimitiveCodec` or `RecordCodecBuilder` answers: a non-null object of that
//! declared type that does nothing. Its class is unknown (`ClassRule::Unknown`, the declared type
//! and its supertypes), and so is its identity: the key is the declared type alone, never
//! canonical, because `ByteBufCodecs.INT` read twice is one object and two `composite(..)` calls
//! are two. `equals` is identity (counted unless one handle flows to both sides).
//!
//! **Modelled codecs** (`attachment_codec.rs`): `CompoundTag.CODEC` and the `fieldOf`/`flatXmap`
//! nodes built from it. The key is `compound-tag` or `codec:<n>`/`map:<n>`, the node's number in
//! the jar's table and whether it is a `MapCodec`, all the runtime's own text: one key, one
//! object. The classes are DataFixerUpper's private implementations: unknown, declared `Codec`
//! or `MapCodec`.

use std::sync::atomic::{AtomicU64, Ordering};

use crate::runtime::host_ref::{
    ClassHierarchy, ClassSide, HostKind, HostRef, Methods, Nested, exact_class_rules_out,
};
use crate::runtime::{NativeError, Value};

pub(crate) const PAIR: &str = "com/mojang/datafixers/util/Pair";
const APP: &str = "com/mojang/datafixers/kinds/App";
const OBJECT: &str = "java/lang/Object";
const CODEC: &str = "com/mojang/serialization/Codec";
const MAP_CODEC: &str = "com/mojang/serialization/MapCodec";
/// The key of `CompoundTag.CODEC`.
pub(crate) const COMPOUND_TAG_KEY: &str = "compound-tag";

static NEXT_PAIR: AtomicU64 = AtomicU64::new(0);

/// `Pair.of(first, second)`: a new pair.
pub(crate) fn pair(first: &Value, second: &Value) -> Value {
    Value::Host(HostRef::of_values(
        HostKind::DfuPair,
        &[
            Value::Long(NEXT_PAIR.fetch_add(1, Ordering::Relaxed) as i64),
            first.clone(),
            second.clone(),
        ],
    ))
}

fn pair_fields(handle: &HostRef) -> Option<(Value, Value)> {
    if handle.kind() != HostKind::DfuPair {
        return None;
    }
    match <[Value; 3]>::try_from(handle.values()?) {
        Ok([_, first, second]) => Some((first, second)),
        Err(_) => None,
    }
}

/// The two values of a pair handle, `None` for any other value.
pub(crate) fn pair_parts(value: &Value) -> Option<(Value, Value)> {
    match value {
        Value::Host(handle) => pair_fields(handle),
        _ => None,
    }
}

fn pair_text(handle: &HostRef, nested: &mut dyn Nested) -> Result<Option<String>, NativeError> {
    let Some((first, second)) = pair_fields(handle) else {
        return Ok(None);
    };
    let (Some(first), Some(second)) = (nested.text(&first)?, nested.text(&second)?) else {
        return Ok(None);
    };
    Ok(Some(format!("({first}, {second})")))
}

/// Guava's `Objects.hashCode(first, second)`: `Arrays.hashCode`.
fn pair_hash(handle: &HostRef, nested: &mut dyn Nested) -> Result<Option<i32>, NativeError> {
    let Some((first, second)) = pair_fields(handle) else {
        return Ok(None);
    };
    let (Some(first), Some(second)) = (nested.hash(&first)?, nested.hash(&second)?) else {
        return Ok(None);
    };
    Ok(Some(
        31_i32
            .wrapping_add(first)
            .wrapping_mul(31)
            .wrapping_add(second),
    ))
}

fn pair_equals(
    handle: &HostRef,
    other: &Value,
    nested: &mut dyn Nested,
) -> Result<Option<bool>, NativeError> {
    let Some((first, second)) = pair_fields(handle) else {
        return Ok(None);
    };
    let (their_first, their_second) = match other {
        Value::Host(other) => match pair_fields(other) {
            Some(fields) => fields,
            None => return Ok(Some(false)),
        },
        // A jar object may be an instance of a `Pair` subclass.
        Value::Ref(Some(_)) if crate::runtime::jar_object::object_key(other).is_some() => return Ok(None),
        _ => return Ok(Some(false)),
    };
    Ok(match nested.equals(&first, &their_first)? {
        Some(false) => Some(false),
        Some(true) => nested.equals(&second, &their_second)?,
        None => match nested.equals(&second, &their_second)? {
            Some(false) => Some(false),
            _ => None,
        },
    })
}

/// `class instanceof wanted` for an object of exactly `Pair`.
fn pair_instance_of(class: &str, wanted: &str, hierarchy: &dyn ClassHierarchy) -> Option<bool> {
    if [PAIR, APP, OBJECT].contains(&wanted) || hierarchy.is_instance(class, wanted) == Some(true) {
        return Some(true);
    }
    (hierarchy.is_interface(wanted) == Some(false) || exact_class_rules_out(class, wanted))
        .then_some(false)
}

/// A pair holds the jar's own values: it crosses to another jar only if they all do.
fn pair_crosses(handle: &HostRef, crosses: &dyn Fn(&Value) -> bool) -> bool {
    pair_fields(handle).is_some_and(|(first, second)| crosses(&first) && crosses(&second))
}

pub(crate) const PAIR_METHODS: Methods = Methods {
    to_string: pair_text,
    hash_code: pair_hash,
    equals: pair_equals,
    instance_of: |_, wanted, hierarchy| pair_instance_of(PAIR, wanted, hierarchy),
    crosses_jars: pair_crosses,
    compare_to: None,
};

pub(crate) fn pair_class(_: &str) -> &str {
    PAIR
}

pub(crate) const PAIR_CLASS_SIDE: ClassSide = ClassSide {
    owns: |class| class == PAIR,
    instance_of: pair_instance_of,
    superclass: |_| Some(OBJECT.to_owned()),
    is_interface: |_| Some(false),
    classes: || vec![PAIR.to_owned()],
};

/// An opaque codec of the declared type `declared`.
pub(crate) fn opaque_codec(declared: &str) -> Value {
    Value::Host(HostRef::new(HostKind::OpaqueCodec, declared))
}

/// The declared type of an opaque codec handle, `None` for any other value.
pub(crate) fn opaque_codec_type(value: &Value) -> Option<&str> {
    match value {
        Value::Host(handle) if handle.kind() == HostKind::OpaqueCodec => Some(handle.key()),
        _ => None,
    }
}

/// The declared type of an opaque codec and every supertype it names (`javap`: DataFixerUpper
/// 10.0.21, Minecraft 26.3).
pub(crate) fn opaque_codec_types(key: &str) -> &'static [&'static str] {
    match key {
        "net/minecraft/network/codec/StreamCodec" => &[
            "net/minecraft/network/codec/StreamCodec",
            "net/minecraft/network/codec/StreamEncoder",
            "net/minecraft/network/codec/StreamDecoder",
        ],
        CODEC => &[
            CODEC,
            "com/mojang/serialization/Encoder",
            "com/mojang/serialization/Decoder",
        ],
        "com/mojang/serialization/codecs/PrimitiveCodec" => &[
            "com/mojang/serialization/codecs/PrimitiveCodec",
            CODEC,
            "com/mojang/serialization/Encoder",
            "com/mojang/serialization/Decoder",
        ],
        MAP_CODEC => MAP_CODEC_TYPES,
        "com/mojang/serialization/codecs/RecordCodecBuilder" => {
            &["com/mojang/serialization/codecs/RecordCodecBuilder", APP]
        }
        _ => &[],
    }
}

const MAP_CODEC_TYPES: &[&str] = &[
    MAP_CODEC,
    "com/mojang/serialization/CompressorHolder",
    "com/mojang/serialization/Compressable",
    "com/mojang/serialization/MapDecoder",
    "com/mojang/serialization/MapEncoder",
    "com/mojang/serialization/Keyable",
];

/// A modelled codec: `CompoundTag.CODEC` ([`COMPOUND_TAG_KEY`]) or node `n` of the jar's table.
pub(crate) fn modelled_codec(key: String) -> Value {
    Value::Host(HostRef::new(HostKind::ModelledCodec, key))
}

/// The key of a modelled codec handle, `None` for any other value.
pub(crate) fn modelled_codec_key(value: &Value) -> Option<&str> {
    match value {
        Value::Host(handle) if handle.kind() == HostKind::ModelledCodec => Some(handle.key()),
        _ => None,
    }
}

pub(crate) fn modelled_codec_types(key: &str) -> &'static [&'static str] {
    if key.starts_with("map:") {
        MAP_CODEC_TYPES
    } else {
        &[
            CODEC,
            "com/mojang/serialization/Encoder",
            "com/mojang/serialization/Decoder",
        ]
    }
}

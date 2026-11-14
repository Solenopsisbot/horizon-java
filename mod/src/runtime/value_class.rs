//! The runtime class of the two values the interpreter keeps untyped: a
//! plain string and an array.
//!
//! A `String` is a bare `Value::Ref(Some(text))`, the same shape every runtime
//! and host marker has, so `"x" instanceof String` used to answer the neutral
//! `false`. That stayed invisible until a library model acted on it: Guava's
//! `Preconditions.checkState(validator.test(defaultValue))` with
//! `validator = o -> o instanceof String` refused FarmersDelight's own string
//! defaults ("Config defaults are invalid").
//!
//! An array once named the frame that built it, not its element type, so
//! `array.getClass()` was null and `getComponentType()` of it too.
//! Architectury's `EventFactory.createLoop(T... empty)` reads its event
//! interface exactly that way, and the null reached `Proxy.newProxyInstance`
//! as an NPE. An array handle's key now carries its class (`host_array.rs`).

use super::*;

/// `java.lang.String` and every type it extends or implements.
pub(crate) const STRING_TYPES: &[&str] = &[
    "java/lang/String",
    "java/lang/CharSequence",
    "java/lang/Comparable",
    "java/io/Serializable",
    "java/lang/constant/Constable",
    "java/lang/constant/ConstantDesc",
];

/// The prefixes of marker families that are host handles now (#156): no
/// runtime value is text spelled so, so such a text is a `String`.
const RETIRED_PREFIXES: &[&str] = &[
    "array/",
    "brigadier/",
    "config-holder/",
    "config-builder/",
    "configlib-value/",
    "mixin/callback-info/",
    "mixin-plugin/",
    "mixin-side/",
    // Iterators, enumerations and list iterators are handles, and `Collectors`' results
    // `Collector` handles (#156 P14f, `host_iterator.rs`, `host_collector.rs`).
    "iterator/",
    "resource-enumeration/",
    "enumeration/",
    "list-iterator/",
    "collector/",
    // Map entries are handles (#156 P14e, `host_map_entry.rs`).
    "map-entry/",
    // The client's objects and values are game handles (#156 P16d,
    // `client_values.rs`, `game_object::client_value`).
    "client/",
    "key-category/",
];

/// Whether a reference `marker` is a plain `java.lang.String` and `wanted`
/// is one of the types a string is an instance of.
pub(crate) fn string_instance_of(marker: &str, wanted: &str) -> bool {
    STRING_TYPES.contains(&wanted) && is_plain_string(marker)
}

/// Whether a reference's text is a string rather than a runtime or host
/// marker.
///
/// The interpreter cannot know for certain, so this answers from the marker
/// shapes the runtime builds, and leaves a text that looks like one of them
/// a non-string (the previous answer for every text):
/// - a box, a builder, a collection, a path or an enum constant: whatever
///   `Object.getClass` names (`class_of_value`), including `true`/`false`;
/// - a host marker `<class>#<kind>`: a `#` after a non-empty,
///   separator-free class. An object's key, `<class>#<n>` or a host-made
///   collection's `<class>#neutral<n>`/`#immutable<n>`/`#reversed<n>`, is a handle,
///   never text (#156 P12b, P14c), so a text spelled like one is a `String`;
/// - a callback or comparator: a leading `@`;
/// - a marker kind: a `/` whose first segment has no `:`, `.` or space
///   (`list/3`, `registry/minecraft:item`), which a resource id like
///   `minecraft:block/stone` does not match.
///
/// A class is a handle (`host_class.rs`, #156 P13c), so a text spelled like
/// one is a `String`: an array descriptor (`[I`, `[Ljava/lang/String;`) and a
/// text shaped like an internal class name ([`class_name_shaped`]:
/// `java/lang/String`, `demo/Probe$Foo`) are no markers.
///
/// Arrays, `ColorCollection`s and the command surface's objects are
/// handles, never text (`host_array.rs`, `host_brigadier.rs`): a text spelled
/// like their old markers ([`RETIRED_PREFIXES`]) or their keys (`[I#0`,
/// `color-collection#0`) is a `String`.
pub(crate) fn is_plain_string(marker: &str) -> bool {
    if RETIRED_PREFIXES
        .iter()
        .any(|prefix| marker.starts_with(prefix))
    {
        return true;
    }
    // The last separator-free markers (`guava-cache-builder`,
    // `mixin-environment`) became handles in #179 and #180, so a text with no
    // `/`, `#`, `@` or `:` is never a runtime marker any more.
    if marker.starts_with('@') {
        return false;
    }
    // An array class's descriptor (`[I`, `[Ljava/lang/String;`) names a
    // `Class`, which is a handle (#156 P13c): as text it is a `String`.
    if marker.starts_with('[') && !marker.contains('#') {
        return true;
    }
    // An object is a handle (`jar_object.rs`), a host-made collection too
    // (P14c): a text spelling either's key is a `String`.
    if crate::runtime::jar_object::spells_key_text(marker) {
        return true;
    }
    // A host map entry's key (`<class>#entry<n>`/`#node<n>`, #156 P14e) is a handle's,
    // never text.
    if crate::runtime::host_map_entry::spells_entry_key(marker) {
        return true;
    }
    if let Some((class, _)) = marker.split_once('#')
        && !class.is_empty()
        && !class.contains([':', ' '])
    {
        return host_array::handle_only_class(class);
    }
    if !marker.contains('#') && class_of_text(marker).is_some() {
        return false;
    }
    if let Some((head, _)) = marker.split_once('/')
        && !head.is_empty()
        && !head.contains([':', '.', ' '])
        && !class_name_shaped(marker)
    {
        return false;
    }
    true
}

/// The first segment of the runtime's own `word/rest` markers that a class
/// name's could be: lowercase words a package might be named, which the
/// runtime writes ahead of a capitalized segment (`component/Hello`,
/// `enum/…`, `game/…`). A hyphenated kind (`map-entry/`) is no package name,
/// so it needs no entry. Until #156 P17 retires these markers, a text with one
/// of these heads keeps reading as the marker.
const MARKER_HEADS: &[&str] = &[
    "annotation",
    "array",
    "attachments",
    "block",
    "brigadier",
    "builder",
    "cancelled",
    "capability",
    "charset",
    "classloader",
    "codec",
    "collector",
    "component",
    "compound",
    "computed",
    "contains",
    "deferred",
    "direction",
    "dist",
    "duration",
    "enum",
    "event",
    "file",
    "game",
    "handler",
    "holder",
    "identifier",
    "iterator",
    "life",
    "linked",
    "list",
    "logger",
    "map",
    "mixin",
    "module",
    "null",
    "object",
    "optional",
    "path",
    "payload",
    "rarity",
    "recipe",
    "registered",
    "registry",
    "runtime",
    "set",
    "stack",
    "stream",
    "thread",
    "tick",
    "token",
    "type",
    "typed",
    "typetoken",
    "uuid",
];

/// Whether `text` is shaped like a class's internal name: package segments
/// that are lowercase Java identifiers, then a class name that starts with an
/// uppercase letter or `$` (`java/lang/String`, `demo/Probe$Foo`,
/// `net/minecraft/world/item/Item`), under no [`MARKER_HEADS`] head. A
/// `Class` is a handle (#156 P13c), so such a text is a `String`.
pub(crate) fn class_name_shaped(text: &str) -> bool {
    let Some((package, class)) = text.rsplit_once('/') else {
        return false;
    };
    let identifier = |segment: &str, first: fn(char) -> bool| {
        segment.chars().next().is_some_and(first)
            && segment
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
    };
    let head = package.split('/').next().unwrap_or(package);
    !MARKER_HEADS.contains(&head)
        && package
            .split('/')
            .all(|segment| identifier(segment, |c| c.is_ascii_lowercase()))
        && identifier(class, |c| c.is_ascii_uppercase() || c == '$')
}

/// The class marker of an array whose elements are `component`: the JVM's
/// descriptor form (`[Ljava/lang/String;`, `[I`, `[[I`), which the class
/// surface already reads as an array class.
pub(crate) fn array_class_marker(component: &str) -> String {
    let element = match component {
        "boolean" => "Z".to_owned(),
        "byte" => "B".to_owned(),
        "char" => "C".to_owned(),
        "short" => "S".to_owned(),
        "int" => "I".to_owned(),
        "long" => "J".to_owned(),
        "float" => "F".to_owned(),
        "double" => "D".to_owned(),
        array if array.starts_with('[') => array.to_owned(),
        other => format!("L{};", other.replace('.', "/")),
    };
    format!("[{element}")
}

/// The component class marker of an array class marker, or `None` when the
/// marker is not an array class.
pub(crate) fn component_of_array_class(marker: &str) -> Option<String> {
    let element = marker.strip_prefix('[')?;
    Some(match element {
        "Z" => "boolean".to_owned(),
        "B" => "byte".to_owned(),
        "C" => "char".to_owned(),
        "S" => "short".to_owned(),
        "I" => "int".to_owned(),
        "J" => "long".to_owned(),
        "F" => "float".to_owned(),
        "D" => "double".to_owned(),
        nested if nested.starts_with('[') => nested.to_owned(),
        object => object
            .strip_prefix('L')
            .and_then(|name| name.strip_suffix(';'))?
            .replace('.', "/"),
    })
}

/// The component class marker of a `newarray` element type code.
pub(crate) fn primitive_array_component(atype: u8) -> Option<&'static str> {
    Some(match atype {
        4 => "boolean",
        5 => "char",
        6 => "float",
        7 => "double",
        8 => "byte",
        9 => "short",
        10 => "int",
        11 => "long",
        _ => return None,
    })
}

/// The component class of an array (`java/lang/String`, `int`, `[I`);
/// `None` for any other value, and for a shared empty array, whose exact
/// class is unknown (`host_array.rs`).
pub(crate) fn array_component(value: &Value) -> Option<String> {
    let key = host_array::array_key(value)?;
    if host_array::is_shared_empty(key) {
        return None;
    }
    component_of_array_class(host_array::class_of_key(key))
}

impl<H: Host> Runtime<'_, H> {
    /// `array.getClass()`, and `Class.getComponentType()` of an array class
    /// (null for any other class, as on the JVM).
    /// A shared empty array's exact class is unknown (`host_array.rs`):
    /// its `getClass()` is counted.
    pub(super) fn invoke_array_class(
        &mut self,
        r: &MethodRef,
        receiver: Option<&Value>,
    ) -> Result<Option<Value>, NativeError> {
        let Some(receiver) = receiver else {
            return Ok(None);
        };
        Ok(
            match (r.owner.as_str(), r.name.as_str(), r.descriptor.as_str()) {
                ("java/lang/Object", "getClass", "()Ljava/lang/Class;") => {
                    let Some(key) = host_array::array_key(receiver) else {
                        return Ok(None);
                    };
                    if host_array::is_shared_empty(key) {
                        return self.host.unknown_operand_call(r).map(Some);
                    }
                    Some(crate::runtime::host_class::class_value(host_array::class_of_key(
                        key,
                    )))
                }
                // The element class's own `Class` object; `null` for a class
                // that is no array.
                ("java/lang/Class", "getComponentType", "()Ljava/lang/Class;") => {
                    crate::runtime::host_class::class_name(receiver).map(|class| {
                        component_of_array_class(class).map_or(Value::Ref(None), |component| {
                            crate::runtime::host_class::class_value(&component)
                        })
                    })
                }
                _ => None,
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_is_a_string_and_runtime_markers_are_not() {
        for text in [
            "farmersdelight:rope",
            "",
            "hello world",
            "#ff0000",
            "minecraft:block/stone",
            "config.json",
            // Spelled like the boxes once were; boxes are no longer text.
            "integer:5",
            "true",
            // Spelled like a path and the stack walker once were (#156 P2).
            "path:/horizon/game",
            "stack-walker",
            // Spelled like a `Lookup` once was (#156 P10b).
            "method-handles-lookup",
            // Spelled like an object's key (#156 P12b).
            "demo/Main#3",
            "java/util/concurrent/ThreadPoolExecutor#0",
            // Spelled like a host-made collection's key (#156 P14c).
            "java/util/ArrayList#neutral0",
            "java/util/ImmutableCollections$ListN#immutable3",
            "java/util/ReverseOrderListView$Rand#reversed0",
            // Spelled like a class (#156 P13c): a `Class` is a handle.
            "java/lang/String",
            "demo/Probe$Foo",
            "net/minecraft/world/item/Item",
            "[Ljava/lang/String;",
            "[I",
            "int",
        ] {
            assert!(is_plain_string(text), "{text:?} should be a string");
        }
        for marker in [
            "java/util/ArrayList#weird0",
            "list/4",
            "registry/minecraft:item",
            "string-builder/2",
            "@indy:demo/Main:0:demo/Main#lambda$0()V:",
            "@cmp:natural",
            // A marker kind ahead of a capitalized segment is no class name.
            "component/Hello",
            "enum/net/minecraft/core/Direction/NORTH",
        ] {
            assert!(
                !is_plain_string(marker),
                "{marker:?} should not be a string"
            );
        }
    }
}

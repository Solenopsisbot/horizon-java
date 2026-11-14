//! Brigadier's and the command surface's objects as host handles (#156 P7b):
//! the dispatcher and build context a registration callback receives, the
//! builders, nodes and argument types it makes, the `CommandContext` and
//! `CommandSourceStack` a running command sees, and the permission values
//! (`brigadier.rs` answers their calls).
//!
//! They used to be text a jar `String` could spell (`brigadier/dispatcher`,
//! `brigadier/node/3`, `brigadier/source/0`, `brigadier/requires/2`, ...), so
//! the String `"brigadier/requires/0"` passed a `requires` check as level 0.
//! Each is now a [`HostRef`] whose key holds what the object is through the
//! escaping encoder (`HostRef::of_values`): a node's literal or argument name
//! is jar text, and a name with any separator stays that name.
//!
//! What each is on a JVM (`javap` of brigadier 1.3.11, the version 26.3
//! ships, and 26.3's sources):
//!
//! | kind | key | surely a | identity |
//! |---|---|---|---|
//! | [`HostKind::BrigadierDispatcher`] | none | `CommandDispatcher` | the server's: never canonical (a reload makes another) |
//! | [`HostKind::CommandBuildContext`] | none | `CommandBuildContext`, a `HolderLookup.Provider` | never canonical |
//! | [`HostKind::CommandContext`] | the run's serial | `CommandContext` | one per run: canonical |
//! | [`HostKind::CommandSource`] | the run's serial, the source's index in it | `CommandSourceStack` | one per run: canonical |
//! | [`HostKind::CommandNode`] | arena id, kind, name | `LiteralCommandNode`/`ArgumentCommandNode`/`RootCommandNode`, a `CommandNode`, `Comparable` | one per id: canonical |
//! | [`HostKind::ArgumentBuilder`] | builder id, kind | `LiteralArgumentBuilder`/`RequiredArgumentBuilder` | one per id: canonical |
//! | [`HostKind::ArgumentType`] | arena id, factory class | the factory's class, an `ArgumentType` | one per id: canonical |
//! | [`HostKind::CommandPermission`] | what it is, its level | `Permission`/`PermissionCheck`/`PermissionSet`, or the `Predicate` a `requires` takes | never canonical (several constants share a level) |
//!
//! Every kind is `ClassRule::Unknown` (an exact `getClass()` needs a class
//! side for these classes, #174's `ClassSide`; not added here): `getClass()`
//! is counted, `instanceof` is exact for the declared types and their
//! supertypes; a node, builder, dispatcher or context against brigadier's
//! classes, and one of brigadier's argument types against the others, is
//! exact too (the runtime builds exactly that class). The `Object` methods
//! follow the real classes:
//!
//! * `CommandNode.equals`/`hashCode` compare and hash the children and the
//!   command (and the literal, the argument's name and type): the same node
//!   is equal; a node of another kind, literal or name is not; another node
//!   of the same kind and name is unknown here (counted), and so is
//!   `hashCode`; anything that is no `CommandNode` is not equal. `compareTo` sorts
//!   literals first, then by literal or argument name (`String.compareTo`).
//!   `toString` is `<literal NAME>`, `<root>`, and for an argument `<argument
//!   NAME:TYPE>`, whose type text is counted.
//! * `CommandContext.equals`, an argument type's, and the permission records'
//!   compare their state: equal for the same object, unknown (counted) for
//!   another of the kind, unequal to anything else.
//! * The dispatcher, the build context, the source and the builders keep
//!   `Object`'s identity `equals`. Every other `toString`/`hashCode` is
//!   identity based or reads state the handle does not hold: counted.

use crate::host_ref::{ClassHierarchy, HostKind, HostRef, Methods, Nested, instance_of_class};
use crate::{NativeError, Value};

/// A node's kind, in its key.
pub(crate) const LITERAL: &str = "literal";
pub(crate) const ARGUMENT: &str = "argument";
pub(crate) const ROOT: &str = "root";

/// What a [`HostKind::CommandPermission`] is, in its key.
pub(crate) const PERMISSION: &str = "permission";
pub(crate) const PERMISSION_CHECK: &str = "permission-check";
pub(crate) const PERMISSION_SET: &str = "permission-set";
pub(crate) const REQUIRES: &str = "requires";

fn text(text: &str) -> Value {
    Value::Ref(Some(text.to_owned()))
}

fn index(at: usize) -> Value {
    Value::Int(i32::try_from(at).unwrap_or(i32::MAX))
}

fn handle(kind: HostKind, values: &[Value]) -> Value {
    Value::Host(HostRef::of_values(kind, values))
}

fn values_of(value: &Value, kind: HostKind) -> Option<Vec<Value>> {
    match value {
        Value::Host(handle) if handle.kind() == kind => handle.values(),
        _ => None,
    }
}

fn int_at(values: &[Value], at: usize) -> Option<usize> {
    match values.get(at)? {
        Value::Int(value) => usize::try_from(*value).ok(),
        _ => None,
    }
}

fn text_at(values: &[Value], at: usize) -> Option<&str> {
    match values.get(at)? {
        Value::Ref(Some(text)) => Some(text),
        _ => None,
    }
}

/// Whether `value` is a handle of `kind`.
pub(crate) fn is(value: &Value, kind: HostKind) -> bool {
    matches!(value, Value::Host(handle) if handle.kind() == kind)
}

pub(crate) fn dispatcher() -> Value {
    handle(HostKind::BrigadierDispatcher, &[])
}

pub(crate) fn build_context() -> Value {
    handle(HostKind::CommandBuildContext, &[])
}

/// The context of command run `run`.
pub(crate) fn context(run: usize) -> Value {
    handle(HostKind::CommandContext, &[index(run)])
}

pub(crate) fn context_run(value: &Value) -> Option<usize> {
    int_at(&values_of(value, HostKind::CommandContext)?, 0)
}

/// The `at`th source of command run `run` (0: the one it runs as).
pub(crate) fn source(run: usize, at: usize) -> Value {
    handle(HostKind::CommandSource, &[index(run), index(at)])
}

/// A source handle's run and index.
pub(crate) fn source_of(value: &Value) -> Option<(usize, usize)> {
    let values = values_of(value, HostKind::CommandSource)?;
    Some((int_at(&values, 0)?, int_at(&values, 1)?))
}

/// Node `id` of the arena: a `kind` ([`LITERAL`], [`ARGUMENT`], [`ROOT`])
/// named `name`.
pub(crate) fn node(id: usize, kind: &str, name: &str) -> Value {
    handle(HostKind::CommandNode, &[index(id), text(kind), text(name)])
}

pub(crate) fn node_id(value: &Value) -> Option<usize> {
    int_at(&values_of(value, HostKind::CommandNode)?, 0)
}

/// Builder `id`, of a literal or an argument.
pub(crate) fn builder(id: usize, kind: &str) -> Value {
    handle(HostKind::ArgumentBuilder, &[index(id), text(kind)])
}

pub(crate) fn builder_id(value: &Value) -> Option<usize> {
    int_at(&values_of(value, HostKind::ArgumentBuilder)?, 0)
}

/// Argument type `id`, made by a factory of `class`.
pub(crate) fn argument_type(id: usize, class: &str) -> Value {
    handle(HostKind::ArgumentType, &[index(id), text(class)])
}

pub(crate) fn argument_type_id(value: &Value) -> Option<usize> {
    int_at(&values_of(value, HostKind::ArgumentType)?, 0)
}

/// A permission value: `what` it is ([`PERMISSION`], [`PERMISSION_CHECK`],
/// [`PERMISSION_SET`], [`REQUIRES`]) for operator `level`.
pub(crate) fn permission(what: &str, level: u8) -> Value {
    handle(
        HostKind::CommandPermission,
        &[text(what), Value::Int(i32::from(level))],
    )
}

/// What a permission value is and its level.
pub(crate) fn permission_of(value: &Value) -> Option<(String, u8)> {
    let values = values_of(value, HostKind::CommandPermission)?;
    let what = text_at(&values, 0)?.to_owned();
    let level = u8::try_from(int_at(&values, 1)?).ok()?;
    Some((what, level))
}

pub(crate) fn dispatcher_types(_key: &str) -> &'static [&'static str] {
    &[DISPATCHER]
}

pub(crate) fn build_context_types(_key: &str) -> &'static [&'static str] {
    &[
        "net/minecraft/commands/CommandBuildContext",
        "net/minecraft/core/HolderLookup$Provider",
    ]
}

pub(crate) fn context_types(_key: &str) -> &'static [&'static str] {
    &[CONTEXT]
}

pub(crate) fn source_types(_key: &str) -> &'static [&'static str] {
    &[
        "net/minecraft/commands/CommandSourceStack",
        "net/minecraft/commands/SharedSuggestionProvider",
        "net/minecraft/commands/ExecutionCommandSource",
        "net/minecraft/server/permissions/PermissionSetSupplier",
    ]
}

const COMMAND_NODE: &str = "com/mojang/brigadier/tree/CommandNode";
const LITERAL_NODE: &str = "com/mojang/brigadier/tree/LiteralCommandNode";
const ARGUMENT_NODE: &str = "com/mojang/brigadier/tree/ArgumentCommandNode";
const ROOT_NODE: &str = "com/mojang/brigadier/tree/RootCommandNode";
const LITERAL_BUILDER: &str = "com/mojang/brigadier/builder/LiteralArgumentBuilder";
const REQUIRED_BUILDER: &str = "com/mojang/brigadier/builder/RequiredArgumentBuilder";
const DISPATCHER: &str = "com/mojang/brigadier/CommandDispatcher";
const CONTEXT: &str = "com/mojang/brigadier/context/CommandContext";

pub(crate) fn node_types(key: &str) -> &'static [&'static str] {
    match node_parts(key).as_ref().map(|(kind, _)| kind.as_str()) {
        Some(LITERAL) => &[LITERAL_NODE, COMMAND_NODE, "java/lang/Comparable"],
        Some(ARGUMENT) => &[ARGUMENT_NODE, COMMAND_NODE, "java/lang/Comparable"],
        Some(ROOT) => &[ROOT_NODE, COMMAND_NODE, "java/lang/Comparable"],
        _ => &[COMMAND_NODE, "java/lang/Comparable"],
    }
}

pub(crate) fn builder_types(key: &str) -> &'static [&'static str] {
    let values = HostRef::new(HostKind::ArgumentBuilder, key).values();
    match values.as_deref().and_then(|values| text_at(values, 1)) {
        Some(LITERAL) => &[
            LITERAL_BUILDER,
            "com/mojang/brigadier/builder/ArgumentBuilder",
        ],
        _ => &[
            REQUIRED_BUILDER,
            "com/mojang/brigadier/builder/ArgumentBuilder",
        ],
    }
}

pub(crate) fn argument_type_types(_key: &str) -> &'static [&'static str] {
    &["com/mojang/brigadier/arguments/ArgumentType"]
}

pub(crate) fn permission_types(key: &str) -> &'static [&'static str] {
    let values = HostRef::new(HostKind::CommandPermission, key).values();
    match values.as_deref().and_then(|values| text_at(values, 0)) {
        Some(PERMISSION) => &["net/minecraft/server/permissions/Permission"],
        Some(PERMISSION_CHECK) => &["net/minecraft/server/permissions/PermissionCheck"],
        Some(PERMISSION_SET) => &["net/minecraft/server/permissions/PermissionSet"],
        _ => &["java/util/function/Predicate"],
    }
}

/// Brigadier's classes the command handles can be (`javap` of 1.3.11): every
/// one a class, none an interface, each extending `Object` or the one before
/// it in its handle's declared types.
const BRIGADIER_CLASSES: &[&str] = &[
    COMMAND_NODE,
    LITERAL_NODE,
    ARGUMENT_NODE,
    ROOT_NODE,
    "com/mojang/brigadier/builder/ArgumentBuilder",
    LITERAL_BUILDER,
    REQUIRED_BUILDER,
    DISPATCHER,
    CONTEXT,
];

/// `instanceof` on a handle the runtime builds as exactly the first of its
/// declared types, which list its whole superclass chain: against one of
/// [`BRIGADIER_CLASSES`] that is exact (a class is an instance of its chain
/// only, and a Mixin adds interfaces, never a superclass). Anything else as
/// the declared types decide.
fn exact_chain_instance_of(
    handle: &HostRef,
    wanted: &str,
    hierarchy: &dyn ClassHierarchy,
) -> Option<bool> {
    if BRIGADIER_CLASSES.contains(&wanted) {
        let declared = match handle.kind() {
            HostKind::CommandNode => node_types(handle.key()),
            HostKind::ArgumentBuilder => builder_types(handle.key()),
            HostKind::BrigadierDispatcher => dispatcher_types(handle.key()),
            _ => context_types(handle.key()),
        };
        return Some(declared.contains(&wanted));
    }
    instance_of_class(handle, wanted, hierarchy)
}

/// Brigadier's own argument types (`javap` of 1.3.11): classes that extend
/// `Object` and implement `ArgumentType`, so one is never another.
const BRIGADIER_ARGUMENT_TYPES: &[&str] = &[
    "com/mojang/brigadier/arguments/BoolArgumentType",
    "com/mojang/brigadier/arguments/DoubleArgumentType",
    "com/mojang/brigadier/arguments/FloatArgumentType",
    "com/mojang/brigadier/arguments/IntegerArgumentType",
    "com/mojang/brigadier/arguments/LongArgumentType",
    "com/mojang/brigadier/arguments/StringArgumentType",
];

/// An argument type is its factory's class too (`IntegerArgumentType`,
/// `EntityArgument`): the declared types plus that class.
fn argument_type_instance_of(
    handle: &HostRef,
    wanted: &str,
    hierarchy: &dyn ClassHierarchy,
) -> Option<bool> {
    let class = handle
        .values()
        .and_then(|values| text_at(&values, 1).map(str::to_owned));
    if class.as_deref() == Some(wanted) {
        return Some(true);
    }
    // The factory builds exactly its class: brigadier's are no other of them.
    if let Some(class) = class.as_deref()
        && BRIGADIER_ARGUMENT_TYPES.contains(&class)
        && BRIGADIER_ARGUMENT_TYPES.contains(&wanted)
    {
        return Some(false);
    }
    instance_of_class(handle, wanted, hierarchy)
}

/// A node's kind and name.
fn node_parts(key: &str) -> Option<(String, String)> {
    let values = HostRef::new(HostKind::CommandNode, key).values()?;
    Some((
        text_at(&values, 1)?.to_owned(),
        text_at(&values, 2)?.to_owned(),
    ))
}

/// `Object.equals`: the same object where identity can tell.
fn identity_equals(
    handle: &HostRef,
    other: &Value,
    _nested: &mut dyn Nested,
) -> Result<Option<bool>, NativeError> {
    Ok(match other {
        Value::Host(other) => handle.same_object(other),
        _ => Some(false),
    })
}

/// A class whose `equals` compares its state (`CommandNode`,
/// `CommandContext`, `IntegerArgumentType`, a permission record): the same
/// object is equal, another object of the kind is unknown here, and a value
/// that is no such object (a string, a box, another kind) is not equal. A jar
/// object is unknown: it could extend the class.
fn state_equals(
    handle: &HostRef,
    other: &Value,
    _nested: &mut dyn Nested,
) -> Result<Option<bool>, NativeError> {
    Ok(match other {
        Value::Host(other) if other.kind() == handle.kind() => {
            handle.same_object(other).filter(|same| *same)
        }
        Value::Ref(Some(text)) if !crate::value_class::is_plain_string(text) => None,
        _ => Some(false),
    })
}

/// `CommandNode.equals` (brigadier 1.3.11): `LiteralCommandNode`'s checks
/// `instanceof LiteralCommandNode`, then the literal; `ArgumentCommandNode`'s
/// `instanceof ArgumentCommandNode`, then the name (and the type);
/// `RootCommandNode`'s `instanceof RootCommandNode`; each then compares the
/// children and the command (`CommandNode.equals`). A node handle is exactly
/// the class its kind names (the runtime builds it), so another kind, or
/// another literal or name, is not equal; the same kind and name in another
/// node needs the children compared: unknown here.
fn node_equals(
    handle: &HostRef,
    other: &Value,
    nested: &mut dyn Nested,
) -> Result<Option<bool>, NativeError> {
    let Value::Host(other_handle) = other else {
        return state_equals(handle, other, nested);
    };
    if other_handle.kind() != HostKind::CommandNode
        || handle.same_object(other_handle) == Some(true)
    {
        return state_equals(handle, other, nested);
    }
    Ok(
        match (node_parts(handle.key()), node_parts(other_handle.key())) {
            (Some(mine), Some(theirs)) if mine != theirs => Some(false),
            _ => None,
        },
    )
}

/// `String.compareTo`: the first differing UTF-16 unit's difference, else the
/// lengths'.
fn string_compare(a: &str, b: &str) -> i32 {
    let mut left = a.encode_utf16();
    let mut right = b.encode_utf16();
    loop {
        match (left.next(), right.next()) {
            (Some(x), Some(y)) if x != y => return i32::from(x) - i32::from(y),
            (Some(_), Some(_)) => {}
            _ => {
                let length = |text: &str| text.encode_utf16().count() as i32;
                return length(a) - length(b);
            }
        }
    }
}

/// `CommandNode.compareTo`: literals before other nodes, then the sorted
/// key (a literal's text, an argument's name, the root's `""`).
fn node_compare(
    handle: &HostRef,
    other: &Value,
    _nested: &mut dyn Nested,
) -> Result<Option<i32>, NativeError> {
    let Value::Host(other) = other else {
        return Ok(None);
    };
    if other.kind() != HostKind::CommandNode {
        return Ok(None);
    }
    let (Some((kind, key)), Some((other_kind, other_key))) =
        (node_parts(handle.key()), node_parts(other.key()))
    else {
        return Ok(None);
    };
    let (literal, other_literal) = (kind == LITERAL, other_kind == LITERAL);
    Ok(Some(if literal == other_literal {
        string_compare(&key, &other_key)
    } else if other_literal {
        1
    } else {
        -1
    }))
}

/// `LiteralCommandNode.toString` `<literal NAME>`, `RootCommandNode`'s
/// `<root>`; an argument's prints its type, unknown here.
fn node_text(handle: &HostRef, _nested: &mut dyn Nested) -> Result<Option<String>, NativeError> {
    Ok(match node_parts(handle.key()) {
        Some((kind, name)) if kind == LITERAL => Some(format!("<literal {name}>")),
        Some((kind, _)) if kind == ROOT => Some("<root>".to_owned()),
        _ => None,
    })
}

/// Identity `toString`/`hashCode` (counted), identity `equals`.
pub(crate) const IDENTITY_METHODS: Methods = Methods {
    to_string: |_, _| Ok(None),
    hash_code: |_, _| Ok(None),
    equals: identity_equals,
    instance_of: instance_of_class,
    crosses_jars: |_, _| false,
    compare_to: None,
};

/// A class whose `equals` compares its state; its `hashCode` (counted).
pub(crate) const STATE_METHODS: Methods = Methods {
    equals: state_equals,
    ..IDENTITY_METHODS
};

pub(crate) const NODE_METHODS: Methods = Methods {
    to_string: node_text,
    equals: node_equals,
    instance_of: exact_chain_instance_of,
    compare_to: Some(node_compare),
    ..STATE_METHODS
};

/// A builder: identity, and its exact class among the builders.
pub(crate) const BUILDER_METHODS: Methods = Methods {
    instance_of: exact_chain_instance_of,
    ..IDENTITY_METHODS
};

/// The dispatcher: identity, and its exact class among brigadier's.
pub(crate) const DISPATCHER_METHODS: Methods = BUILDER_METHODS;

/// A context: `CommandContext.equals` compares its state; its exact class
/// among brigadier's.
pub(crate) const CONTEXT_METHODS: Methods = Methods {
    instance_of: exact_chain_instance_of,
    ..STATE_METHODS
};

/// An argument type's `equals`: brigadier 1.3.11's `IntegerArgumentType`,
/// `LongArgumentType`, `FloatArgumentType` and `DoubleArgumentType` compare
/// their bounds (another one is counted); `StringArgumentType` and
/// `BoolArgumentType` keep `Object`'s identity (`javap`); any other factory's
/// class is treated as comparing its state.
fn argument_type_equals(
    handle: &HostRef,
    other: &Value,
    nested: &mut dyn Nested,
) -> Result<Option<bool>, NativeError> {
    let class = handle
        .values()
        .and_then(|values| text_at(&values, 1).map(str::to_owned));
    match class.as_deref() {
        Some(
            "com/mojang/brigadier/arguments/StringArgumentType"
            | "com/mojang/brigadier/arguments/BoolArgumentType",
        ) => identity_equals(handle, other, nested),
        _ => state_equals(handle, other, nested),
    }
}

pub(crate) const ARGUMENT_TYPE_METHODS: Methods = Methods {
    equals: argument_type_equals,
    instance_of: argument_type_instance_of,
    ..STATE_METHODS
};

#[cfg(test)]
mod tests {
    use super::*;

    /// A node's name and a factory's class reach a key only through the
    /// escaping encoder: names with every separator read back whole and never
    /// alias another node.
    #[test]
    fn names_with_every_separator_never_alias() {
        let names = [
            "a", "a,b", "a\\", ",", "\\,", "hcnode:r", "i1", "", "#", "a b",
        ];
        let mut keys = std::collections::BTreeSet::new();
        for (id, name) in names.iter().enumerate() {
            for kind in [LITERAL, ARGUMENT] {
                let value = node(id, kind, name);
                let Value::Host(handle) = &value else {
                    unreachable!()
                };
                assert!(keys.insert(handle.key().to_owned()), "{name:?}");
                assert_eq!(node_id(&value), Some(id));
                assert_eq!(
                    node_parts(handle.key()),
                    Some((kind.to_owned(), (*name).to_owned()))
                );
            }
        }
        assert_eq!(node_id(&Value::Ref(Some("brigadier/node/0".into()))), None);
    }

    /// `CommandNode.compareTo` (brigadier 1.3.11): literals first, then the
    /// sorted key by `String.compareTo`.
    #[test]
    fn nodes_compare_as_brigadier_does() {
        let compare = |a: &Value, b: &Value| {
            let Value::Host(a) = a else { unreachable!() };
            a.compare_to(b, &mut crate::MarkerText).unwrap()
        };
        let apple = node(0, LITERAL, "apple");
        let apricot = node(1, LITERAL, "apricot");
        let player = node(2, ARGUMENT, "player");
        let root = node(3, ROOT, "");
        assert_eq!(compare(&apple, &apricot), Some(-2));
        assert_eq!(compare(&apple, &player), Some(-1));
        assert_eq!(compare(&player, &apple), Some(1));
        assert_eq!(compare(&root, &player), Some(-6));
        assert_eq!(compare(&apple, &Value::Ref(Some("x".into()))), None);
    }
}

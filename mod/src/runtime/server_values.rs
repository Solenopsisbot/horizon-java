//! The server's value objects and registry entries as host handles (#156
//! P16c): `BlockPos`, `LevelResource`, and the text of `GameRule`, `Item`,
//! `Block` and `BlockState`. Each key holds every field Java's `equals`,
//! `hashCode`, `toString` and `compareTo` read, so those answer exactly
//! (vanilla 26.3 on Java 25, `tests/fixture_server_values.rs`); what a key
//! lacks is counted.

use crate::runtime::Value;

pub(crate) const BLOCK_POS: &str = "block-pos/";
pub(crate) const LEVEL_RESOURCE: &str = "level-resource/";
pub(crate) const GAME_RULE: &str = "game-rule/";
const ITEM: &str = "game/item/";
const BLOCK: &str = "game/block/";
const BLOCK_STATE: &str = "game/block-state/";

/// The coordinates of a `block-pos/x,y,z` key.
pub(crate) fn block_pos(key: &str) -> Option<[i32; 3]> {
    let mut parts = key.strip_prefix(BLOCK_POS)?.split(',').map(str::parse);
    let pos = [
        parts.next()?.ok()?,
        parts.next()?.ok()?,
        parts.next()?.ok()?,
    ];
    parts.next().is_none().then_some(pos)
}

/// `toString()` of a server value or registry key; `None` when it is not
/// one, or needs a fact the key lacks (counted).
pub(crate) fn to_string(key: &str) -> Option<String> {
    // `Vec3i.toString`: Guava's `toStringHelper` names the class's simple name.
    if let Some([x, y, z]) = block_pos(key) {
        return Some(format!("BlockPos{{x={x}, y={y}, z={z}}}"));
    }
    if let Some(id) = key.strip_prefix(LEVEL_RESOURCE) {
        return Some(format!("/{id}"));
    }
    // `GameRule.toString` is its id's short form: a vanilla rule's id (the
    // only rules a key names) is in `minecraft`.
    if let Some(id) = key.strip_prefix(GAME_RULE) {
        return Some(id.to_owned());
    }
    // `Item.toString` is its registered name; `Block`'s is `Block{<name>}`.
    if let Some(id) = key.strip_prefix(ITEM) {
        return Some(id.to_owned());
    }
    if let Some(id) = key.strip_prefix(BLOCK) {
        return Some(format!("Block{{{id}}}"));
    }
    // `StateHolder.toString` appends the state's properties when the block
    // has any: a key names them only in `[...]`, else they are unknown.
    let state = key.strip_prefix(BLOCK_STATE)?;
    let (block, properties) = state.split_once('[')?;
    Some(format!("Block{{{block}}}[{properties}"))
}

/// `hashCode()` of a server value key: `Vec3i`'s
/// `(y + z * 31) * 31 + x`, and a record of one `String` (`LevelResource`)
/// hashes as that string. Registry entries hash by identity (counted).
pub(crate) fn hash_code(key: &str) -> Option<i32> {
    if let Some([x, y, z]) = block_pos(key) {
        return Some(
            y.wrapping_add(z.wrapping_mul(31))
                .wrapping_mul(31)
                .wrapping_add(x),
        );
    }
    Some(crate::runtime::hash_map_order::java_string_hash(
        key.strip_prefix(LEVEL_RESOURCE)?,
    ))
}

/// The class a jar's own object of the family has (`new BlockPos(..)`, `new
/// LevelResource(id)`), which `equals` cannot see into.
fn family_classes(key: &str) -> Option<&'static [&'static str]> {
    if key.starts_with(BLOCK_POS) {
        return Some(&[
            "net/minecraft/core/BlockPos",
            "net/minecraft/core/BlockPos$MutableBlockPos",
            "net/minecraft/core/Vec3i",
        ]);
    }
    key.starts_with(LEVEL_RESOURCE)
        .then_some(&["net/minecraft/world/level/storage/LevelResource"][..])
}

/// `equals(other)` of a server value key against another handle's key
/// (`other_key`, `None` for a value that is no game handle): `Vec3i.equals`
/// by coordinates, a record by its id. `None` when the key is no server
/// value; `Some(None)` when Java's answer needs fields the model lacks.
pub(crate) fn equals(key: &str, other_key: Option<&str>, other: &Value) -> Option<Option<bool>> {
    let classes = family_classes(key)?;
    match other_key.filter(|other| family_classes(other) == Some(classes)) {
        Some(other_key) => Some(Some(key == other_key)),
        // A jar's own `BlockPos`/`LevelResource`: its fields are not ours.
        None => {
            let foreign = crate::runtime::jar_object::object_class(other)
                .is_some_and(|class| classes.contains(&class));
            Some((!foreign).then_some(false))
        }
    }
}

/// `Vec3i.compareTo`: by `y`, then `z`, then `x`, as differences (`int`
/// arithmetic). `None` for another value (counted).
pub(crate) fn compare_to(key: &str, other: &Value) -> Option<i32> {
    let [x, y, z] = block_pos(key)?;
    let [ox, oy, oz] = match other {
        Value::Host(other) => block_pos(other.key())?,
        _ => return None,
    };
    Some(if y != oy {
        y.wrapping_sub(oy)
    } else if z != oz {
        z.wrapping_sub(oz)
    } else {
        x.wrapping_sub(ox)
    })
}

/// Whether two handles of one server value key are one object: a
/// `LevelResource` key names one of its constants (a jar's own is a jar
/// object); a `BlockPos` is a new object per producer.
pub(crate) fn canonical(key: &str) -> bool {
    key.starts_with(LEVEL_RESOURCE)
}

/// Whether two handles of one registry key are one object: every entry is,
/// but a block state whose key names no properties (`game/block-state/<id>`)
/// may be any of its block's states.
pub(crate) fn registry_canonical(key: &str) -> bool {
    key.strip_prefix(BLOCK_STATE)
        .is_none_or(|state| state.contains('['))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `Vec3i` against vanilla 26.3 on Java 25 (`fixture_server_values`'s
    /// oracle): zero, negative and extreme coordinates.
    #[test]
    fn server_values_hash_and_print_as_java() {
        for (key, text, hash) in [
            ("block-pos/1,2,3", "BlockPos{x=1, y=2, z=3}", 2946),
            ("block-pos/0,0,0", "BlockPos{x=0, y=0, z=0}", 0),
            ("block-pos/-1,-2,-3", "BlockPos{x=-1, y=-2, z=-3}", -2946),
            (
                "block-pos/30000000,319,-30000000",
                "BlockPos{x=30000000, y=319, z=-30000000}",
                1_264_780_961,
            ),
            (
                "block-pos/-33554432,-2048,33554431",
                "BlockPos{x=-33554432, y=-2048, z=33554431}",
                2_147_419_199,
            ),
        ] {
            assert_eq!(to_string(key).as_deref(), Some(text));
            assert_eq!(hash_code(key), Some(hash));
        }
        // `item_text`, `block_text`, `state_log_text`, `rule_text`, `res_text`;
        // `state_stone_text` needs the block's property set (counted).
        assert_eq!(
            to_string("game/item/minecraft:stone").as_deref(),
            Some("minecraft:stone")
        );
        assert_eq!(
            to_string("game/block/minecraft:stone").as_deref(),
            Some("Block{minecraft:stone}")
        );
        assert_eq!(
            to_string("game/block-state/minecraft:oak_log[axis=y]").as_deref(),
            Some("Block{minecraft:oak_log}[axis=y]")
        );
        assert_eq!(to_string("game/block-state/minecraft:stone"), None);
        assert_eq!(
            to_string("game-rule/keep_inventory").as_deref(),
            Some("keep_inventory")
        );
        assert_eq!(to_string("level-resource/data").as_deref(), Some("/data"));
        assert_eq!(hash_code("level-resource/data"), Some(3_076_010));
        // `pos_compare`: `-1,-1,1,0`.
        let pos = |key: &str| crate::runtime::game_object::value(key).unwrap();
        assert_eq!(
            compare_to("block-pos/1,2,3", &pos("block-pos/1,2,4")),
            Some(-1)
        );
        assert_eq!(
            compare_to("block-pos/5,1,0", &pos("block-pos/0,2,0")),
            Some(-1)
        );
        assert_eq!(
            compare_to("block-pos/2,2,2", &pos("block-pos/1,2,2")),
            Some(1)
        );
        assert_eq!(
            compare_to("block-pos/1,2,3", &pos("block-pos/1,2,3")),
            Some(0)
        );
    }
}

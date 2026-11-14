//! The client's value objects as host handles (#156 P16d): `Style`,
//! `TextColor`, `KeyMapping.Category`, `Vec3` and `java.awt.Color`. Their key
//! holds every field Java's `equals`, `hashCode` and `toString` read, so
//! those answer exactly (vanilla 26.3 and Java 25, `tests/fixture_client_values.rs`).
//!
//! A style's key is its fields in one fixed order, never the calls that made
//! it: `client/style/` then the colour (`§6`, or `§#rrggbb` for an RGB one),
//! then each flag set (`§l` bold, `§o` italic, `§n` underlined, `§m`
//! strikethrough, `§k` obfuscated; `§!l` for an explicit `false`), then `§?`
//! when the style holds a field the model does not keep (a click or hover
//! event, an insertion, a font, a shadow colour). Such a style's `equals`,
//! `hashCode` and `toString` are counted. The empty key is `Style.EMPTY`,
//! which every style without fields is (`Style.checkEmptyAfterChange`).

use crate::Value;
use crate::hash_map_order::{identifier_hash, java_string_hash};

pub(crate) const STYLE: &str = "client/style/";
pub(crate) const TEXT_COLOR: &str = "client/text-color/";
pub(crate) const CATEGORY: &str = "key-category/";
pub(crate) const VEC3: &str = "client/vec3/";
pub(crate) const AWT_COLOR: &str = "client/awt-color/";

/// `ChatFormatting`'s colours: code, name (`TextColor`'s), RGB.
const COLOURS: &[(char, &str, u32)] = &[
    ('0', "black", 0x000000),
    ('1', "dark_blue", 0x0000aa),
    ('2', "dark_green", 0x00aa00),
    ('3', "dark_aqua", 0x00aaaa),
    ('4', "dark_red", 0xaa0000),
    ('5', "dark_purple", 0xaa00aa),
    ('6', "gold", 0xffaa00),
    ('7', "gray", 0xaaaaaa),
    ('8', "dark_gray", 0x555555),
    ('9', "blue", 0x5555ff),
    ('a', "green", 0x55ff55),
    ('b', "aqua", 0x55ffff),
    ('c', "red", 0xff5555),
    ('d', "light_purple", 0xff55ff),
    ('e', "yellow", 0xffff55),
    ('f', "white", 0xffffff),
];

/// The flags in `Style`'s field order: code and `toString` name.
pub(crate) const FLAGS: [(char, &str); 5] = [
    ('l', "bold"),
    ('o', "italic"),
    ('n', "underlined"),
    ('m', "strikethrough"),
    ('k', "obfuscated"),
];

/// A `TextColor`: one of `ChatFormatting`'s (a named constant) or an RGB one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Colour {
    Legacy(char),
    Rgb(u32),
}

impl Colour {
    /// The colour a code names: `§6` or `§#rrggbb`.
    pub(crate) fn parse(code: &str) -> Option<Self> {
        let code = code.strip_prefix('§')?;
        if let Some(hex) = code.strip_prefix('#') {
            return (hex.len() == 6)
                .then(|| u32::from_str_radix(hex, 16).ok().map(Self::Rgb))
                .flatten();
        }
        let mut chars = code.chars();
        let c = chars.next()?;
        (chars.next().is_none() && COLOURS.iter().any(|(k, ..)| *k == c)).then_some(Self::Legacy(c))
    }

    pub(crate) fn code(self) -> String {
        match self {
            Self::Legacy(c) => format!("§{c}"),
            Self::Rgb(rgb) => format!("§#{rgb:06x}"),
        }
    }

    fn name(self) -> Option<&'static str> {
        match self {
            Self::Legacy(c) => COLOURS.iter().find(|(k, ..)| *k == c).map(|(_, n, _)| *n),
            Self::Rgb(_) => None,
        }
    }

    /// `TextColor.getValue()`.
    pub(crate) fn value(self) -> u32 {
        match self {
            Self::Legacy(c) => COLOURS
                .iter()
                .find(|(k, ..)| *k == c)
                .map_or(0, |(.., v)| *v),
            Self::Rgb(rgb) => rgb,
        }
    }

    /// The colour a component's JSON `color` names: `gold` or `#RRGGBB`.
    pub(crate) fn from_text(text: &str) -> Option<Self> {
        if let Some(hex) = text.strip_prefix('#') {
            return Self::parse(&format!("§#{}", hex.to_ascii_lowercase()));
        }
        COLOURS
            .iter()
            .find(|(_, name, _)| *name == text)
            .map(|(code, ..)| Self::Legacy(*code))
    }

    /// `TextColor.toString()` (its `serialize()`, the component JSON's
    /// `color`): its name, else `#RRGGBB`.
    pub(crate) fn text(self) -> String {
        self.name()
            .map_or_else(|| format!("#{:06X}", self.value()), str::to_owned)
    }

    /// `TextColor.hashCode()`: `Objects.hash(value, name)`.
    fn hash(self) -> i32 {
        let name = self.name().map_or(0, java_string_hash);
        31i32
            .wrapping_add(self.value() as i32)
            .wrapping_mul(31)
            .wrapping_add(name)
    }
}

/// A `Style`'s fields as its key spells them.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Style {
    pub(crate) colour: Option<Colour>,
    /// [`FLAGS`]' values: `None` unset, else the `Boolean`.
    pub(crate) flags: [Option<bool>; 5],
    /// A field the model does not keep.
    pub(crate) extra: bool,
}

impl Style {
    /// The fields of a style key's codes (after [`STYLE`]).
    pub(crate) fn parse(codes: &str) -> Option<Self> {
        let mut style = Self::default();
        for code in codes.split('§').skip(1) {
            match code {
                "?" => style.extra = true,
                _ if code.starts_with('#')
                    || code.len() == 1 && Colour::parse(&format!("§{code}")).is_some() =>
                {
                    style.colour = Some(Colour::parse(&format!("§{code}"))?);
                }
                _ => {
                    let (value, flag) = match code.strip_prefix('!') {
                        Some(flag) => (false, flag),
                        None => (true, code),
                    };
                    let at = FLAGS
                        .iter()
                        .position(|(c, _)| flag.len() == 1 && flag.starts_with(*c))?;
                    style.flags[at] = Some(value);
                }
            }
        }
        Some(style)
    }

    /// The canonical codes of these fields.
    pub(crate) fn codes(&self) -> String {
        let mut codes = self.colour.map(Colour::code).unwrap_or_default();
        for ((code, _), value) in FLAGS.iter().zip(self.flags) {
            match value {
                Some(true) => codes.push_str(&format!("§{code}")),
                Some(false) => codes.push_str(&format!("§!{code}")),
                None => {}
            }
        }
        if self.extra {
            codes.push_str("§?");
        }
        codes
    }

    /// `applyFormat`/`applyLegacyFormat` with the `ChatFormatting` of `code`
    /// (`l`, `6`, `r`...): a flag sets it; a colour sets it, and the legacy
    /// form also turns every flag off; `RESET` is `Style.EMPTY`.
    pub(crate) fn apply(mut self, code: char, legacy: bool) -> Option<Self> {
        if code == 'r' {
            return Some(Self::default());
        }
        if let Some(at) = FLAGS.iter().position(|(c, _)| *c == code) {
            self.flags[at] = Some(true);
            return Some(self);
        }
        self.colour = Some(Colour::parse(&format!("§{code}"))?);
        if legacy {
            self.flags = [Some(false); 5];
        }
        Some(self)
    }

    fn text(&self) -> Option<String> {
        if self.extra {
            return None;
        }
        let mut parts: Vec<String> = self
            .colour
            .map(|c| format!("color={}", c.text()))
            .into_iter()
            .collect();
        for ((_, name), value) in FLAGS.iter().zip(self.flags) {
            match value {
                Some(true) => parts.push((*name).to_owned()),
                Some(false) => parts.push(format!("!{name}")),
                None => {}
            }
        }
        Some(format!("{{{}}}", parts.join(",")))
    }

    /// `Objects.hash(color, shadowColor, bold, italic, underlined,
    /// strikethrough, obfuscated, clickEvent, hoverEvent, insertion)`.
    fn hash(&self) -> Option<i32> {
        if self.extra {
            return None;
        }
        let boolean = |value: Option<bool>| value.map_or(0, |v| if v { 1231 } else { 1237 });
        let fields = [self.colour.map_or(0, Colour::hash), 0]
            .into_iter()
            .chain(self.flags.map(boolean))
            .chain([0, 0, 0]);
        Some(fields.fold(1i32, |hash, field| {
            hash.wrapping_mul(31).wrapping_add(field)
        }))
    }

    /// `Style.equals`: the colours by `TextColor.equals` (the value), the
    /// flags by `Boolean`.
    fn same(&self, other: &Self) -> Option<bool> {
        if self.extra || other.extra {
            return None;
        }
        Some(
            self.colour.map(Colour::value) == other.colour.map(Colour::value)
                && self.flags == other.flags,
        )
    }
}

/// The `Vec3` of a key's `x,y,z`.
pub(crate) fn vec3(coords: &str) -> Option<[f64; 3]> {
    let parts: Vec<f64> = coords
        .split(',')
        .map(str::parse)
        .collect::<Result<_, _>>()
        .ok()?;
    parts.try_into().ok()
}

/// `Double.doubleToLongBits`.
fn double_bits(value: f64) -> u64 {
    if value.is_nan() {
        0x7ff8_0000_0000_0000
    } else {
        value.to_bits()
    }
}

/// `toString()` of a client value key; `None` when it is not one, or needs a
/// field the key lacks (counted).
pub(crate) fn to_string(key: &str) -> Option<String> {
    if let Some(codes) = key.strip_prefix(STYLE) {
        return Style::parse(codes)?.text();
    }
    if let Some(code) = key.strip_prefix(TEXT_COLOR) {
        return Some(Colour::parse(code)?.text());
    }
    if let Some(id) = key.strip_prefix(CATEGORY) {
        return Some(format!("Category[id={id}]"));
    }
    if let Some(coords) = key.strip_prefix(VEC3) {
        let [x, y, z] = vec3(coords)?.map(crate::jdk_float_text::double_to_string);
        return Some(format!("({x}, {y}, {z})"));
    }
    let argb = u32::from_str_radix(key.strip_prefix(AWT_COLOR)?, 16).ok()?;
    let [_, r, g, b] = argb.to_be_bytes();
    Some(format!("java.awt.Color[r={r},g={g},b={b}]"))
}

/// `hashCode()` of a client value key.
pub(crate) fn hash_code(key: &str) -> Option<i32> {
    if let Some(codes) = key.strip_prefix(STYLE) {
        return Style::parse(codes)?.hash();
    }
    if let Some(code) = key.strip_prefix(TEXT_COLOR) {
        return Some(Colour::parse(code)?.hash());
    }
    // A record of one `Identifier`: the identifier's hash.
    if let Some(id) = key.strip_prefix(CATEGORY) {
        return Some(identifier_hash(id));
    }
    if let Some(coords) = key.strip_prefix(VEC3) {
        let fold = |hash: i32, value: f64| {
            let bits = double_bits(value);
            hash.wrapping_mul(31)
                .wrapping_add((bits ^ (bits >> 32)) as i32)
        };
        let [x, y, z] = vec3(coords)?;
        let bits = double_bits(x);
        return Some(fold(fold((bits ^ (bits >> 32)) as i32, y), z));
    }
    Some(u32::from_str_radix(key.strip_prefix(AWT_COLOR)?, 16).ok()? as i32)
}

/// The class of a client value family, for a jar's own object of it (`new
/// Vec3(..)`, `new KeyMapping.Category(id)`), which `equals` cannot see into.
fn family_class(key: &str) -> Option<&'static str> {
    [
        (STYLE, "net/minecraft/network/chat/Style"),
        (TEXT_COLOR, "net/minecraft/network/chat/TextColor"),
        (CATEGORY, "net/minecraft/client/KeyMapping$Category"),
        (VEC3, "net/minecraft/world/phys/Vec3"),
        (AWT_COLOR, "java/awt/Color"),
    ]
    .into_iter()
    .find(|(prefix, _)| key.starts_with(prefix))
    .map(|(_, class)| class)
}

/// `equals(other)` of a client value key against another value's key
/// (`other_key`: `None` for a value that is no game handle). `None` when the
/// key is no client value, or Java's answer needs a field the model lacks.
pub(crate) fn equals(key: &str, other_key: Option<&str>, other: &Value) -> Option<Option<bool>> {
    let class = family_class(key)?;
    let Some(other_key) = other_key.filter(|other| family_class(other) == Some(class)) else {
        // A jar's own object of the family's class: its fields are not ours.
        let foreign = crate::jar_object::object_class(other) == Some(class);
        return Some((!foreign).then_some(false));
    };
    Some(
        match (key.strip_prefix(STYLE), other_key.strip_prefix(STYLE)) {
            (Some(a), Some(b)) => Style::parse(a)
                .zip(Style::parse(b))
                .and_then(|(a, b)| a.same(&b)),
            _ => match (
                key.strip_prefix(TEXT_COLOR),
                other_key.strip_prefix(TEXT_COLOR),
            ) {
                (Some(a), Some(b)) => Colour::parse(a)
                    .zip(Colour::parse(b))
                    .map(|(a, b)| a.value() == b.value()),
                // `Double.compare` per coordinate is `doubleToLongBits` equality.
                _ => match (key.strip_prefix(VEC3), other_key.strip_prefix(VEC3)) {
                    (Some(a), Some(b)) => vec3(a)
                        .zip(vec3(b))
                        .map(|(a, b)| a.map(double_bits) == b.map(double_bits)),
                    // A category by its id, an AWT colour by its ARGB: the key.
                    _ => Some(key == other_key),
                },
            },
        },
    )
}

/// Whether two handles of one client value key are one object: the
/// constants (`Style.EMPTY`, `TextColor`'s named colours, the registered
/// categories, `java.awt.Color`'s constants) are; a built style, an RGB
/// colour or a position is a new object each time.
pub(crate) fn canonical(key: &str) -> bool {
    key == STYLE
        || key.starts_with(CATEGORY)
        || key.starts_with(AWT_COLOR)
        || key
            .strip_prefix(TEXT_COLOR)
            .and_then(Colour::parse)
            .is_some_and(|colour| matches!(colour, Colour::Legacy(_)))
}

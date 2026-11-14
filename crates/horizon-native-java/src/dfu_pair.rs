//! DataFixerUpper's `com.mojang.datafixers.util.Pair`, which the game ships.
//!
//! Config trees pair a sort key with a reflected field (`Pair.of(ordinal,
//! field)`, then `pair.getSecond()`), so the pair must hand back exactly the
//! values it was built from. A pair is a host handle holding both values
//! (`host_codec.rs`); `equals`/`hashCode`/`toString` are its kind's row.

use super::*;

const OWNER: &str = "com/mojang/datafixers/util/Pair";

/// `Pair.of(first, second)`; `None` for any other static call.
pub(crate) fn static_call(r: &MethodRef, args: &[Value]) -> Option<Result<Value, NativeError>> {
    if r.owner != OWNER
        || r.name != "of"
        || r.descriptor != "(Ljava/lang/Object;Ljava/lang/Object;)Lcom/mojang/datafixers/util/Pair;"
    {
        return None;
    }
    let [first, second] = args else {
        return None;
    };
    Some(Ok(crate::host_codec::pair(first, second)))
}

/// The accessors of a pair the host built; `None` for anything else, so an
/// unmodelled member keeps the neutral answer (`equals`, `hashCode` and
/// `toString` are the kind's row, `host_codec.rs`).
pub(crate) fn instance_call(
    r: &MethodRef,
    receiver: &Value,
    _args: &[Value],
) -> Option<Result<Value, NativeError>> {
    if r.owner != OWNER {
        return None;
    }
    let (first, second) = crate::host_codec::pair_parts(receiver)?;
    Some(Ok(match (r.name.as_str(), r.descriptor.as_str()) {
        ("getFirst", "()Ljava/lang/Object;") => first,
        ("getSecond", "()Ljava/lang/Object;") => second,
        ("swap", "()Lcom/mojang/datafixers/util/Pair;") => crate::host_codec::pair(&second, &first),
        _ => return None,
    }))
}

//! `String.valueOf(Object)` of a model object: its modelled `toString`.
//!
//! javac (19 and later) lowers every non-`String` concat operand to
//! `String.valueOf(operand)`; older javac passes the object to the concat
//! site; `StringBuilder.append(Object)` is `String.valueOf` too. All three
//! printed the internal marker (`uuid/…`, `list/a`). [`owner`] names
//! the class whose modelled `toString` the host answers for a marker, so
//! `"" + x` is the text `x.toString()` gives. A plain string, a box, a builder
//! and a jar object keep their callers' existing paths.

use crate::runtime::{Host, MethodRef, Runtime, Value, expect_ref};

/// The owner whose `toString` answers `value` (a marker, or an object by
/// its key), or `None` when no model owns its text.
pub(crate) fn owner(value: &Value) -> Option<String> {
    let marker = expect_ref(value)?.as_str();
    // A plain string is its own text.
    if matches!(value, Value::Ref(_)) && crate::runtime::value_class::is_plain_string(marker) {
        return None;
    }
    // The optionals are host handles (`host_ref.rs`), printed through
    // [`modelled`].
    // A host map entry, when the text is one (a string that merely starts
    // like one stays a string).
    if marker.starts_with("map-entry/")
        && crate::runtime::map_entry_parts(&Value::Ref(Some(marker.to_owned()))).is_ok()
    {
        return Some("java/util/Map$Entry".to_owned());
    }
    // Prefix families the host models before any class marker.
    let prefix = [
        ("uuid/", "java/util/UUID"),
        ("duration/", "java/time/Duration"),
        ("regex-pattern/", "java/util/regex/Pattern"),
        ("charset/", "java/nio/charset/Charset"),
        ("coding-error-action/", "java/nio/charset/CodingErrorAction"),
        ("locale/", "java/util/Locale"),
        ("unicode-block/", "java/lang/Character$UnicodeBlock"),
        // `key + "=" + value` (`Map.entry`, an entry from `entrySet()`).
        ("map-entry/", "java/util/Map$Entry"),
    ];
    for (prefix, owner) in prefix {
        if marker.starts_with(prefix) {
            return Some(owner.to_owned());
        }
    }
    // A platform enum constant; a jar enum may override `toString`.
    if let Some((class, _)) = marker
        .strip_prefix("enum/")
        .and_then(|rest| rest.rsplit_once('/'))
        && crate::runtime::host_models_class(class)
    {
        return Some("java/lang/Enum".to_owned());
    }
    // A collection marker: `list/a,b`, `set/...`, `map/k,v`, immutable forms
    // `list#i1/...`, and the bare empty kinds.
    let kind = marker.split_once('/').map_or(marker, |(kind, _)| kind);
    let kind = kind.split_once('#').map_or(kind, |(kind, _)| kind);
    match kind {
        "list" => return Some("java/util/List".to_owned()),
        "set" => return Some("java/util/Set".to_owned()),
        "map" => return Some("java/util/Map".to_owned()),
        _ => {}
    }
    // A class marker (`class#n`): only a class the host models has a body to
    // answer, so a jar object keeps the caller's text. A builder's text lives
    // in the runtime ([`Runtime::invoke_value_of_builder`]), not the host.
    if let Some(class) = crate::runtime::jar_object::value_marker_class(value)
        && crate::runtime::host_models_class(class)
        && !matches!(class, "java/lang/StringBuilder" | "java/lang/StringBuffer")
    {
        // An unmodifiable collection prints through its interface's model.
        let interface = crate::runtime::java_immutable::supertypes(class).first();
        return Some(interface.copied().unwrap_or(class).to_owned());
    }
    None
}

/// The text `String.valueOf(value)` gives for a model object.
pub(crate) fn modelled<H: Host>(host: &mut H, value: &Value) -> Option<String> {
    // A host object (an `Optional`) prints through its class's model; an
    // allocated object through its class's, below.
    if let Value::Host(handle) = value
        && !crate::runtime::jar_object::is_object_handle(handle)
    {
        let to_string = MethodRef {
            // A handle whose class is not known still has `Object.toString`.
            owner: handle.class().unwrap_or("java/lang/Object").to_owned(),
            name: "toString".into(),
            descriptor: "()Ljava/lang/String;".into(),
        };
        return match host.invoke_instance(&to_string, value, &[]) {
            Ok(Value::Ref(Some(text))) => Some(text),
            _ => None,
        };
    }
    modelled_marker(host, value)
}

/// [`modelled`] for a marker or an allocated object.
pub(crate) fn modelled_marker<H: Host>(host: &mut H, value: &Value) -> Option<String> {
    let owner = owner(value)?;
    let marker = expect_ref(value)?;
    let to_string = MethodRef {
        owner,
        name: "toString".into(),
        descriptor: "()Ljava/lang/String;".into(),
    };
    match host.invoke_instance(&to_string, value, &[]) {
        Ok(Value::Ref(Some(text))) if text != *marker => Some(text),
        _ => None,
    }
}

impl<H: Host> Runtime<'_, H> {
    /// `String.valueOf(Object)`/`Objects.toString(Object)` of a builder this
    /// runtime holds: its current text.
    ///
    /// javac compiles `"" + builder` to `String.valueOf(builder)` before the
    /// concat site, and the builder's text is runtime state the host cannot
    /// see, so the host would print its marker.
    pub(crate) fn invoke_value_of_builder(&self, r: &MethodRef, args: &[Value]) -> Option<Value> {
        if !matches!(
            (r.owner.as_str(), r.name.as_str()),
            ("java/lang/String", "valueOf") | ("java/util/Objects", "toString")
        ) || r.descriptor != "(Ljava/lang/Object;)Ljava/lang/String;"
        {
            return None;
        }
        let text = self.builder_text(args.first()?)?;
        Some(Value::Ref(Some(text.clone())))
    }
}

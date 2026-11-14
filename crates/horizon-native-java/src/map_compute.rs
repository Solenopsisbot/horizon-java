//! `Map.computeIfAbsent` with a jar callback, and `Map.ofEntries`.
//!
//! `Map.ofEntries(Map.entry(k, v), ...)` takes a varargs array the interpreter
//! owns, so the runtime unpacks it into the same immutable map marker
//! `Map.of(k1, v1, ...)` builds (Carpet's rule converter table).
//!
//! The host keeps map contents but cannot run jar code, so on its own it
//! answers an absent key with a placeholder marker. Libraries cache per-mod
//! helpers with `map.computeIfAbsent(modid, Handler::new)` and then read the
//! helper's fields, so the interpreter runs the mapping function itself:
//! exactly the JVM's contract of `get`, then (when absent or null) apply the
//! function to the key and `put` a non-null result.

use super::*;

impl<H: Host> Runtime<'_, H> {
    /// Answer `computeIfAbsent` on a host map when its function is a callback
    /// this runtime can run; `None` leaves every other call to the host.
    pub(super) fn invoke_map_compute(
        &mut self,
        r: &MethodRef,
        receiver: Option<&Value>,
        args: &[Value],
    ) -> Result<Option<Value>, NativeError> {
        if r.owner == "java/util/Map"
            && r.name == "entry"
            && r.descriptor == "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/util/Map$Entry;"
        {
            let [key, value] = args else {
                return Ok(None);
            };
            // `new KeyValueHolder(k, v)`: `Objects.requireNonNull` of both.
            if *key == Value::Ref(None) || *value == Value::Ref(None) {
                return Err(NativeError::UncaughtException {
                    class: "java/lang/NullPointerException".into(),
                    message: None,
                    frame: None,
                });
            }
            let entry = self.host.mint_map_entry(
                None,
                key,
                value,
                crate::host_map_entry::EntryMade::Holder,
            );
            return Ok(Some(entry));
        }
        // `Map.Entry.copyOf(e)`: a `KeyValueHolder` itself, else `Map.entry(e.getKey(),
        // e.getValue())`.
        if r.owner == "java/util/Map$Entry"
            && r.name == "copyOf"
            && r.descriptor == "(Ljava/util/Map$Entry;)Ljava/util/Map$Entry;"
        {
            let [entry] = args else {
                return Ok(None);
            };
            if crate::host_map_entry::is_key_value_holder(entry) {
                return Ok(Some(entry.clone()));
            }
            let Some((key, value)) = self.host.map_entry_parts(entry) else {
                return Ok(None);
            };
            if key == Value::Ref(None) || value == Value::Ref(None) {
                return Err(NativeError::UncaughtException {
                    class: "java/lang/NullPointerException".into(),
                    message: None,
                    frame: None,
                });
            }
            let copy = self.host.mint_map_entry(
                None,
                &key,
                &value,
                crate::host_map_entry::EntryMade::Holder,
            );
            return Ok(Some(copy));
        }
        if r.owner == "java/util/Map"
            && r.name == "ofEntries"
            && r.descriptor == "([Ljava/util/Map$Entry;)Ljava/util/Map;"
        {
            let Some(array) = args.first() else {
                return Ok(None);
            };
            let mut pairs = Vec::new();
            for entry in self.array(array)?.clone() {
                let Some((key, value)) = self.host.map_entry_parts(&entry) else {
                    return Ok(None);
                };
                pairs.extend([key, value]);
            }
            return Ok(Some(collection_value("map", &pairs)));
        }
        if r.name != "computeIfAbsent"
            || r.descriptor != "(Ljava/lang/Object;Ljava/util/function/Function;)Ljava/lang/Object;"
            || !is_map_owner(&r.owner)
        {
            return Ok(None);
        }
        let (Some(map), [key, function]) = (receiver, args) else {
            return Ok(None);
        };
        // Only a callback marker is the interpreter's to run; the host keeps
        // its own answer for anything else.
        if !expect_ref(function).is_some_and(|marker| marker.starts_with("@indy:")) {
            return Ok(None);
        }
        let get = MethodRef {
            owner: r.owner.clone(),
            name: "get".into(),
            descriptor: "(Ljava/lang/Object;)Ljava/lang/Object;".into(),
        };
        let current = self
            .host
            .invoke_instance(&get, map, std::slice::from_ref(key))?;
        if current != Value::Ref(None) {
            return Ok(Some(current));
        }
        let Some(computed) = self.invoke_callback(function, std::slice::from_ref(key))? else {
            return Ok(None);
        };
        if computed != Value::Ref(None) {
            let put = MethodRef {
                owner: r.owner.clone(),
                name: "put".into(),
                descriptor: "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;".into(),
            };
            self.host
                .invoke_instance(&put, map, &[key.clone(), computed.clone()])?;
        }
        Ok(Some(computed))
    }
}

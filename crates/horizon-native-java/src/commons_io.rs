//! The Commons IO file-text calls a config bootstrap writes its defaults
//! through (`write`/`writeStringToFile`, with or without a `Charset`) and
//! reads them back with.
//!
//! BiomesOPlenty checks for its `biome_toggles.json`, writes the defaults
//! when it is absent, then reads the file back and parses it. The runtime's
//! virtual file table is the only file system a jar may touch, so these two
//! calls answer from it: `write` stores the Java text, `readFileToString`
//! returns it. A read of a file the host does not hold raises the
//! `IOException` the real call raises instead of answering an empty string.

use crate::{Host, MethodRef, NativeError, Runtime, Value, expect_ref};

const FILE_UTILS: &str = "org/apache/commons/io/FileUtils";

impl<H: Host> Runtime<'_, H> {
    pub(super) fn invoke_commons_io(
        &mut self,
        r: &MethodRef,
        args: &[Value],
    ) -> Result<Option<Value>, NativeError> {
        if r.owner != FILE_UTILS {
            return Ok(None);
        }
        match (r.name.as_str(), r.descriptor.as_str()) {
            ("readFileToString", "(Ljava/io/File;)Ljava/lang/String;")
            | (
                "readFileToString",
                "(Ljava/io/File;Ljava/nio/charset/Charset;)Ljava/lang/String;",
            ) => {
                let Some(path) = args.first().and_then(|value| self.io_path_of(value)) else {
                    return Ok(None);
                };
                match self.io_text(&path) {
                    // The file holds the text as written in UTF-8; another
                    // charset decodes those bytes into other text unless they
                    // are ASCII: left to the host, which counts it.
                    Some(text) if !decodes_alike(args.get(1), &text) => Ok(None),
                    Some(text) => Ok(Some(Value::Ref(Some(text)))),
                    None => Err(NativeError::UncaughtException {
                        class: "java/io/FileNotFoundException".into(),
                        message: Some(path),
                        frame: None,
                    }),
                }
            }
            ("write", "(Ljava/io/File;Ljava/lang/CharSequence;)V")
            | ("write", "(Ljava/io/File;Ljava/lang/CharSequence;Ljava/nio/charset/Charset;)V")
            | ("writeStringToFile", "(Ljava/io/File;Ljava/lang/String;)V")
            | (
                "writeStringToFile",
                "(Ljava/io/File;Ljava/lang/String;Ljava/nio/charset/Charset;)V",
            ) => {
                let Some(path) = args.first().and_then(|value| self.io_path_of(value)) else {
                    return Ok(None);
                };
                // A `String`'s text only: a `StringBuilder` (or any other
                // `CharSequence`) writes its contents, which its marker is
                // not. Counted by the host instead.
                let Some(text) = args.get(1).and_then(|value| match value {
                    Value::Ref(Some(text)) if crate::value_class::is_plain_string(text) => {
                        Some(text)
                    }
                    _ => None,
                }) else {
                    return Ok(None);
                };
                // The table holds Java text as UTF-8 bytes would read back;
                // another charset's bytes differ unless the text is ASCII.
                if !decodes_alike(args.get(2), text) {
                    return Ok(None);
                }
                // commons-io may make parent folders and opens its own stream:
                // not events the model can name (`watch_service.rs`).
                self.watch_change(&path, crate::watch_service::Change::Unmodelled);
                self.io_files.insert(path, text.clone());
                Ok(Some(Value::Void))
            }
            _ => Ok(None),
        }
    }

    /// The text of a virtual file, if the host holds one at `path`.
    fn io_text(&self, path: &str) -> Option<String> {
        self.io_files.get(path).cloned()
    }
}

/// Whether text written or read with `charset` (none: the JVM default, UTF-8
/// since Java 18) is the same text the UTF-8 table holds: UTF-8 itself, or an
/// ASCII-compatible charset over ASCII text. Anything else changes the text
/// (`h\u{e9}llo` in ISO-8859-1 reads back in UTF-8 as `h\u{fffd}llo`) and
/// is counted, never stored as is.
fn decodes_alike(charset: Option<&Value>, text: &str) -> bool {
    match charset.and_then(expect_ref).map(String::as_str) {
        None | Some("charset/UTF-8") => true,
        Some("charset/US-ASCII" | "charset/ISO-8859-1") => text.is_ascii(),
        Some(_) => false,
    }
}

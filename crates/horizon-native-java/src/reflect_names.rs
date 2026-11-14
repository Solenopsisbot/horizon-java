//! `Class.getSimpleName` and `Class.getEnclosingClass` for nested jar
//! classes.
//!
//! The JVM takes a nested class's simple name from its `InnerClasses`
//! attribute: `Internals$EntryStackProvider` is `EntryStackProvider`, and an
//! anonymous class has the empty name. REI binds its service suppliers to
//! fields named after the interface's simple name, so answering
//! `Internals$EntryStackProvider` matched no field. A top-level class (no
//! entry for itself) keeps the host's answer, the text after the package.
//!
//! `getEnclosingClass` follows JDK 25 (`Class.getEnclosingClass` over
//! HotSpot's `InstanceKlass::compute_enclosing_class`):
//!
//! 1. a class with an `EnclosingMethod` attribute (a local or anonymous
//!    class) answers that attribute's class;
//! 2. otherwise its own `InnerClasses` row with a non-zero `outer_class_info`
//!    (a member class) answers that outer class, after
//!    `Reflection::check_for_inner_class`: the outer class must list the same
//!    (inner, outer) pair, or the call throws `IncompatibleClassChangeError`
//!    "`<outer> and <inner> disagree on InnerClasses attribute`";
//! 3. anything else (a top-level class) answers `null`.
//!
//! Only a class the runtime holds has those attributes to read. A JDK or game
//! class (`Map.Entry`, `String`) is counted instead of guessed from its name.

use super::*;

impl ClassFile {
    /// The simple name this class's own `InnerClasses` entry declares:
    /// `Some(Some(name))` for a member or local class, `Some(None)` for an
    /// anonymous one, `None` when the class is not nested (or the table is
    /// unreadable).
    pub(super) fn nested_simple_name(&self) -> Option<Option<String>> {
        self.own_inner_class_row().map(|(simple, _)| simple)
    }

    /// The access flags this class's own `InnerClasses` entry declares
    /// (`static`, `private`, ... of a member class), which JDK 25's
    /// `Class.getModifiers()` answers for a nested class in place of the
    /// class file's own flags. `None` when the class is not nested.
    pub(super) fn nested_access_flags(&self) -> Option<u16> {
        self.own_inner_class_row().map(|(_, flags)| flags)
    }

    /// This class's own `InnerClasses` entry: its simple name (`None` for an
    /// anonymous class) and its flags. `None` when the class is not nested
    /// (or the table is unreadable).
    fn own_inner_class_row(&self) -> Option<(Option<String>, u16)> {
        let mut r = Reader::new(&self.raw);
        if r.u4().ok()? != 0xCAFE_BABE {
            return None;
        }
        r.skip(4).ok()?;
        let pool = read_pool(&mut r).ok()?;
        r.skip(2).ok()?; // access
        let this = r.u2().ok()?;
        r.skip(2).ok()?; // super
        let interfaces = r.u2().ok()? as usize;
        r.skip(interfaces.checked_mul(2)?).ok()?;
        for _ in 0..2 {
            // fields, then methods
            let count = r.u2().ok()? as usize;
            for _ in 0..count {
                r.skip(6).ok()?;
                skip_attributes(&mut r, &pool).ok()?;
            }
        }
        let attributes = r.u2().ok()? as usize;
        for _ in 0..attributes {
            let name = utf8(&pool, r.u2().ok()?).ok()?;
            let length = r.u4().ok()? as usize;
            let bytes = r.bytes(length).ok()?;
            if name != "InnerClasses" {
                continue;
            }
            let mut table = Reader::new(bytes);
            let entries = table.u2().ok()? as usize;
            for _ in 0..entries {
                let inner = table.u2().ok()?;
                let _outer = table.u2().ok()?;
                let simple = table.u2().ok()?;
                let flags = table.u2().ok()?;
                if inner == this {
                    let simple = if simple == 0 {
                        None
                    } else {
                        Some(utf8(&pool, simple).ok()?)
                    };
                    return Some((simple, flags));
                }
            }
        }
        None
    }
}

impl ClassFile {
    /// The class's `EnclosingMethod` class, and its own `InnerClasses` rows as
    /// `(inner, outer)` names (`outer` is `None` for a local or anonymous
    /// class). `None` when the class file is unreadable.
    fn nesting_attributes(&self) -> Option<Nesting> {
        let mut r = Reader::new(&self.raw);
        if r.u4().ok()? != 0xCAFE_BABE {
            return None;
        }
        r.skip(4).ok()?;
        let pool = read_pool(&mut r).ok()?;
        r.skip(2).ok()?; // access
        let this = class_name(&pool, r.u2().ok()?).ok()?;
        r.skip(2).ok()?; // super
        let interfaces = r.u2().ok()? as usize;
        r.skip(interfaces.checked_mul(2)?).ok()?;
        for _ in 0..2 {
            // fields, then methods
            let count = r.u2().ok()? as usize;
            for _ in 0..count {
                r.skip(6).ok()?;
                skip_attributes(&mut r, &pool).ok()?;
            }
        }
        let mut nesting = Nesting {
            this,
            enclosing_method_class: None,
            inner_classes: Vec::new(),
        };
        let attributes = r.u2().ok()? as usize;
        for _ in 0..attributes {
            let name = utf8(&pool, r.u2().ok()?).ok()?;
            let length = r.u4().ok()? as usize;
            let bytes = r.bytes(length).ok()?;
            let mut table = Reader::new(bytes);
            match name.as_str() {
                "EnclosingMethod" => {
                    nesting.enclosing_method_class =
                        Some(class_name(&pool, table.u2().ok()?).ok()?);
                }
                "InnerClasses" => {
                    let entries = table.u2().ok()? as usize;
                    for _ in 0..entries {
                        let inner = table.u2().ok()?;
                        let outer = table.u2().ok()?;
                        table.skip(4).ok()?; // inner name, flags
                        if inner == 0 {
                            continue;
                        }
                        let outer = match outer {
                            0 => None,
                            index => Some(class_name(&pool, index).ok()?),
                        };
                        nesting
                            .inner_classes
                            .push((class_name(&pool, inner).ok()?, outer));
                    }
                }
                _ => {}
            }
        }
        Some(nesting)
    }
}

/// What `Class.getEnclosingClass` reads from one class file.
struct Nesting {
    this: String,
    enclosing_method_class: Option<String>,
    inner_classes: Vec<(String, Option<String>)>,
}

impl Nesting {
    /// The `outer_class_info` of this class's own `InnerClasses` row: the
    /// first row naming it, as HotSpot's iterator stops at the first match.
    fn own_row(&self) -> Option<Option<&str>> {
        self.inner_classes
            .iter()
            .find(|(inner, _)| *inner == self.this)
            .map(|(_, outer)| outer.as_deref())
    }
}

impl<H: Host> Runtime<'_, H> {
    /// `Class.getEnclosingClass` for a class the runtime holds. `Ok(None)` for
    /// any other call and for a class the runtime does not hold, which the
    /// host counts as before.
    pub(super) fn invoke_reflect_enclosing_class(
        &mut self,
        r: &MethodRef,
        receiver: Option<&Value>,
    ) -> Result<Option<Value>, NativeError> {
        if r.owner != "java/lang/Class"
            || r.name != "getEnclosingClass"
            || r.descriptor != "()Ljava/lang/Class;"
        {
            return Ok(None);
        }
        let class = reflection_class_name(receiver, r)?.replace('.', "/");
        let Some(nesting) = self.class(&class).and_then(ClassFile::nesting_attributes) else {
            return Ok(None);
        };
        if let Some(enclosing) = nesting.enclosing_method_class {
            if enclosing == nesting.this {
                return Err(NativeError::UncaughtException {
                    class: "java/lang/InternalError".into(),
                    message: Some("Malformed enclosing method information".into()),
                    frame: None,
                });
            }
            return Ok(Some(Value::Ref(Some(enclosing))));
        }
        let Some(Some(outer)) = nesting.own_row() else {
            return Ok(Some(Value::Ref(None)));
        };
        let outer = outer.to_owned();
        // The outer class must agree. One the runtime doesn't hold can't be
        // checked, so the host counts the call rather than this answering it
        // unchecked.
        let Some(outer_nesting) = self.class(&outer).and_then(ClassFile::nesting_attributes) else {
            return Ok(None);
        };
        let agrees = outer_nesting
            .inner_classes
            .iter()
            .any(|(inner, listed)| *inner == nesting.this && listed.as_deref() == Some(&outer));
        if !agrees {
            return Err(NativeError::UncaughtException {
                class: "java/lang/IncompatibleClassChangeError".into(),
                message: Some(format!(
                    "{} and {} disagree on InnerClasses attribute",
                    outer.replace('/', "."),
                    nesting.this.replace('/', ".")
                )),
                frame: None,
            });
        }
        Ok(Some(Value::Ref(Some(outer))))
    }
}

impl<H: Host> Runtime<'_, H> {
    pub(super) fn invoke_reflect_simple_name(
        &mut self,
        r: &MethodRef,
        receiver: Option<&Value>,
    ) -> Result<Option<Value>, NativeError> {
        if r.owner != "java/lang/Class"
            || r.name != "getSimpleName"
            || r.descriptor != "()Ljava/lang/String;"
        {
            return Ok(None);
        }
        let class = reflection_class_name(receiver, r)?.replace('.', "/");
        if class.starts_with('[') {
            return match self.array_simple_name(&class) {
                Some(simple) => Ok(Some(Value::Ref(Some(simple)))),
                None => self.host.unknown_operand_call(r).map(Some),
            };
        }
        Ok(self
            .class(&class)
            .and_then(ClassFile::nested_simple_name)
            .map(|simple| Value::Ref(Some(simple.unwrap_or_default()))))
    }
}

impl<H: Host> Runtime<'_, H> {
    /// `Class.getCanonicalName()` of `class` (an internal name), as JDK 25
    /// builds it: an array's component's name plus `[]`; `null` for a local
    /// or anonymous class; a member class's enclosing class's canonical name,
    /// `.`, and its simple name; a top-level class's binary name. A class the
    /// runtime does not hold is canonical by its name unless it is nested
    /// (`$`), which only its `InnerClasses` can tell: `None` (unknown).
    pub(crate) fn canonical_class_name(&self, class: &str) -> Option<Option<String>> {
        self.canonical_class_name_at(class, 0)
    }

    fn canonical_class_name_at(&self, class: &str, depth: usize) -> Option<Option<String>> {
        if depth > MAX_SUPERCLASS_WALK {
            return None;
        }
        if let Some(component) = class.strip_prefix('[') {
            let component = erased_field_type(component);
            let name = if PRIMITIVE_CLASS_NAMES.contains(&component.as_str()) {
                Some(component)
            } else {
                self.canonical_class_name_at(&component, depth + 1)?
            };
            return Some(name.map(|name| format!("{name}[]")));
        }
        let binary = class.replace('/', ".");
        let Some(file) = self.class(class) else {
            return (!class.contains('$')).then_some(Some(binary));
        };
        let nesting = file.nesting_attributes()?;
        if nesting.enclosing_method_class.is_some() {
            return Some(None);
        }
        match nesting.own_row() {
            None => Some(Some(binary)),
            Some(None) => Some(None),
            Some(Some(outer)) => {
                let outer = outer.to_owned();
                let simple = file.nested_simple_name()??;
                Some(
                    self.canonical_class_name_at(&outer, depth + 1)?
                        .map(|outer| format!("{outer}.{simple}")),
                )
            }
        }
    }
}

/// The primitive class names an array component can be.
const PRIMITIVE_CLASS_NAMES: &[&str] = &[
    "boolean", "byte", "char", "short", "int", "long", "float", "double",
];

//! The threads a jar started, named in the run's report.
//!
//! The interpreter runs every call on the Rust stack, so it can't run a second
//! thread beside the one it is on (decided 2026-11-13). `Thread.start` stays a
//! counted unknown call, and the started thread's body never runs. A bare
//! `java/lang/Thread#start()V` count didn't say *what* never ran, though. Forge
//! Config API Port's night-config `FileWatcher` starts a watcher thread
//! (`FileWatcher$FsWatcher`) for every config it opens. That thread does
//! nothing observable inside the run (it waits for a file change), but an
//! update checker or a metrics server is a different story. The report has to
//! say which one it was.
//!
//! So `new Thread(..)` keeps the `Runnable` (and the name, when the
//! constructor is given one), and `start()` records a [`ParkedThread`] beside
//! the count. The probe's `--json` report lists them as `parked_threads`, and
//! the game's mod logs each one (`parked thread ..: started, its body never
//! runs`).
//!
//! The `Runnable` is named by what the runtime holds:
//! * an object of a jar class: its internal class name
//!   (`com/electronwill/nightconfig/core/file/FileWatcher$FsWatcher`);
//! * a lambda or method reference: its target, `owner#name`;
//! * `null` (no `Runnable`, so `Thread.run()` does nothing): `null`;
//! * any other host value: the class part of its marker, else the marker.
//!
//! A thread made without a name gets `Thread-<n>` from a JVM-wide counter,
//! which depends on every unnamed thread the whole game made, so its name is
//! reported as unknown (`None`) rather than invented, and `getName()` on it is
//! counted.
//!
//! A jar class extending `Thread` (#194) is a thread like any other: its
//! `super(..)` runs `Thread`'s constructor here, and a subclass given no
//! `Runnable` is named by its own class (its `run()` is what would have run).
//!
//! **State.** Each constructed thread keeps what Java 25's `Thread` keeps and
//! the jar can read back ([`ThreadState`]): its name, daemon flag and
//! priority, and whether it was started. A new thread inherits the daemon flag
//! and priority of the thread that made it, the game's thread a mod's code
//! runs on: not a daemon (neither the client's render thread nor the server
//! thread is one), but its priority is not known (the server thread's is 8 on a
//! machine with more than four processors, 5 otherwise), so a priority the jar
//! never set is counted when read. `setDaemon` on a started thread and a second
//! `start()` throw `IllegalThreadStateException`; `setPriority` outside 1..10
//! throws `IllegalArgumentException`, as Java 25 does.

use super::*;

/// A thread a jar started whose body the runtime never ran.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParkedThread {
    /// The name the jar gave the thread's constructor, if it gave one.
    pub name: Option<String>,
    /// What the thread would have run (see the module docs).
    pub runnable: String,
}

/// What a constructed thread keeps (see the module docs).
#[derive(Debug, Clone, Default)]
struct ThreadState {
    /// `None`: the JVM's `Thread-<n>`, not known here.
    name: Option<String>,
    daemon: bool,
    /// `None`: the creating thread's, not known here.
    priority: Option<i32>,
    started: bool,
}

/// The `Runnable` and name each constructed `Thread` marker was given.
#[derive(Debug, Clone, Default)]
pub(crate) struct ThreadTable {
    made: BTreeMap<String, ParkedThread>,
    state: BTreeMap<String, ThreadState>,
    parked: Vec<ParkedThread>,
    /// How many of `parked` [`StdHost::take_new_parked_threads`] handed out.
    reported: usize,
}

/// How the report names a `Runnable` value.
fn runnable_name(value: &Value) -> String {
    if let Some(class) = crate::runtime::jar_object::object_class(value) {
        return class.to_owned();
    }
    let Some(marker) = expect_ref(value) else {
        return "null".into();
    };
    if marker.starts_with("@indy:") {
        // `@indy:<source>:<index>:<owner#name(descriptor)>:<captures>`.
        if let Some(target) = marker.split(':').nth(3) {
            return target
                .split_once('(')
                .map_or(target, |(name, _)| name)
                .to_owned();
        }
    }
    // A host model's own object (`java/lang/Thread#neutral2`) names its class.
    marker
        .split_once('#')
        .map_or(marker.as_str(), |(class, _)| class)
        .to_owned()
}

impl StdHost {
    /// `new Thread(..)`: keep the `Runnable` and the name the constructor was
    /// given. Every `Thread` constructor takes the `Runnable` as its first
    /// `Runnable` parameter and the name as its first `String` one.
    pub(crate) fn note_thread_made(&mut self, marker: &str, descriptor: &str, args: &[Value]) {
        let parameters = descriptor_arguments(descriptor);
        let at = |wanted: &str| {
            parameters
                .iter()
                .position(|parameter| parameter == wanted)
                .and_then(|index| args.get(index))
        };
        // A jar subclass given no `Runnable` runs its own `run()`: name it.
        let class = crate::runtime::jar_object::key_class(marker).unwrap_or("java/lang/Thread");
        let runnable = match at("Ljava/lang/Runnable;") {
            Some(runnable) => runnable_name(runnable),
            None if class != "java/lang/Thread" => class.to_owned(),
            None => "null".into(),
        };
        let name = at("Ljava/lang/String;").and_then(expect_ref).cloned();
        self.threads.state.insert(
            marker.to_owned(),
            ThreadState {
                name: name.clone(),
                ..ThreadState::default()
            },
        );
        self.threads
            .made
            .insert(marker.to_owned(), ParkedThread { name, runnable });
    }

    /// A member of a thread the jar constructed (`marker` its key). `None`
    /// leaves the call to be counted.
    pub(crate) fn constructed_thread_call(
        &mut self,
        r: &MethodRef,
        marker: &str,
        args: &[Value],
    ) -> Option<Result<Value, NativeError>> {
        let state = self.threads.state.get_mut(marker)?;
        let illegal_state = || {
            Err(NativeError::UncaughtException {
                class: "java/lang/IllegalThreadStateException".into(),
                message: None,
                frame: None,
            })
        };
        Some(match (r.name.as_str(), r.descriptor.as_str()) {
            ("getName", "()Ljava/lang/String;") => Ok(Value::Ref(Some(state.name.clone()?))),
            ("setName", "(Ljava/lang/String;)V") => {
                let Some(name) = args.first().and_then(expect_ref).cloned() else {
                    return Some(Err(NativeError::UncaughtException {
                        class: "java/lang/NullPointerException".into(),
                        message: Some("name cannot be null".into()),
                        frame: None,
                    }));
                };
                state.name = Some(name.clone());
                if let Some(made) = self.threads.made.get_mut(marker) {
                    made.name = Some(name);
                }
                Ok(Value::Void)
            }
            ("isDaemon", "()Z") => Ok(Value::Int(i32::from(state.daemon))),
            ("setDaemon", "(Z)V") if state.started => illegal_state(),
            ("setDaemon", "(Z)V") => {
                state.daemon = matches!(args.first(), Some(Value::Int(flag)) if *flag != 0);
                Ok(Value::Void)
            }
            ("getPriority", "()I") => Ok(Value::Int(state.priority?)),
            ("setPriority", "(I)V") => match args.first() {
                Some(Value::Int(priority)) if (1..=10).contains(priority) => {
                    state.priority = Some(*priority);
                    Ok(Value::Void)
                }
                Some(Value::Int(_)) => Err(NativeError::UncaughtException {
                    class: "java/lang/IllegalArgumentException".into(),
                    message: None,
                    frame: None,
                }),
                _ => return None,
            },
            ("start", "()V") if state.started => illegal_state(),
            _ => return None,
        })
    }

    /// `start()` on a thread the jar constructed: mark it started (its body
    /// never runs; the caller counts the call and parks it).
    pub(crate) fn mark_thread_started(&mut self, marker: &str) {
        if let Some(state) = self.threads.state.get_mut(marker) {
            state.started = true;
        }
    }

    /// `thread.start()`: the body never runs, so name it in the report. The
    /// caller still counts the call.
    pub(crate) fn note_thread_parked(&mut self, marker: &str) {
        let thread = self
            .threads
            .made
            .get(marker)
            .cloned()
            .unwrap_or(ParkedThread {
                name: None,
                runnable: "unknown".into(),
            });
        self.threads.parked.push(thread);
    }

    /// The threads the jar started whose bodies never ran, in start order.
    pub fn parked_threads(&self) -> &[ParkedThread] {
        &self.threads.parked
    }

    /// The threads parked since the last call, for a caller that logs each
    /// one once (the game's mod, after every phase).
    pub fn take_new_parked_threads(&mut self) -> Vec<ParkedThread> {
        let new = self.threads.parked[self.threads.reported..].to_vec();
        self.threads.reported = self.threads.parked.len();
        new
    }
}

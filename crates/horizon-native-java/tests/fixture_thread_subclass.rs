//! Jar classes extending `java.lang.Thread` (#194).
//!
//! `fixtures/thread_subclass/Probe.java` is the spec. `subclasses` and
//! `starts` are what OpenJDK 25.0.5 printed (`javac --release 21`, two runs
//! alike): a subclass constructs through `super()`, `super(name)`,
//! `super(target)` and `super(target, name)`; `getName`/`setName`,
//! `isDaemon`/`setDaemon` and `getPriority`/`setPriority` answer as Java 25
//! does; an overridden `run()` runs (through the subclass's type and through
//! `Thread`'s), and an inherited one runs its target; `start()` twice throws
//! `IllegalThreadStateException`.
//!
//! What the runtime cannot know is counted: `start()` (the body never runs,
//! the thread is parked and named), an unnamed thread's `Thread-<n>` (Java
//! printed `Thread-4`: a JVM-wide counter) and a priority inherited from the
//! game's thread.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::{ParkedThread, StdHost};
use javac_fixture::Fixture;

const PROBE: &str = include_str!("fixtures/thread_subclass/Probe.java");

fn fixture() -> Fixture {
    Fixture::build("thread-subclass", &[("demo/Probe.java", PROBE)])
}

fn counted(host: &StdHost) -> Vec<(String, usize)> {
    host.unknown_host_calls()
}

#[test]
fn a_thread_subclass_constructs_and_answers_as_java_25() {
    let fixture = fixture();
    let mut host = fixture.jar.seed_host(StdHost::new());
    assert_eq!(
        fixture.text_on(&mut host, "subclasses"),
        "loader,renamed,falsetrue,7,iae,2,11,carrier,111,plain,truetrue,false,npe"
    );
    assert_eq!(counted(&host), Vec::<(String, usize)>::new());
}

#[test]
fn start_is_counted_and_the_thread_parked_by_name() {
    let fixture = fixture();
    let mut host = fixture.jar.seed_host(StdHost::new());
    assert_eq!(fixture.text_on(&mut host, "starts"), "itse");
    assert_eq!(
        counted(&host),
        vec![("java/lang/Thread#start()V".to_owned(), 2)]
    );
    let parked = host.parked_threads();
    assert_eq!(
        parked,
        &[
            ParkedThread {
                name: Some("watchdog".into()),
                runnable: "demo/Probe$Worker".into(),
            },
            ParkedThread {
                name: None,
                runnable: "demo/Probe#lambda$starts$0".into(),
            },
        ],
        "{parked:?}"
    );
}

#[test]
fn an_unnamed_threads_name_and_an_inherited_priority_are_counted() {
    let fixture = fixture();
    let mut host = fixture.jar.seed_host(StdHost::new());
    fixture.text_on(&mut host, "unknowns");
    let calls: Vec<String> = counted(&host).into_iter().map(|(call, _)| call).collect();
    assert_eq!(
        calls,
        vec![
            "java/lang/Thread#getName()Ljava/lang/String;".to_owned(),
            "java/lang/Thread#getPriority()I".to_owned(),
        ]
    );
}

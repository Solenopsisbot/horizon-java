//! #156 P2 (second half): the main thread, loggers, the stack walker, its
//! frames, stack trace elements and modules are host handles, not text a jar
//! `String` can spell.
//!
//! `fixtures/host_values/P2b.java` holds one `static String` method per case.
//! The expected strings are what OpenJDK 25.0.5 printed for it (`javac
//! --release 21`, two runs alike, on elysium), with the game's logging jars on
//! the class path (log4j-api/core 2.26.0, log4j-slf4j2-impl 2.26.0, slf4j-api
//! 2.0.17, Mojang logging 1.7.12). Here they are stubs that never ship, so
//! the calls reach the runtime's and the host's logger models. `Exact`: Java's answer, nothing counted. `Counted`:
//! Java's answer needs what no run can reproduce (an identity hash, the
//! process's thread id, a line number, the backend's logger class, two
//! objects with equal keys), or Java throws where the runtime counts.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::{StdHost, Value};
use javac_fixture::Fixture;

enum Outcome {
    Exact(&'static str),
    Counted,
}
use Outcome::*;

const CASES: &[(&str, Outcome)] = &[
    // Class.isInstance, isAssignableFrom (a FromKey class through Java 25's table) and instanceof.
    (
        "genClassChecks",
        Exact(
            r#"truetruefalsetruetruetruetruetruefalse|truetruefalsetruetruetrue|truefalsetruetruefalse|java.lang.Threadjava.lang.StackWalkerjava.lang.Modulejava.lang.StackTraceElement"#,
        ),
    ),
    // Inside host collections: text, HashMap, contains, indexOf.
    (
        "genCollections",
        Exact(r#"[module java.base]|{module java.base=1}|1truetruetrue0"#),
    ),
    // None of these kinds is Comparable: Java 25 throws ClassCastException, the runtime counts.
    ("genOrder", Counted),
    // Type switch labels reach each kind's row (Runnable, StackWalker, Module, StackTraceElement, StackFrame).
    ("genSwitch", Exact(r#"RWMEFS"#)),
    // A HashSet holding the thread.
    ("genThreadSet", Exact(r#"true"#)),
    // As `genOrder`, through a TreeSet.
    ("genTreeSet", Counted),
    // log4j's `getLogger()` is the caller class's logger; `Class`/`Object` overloads name it by the canonical name (#173 review).
    // Java 25: `org.apache.logging.log4j.core.Logger`; the backend's class is unknown (`ClassRule::Unknown`).
    ("loggerClass", Counted),
    // A logger matches its declared type exactly.
    ("loggerDeclared", Exact(r#"LdemoJdemodemo"#)),
    // Names holding a key's separators are their own literal names (keys through `of_values`).
    (
        "loggerNames",
        Exact(r#"a,b|a,b;a\b|a\b;x@y|x@y;a/b|a/b;rlog4j,ra|rlog4j,ra;false"#),
    ),
    // log4j and java.util.logging cache one logger per name (canonical per API).
    (
        "loggerReal",
        Exact(r#"truetruefalsefalsedemo|truefalse|truetruetruedemo"#),
    ),
    (
        "loggerSpelled",
        Exact(r#"falsefalse|logger/demo|logger/demo|2sr"#),
    ),
    // A logger may implement a label no declared type rules out (`Runnable`): counted (Java 25: `LJ`).
    ("loggerSwitch", Counted),
    // Two anonymous loggers are two objects with equal keys: identity counted.
    ("loggerTwo", Counted),
    // Java 25: `true`; a named module's descriptor is not modelled: counted.
    ("moduleDescriptor", Counted),
    // Java 25: `true`; another system module's exports are not modelled: counted.
    ("moduleExports", Counted),
    // A JDK class's module is Java 25's for its package; an array's is its element's.
    (
        "moduleNames",
        Exact(
            r#"java.loggingtruemodule java.logging|falsetruefalse|java.sql|nulljava.basetruetrue|null|true"#,
        ),
    ),
    (
        "moduleReal",
        Exact(
            r#"truefalsetruefalsefalsetruejava.base|module java.base|module java.basemodule java.base|truefalsejava.lang.Module|1"#,
        ),
    ),
    (
        "moduleSpelled",
        Exact(
            r#"falsefalse|module/unnamed|module/unnamed|2sr|falsefalse|module/java.base|module/java.base|2sr"#,
        ),
    ),
    // Java 25: `unnamed module @…`, an identity hash.
    ("moduleText", Counted),
    // Two elements of one method may be two lines: equals counted (Java 25: `false`).
    ("stackElements", Counted),
    (
        "stackReal",
        Exact(
            r#"truetruetruefalsejava.lang.StackWalker|stackRealdemo.P2btruetruetruefalsejava.lang.StackFrameInfo|stackRealdemo.P2btruetruefalsetruefalsejava.lang.StackTraceElement"#,
        ),
    ),
    // `stack-walker` was a bare marker word; now it is only a String.
    (
        "stackSpelled",
        Exact(
            r#"java.lang.Stringtruefalsefalse1525255963|stack-walker|stack-walker|2sr|falsefalse|stack-frame/stackSpelled@demo/P2|stack-frame/stackSpelled@demo/P2|2sr|falsefalse|stack-trace-element/stackSpelled@demo/P2|stack-trace-element/stackSpelled@demo/P2|2sr"#,
        ),
    ),
    // Java 25: `demo.P2b.stackText(P2b.java:114)`; the runtime keeps no file or line.
    ("stackText", Counted),
    // Two walks hand out two frame objects with equal keys: identity counted.
    ("stackTwo", Counted),
    // Java 25: `true5RUNNABLEfalsefalse`; unmodelled Thread members are counted, not a MissingMethod stop.
    ("threadMembers", Counted),
    // One thread: canonical, so an IdentityHashMap holds it once.
    (
        "threadReal",
        Exact(r#"main|truetruefalse|truetruefalsejava.lang.Thread|1"#),
    ),
    // A String spelled like the old markers stays a String (equals, concat, toString, HashMap key); getClass/instanceof of `word/` text wait for #156 P17 (`wordedClasses`).
    (
        "threadSpelled",
        Exact(r#"falsefalse|thread/main|thread/main|2sr"#),
    ),
    // Java 25: `Thread[#3,main,5,main]`: the id and name are the process's.
    ("threadText", Counted),
    // `getInstance(Set)` of an empty set is the default walker; the depth overload is a new one (#173 B1).
    ("walkerEmptySet", Exact(r#"truetruefalsetrue"#)),
    // getClass of `word/` text: #156 P17 (counted).
    ("wordedClasses", Counted),
];

const LOG_MANAGER_STUB: &str = r#"
package org.apache.logging.log4j;
public final class LogManager {
  public static Logger getLogger(String name) { return null; }
}
"#;

const LOGGER_STUB: &str = r#"
package org.apache.logging.log4j;
public interface Logger { String getName(); }
"#;

#[test]
fn p2b_handles_answer_as_java_25_or_count() {
    let fixture = Fixture::build(
        "host-values-p2b",
        &[
            (
                "demo/P2b.java",
                include_str!("fixtures/host_values/P2b.java"),
            ),
            ("org/apache/logging/log4j/LogManager.java", LOG_MANAGER_STUB),
            ("org/apache/logging/log4j/Logger.java", LOGGER_STUB),
        ],
    );
    let mut failures = Vec::new();
    for (method, outcome) in CASES {
        let mut host = fixture.jar.seed_host(StdHost::new());
        let answer = fixture.jar.invoke_method_with_args(
            "demo/P2b",
            method,
            "()Ljava/lang/String;",
            None,
            &[],
            &mut host,
        );
        let unknown = host.unknown_host_calls();
        let ok = match outcome {
            Exact(expected) => {
                matches!(&answer, Ok(Value::Ref(Some(text))) if text == expected)
                    && unknown.is_empty()
            }
            Counted => !unknown.is_empty(),
        };
        if !ok {
            failures.push(format!(
                "{method}\n  native:  {answer:?}\n  unknown: {unknown:?}"
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} cases:\n{}",
        failures.len(),
        CASES.len(),
        failures.join("\n")
    );
}

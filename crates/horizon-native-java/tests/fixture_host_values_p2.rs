//! #156 P2: paths, files, `FMLPaths`, the watch service's objects, the main
//! thread, loggers, stack walking and modules are host handles, not text a
//! jar `String` can spell.
//!
//! `fixtures/host_values/P2.java` holds one `static String` method per case.
//! The expected strings are what OpenJDK 25.0.5 printed for it (`javac
//! --release 21`, two runs alike, on elysium), with the real NeoForge
//! `FMLPaths` (loader 12.0.12; `LogUtils` stubbed so its `<clinit>` runs) and
//! log4j-api 2.26.0 on the class path. Here those classes are stubs that
//! never ship, so the calls reach the host's models. Outcomes as in
//! `fixture_host_values_review.rs`:
//!
//! * `Exact`: Java's answer, nothing counted;
//! * `Counted`: Java's answer needs what no run can reproduce (an identity
//!   hash, the process's thread id, a line number, the machine's
//!   `sun.jnu.encoding`, two objects with equal keys), so the run counts it;
//! * `Gap`: a known silent difference, with its plan (not asserted).

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::{StdHost, Value};
use javac_fixture::Fixture;

enum Outcome {
    Exact(&'static str),
    Counted,
    #[allow(dead_code)]
    Gap(&'static str),
}
use Outcome::*;

const CASES: &[(&str, Outcome)] = &[
    // A String spelled like a path stays a String; `Path.of("path:/x")` is
    // the relative path it spells.
    (
        "pathSpelled",
        Exact(
            "java.lang.Stringtruefalsefalse-1250755840|path:/horizon/game|path:/horizon/game|2sr|path:/xxpath:|true",
        ),
    ),
    // `UnixPath`: normalized text, `equals`/`hashCode` over its bytes, its
    // class and supertypes, in host collections too.
    (
        "pathReal",
        Exact(
            "/horizon/game|/horizon/game|sun.nio.fs.UnixPathtruetruetruetruefalse|truetruefalsefalse|1103596949,97,0,47|[]rel/xa/b/c/d|/horizon/game/xgame/horizontruefalse|1true0true|955364961",
        ),
    ),
    ("pathSameHandle", Exact("truefalse12")),
    ("pathTwo", Counted),
    ("pathIdMap", Counted),
    // Java 25 threw `InvalidPathException` under the POSIX locale and
    // printed `6214` under `C.UTF-8`.
    ("pathNonAscii", Counted),
    (
        "fileSpelled",
        Exact("java.lang.Stringtruefalsefalse-1352516893|file:/a/b|file:/a/b|2sr"),
    ),
    // `java.io.File` on Linux: `UnixFileSystem.normalize`/`resolve`,
    // `hashCode` `path.hashCode() ^ 1234321`, `equals` by path.
    (
        "fileReal",
        Exact(
            "/a/b|/a/b|truetruefalsefalse|268852,1234416,1234321|a/ba/b|1true|b/a|truetrue|truetruejava.io.File|false",
        ),
    ),
    (
        "fmlSpelled",
        Exact(
            "falsefalse|fml-path/GAMEDIR/path:/horizon/game|fml-path/GAMEDIR/path:/horizon/game|2sr",
        ),
    ),
    (
        "fmlReal",
        Exact(
            "GAMEDIR|CONFIGDIR|truefalsetruefalsefalse|truetruefalsenet.neoforged.fml.loading.FMLPaths|1",
        ),
    ),
    (
        "watchSpelled",
        Exact("falsefalse|watch-kind/ENTRY_CREATE|watch-kind/ENTRY_CREATE|2sr|false"),
    ),
    (
        "watchKinds",
        Exact(
            "ENTRY_CREATE|ENTRY_MODIFYOVERFLOW|truefalsetruefalse|truefalsejava.nio.file.StandardWatchEventKinds$StdWatchEventKind|1",
        ),
    ),
    // Registering again answers the same key; the event's handle is one object.
    (
        "watchService",
        Exact("truetruetruetruetruefalsetruetrue|truetruetruea.txttruetruetruetrue|false"),
    ),
    (
        "threadSpelled",
        Exact("falsefalse|thread/main|thread/main|2sr"),
    ),
    // One thread: canonical, so an `IdentityHashMap` holds it once.
    (
        "threadReal",
        Exact("main|truetruefalse|truetruefalsejava.lang.Thread|1"),
    ),
    // Java 25: `Thread[#3,main,5,main]`.
    ("threadText", Counted),
    (
        "loggerSpelled",
        Exact("falsefalse|logger/demo|logger/demo|2sr"),
    ),
    // log4j and `java.util.logging` cache one logger per name.
    (
        "loggerReal",
        Exact("truetruefalsefalsedemo|truefalse|truetruetruedemo"),
    ),
    ("loggerTwo", Counted),
    // `stack-walker` was a bare marker word; now it is only a String.
    (
        "stackSpelled",
        Exact(
            "java.lang.Stringtruefalsefalse1525255963|stack-walker|stack-walker|2sr|falsefalse|stack-frame/stackSpelled@demo/P2|stack-frame/stackSpelled@demo/P2|2sr|falsefalse|stack-trace-element/stackSpelled@demo/P2|stack-trace-element/stackSpelled@demo/P2|2sr",
        ),
    ),
    (
        "stackReal",
        Exact(
            "truetruetruefalsejava.lang.StackWalker|stackRealdemo.P2truetruetruefalsejava.lang.StackFrameInfo|stackRealdemo.P2truetruefalsetruefalsejava.lang.StackTraceElement",
        ),
    ),
    ("stackTwo", Counted),
    ("stackElements", Counted),
    // Java 25: `demo.P2.stackText(P2.java:238)`, twice; the runtime keeps no
    // file or line.
    ("stackText", Counted),
    (
        "moduleSpelled",
        Exact(
            "falsefalse|module/unnamed|module/unnamed|2sr|falsefalse|module/java.base|module/java.base|2sr",
        ),
    ),
    (
        "moduleReal",
        Exact(
            "truefalsetruefalsefalsetruejava.base|module java.base|module java.basemodule java.base|truefalsejava.lang.Module|1",
        ),
    ),
    // Java 25: `unnamed module @7a7b0070`.
    ("moduleText", Counted),
    // Java 25: `java.lang.Stringtrue1886486174java.lang.Stringtrue-744991998
    // java.lang.Stringtrue866498461java.lang.Stringtrue42849958
    // java.lang.Stringtrue1589362998java.lang.Stringtrue63063380`.
    (
        "wordedClasses",
        Gap("#156 P17: is_plain_string reads `word/rest` text as a marker, not a String"),
    ),
];

const FMLPATHS_STUB: &str = r#"
package net.neoforged.fml.loading;
public enum FMLPaths {
  GAMEDIR, JIJ_CACHEDIR, MODSDIR, CONFIGDIR, FMLCONFIG;
  public java.nio.file.Path get() { return null; }
}
"#;

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
fn p2_handles_answer_as_java_25_or_count() {
    let fixture = Fixture::build(
        "host-values-p2",
        &[
            ("demo/P2.java", include_str!("fixtures/host_values/P2.java")),
            ("net/neoforged/fml/loading/FMLPaths.java", FMLPATHS_STUB),
            ("org/apache/logging/log4j/LogManager.java", LOG_MANAGER_STUB),
            ("org/apache/logging/log4j/Logger.java", LOGGER_STUB),
        ],
    );
    let mut failures = Vec::new();
    for (method, outcome) in CASES {
        let mut host = fixture.jar.seed_host(StdHost::new());
        let answer = fixture.jar.invoke_method_with_args(
            "demo/P2",
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
            Gap(_) => true,
        };
        if !ok {
            let expected = match outcome {
                Exact(text) => format!("exact {text}"),
                _ => "counted".to_owned(),
            };
            failures.push(format!(
                "{method}\n  want:    {expected}\n  native:  {answer:?}\n  unknown: {unknown:?}"
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

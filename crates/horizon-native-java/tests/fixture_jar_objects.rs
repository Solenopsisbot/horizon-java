//! Objects a jar allocates are typed handles (`HostKind::JarObject`, key
//! `class#n`), never text a `String` can spell (#156 P12b).
//!
//! `fixtures/jar_objects/Probe.java` is the spec. Its expected strings are
//! what OpenJDK 25.0.5 printed for `demo.Probe` (`javac --release 21`, two runs
//! alike):
//!
//! * `forged`: whether a `String` spelling an object's key is only that
//!   string; its test comes with that change (#156 P12b2).
//! * `identity`: one object keeps one identity through a list, a map, an
//!   array, a lambda capture, an `Optional`, a synchronized view, a copied
//!   list and a static field; two allocations are two objects.
//! * `ownMethods`: a class's own `equals`/`hashCode`/`toString`/`compareTo`
//!   run through the interpreter (sets, sorted sets, `Collections.sort`,
//!   `Objects.hash`); `Object`'s identity `hashCode` is not reproducible, so
//!   it is counted. The elements print through the jar's own loop: a host
//!   collection's `toString` cannot run a jar object's (counted, as before).
//! * `classSurface`: `getClass()` of a jar object and of a JDK object a jar
//!   `new`ed, from the class the key names.
//! * `jdkClasses`: JDK objects a jar `new`ed (`StringBuilder`,
//!   `ThreadPoolExecutor`, `ArrayList`) against JDK types: `instanceof`,
//!   casts, `Class.isInstance`/`isAssignableFrom`/`getSuperclass`/
//!   `getInterfaces` and a type switch, from Java 25's table
//!   (`jdk_closure.rs`).
//! * `jarLabel`: a jar object against type-switch labels its held chain rules
//!   out: an exact "no", nothing counted.
//! * `outsideTable`: the same questions about a JDK class outside that table
//!   (`Phaser`): each is counted, never a made-up `false` or `Object` (Java
//!   25 printed `falsetruefalsefalseclass java.lang.Object0other`).
//!
//! `Woven.java` is the one case a plain JVM cannot run: an interface a Mixin
//! adds to a jar class. Mixin merges it into the class, so a woven object is an
//! instance of it.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::{StdHost, Value};
use javac_fixture::Fixture;

const PROBE: &str = include_str!("fixtures/jar_objects/Probe.java");

const WOVEN: &str = r#"package demo;

import org.spongepowered.asm.mixin.Mixin;

public final class Woven {
  public static class Target {}

  public interface Added {}

  @Mixin(Target.class)
  abstract static class TargetMixin implements Added {}

  /** Mixin merges `Added` into `Target`: a woven `Target` is an `Added`. */
  public static String mixin() {
    Object target = new Target();
    Added[] array = new Added[1];
    array[0] = (Added) target;
    return "" + (target instanceof Added) + Added.class.isInstance(target) + (array[0] == target);
  }
}
"#;

const MIXIN_STUB: &str = r#"package org.spongepowered.asm.mixin;

import java.lang.annotation.*;

// Compile-only stub of Mixin's annotation (class retention, as Mixin's own).
@Retention(RetentionPolicy.CLASS)
@Target(ElementType.TYPE)
public @interface Mixin {
    Class<?>[] value() default {};

    String[] targets() default {};
}
"#;

fn fixture() -> Fixture {
    Fixture::build(
        "jar-objects",
        &[
            ("demo/Probe.java", PROBE),
            ("demo/Woven.java", WOVEN),
            ("org/spongepowered/asm/mixin/Mixin.java", MIXIN_STUB),
        ],
    )
}

#[test]
fn an_object_keeps_one_identity_on_every_path() {
    let fixture = fixture();
    let mut host = fixture.jar.seed_host(StdHost::new());
    assert_eq!(
        fixture.text_on(&mut host, "identity"),
        "truetruetruetruetruetruetruetruetrue|falsefalsefalsetrue"
    );
    assert_eq!(host.unknown_host_calls(), Vec::<(String, usize)>::new());
}

#[test]
fn an_object_runs_its_own_methods_and_counts_the_identity_hash() {
    let fixture = fixture();
    let mut host = fixture.jar.seed_host(StdHost::new());
    assert_eq!(
        fixture.text_on(&mut host, "ownMethods"),
        "true,false,true,62,3038,true,Bar5 Bar3 Bar2 ,Bar5 Bar3 Bar2 ,1,Foo(9),Bar2,true"
    );
    // `foo.hashCode()` twice: `Object`'s identity hash, which no run can
    // reproduce.
    assert_eq!(
        host.unknown_host_calls(),
        vec![("java/lang/Object#hashCode()I".to_owned(), 2)]
    );
}

#[test]
fn get_class_names_the_class_the_key_names() {
    let fixture = fixture();
    let mut host = fixture.jar.seed_host(StdHost::new());
    assert_eq!(
        fixture.text_on(&mut host, "classSurface"),
        "demo.Probe$Foo,Foo,java.lang.Object,true,java.util.concurrent.ThreadPoolExecutor,true,\
         java.util.ArrayList,truetruetrue"
    );
}

#[test]
fn a_mixin_added_interface_holds_for_a_jar_object() {
    let fixture = fixture();
    let mut host = fixture.jar.seed_host(StdHost::new());
    let answer = fixture
        .jar
        .invoke_method_with_args(
            "demo/Woven",
            "mixin",
            "()Ljava/lang/String;",
            None,
            &[],
            &mut host,
        )
        .unwrap();
    assert_eq!(answer, Value::Ref(Some("truetruetrue".into())));
    assert_eq!(host.unknown_host_calls(), Vec::<(String, usize)>::new());
}

#[test]
fn jdk_objects_answer_from_java_25s_table() {
    let fixture = fixture();
    let mut host = fixture.jar.seed_host(StdHost::new());
    assert_eq!(
        fixture.text_on(&mut host, "jdkClasses"),
        "truetruefalse,truetruetruefalse,truefalse,falsetruefalsetrue,truefalse,truefalsetrue,\
         java.util.concurrent.AbstractExecutorService,java.lang.AbstractStringBuilder,true,,\
         java.lang.Appendable java.io.Serializable java.lang.Comparable java.lang.CharSequence ,\
         servicetextother"
    );
    assert_eq!(host.unknown_host_calls(), Vec::<(String, usize)>::new());
}

#[test]
fn a_jdk_class_outside_the_table_is_counted() {
    let fixture = fixture();
    let mut host = fixture.jar.seed_host(StdHost::new());
    fixture.text_on(&mut host, "outsideTable");
    let counted: Vec<String> = host
        .unknown_host_calls()
        .into_iter()
        .map(|(call, _)| call)
        .collect();
    for call in [
        "java/lang/Runnable#<instanceof>(Ljava/lang/Object;)Z",
        "java/lang/Runnable#<checkcast>(Ljava/lang/Object;)Ljava/lang/Runnable;",
        "java/lang/Class#isAssignableFrom(Ljava/lang/Class;)Z",
        "java/lang/Class#getSuperclass()Ljava/lang/Class;",
        "java/lang/Class#getInterfaces()[Ljava/lang/Class;",
    ] {
        assert!(
            counted.iter().any(|c| c == call),
            "{call} not counted: {counted:?}"
        );
    }
}

#[test]
fn a_type_switch_on_a_jar_object_rules_labels_out_exactly() {
    let fixture = fixture();
    let mut host = fixture.jar.seed_host(StdHost::new());
    assert_eq!(fixture.text_on(&mut host, "jarLabel"), "otherother");
    assert_eq!(host.unknown_host_calls(), Vec::<(String, usize)>::new());
}

//! `Class` objects are handles, one per name in a run (#156 P13b1).
//!
//! `fixtures/class_objects/Probe.java` is the spec. Its expected strings are
//! what OpenJDK 25.0.5 printed for `demo.Probe` (`javac --release 21`):
//!
//! * `identity`: `Foo.class`, `new Foo().getClass()` and
//!   `Class.forName("demo.Probe$Foo")` are one object; so are a JDK class's
//!   literal, `getClass()` and `forName`; two classes are two objects.
//! * `primitives`: `int.class == Integer.TYPE` (and `void`, `boolean`), apart
//!   from the wrapper; names and `toString` are the keyword.
//! * `arrays`: `int[].class == new int[0].getClass()`, `getComponentType()` is
//!   the component's object; names `[I`, `[Ljava.lang.String;`, type name
//!   `java.lang.String[][]`, simple name `int[]`, `toString` `class [I`.
//! * `names`: a nested class's binary, simple, type and package names, and
//!   `toString` `class X` / `interface X` for jar and JDK classes.
//! * `jdk`: JDK classes answer from Java 25's supertype table
//!   (`jdk_closure.rs`): superclass, `isInterface`, `isAssignableFrom`,
//!   `isInstance`, `getInterfaces`.
//! * `members`: a jar class's members from its class file: `isInterface`,
//!   `getSuperclass`, `getInterfaces`, `isAssignableFrom`, `isInstance`,
//!   `cast` (and its `ClassCastException`), `getEnumConstants`,
//!   `desiredAssertionStatus`, `getModifiers` (a nested class's from its
//!   `InnerClasses` row).
//!
//! `Copy.java` runs on a jar that ships its own copy of a NeoForge loader
//! class (#124, `class_precedence.rs`): the copy's `Class` is the one every
//! producer answers. `Woven` is the one case a plain JVM cannot run: Mixin
//! merges an added interface into its target, so the woven class's `Class`
//! answers it.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::{StdHost, Value};
use javac_fixture::Fixture;

const PROBE: &str = include_str!("fixtures/class_objects/Probe.java");
const COPY: &str = include_str!("fixtures/class_objects/Copy.java");
const TRACKER_COPY: &str = include_str!("fixtures/class_objects/ConfigTracker.java");

const WOVEN: &str = r#"package demo;

import java.util.Arrays;
import org.spongepowered.asm.mixin.Mixin;

public final class Woven {
  public static class Target {}

  public interface Added {}

  @Mixin(Target.class)
  abstract static class TargetMixin implements Added {}

  /** Mixin merges `Added` into `Target`. */
  public static String mixin() {
    Object target = new Target();
    return ""
        + Added.class.isAssignableFrom(Target.class)
        + Added.class.isInstance(target)
        + (Added.class.cast(target) == target)
        + Arrays.asList(Target.class.getInterfaces()).contains(Added.class)
        + (target.getClass() == Target.class);
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
        "class-objects",
        &[
            ("demo/Probe.java", PROBE),
            ("demo/Woven.java", WOVEN),
            ("org/spongepowered/asm/mixin/Mixin.java", MIXIN_STUB),
        ],
    )
}

/// `demo.Probe.<method>()` and the calls the host counted.
fn run(method: &str) -> (String, Vec<(String, usize)>) {
    let fixture = fixture();
    let mut host = fixture.jar.seed_host(StdHost::new());
    let text = fixture.text_on(&mut host, method);
    (text, host.unknown_host_calls())
}

#[test]
fn every_producer_answers_one_object_per_class() {
    // `Foo.class.hashCode() == byName.hashCode()`: `Class`'s identity hash,
    // which no run can reproduce, is counted (both calls).
    assert_eq!(
        run("identity"),
        (
            "111111111111".to_owned(),
            vec![("java/lang/Object#hashCode()I".to_owned(), 2)]
        )
    );
}

#[test]
fn primitive_classes_are_their_own_objects() {
    assert_eq!(
        run("primitives"),
        ("111111111,int,void,int,long,double".to_owned(), Vec::new())
    );
}

#[test]
fn array_classes_are_one_object_per_component() {
    assert_eq!(
        run("arrays"),
        (
            "1111111111,[I,[Ljava.lang.String;,[[Ljava.lang.String;,java.lang.String[][],int[],\
             class [I,[Ldemo.Probe$Foo;,java.lang.Object"
                .to_owned(),
            Vec::new()
        )
    );
}

#[test]
fn a_class_prints_its_kind_and_name() {
    assert_eq!(
        run("names"),
        (
            "demo.Probe$Foo,Foo,demo.Probe$Foo,demo,class demo.Probe$Foo,\
             interface demo.Probe$Shape,class demo.Probe,interface demo.Probe$Shape,\
             interface java.util.List,class java.lang.String"
                .to_owned(),
            Vec::new()
        )
    );
}

#[test]
fn jdk_classes_answer_from_the_java_25_table() {
    assert_eq!(run("jdk"), ("11111111111111".to_owned(), Vec::new()));
}

#[test]
fn a_jar_class_answers_from_its_class_file() {
    assert_eq!(
        run("members"),
        ("1111111111111111,2,GREEN,9,1545,1".to_owned(), Vec::new())
    );
}

#[test]
fn a_jars_own_copy_of_a_loader_class_is_the_class() {
    let fixture = Fixture::build_every_package(
        "class-objects-copy",
        &[
            (
                "demo/Probe.java",
                COPY.replace("class Copy", "class Probe").as_str(),
            ),
            ("net/neoforged/fml/config/ConfigTracker.java", TRACKER_COPY),
        ],
    );
    let mut host = fixture.jar.seed_host(StdHost::new());
    assert_eq!(
        fixture.text_on(&mut host, "copy"),
        "truetruetrue,net.neoforged.fml.config.ConfigTracker,count,2,\
         class net.neoforged.fml.config.ConfigTracker"
    );
    assert_eq!(host.unknown_host_calls(), Vec::<(String, usize)>::new());
}

#[test]
fn a_mixin_added_interface_is_the_woven_classs() {
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
    assert_eq!(answer, Value::Ref(Some("truetruetruetruetrue".into())));
    assert_eq!(host.unknown_host_calls(), Vec::<(String, usize)>::new());
}

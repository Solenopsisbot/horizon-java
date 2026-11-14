//! A static field an interface declares, read through a class or interface
//! that inherits it (JVMS 5.4.3.2: the class, then its superinterfaces
//! recursively, then its superclass).
//!
//! `fixtures/interface_fields/Probe.java` is the spec: OpenJDK 25.0.5
//! (`javac --release 21`) printed `[    ][    ][    ]2BASE    x`, and its
//! `getstatic`s name `Probe$Object1.INDENT`, `Probe$Sub.INDENT`,
//! `Probe$Nested.INDENT` and `Probe$Sub.DEPTH`, none of which declares the
//! field. resourcefulconfig's `JsoncObject` reads `JsoncElement.INDENT` this
//! way when it writes a config file.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::StdHost;
use javac_fixture::Fixture;

#[test]
fn an_interface_field_resolves_through_the_classes_that_inherit_it() {
    let fixture = Fixture::build(
        "interface-fields",
        &[(
            "demo/Probe.java",
            include_str!("fixtures/interface_fields/Probe.java"),
        )],
    );
    let mut host = fixture.jar.seed_host(StdHost::new());
    assert_eq!(
        fixture.text_on(&mut host, "fields"),
        "[    ][    ][    ]2BASE    x"
    );
    assert_eq!(host.unknown_host_calls(), Vec::<(String, usize)>::new());
}

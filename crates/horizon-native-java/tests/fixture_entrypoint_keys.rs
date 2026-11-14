//! One object per Fabric entrypoint key (#219's review).
//!
//! `fixtures/entrypoint_keys/Probe.java` is both the `main` and the `client`
//! entrypoint. Fabric Loader 0.19.5's `EntrypointStorage.add` gives each key
//! its own entry, whose instance map makes its own object, so the loader
//! builds two `Probe`s: `main`'s ran `onInitialize`, `client`'s
//! `onInitializeClient`, and `getEntrypoints(key, ..)` answers each key's own.
//!
//! Expected: what Fabric Loader 0.19.5's own `EntrypointStorage` printed on
//! OpenJDK 25.0.5 (`javac --release 21`, two runs alike), with the mod built
//! as `setupMods` does (a plain candidate with builtin metadata), the
//! default adapter's no-argument construction, `main` then `client` run as
//! `invokeEntrypoints` does: `2,true,true,false,1,10`.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::StdHost;
use javac_fixture::Fixture;

const PROBE: &str = include_str!("fixtures/entrypoint_keys/Probe.java");

// Compile-only stubs of the loader API the probe names.
const MOD_INITIALIZER: &str =
    "package net.fabricmc.api; public interface ModInitializer { void onInitialize(); }";
const CLIENT_MOD_INITIALIZER: &str = "package net.fabricmc.api; \
     public interface ClientModInitializer { void onInitializeClient(); }";
const FABRIC_LOADER: &str = "package net.fabricmc.loader.api; import java.util.List; \
     public interface FabricLoader { static FabricLoader getInstance() { return null; } \
     <T> List<T> getEntrypoints(String key, Class<T> type); }";

const MOD_JSON: &str = r#"{"id":"probe-fixture","version":"1.0","entrypoints":{"main":["demo.Probe"],"client":["demo.Probe"]}}"#;

#[test]
fn each_entrypoint_key_has_its_own_object() {
    let fixture = Fixture::build_with_mod_json(
        "entrypoint-keys",
        &[
            ("demo/Probe.java", PROBE),
            ("net/fabricmc/api/ModInitializer.java", MOD_INITIALIZER),
            (
                "net/fabricmc/api/ClientModInitializer.java",
                CLIENT_MOD_INITIALIZER,
            ),
            ("net/fabricmc/loader/api/FabricLoader.java", FABRIC_LOADER),
        ],
        MOD_JSON,
    );
    let mut host = fixture.jar.seed_host(StdHost::new());
    for kind in ["main", "client"] {
        fixture
            .jar
            .execute("demo.Probe", None, kind, &mut host)
            .unwrap_or_else(|error| panic!("{kind} entrypoint failed: {error}"));
    }
    assert_eq!(
        fixture.text_on(&mut host, "fromLoader"),
        "2,true,true,false,1,10"
    );
    assert_eq!(host.unknown_host_calls(), Vec::<(String, usize)>::new());
}

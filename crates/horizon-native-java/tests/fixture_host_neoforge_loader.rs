//! NeoForge's loader objects are typed handles (`Value::Host`, #156 P6
//! second half, `host_neoforge_loader.rs`): `ModContainer`, `IModInfo`,
//! `ModConfig.Type` and the detected `WorldVersion`, on a `javac` fixture.
//!
//! `fixtures/neoforge_loader/demo/Loader.java` runs unchanged on both sides:
//! Horizon passes what `ModList.get()` answers for mods `demo` (1.2.3, a
//! `javafml` mod of the running jar) and `other` (4.5); the oracle passes
//! FancyModLoader 12.0.12's own objects (see [`ORACLE`]). The stubs under
//! `fixtures/neoforge_loader/stubs/` exist only for `javac`.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::StdHost;

/// What `NeoOracle` printed on Java 25 (Temurin 25.0.5, `javac --release
/// 21`), the same on two runs. It builds FancyModLoader 12.0.12's objects
/// the way the loader does: a `ModInfo` per mod from its `[[mods]]` table
/// (`modId`, `version`), a container per mod (`MinecraftModContainer`: the
/// probes reach only `ModContainer`'s own final `getModId`/`getModInfo`),
/// and `ModList.of(files, infos)` with `setLoadedMods(containers)`; the
/// game's version is the real 26.3 server's `DetectedVersion`.
const ORACLE: &str = "\
containers=demoother|truetruefalsetrueMdemotrue|truetruefalsedemo1.2.34.5Idemo|falsetruefalsetrue|true1anulli1
configTypes=CLIENT|CLIENT13clientstartup|truetruefalse-23|truetruetrueT3|[LOCAL, CLIENT, SYNCED, STARTUP]truecnullCLIENT1
worldVersion=26.3|truetruetruetruetrue
worldIdentity=false
spelled=falsefalsefalsefalsefalseneoforge-mod-container/demo/1.2.3!,falsefalsefalsefalsefalsemod-info/demo/1.2.3!,falsefalsefalsefalsefalsemod-file-info/demo!,falsefalsefalsefalsefalsefml-config-type/CLIENT!,falsefalsefalsefalsefalseworld-version/26.3!,Stringtrue-1591840437falsefalsefalsefalsefalsenfmc:rdemo,i1!,Stringtrue-686962580falsefalsefalsefalsefalsenfct:rCLIENT!,Stringtrue-647828147falsefalsefalsefalsefalsewver:r26.3!,8sfalse
forged=CCECCECCECCE
";

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/neoforge_loader"
);

/// The running jar's `neoforge.mods.toml`: `demo` is a `javafml` mod.
const MODS_TOML: &[u8] = br#"modLoader="javafml"
loaderVersion="[1,)"
license="MIT"
[[mods]]
modId="demo"
version="1.2.3"
"#;

fn expected(method: &str) -> &'static str {
    ORACLE
        .lines()
        .find_map(|line| line.strip_prefix(method)?.strip_prefix('='))
        .unwrap_or_else(|| panic!("no oracle line for {method}"))
}

fn sources() -> Vec<(String, String)> {
    fn walk(dir: &std::path::Path, root: &std::path::Path, out: &mut Vec<(String, String)>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(&path, root, out);
            } else {
                let name = path
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                out.push((name, std::fs::read_to_string(&path).unwrap()));
            }
        }
    }
    let mut out = Vec::new();
    let root = std::path::Path::new(FIXTURE);
    walk(&root.join("demo"), root, &mut out);
    walk(&root.join("stubs"), &root.join("stubs"), &mut out);
    out
}

/// Run `method` on a host seeded with the two mods and the game version:
/// its text and what it counted.
fn run(test: &str, method: &str) -> (String, Vec<(String, usize)>) {
    let sources = sources();
    let sources = sources
        .iter()
        .map(|(name, text)| (name.as_str(), text.as_str()))
        .collect::<Vec<_>>();
    let fixture = javac_fixture::Fixture::build_with_lib(
        test,
        &sources,
        &[("META-INF/neoforge.mods.toml", MODS_TOML)],
    );
    let seed = StdHost::new()
        .with_game_version("26.3")
        .with_mod("demo", "1.2.3")
        .with_mod("other", "4.5");
    let mut host = fixture.jar.seed_host(seed);
    let text = fixture.text_on(&mut host, method);
    (text, host.unknown_host_calls())
}

/// `ModList.getModContainerById` answers the mod's one container, whose
/// `getModInfo` is its one `IModInfo` (`==`, `equals`, `IdentityHashMap`,
/// host lists, a type switch, `Class.isInstance`); `ModConfig.Type`'s
/// constants are one object each and in declaration order (`compareTo`,
/// `Collections.sort`, `TreeSet`, `Map.of`), and two detected versions are
/// two equal records: all as Java 25 answers, with nothing counted.
#[test]
fn the_loaders_objects_answer_as_java_25() {
    for method in ["containers", "configTypes", "worldVersion"] {
        let (text, unknown) = run(method, method);
        assert_eq!(text, expected(method), "{method}: {unknown:?}");
        assert_eq!(unknown, Vec::<(String, usize)>::new(), "{method}");
    }
}

/// Two `DetectedVersion.tryDetectVersion()` results are two records
/// (`false` on Java 25); the host's two handles have one key, so their
/// identity is Java's or counted, never a silent `true`.
#[test]
fn two_detected_versions_are_never_silently_one_object() {
    let (text, unknown) = run("world-identity", "worldIdentity");
    assert!(
        text == expected("worldIdentity") || !unknown.is_empty(),
        "{text}"
    );
}

/// A string spelled like an old marker (`neoforge-mod-container/…`,
/// `mod-info/…`, `mod-file-info/…`, `fml-config-type/…`,
/// `world-version/…`) or like a handle's key is a `String`: its class, hash,
/// `equals` both ways and `HashMap` keying are `String`'s. The container's
/// identity hash (`map.containsKey(container)`) is counted.
#[test]
fn no_string_spells_a_loader_object() {
    let (text, unknown) = run("spelled", "spelled");
    assert_eq!(text, expected("spelled"), "{unknown:?}");
    assert!(
        unknown
            .iter()
            .all(|(call, _)| call.ends_with("#hashCode()I")),
        "{unknown:?}"
    );
}

/// A `String` cast to `ModContainer`, `ModConfig.Type`, `IModInfo` or
/// `WorldVersion` is a `ClassCastException` on Java 25. Horizon's cast of
/// marker-shaped text is still a counted pass until #156 P17; whatever it
/// answers, the string never drives the object it spells.
#[test]
fn a_string_cast_to_a_loader_type_is_never_the_object() {
    let (text, unknown) = run("forged", "forged");
    if text != expected("forged") {
        assert!(!unknown.is_empty(), "{text}");
        for spelled in ["demo", "CLIENT", "26.3"] {
            assert!(!text.contains(spelled), "{text} {unknown:?}");
        }
    }
}

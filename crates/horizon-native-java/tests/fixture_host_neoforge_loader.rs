//! NeoForge's loader objects are typed handles (`Value::Host`, #156 P6
//! second half, `host_neoforge_loader.rs`): `ModContainer`, `IModInfo`,
//! `ModConfig.Type` and the detected `WorldVersion`, on a `javac` fixture.
//!
//! `fixtures/neoforge_loader/demo/Loader.java` runs unchanged on both sides:
//! Horizon passes what `ModList.get()` (and `FMLLoader`'s `LoadingModList`)
//! answer for a run of: the running jar, a `javafml` file of `demo` 1.2.3 and
//! `second` 2.0; a linked NeoForge jar of `lib` 1.4 and `lib_core` 1.4.1;
//! `other` 4.5; the game 26.3. The oracle passes FancyModLoader 12.0.12's own
//! objects for the same files (see [`ORACLE`]). The stubs under
//! `fixtures/neoforge_loader/stubs/` exist only for `javac`.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::StdHost;

/// What `NeoOracle` printed on Java 25 (Temurin 25.0.5, `javac --release
/// 21`), the same on two runs. It builds FancyModLoader 12.0.12's objects
/// the way the loader does: a real `ModFileInfo` per file from its mods.toml
/// tables (which builds the real `ModInfo`s; its `ModFile` is only
/// allocated), a container per mod (`MinecraftModContainer`: the probes reach
/// only `ModContainer`'s own final `getModId`/`getModInfo`), then
/// `LoadingModList.of`, `ModList.of(files, infos)` and
/// `setLoadedMods(containers)`; the game's version is the real 26.3 server's
/// `DetectedVersion`. The files: `demo` + `second`, `lib` + `lib_core`,
/// `other`, `minecraft`, `neoforge`.
const ORACLE: &str = "\
containers=demoother|truetruefalsetrueMdemotrue|truetruefalsedemo1.2.34.5Idemo|falsetruefalsetruetruetrue|true1anulli1
mods=[demo, lib, lib_core, minecraft, neoforge, other, second]|true
files=truetruetruetruetruetruefalsefalsefnulltruefalse|1.2.3|1.4|1.2.3|true
switched=y3trueLOCAL0CLIENT1SYNCED2STARTUP34true
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

/// The running jar's `neoforge.mods.toml`: `demo` and `second`, `javafml`.
const MODS_TOML: &[u8] = br#"modLoader="javafml"
loaderVersion="[1,)"
license="MIT"
[[mods]]
modId="demo"
version="1.2.3"
[[mods]]
modId="second"
version="2.0"
"#;

/// The linked library jar's: `lib` and `lib_core` in one file.
const LIB_TOML: &[u8] = br#"modLoader="javafml"
loaderVersion="[1,)"
license="MIT"
[[mods]]
modId="lib"
version="1.4"
[[mods]]
modId="lib_core"
version="1.4.1"
"#;

/// A NeoForge jar holding only `toml`.
fn neoforge_jar(toml: &[u8]) -> horizon_native_java::NativeJar {
    use std::io::Write;
    let mut bytes = std::io::Cursor::new(Vec::new());
    let mut zip = zip::ZipWriter::new(&mut bytes);
    zip.start_file(
        "META-INF/neoforge.mods.toml",
        zip::write::SimpleFileOptions::default(),
    )
    .unwrap();
    zip.write_all(toml).unwrap();
    zip.finish().unwrap();
    horizon_native_java::NativeJar::from_bytes(bytes.get_ref()).unwrap()
}

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
        .with_mod("other", "4.5")
        .with_mod("dep", "unknown");
    let mut host = fixture.jar.seed_host(seed);
    host.link_jar("lib", &neoforge_jar(LIB_TOML)).unwrap();
    let text = fixture.text_on(&mut host, method);
    (text, host.unknown_host_calls())
}

/// `ModList.getModContainerById` answers the mod's one container (the
/// running jar's second `[[mods]]` and a linked jar's second mod too), whose
/// `getModInfo` is its one `IModInfo` (`==`, `equals`, `IdentityHashMap`,
/// host lists, a type switch, `Class.isInstance`); `getModFileById` is the
/// mod's *file*, one object for two mods of one file, whose `versionString`
/// is its first mod's version; `ModConfig.Type`'s constants are one object
/// each and in declaration order (`compareTo`, `Collections.sort`,
/// `TreeSet`, `Map.of`, an enum `switch`, `values()`, `valueOf`), and two
/// detected versions are two equal records: all as Java 25 answers, with
/// nothing counted.
#[test]
fn the_loaders_objects_answer_as_java_25() {
    for method in [
        "containers",
        "files",
        "switched",
        "configTypes",
        "worldVersion",
    ] {
        let (text, unknown) = run(method, method);
        assert_eq!(text, expected(method), "{method}: {unknown:?}");
        assert_eq!(unknown, Vec::<(String, usize)>::new(), "{method}");
    }
}

/// `ModList.getMods()` lists every mod of the game in `ModSorter`'s order,
/// which the host cannot know (other jars): the call is counted, and the list
/// is never short of a mod the host knows (the running jar's two, the linked
/// jar's two, `other`, `minecraft`, `neoforge`), and holds the container's
/// own `IModInfo`.
#[test]
fn get_mods_is_counted_and_never_short() {
    let (text, unknown) = run("mods", "mods");
    assert!(
        unknown
            .iter()
            .any(|(call, _)| call == "net/neoforged/fml/ModList#getMods()Ljava/util/List;"),
        "{unknown:?}"
    );
    let (java_ids, java_holds) = expected("mods").split_once('|').unwrap();
    let (ids, holds) = text.split_once('|').unwrap();
    assert_eq!(holds, java_holds, "{text}");
    for id in java_ids.trim_matches(['[', ']']).split(", ") {
        assert!(
            ids.trim_matches(['[', ']'])
                .split(", ")
                .any(|listed| listed == id),
            "{id} missing from {text}"
        );
    }
}

/// `getModFileById` of a mod the host only assumes present (a dependency
/// seeded as `unknown`, whose file it does not know) is counted, never a
/// file handle of its own: two such mods may share a file.
#[test]
fn an_unknown_file_is_counted() {
    let (_, unknown) = run("dependency-file", "dependencyFile");
    assert!(
        unknown
            .iter()
            .any(|(call, _)| call
                .starts_with("net/neoforged/fml/loading/LoadingModList#getModFileById")),
        "{unknown:?}"
    );
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

//! Fabric Loader's and Fabric API's library singletons are typed handles
//! (`Value::Host`, #156 P6c, `host_fabric_runtime.rs`): the loader, its
//! `MappingResolver`, `EnvType`, entrypoint containers and instances, the
//! `PayloadTypeRegistry`s and the `ResourceLoader`s, on a `javac` fixture.
//!
//! `fixtures/fabric_runtime/demo/Loader.java` runs unchanged on both sides:
//! Horizon passes what `FabricLoader.getInstance()` answers on a dedicated
//! server whose run declares the `main` entrypoint `demo.Probe` of mod
//! `probe-fixture` (the fixture jar's own `fabric.mod.json`); the oracle
//! passes Fabric Loader 0.19.5's own objects (see [`ORACLE`]). The stubs under
//! `fixtures/fabric_runtime/stubs/` exist only for `javac`.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::StdHost;

/// What `FabricOracle` printed on Java 25 (Temurin 25.0.5, `javac --release
/// 21`), the same on two runs. It runs Fabric Loader 0.19.5's own
/// `FabricLoaderImpl.INSTANCE` on a dedicated server: a launcher answering
/// the side, the mapping configuration (runtime namespace `official`) and
/// class loading, and the mod `probe-fixture` (`ModContainerImpl` from
/// `ModCandidateImpl.createPlain`) added to the loader's mod map and to its
/// `EntrypointStorage` with the default language adapter, as `setupMods`
/// does, for `"entrypoints":{"main":["demo.Probe"]}` (Fabric's parser takes
/// only the list form; the fixture jar's string form means the same). Fabric
/// API 0.165.0's `fabric-api-base`, `fabric-networking-api-v1` and
/// `fabric-resource-loader-v1` and the 26.3 server are on the class path.
const ORACLE: &str = "\
loader=truetruetruetruetruetrueartruefalse|truetruefalse
envType=CLIENT|CLIENT01-1sCLIENT0SERVER12true|truetruetruefalsetrue|[CLIENT, SERVER]CLIENTcnull11
payloads=truefalsefalsetruetruepnulltrue
resources=truefalsetruetruednull
entrypoints=1trueprobe-fixturetruetrue0
entrypointIdentity=falsefalsetruetrue
spelled=Stringtrue-1040676293falsefalsefalsefalsefalsefalsefalsefalsefabric-loader!,Stringtrue-1221724595falsefalsefalsefalsefalsefalsefalsefalsefabric-mapping-resolver!,Stringtrue1990584267falsefalsefalsefalsefalsefalsefalsefalseCLIENT!,Stringtrue-1852497085falsefalsefalsefalsefalsefalsefalsefalseSERVER!,Stringtrue3145783falsefalsefalsefalsefalsefalsefalsefalsefli:!,Stringtrue-952377214falsefalsefalsefalsefalsefalsefalsefalsefenv:rSERVER!,Stringtrue1041408092falsefalsefalsefalsefalsefalsefalsefalsefpt:rplayToClient!,Stringtrue-116919410falsefalsefalsefalsefalsefalsefalsefalsefrl:rSERVER_DATA!,falsefalsefalsefabric-entrypoint/main/demo/Probe/probe-fixture!,falsefalsefalsefabric-payload-types/playToClient!,falsefalsefalsefabric-resource-loader/SERVER_DATA!,falsefalsefalsefabric-entrypoint-instance/demo/Probe!,12sfalse
forged=CCECCE
";

const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/fabric_runtime");

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

/// Run `method` on a dedicated server's host seeded from the fixture jar
/// (`probe-fixture`, entrypoint `main` = `demo.Probe`): its text and what it
/// counted.
fn run(test: &str, method: &str) -> (String, Vec<(String, usize)>) {
    let sources = sources();
    let sources = sources
        .iter()
        .map(|(name, text)| (name.as_str(), text.as_str()))
        .collect::<Vec<_>>();
    let fixture = javac_fixture::Fixture::build(test, &sources);
    let mut host = fixture
        .jar
        .seed_host(StdHost::new().with_environment_type("SERVER"));
    let text = fixture.text_on(&mut host, method);
    (text, host.unknown_host_calls())
}

/// The loader and its mapping resolver are one object each (`==`, `equals`,
/// `IdentityHashMap`, host lists, `instanceof`); `getEnvironmentType()` is the
/// `EnvType` constant itself; `EnvType`'s constants are one object each in
/// declaration order (`name`, `ordinal`, `compareTo`, a `switch`, `values()`,
/// `valueOf`, `Collections.sort`, `TreeSet`, `Map.of`); the payload type
/// registries and resource loaders are one per direction and per `PackType`;
/// an entrypoint container's provider is the mod's one container and its
/// instance an instance of the entrypoint class: all as Java 25 answers, with
/// nothing counted.
#[test]
fn the_singletons_answer_as_java_25() {
    for method in ["loader", "envType", "payloads", "resources", "entrypoints"] {
        let (text, unknown) = run(method, method);
        assert_eq!(text, expected(method), "{method}: {unknown:?}");
        assert_eq!(unknown, Vec::<(String, usize)>::new(), "{method}");
    }
}

/// `getEntrypointContainers` builds new containers on every call, and an
/// entry's instance is cached per requested type, which the host's key does
/// not hold: identity between two calls' containers or instances is Java
/// 25's or counted, never a silent answer from equal keys.
#[test]
fn entrypoint_identity_is_java_25s_or_counted() {
    let (text, unknown) = run("entrypoint-identity", "entrypointIdentity");
    assert!(
        text == expected("entrypointIdentity") || !unknown.is_empty(),
        "{text}"
    );
}

/// A string spelled like an old marker (`fabric-loader`, `CLIENT`,
/// `SERVER`, `fabric-entrypoint/...`, ...) or like a handle's key is a
/// `String`: its class, hash, `instanceof`, `equals` both ways, `==` against
/// `EnvType.SERVER` and the loader's environment type, and `HashMap` keying
/// are `String`'s. (`SERVER` *was* `EnvType.SERVER` before this change.)
#[test]
fn no_string_spells_a_singleton() {
    let (text, unknown) = run("spelled", "spelled");
    assert_eq!(text, expected("spelled"), "{unknown:?}");
    // An old `word/rest` marker's `instanceof` against a loader type is
    // still read as a marker until #156 P17.
    assert!(
        unknown.iter().all(
            |(call, _)| call.ends_with("#<instanceof>(Ljava/lang/Object;)Z")
                || call.ends_with("#hashCode()I")
        ),
        "{unknown:?}"
    );
}

/// A `String` cast to `EnvType` or `FabricLoader` is a `ClassCastException`
/// on Java 25. Horizon's cast of text is still a counted pass until #156 P17;
/// whatever it answers, the string never drives the object it spells.
#[test]
fn a_string_cast_to_a_singleton_type_is_never_the_object() {
    let (text, unknown) = run("forged", "forged");
    if text != expected("forged") {
        assert!(!unknown.is_empty(), "{text}");
        for spelled in ["1", "SERVER"] {
            assert!(!text.contains(spelled), "{text} {unknown:?}");
        }
    }
}

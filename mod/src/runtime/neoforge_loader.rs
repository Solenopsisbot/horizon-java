//! The NeoForge loader surface a mod reaches during bootstrap: the physical
//! side, the loader's directory handles, the mod list and its containers.
//!
//! Every answer is a fact the loader exposes, not a guess: the side is the
//! host's own role, the directories are the same virtual roots Fabric's loader
//! answers, a mod container exists exactly for an id the jar's metadata named
//! (plus the always-loaded ids), and a game version is whatever the caller
//! seeded. A question outside that set is left to the rest of the dispatch
//! chain, which still reports and refuses it.

use crate::runtime::host_neoforge_loader as loader;
use crate::runtime::host_path::FML_PATHS_CLASS;
use crate::runtime::{
    AnnotationValue, Method, MethodRef, NativeJar, StdHost, Value, expect_ref, expect_string,
    is_always_loaded_mod, neutral_args,
};

/// `FMLEnvironment.dist` and `FMLEnvironment.getDist` answer one of these.
const CLIENT: &str = "dist/CLIENT";
const DEDICATED_SERVER: &str = "dist/DEDICATED_SERVER";

/// The virtual roots `FMLPaths` resolves to. They match the Fabric loader's
/// answers, because both runtimes put a mod's files under the same game root.
fn fml_path_root(name: &str) -> Option<&'static str> {
    match name {
        "GAMEDIR" => Some("/horizon/game"),
        "CONFIGDIR" => Some("/horizon/config"),
        "MODSDIR" => Some("/horizon/mods"),
        _ => None,
    }
}

fn dist_value(server: bool) -> Value {
    Value::Ref(Some(if server { DEDICATED_SERVER } else { CLIENT }.into()))
}

/// The concrete descriptor of a loader method `Class.getMethod` can select
/// on a class the jar does not hold, so `Method.invoke` can dispatch it
/// instead of refusing a method the host really models.
///
/// Only the entries the module answers are listed; anything else keeps the
/// old refusal.
pub(crate) fn reflected_static_descriptor(
    owner: &str,
    name: &str,
    arguments: usize,
) -> Option<&'static str> {
    match (owner, name, arguments) {
        ("net/neoforged/fml/loading/FMLEnvironment", "getDist", 0) => {
            Some("()Lnet/neoforged/api/distmarker/Dist;")
        }
        ("net/neoforged/fml/loading/FMLEnvironment", "isProduction", 0) => Some("()Z"),
        ("net/neoforged/fml/ModList", "get", 0) => Some("()Lnet/neoforged/fml/ModList;"),
        _ => None,
    }
}

impl NativeJar {
    /// The arguments an entry class's constructor receives.
    ///
    /// FML constructs a `@Mod` class with its own mod's container: the
    /// `ModContainer` parameter is the container of the mod the annotation
    /// names, so `getModId()` answers that id and `registerConfig` names its
    /// default file after it. Every other parameter (and every parameter of a
    /// class without a readable `@Mod` id) keeps its neutral marker.
    pub(crate) fn entry_constructor_args(&self, class: &str, constructor: &Method) -> Vec<Value> {
        let mut args = neutral_args(&constructor.descriptor);
        let Some(id) = self.mod_annotation_id(class) else {
            return args;
        };
        for arg in &mut args {
            if matches!(arg, Value::Ref(Some(marker)) if marker == "Lnet/neoforged/fml/ModContainer;")
            {
                *arg = self.neoforge_container(id);
            }
            // The bus FML passes is that container's mod bus.
            if matches!(arg, Value::Ref(Some(marker)) if marker == "Lnet/neoforged/bus/api/IEventBus;")
            {
                *arg = crate::runtime::neoforge_bus::mod_bus(Some(id));
            }
        }
        args
    }

    /// Mod `id`'s one container (`host_neoforge_loader.rs`), as
    /// [`StdHost::neoforge_container`] builds it from the same
    /// `neoforge.mods.toml` (the running jar's, which `seed_host` copies).
    pub(crate) fn neoforge_container(&self, id: &str) -> Value {
        loader::container(id, is_javafml(self.resource_bytes.get(MODS_TOML), id))
    }

    /// The id a class's NeoForge `@Mod("id")` annotation names.
    pub(crate) fn mod_annotation_id(&self, class: &str) -> Option<&str> {
        let annotation = self
            .classes
            .get(class)?
            .annotations
            .iter()
            .find(|annotation| annotation.descriptor == "Lnet/neoforged/fml/common/Mod;")?;
        match annotation.values.get("value")? {
            AnnotationValue::String(id) if !id.is_empty() && !id.contains('/') => Some(id),
            _ => None,
        }
    }
}

impl StdHost {
    /// The side-dependent loader constants and statics.
    pub(super) fn neoforge_loader_static(&mut self, r: &MethodRef) -> Option<Value> {
        let server = self.environment_type == "SERVER";
        match (r.owner.as_str(), r.name.as_str(), r.descriptor.as_str()) {
            (
                "net/neoforged/fml/loading/FMLEnvironment",
                "dist",
                "Lnet/neoforged/api/distmarker/Dist;",
            ) => Some(dist_value(server)),
            (
                "net/neoforged/api/distmarker/Dist",
                "CLIENT",
                "Lnet/neoforged/api/distmarker/Dist;",
            ) => Some(Value::Ref(Some(CLIENT.into()))),
            (
                "net/neoforged/api/distmarker/Dist",
                "DEDICATED_SERVER",
                "Lnet/neoforged/api/distmarker/Dist;",
            ) => Some(Value::Ref(Some(DEDICATED_SERVER.into()))),
            (
                "net/neoforged/fml/loading/FMLPaths",
                name,
                "Lnet/neoforged/fml/loading/FMLPaths;",
            ) => fml_path_root(name).map(|_| crate::runtime::host_path::fml_path_value(name)),
            // The loader's one game bus, told apart from each mod's bus:
            // the two accept different events (`neoforge_bus.rs`).
            (
                "net/neoforged/neoforge/common/NeoForge",
                "EVENT_BUS",
                "Lnet/neoforged/bus/api/IEventBus;",
            ) => Some(crate::runtime::neoforge_bus::game_bus()),
            (
                "net/minecraftforge/common/MinecraftForge",
                "EVENT_BUS",
                "Lnet/minecraftforge/eventbus/api/IEventBus;",
            ) => Some(Value::Ref(Some(
                "net/minecraftforge/eventbus/api/IEventBus".into(),
            ))),
            // `ModConfig.Type`'s constants (and older NeoForge's `COMMON`
            // and `SERVER`), one canonical handle each.
            (
                "net/neoforged/fml/config/ModConfig$Type",
                name,
                "Lnet/neoforged/fml/config/ModConfig$Type;",
            ) if loader::CONFIG_TYPE_NAMES.contains(&name) => Some(loader::config_type(name)),
            // The loader's config tracker. A jar reading it sees no config:
            // its three maps are empty rather than null (a null map is an NPE
            // in the port's getters). `registerConfig` keeps its own record
            // (`mod_config_register.rs`), which these reads do not see.
            (
                "net/neoforged/fml/config/ConfigTracker",
                "INSTANCE",
                "Lnet/neoforged/fml/config/ConfigTracker;",
            ) => Some(Value::Ref(Some("fml-config-tracker".into()))),
            (
                "net/neoforged/fml/config/ConfigTracker",
                "configsByMod" | "fileMap",
                "Ljava/util/concurrent/ConcurrentHashMap;"
                | "Ljava/util/Map;"
                | "Ljava/util/concurrent/ConcurrentMap;",
            ) => Some(Value::Ref(Some("map".into()))),
            (
                "net/neoforged/fml/config/ConfigTracker",
                "configSets",
                "Ljava/util/EnumMap;" | "Ljava/util/Map;" | "Ljava/util/Set;",
            ) => Some(Value::Ref(Some("set".into()))),
            // NeoForge's own registries. The attachment registry is a real
            // registry the loader creates, keyed `neoforge:attachment_types`
            // (`NeoForgeRegistries.Keys.ATTACHMENT_TYPES`, plural; this used
            // to answer `attachment_type`). Its rows and the attachment bridge
            // (`neoforge_attachments.rs`) both use the id.
            (
                "net/neoforged/neoforge/registries/NeoForgeRegistries",
                "ATTACHMENT_TYPES",
                "Lnet/minecraft/core/Registry;",
            ) => Some(crate::runtime::host_registry::registry(
                crate::runtime::neoforge_attachments::ATTACHMENT_TYPES_REGISTRY,
            )),
            _ => None,
        }
    }

    /// The loader's static methods: a call with no receiver in the jar.
    pub(super) fn neoforge_loader_call(&mut self, r: &MethodRef, args: &[Value]) -> Option<Value> {
        let server = self.environment_type == "SERVER";
        match (r.owner.as_str(), r.name.as_str(), r.descriptor.as_str()) {
            (
                "net/neoforged/fml/loading/FMLEnvironment",
                "getDist",
                "()Lnet/neoforged/api/distmarker/Dist;",
            ) => Some(dist_value(server)),
            ("net/neoforged/fml/loading/FMLEnvironment", "isProduction", "()Z") => {
                Some(Value::Int(1))
            }
            ("net/neoforged/fml/ModList", "get", "()Lnet/neoforged/fml/ModList;") => {
                Some(Value::Ref(Some("neoforge-mod-list".into())))
            }
            // `ModConfig.Type.valueOf(name)`: the constant, or `Enum.valueOf`'s
            // `IllegalArgumentException` for a name 12.0.12 does not declare.
            (
                "net/neoforged/fml/config/ModConfig$Type",
                "valueOf",
                "(Ljava/lang/String;)Lnet/neoforged/fml/config/ModConfig$Type;",
            ) => match args.first() {
                Some(Value::Ref(Some(name)))
                    if loader::config_type_ordinal(&loader::config_type(name)).is_some() =>
                {
                    Some(loader::config_type(name))
                }
                // Another name throws; left to the chain, which counts it.
                _ => None,
            },
            // `FMLLoader.getCurrent()` is the loader instance the mod asks for
            // its own file record; the mod list hangs off it.
            (
                "net/neoforged/fml/loading/FMLLoader",
                "getCurrent",
                "()Lnet/neoforged/fml/loading/FMLLoader;",
            ) => Some(Value::Ref(Some("fml-loader".into()))),
            (
                "net/neoforged/neoforge/registries/DeferredRegister",
                "createDataComponents",
                "(Lnet/minecraft/resources/ResourceKey;Ljava/lang/String;)Lnet/neoforged/neoforge/registries/DeferredRegister$DataComponents;",
            ) => {
                let namespace = args
                    .get(1)
                    .and_then(expect_ref)
                    .map_or("default", String::as_str);
                let marker = format!("deferred-register/data-component-type/{namespace}");
                let registry_key = args.first().cloned().unwrap_or(Value::Ref(None));
                self.record_deferred_register(
                    &marker,
                    &registry_key,
                    args.get(1).and_then(expect_ref),
                );
                Some(Value::Ref(Some(marker)))
            }
            // The real game reports the running client's own version; the
            // host answers the version it was seeded with, and nothing when
            // the caller did not supply one.
            (
                "net/minecraft/DetectedVersion",
                "tryDetectVersion",
                "()Lnet/minecraft/WorldVersion;",
            ) => Some(
                self.game_version
                    .as_deref()
                    .map_or(Value::Ref(None), loader::world_version),
            ),
            _ => None,
        }
    }

    /// The loader object methods: `FMLPaths.get`, the side test, a mod
    /// container's event bus and info, and the version string behind it.
    pub(super) fn neoforge_loader_instance(
        &mut self,
        r: &MethodRef,
        receiver: &Value,
        args: &[Value],
    ) -> Option<Value> {
        if (r.owner.as_str(), r.name.as_str(), r.descriptor.as_str())
            == (FML_PATHS_CLASS, "get", "()Ljava/nio/file/Path;")
        {
            let name = crate::runtime::host_path::fml_path_name(receiver)?;
            return fml_path_root(name).map(crate::runtime::path_value);
        }
        if let Some(value) = self.neoforge_loader_handle(r, receiver) {
            return Some(value);
        }
        let marker = match receiver {
            Value::Ref(Some(marker)) => marker.as_str(),
            _ => return None,
        };
        match (r.owner.as_str(), r.name.as_str(), r.descriptor.as_str()) {
            ("net/neoforged/api/distmarker/Dist", "isClient", "()Z") => {
                Some(Value::Int(i32::from(marker == CLIENT)))
            }
            ("net/neoforged/api/distmarker/Dist", "isDedicatedServer", "()Z") => {
                Some(Value::Int(i32::from(marker == DEDICATED_SERVER)))
            }
            ("net/neoforged/api/distmarker/Dist", "toString", "()Ljava/lang/String;") => {
                Some(Value::Ref(Some(enum_name(marker).to_owned())))
            }
            // A container's bus when the receiver is no container (FML's
            // own `getEventBus` is only ever reached on one): no mod's.
            (
                "net/neoforged/fml/ModContainer"
                | "net/minecraftforge/fml/ModContainer"
                | FML_MOD_CONTAINER,
                "getEventBus" | "getModEventBus",
                "()Lnet/neoforged/bus/api/IEventBus;",
            ) => Some(crate::runtime::neoforge_bus::mod_bus(None)),
            (
                "org/apache/maven/artifact/versioning/ArtifactVersion",
                "toString",
                "()Ljava/lang/String;",
            ) => Some(Value::Ref(Some(
                marker
                    .strip_prefix("artifact-version/")
                    .unwrap_or(marker)
                    .to_owned(),
            ))),
            (
                "net/neoforged/fml/loading/FMLLoader",
                "getLoadingModList",
                "()Lnet/neoforged/fml/loading/LoadingModList;",
            ) => Some(Value::Ref(Some("fml-loading-mod-list".into()))),
            (
                "net/neoforged/fml/loading/FMLLoader",
                "getDist",
                "()Lnet/neoforged/api/distmarker/Dist;",
            ) => Some(dist_value(self.environment_type == "SERVER")),
            // A mod asks the loader for its own file record to read back the
            // version it was discovered with: `fileById.get(id)`, the one
            // `ModFileInfo` of the file holding `id`. An id the loader does
            // not hold answers null, as FML does; an id the host assumes
            // present without knowing its file is left to the chain (counted).
            (
                "net/neoforged/fml/loading/LoadingModList",
                "getModFileById",
                "(Ljava/lang/String;)Lnet/neoforged/fml/loading/moddiscovery/ModFileInfo;",
            ) => {
                let id = args.first().map(expect_string).unwrap_or_default();
                match self.neoforge_mod_file(id) {
                    Some((file, _)) => Some(loader::mod_file_info(&file)),
                    None if self.neoforge_mod_version(id).is_none() => Some(Value::Ref(None)),
                    None => None,
                }
            }
            _ => None,
        }
    }

    /// Mod `id`'s one container: every call that hands one out (the `@Mod`
    /// constructor, `getModContainerById`, `getMods`, a loading event's
    /// `getContainer`) builds it here or in [`NativeJar::neoforge_container`],
    /// from the same `neoforge.mods.toml`, so its key is the same.
    pub(crate) fn neoforge_container(&self, id: &str) -> Value {
        loader::container(id, is_javafml(self.resource_bytes.get(MODS_TOML), id))
    }

    /// The methods of the loader's handles (`host_neoforge_loader.rs`): a
    /// container's bus, info and extension points, a mod info's id and
    /// version, a file info's version, a config type's name and ordinal, a
    /// world version's name.
    fn neoforge_loader_handle(&mut self, r: &MethodRef, receiver: &Value) -> Option<Value> {
        if let Some(id) = loader::container_id(receiver) {
            return match (r.name.as_str(), r.descriptor.as_str()) {
                // A `javafml` mod's container is an `FMLModContainer`, whose
                // `getEventBus()` is that mod's bus.
                ("getEventBus" | "getModEventBus", "()Lnet/neoforged/bus/api/IEventBus;") => {
                    Some(crate::runtime::neoforge_bus::mod_bus(Some(&id)))
                }
                // `ModContainer.getModId` is final: the id. (`getNamespace`
                // is the mods.toml's override, which the host does not read.)
                ("getModId", "()Ljava/lang/String;") => Some(Value::Ref(Some(id))),
                ("getModInfo", "()Lnet/neoforged/neoforgespi/language/IModInfo;") => {
                    Some(loader::mod_info(&id))
                }
                // `registerConfig` is answered by the runtime's config model
                // (`mod_config_register.rs`) for a spec it built; any other
                // registration reaches the host and is counted.
                (
                    "registerExtensionPoint",
                    "(Ljava/lang/Class;Lnet/neoforged/fml/IExtensionPoint;)V",
                ) => Some(Value::Void),
                _ => None,
            };
        }
        if let Some(id) = loader::mod_info_id(receiver) {
            return match (r.name.as_str(), r.descriptor.as_str()) {
                ("getModId", "()Ljava/lang/String;") => Some(Value::Ref(Some(id))),
                ("getVersion", "()Lorg/apache/maven/artifact/versioning/ArtifactVersion;") => {
                    let version = self
                        .neoforge_mod_version(&id)
                        .unwrap_or_else(|| "unknown".into());
                    Some(Value::Ref(Some(format!("artifact-version/{version}"))))
                }
                _ => None,
            };
        }
        if let Some(file) = loader::mod_file_info_id(receiver) {
            // `getMods().get(0).getVersion().toString()`: the file's first
            // mod's version; counted (left to the chain) when the host does
            // not know it.
            if r.name == "versionString" && r.descriptor == "()Ljava/lang/String;" {
                return self
                    .neoforge_mod_file(&file)
                    .and_then(|(_, version)| version)
                    .map(|version| Value::Ref(Some(version)));
            }
            return None;
        }
        if let Some(name) = loader::config_type_name(receiver) {
            return match (r.name.as_str(), r.descriptor.as_str()) {
                ("name", "()Ljava/lang/String;") => Some(Value::Ref(Some(name))),
                ("ordinal", "()I") => loader::config_type_ordinal(receiver).map(Value::Int),
                // `StringUtils.toLowerCase(name())`: `Locale.ROOT`, ASCII here.
                ("extension", "()Ljava/lang/String;") => {
                    Some(Value::Ref(Some(name.to_ascii_lowercase())))
                }
                _ => None,
            };
        }
        if let Some(name) = loader::world_version_name(receiver) {
            return matches!(
                (r.name.as_str(), r.descriptor.as_str()),
                ("name" | "getName", "()Ljava/lang/String;")
            )
            .then(|| Value::Ref(Some(name)));
        }
        None
    }

    /// The running jar's own mod file: its `[[mods]]` in order, each with
    /// its version (the first mod's as `seed_host` seeded it when FML
    /// substitutes the jar's).
    fn own_mod_file(&self) -> Vec<(String, Option<String>)> {
        let Some(toml) = self.resource_bytes.get(MODS_TOML) else {
            return Vec::new();
        };
        let entries = mod_file_entries(toml, None);
        let first_version = entries
            .first()
            .and_then(|(id, _)| self.mods.get(id))
            .filter(|version| *version != "unknown")
            .cloned();
        entries
            .into_iter()
            .map(|(id, version)| (id, version.or_else(|| first_version.clone())))
            .collect()
    }

    /// The mod file holding mod `id`, named by the file's first mod, and
    /// that first mod's version (`versionString`, `None` when the host does
    /// not know it): the running jar's own file, a linked NeoForge jar's
    /// (`neoforge_mod_files`), `minecraft`'s and `neoforge`'s. `None` when
    /// the host does not know which file holds `id` (a dependency it only
    /// assumes present).
    fn neoforge_mod_file(&self, id: &str) -> Option<(String, Option<String>)> {
        let own = self.own_mod_file();
        if own.iter().any(|(own_id, _)| own_id == id) {
            return own.into_iter().next();
        }
        if let Some(linked) = self.neoforge_mod_files.get(id) {
            return Some((linked.file.clone(), linked.file_version.clone()));
        }
        match id {
            "minecraft" => Some(("minecraft".into(), self.game_version.clone())),
            "neoforge" => Some(("neoforge".into(), None)),
            _ => None,
        }
    }

    /// `ModList.getMods()`: each mod's `IModInfo`. FML lists every loaded
    /// mod of the game, which the host cannot know (other jars, their order:
    /// `ModSorter`), so the call is counted; the list holds every mod the
    /// host does know: the run's mods, the running jar's and the linked
    /// NeoForge jars' `[[mods]]`, the game (when its version was seeded) and
    /// `neoforge`.
    pub(crate) fn neoforge_mod_infos(&mut self, r: &MethodRef) -> Value {
        *self
            .unknown_host_calls
            .entry(format!("{}#{}{}", r.owner, r.name, r.descriptor))
            .or_default() += 1;
        let mut ids = self
            .mods
            .keys()
            .cloned()
            .collect::<std::collections::BTreeSet<_>>();
        ids.extend(self.own_mod_file().into_iter().map(|(id, _)| id));
        ids.extend(self.neoforge_mod_files.keys().cloned());
        ids.insert("neoforge".into());
        if self.game_version.is_some() {
            ids.insert("minecraft".into());
        }
        let infos = ids
            .iter()
            .map(|id| loader::mod_info(id))
            .collect::<Vec<_>>();
        crate::runtime::collection_value("list", &infos)
    }

    /// Whether `id` is a mod container this host can answer, and its version.
    ///
    /// The jar's own id and its declared dependencies are seeded in `mods`;
    /// the jar's other `[[mods]]` are its own too. The game and the loader
    /// are always present; their version is the caller-seeded game version
    /// (or `unknown` when the host was not told).
    pub(super) fn neoforge_mod_version(&self, id: &str) -> Option<String> {
        // A version a jar declared wins over a dependency's `unknown`
        // stand-in (`seed_host` seeds every required id that way).
        let seeded = self.mods.get(id);
        if let Some(version) = seeded.filter(|version| *version != "unknown") {
            return Some(version.clone());
        }
        let own = self.own_mod_file().into_iter().find(|(own, _)| own == id);
        if let Some(version) = own
            .as_ref()
            .and_then(|(_, version)| version.clone())
            .or_else(|| self.neoforge_mod_files.get(id)?.version.clone())
        {
            return Some(version);
        }
        if seeded.is_some() || own.is_some() || self.neoforge_mod_files.contains_key(id) {
            return Some("unknown".into());
        }
        if id == "minecraft" {
            // The game's own version is only known when the caller seeded it;
            // an empty answer keeps the container absent instead of inventing
            // a release a mod would then compare against.
            return self.game_version.clone();
        }
        is_always_loaded_mod(id).then(|| "unknown".into())
    }
}

/// A mod of a linked NeoForge jar (`StdHost::neoforge_mod_files`).
#[derive(Clone, Debug)]
pub(crate) struct LinkedMod {
    /// Its own `[[mods]]` version.
    pub(crate) version: Option<String>,
    /// Its file's first mod, which names the file.
    pub(crate) file: String,
    /// That first mod's version (`ModFileInfo.versionString`).
    pub(crate) file_version: Option<String>,
}

const FML_MOD_CONTAINER: &str = "net/neoforged/fml/javafmlmod/FMLModContainer";

const MODS_TOML: &str = "META-INF/neoforge.mods.toml";

/// Whether `toml` (a `neoforge.mods.toml`) declares mod `id` under
/// `modLoader="javafml"`: FML then builds an `FMLModContainer` for it.
fn is_javafml(toml: Option<&Vec<u8>>, id: &str) -> bool {
    toml.is_some_and(|bytes| javafml_mod_ids(&String::from_utf8_lossy(bytes)).contains(&id))
}

/// The `modId`s a `neoforge.mods.toml` declares when its top-level
/// `modLoader` is `javafml`; none for any other language loader.
fn javafml_mod_ids(text: &str) -> Vec<&str> {
    let (javafml, mods) = toml_mods(text);
    if javafml {
        mods.into_iter().map(|(id, _)| id).collect()
    } else {
        Vec::new()
    }
}

/// A `neoforge.mods.toml`'s `[[mods]]` entries in file order, each id with
/// its literal `version` (`None` when absent), and whether the file's
/// top-level `modLoader` is `javafml`.
fn toml_mods(text: &str) -> (bool, Vec<(&str, Option<&str>)>) {
    let mut javafml = false;
    let mut in_mods = false;
    let mut top_level = true;
    let mut mods: Vec<(&str, Option<&str>)> = Vec::new();
    for raw in text.lines() {
        let line = raw.split('#').next().unwrap_or_default().trim();
        if line.starts_with('[') && line.ends_with(']') {
            top_level = false;
            in_mods = line.starts_with("[[mods]]");
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim().trim_matches('"').trim_matches('\'');
        match key.trim() {
            "modLoader" if top_level => javafml = value == "javafml",
            "modId" if in_mods => mods.push((value, None)),
            "version" if in_mods => {
                if let Some(last) = mods.last_mut() {
                    last.1 = Some(value);
                }
            }
            _ => {}
        }
    }
    (javafml, mods)
}

/// One mod file's mods (`(id, version)` in `[[mods]]` order) from its
/// `neoforge.mods.toml`. A version FML substitutes (`${file.jarVersion}`)
/// is `jar_version`, the version the jar's own metadata resolved.
pub(crate) fn mod_file_entries(
    toml: &[u8],
    jar_version: Option<&str>,
) -> Vec<(String, Option<String>)> {
    let text = String::from_utf8_lossy(toml);
    toml_mods(&text)
        .1
        .into_iter()
        .map(|(id, version)| {
            let version = match version {
                Some(version) if !version.contains("${") => Some(version.to_owned()),
                _ => jar_version.map(str::to_owned),
            };
            (id.to_owned(), version)
        })
        .collect()
}

fn enum_name(marker: &str) -> &str {
    marker.rsplit('/').next().unwrap_or(marker)
}

package demo;

import net.neoforged.fml.ModList;

/** Horizon's side: what `ModList.get()` answers, passed to {@link Loader}. */
public final class Probe {
    public static String containers() {
        return Loader.containers(ModList.get());
    }

    public static String mods() {
        return Loader.mods(ModList.get());
    }

    public static String files() {
        return Loader.files(net.neoforged.fml.loading.FMLLoader.getCurrent().getLoadingModList());
    }

    /** A mod the host only assumes present (a declared dependency): its file is unknown. */
    public static String dependencyFile() {
        return "" + (net.neoforged.fml.loading.FMLLoader.getCurrent().getLoadingModList().getModFileById("dep") == null);
    }

    public static String switched() {
        return Loader.switched();
    }

    public static String configTypes() {
        return Loader.configTypes();
    }

    public static String worldVersion() {
        return Loader.worldVersion();
    }

    public static String worldIdentity() {
        return Loader.worldIdentity();
    }

    public static String spelled() {
        return Loader.spelled(ModList.get().getModContainerById("demo").get());
    }

    public static String forged() {
        return Loader.forged();
    }
}

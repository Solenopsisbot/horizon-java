package demo;

import net.fabricmc.api.ModInitializer;
import net.fabricmc.loader.api.FabricLoader;

/**
 * The run's `main` entrypoint (the fixture jar's `fabric.mod.json`): its `onInitialize` prints what it sees of
 * itself. Horizon's side also passes what `FabricLoader.getInstance()` answers to {@link Loader}.
 */
public final class Probe implements ModInitializer {
    @Override
    public void onInitialize() {
        System.out.println("entrypointSelf=" + Loader.entrypointSelf(this, FabricLoader.getInstance()));
    }

    public static String loader() {
        return Loader.loader(FabricLoader.getInstance(), FabricLoader.getInstance());
    }

    public static String envType() {
        return Loader.envType();
    }

    public static String envClass() {
        return Loader.envClass();
    }

    public static String payloads() {
        return Loader.payloads();
    }

    public static String resources() {
        return Loader.resources();
    }

    public static String entrypoints() {
        return Loader.entrypoints(FabricLoader.getInstance());
    }

    public static String otherTypes() {
        return Loader.otherTypes(FabricLoader.getInstance());
    }

    public static String spelled() {
        return Loader.spelled(FabricLoader.getInstance());
    }

    public static String forged() {
        return Loader.forged();
    }
}

package demo;

import net.fabricmc.loader.api.FabricLoader;

/** Horizon's side: what `FabricLoader.getInstance()` answers, passed to {@link Loader}; also the run's entrypoint. */
public final class Probe {
    public static String loader() {
        return Loader.loader(FabricLoader.getInstance(), FabricLoader.getInstance());
    }

    public static String envType() {
        return Loader.envType();
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

    public static String entrypointIdentity() {
        return Loader.entrypointIdentity(FabricLoader.getInstance());
    }

    public static String spelled() {
        return Loader.spelled(FabricLoader.getInstance());
    }

    public static String forged() {
        return Loader.forged();
    }
}

package net.fabricmc.loader.api;
public interface FabricLoader {
    static FabricLoader getInstance() { throw new AssertionError(); }
    MappingResolver getMappingResolver();
    net.fabricmc.api.EnvType getEnvironmentType();
    java.util.Optional<ModContainer> getModContainer(String id);
    <T> java.util.List<T> getEntrypoints(String key, Class<T> type);
    <T> java.util.List<net.fabricmc.loader.api.entrypoint.EntrypointContainer<T>> getEntrypointContainers(String key, Class<T> type);
}

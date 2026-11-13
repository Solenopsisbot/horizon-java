package net.fabricmc.loader.api;
public interface FabricLoader {
    static FabricLoader getInstance() { throw new AssertionError(); }
    java.util.Collection<ModContainer> getAllMods();
    java.util.Optional<ModContainer> getModContainer(String id);
}

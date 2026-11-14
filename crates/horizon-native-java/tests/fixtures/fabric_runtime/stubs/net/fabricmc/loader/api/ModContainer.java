package net.fabricmc.loader.api;
public interface ModContainer {
    net.fabricmc.loader.api.metadata.ModMetadata getMetadata();
    java.util.List<java.nio.file.Path> getRootPaths();
}

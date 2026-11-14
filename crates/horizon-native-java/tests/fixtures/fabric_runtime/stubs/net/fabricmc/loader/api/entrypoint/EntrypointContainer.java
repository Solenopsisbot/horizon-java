package net.fabricmc.loader.api.entrypoint;
public interface EntrypointContainer<T> {
    T getEntrypoint();
    net.fabricmc.loader.api.ModContainer getProvider();
}

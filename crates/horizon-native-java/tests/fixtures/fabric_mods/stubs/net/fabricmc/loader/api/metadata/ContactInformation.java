package net.fabricmc.loader.api.metadata;
public interface ContactInformation {
    ContactInformation EMPTY = null;
    java.util.Optional<String> get(String key);
}

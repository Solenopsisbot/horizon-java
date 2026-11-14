package net.fabricmc.loader.api.metadata;
public interface ModMetadata {
    String getId();
    java.util.Collection<Person> getAuthors();
    java.util.Collection<String> getLicense();
}

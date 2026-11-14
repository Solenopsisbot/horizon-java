package net.fabricmc.loader.api.metadata;
public interface ModOrigin {
    Kind getKind();
    enum Kind { PATH, NESTED, UNKNOWN }
}

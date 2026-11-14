package net.neoforged.neoforgespi.language;
public interface IModInfo {
    String getModId();
    org.apache.maven.artifact.versioning.ArtifactVersion getVersion();
}

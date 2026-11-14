package net.neoforged.fml.config;
public final class ModConfig {
    public enum Type {
        LOCAL, CLIENT, SYNCED, STARTUP;
        public String extension() { throw new AssertionError(); }
    }
}

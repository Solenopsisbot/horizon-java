package net.neoforged.fml.config;

/** The jar's own copy of a NeoForge loader class (Forge Config API Port's case, #124). */
public class ConfigTracker {
  public static final ConfigTracker INSTANCE = new ConfigTracker();
  private int registered;

  public int count() {
    return registered;
  }
}

package me.shedaniel.autoconfig;
public class AutoConfig {
  public static <T extends ConfigData> ConfigHolder<T> register(Class<T> c, me.shedaniel.autoconfig.serializer.ConfigSerializer.Factory<T> f) { return null; }
  public static <T extends ConfigData> ConfigHolder<T> getConfigHolder(Class<T> c) { return null; }
}

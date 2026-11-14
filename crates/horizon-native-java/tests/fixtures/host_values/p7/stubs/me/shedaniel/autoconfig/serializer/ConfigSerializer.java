package me.shedaniel.autoconfig.serializer;
public interface ConfigSerializer<T extends me.shedaniel.autoconfig.ConfigData> {
  void serialize(T c);
  T deserialize();
  T createDefault();
  interface Factory<T extends me.shedaniel.autoconfig.ConfigData> {
    ConfigSerializer<T> create(me.shedaniel.autoconfig.annotation.Config c, Class<T> cls);
  }
}

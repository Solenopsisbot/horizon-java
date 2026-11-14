package demo;

import java.util.*;
import java.util.function.Supplier;
import me.shedaniel.autoconfig.AutoConfig;
import me.shedaniel.autoconfig.ConfigData;
import me.shedaniel.autoconfig.ConfigHolder;
import me.shedaniel.autoconfig.annotation.Config;
import me.shedaniel.autoconfig.serializer.ConfigSerializer;
import dev.isxander.yacl3.config.v2.api.ConfigClassHandler;
import com.supermartijn642.configlib.api.ConfigBuilders;
import com.supermartijn642.configlib.api.IConfigBuilder;

/** The config libraries' objects: AutoConfig holders, YACL builders, configlib builders and values. */
public final class Configs {
  @Config(name = "demo") public static class Cfg implements ConfigData { public int count = 7; }
  @Config(name = "other") public static class Other implements ConfigData { public int count = 3; }

  static <T extends ConfigData> ConfigSerializer.Factory<T> mem(Supplier<T> make) {
    return (def, cls) -> new ConfigSerializer<T>() {
      public void serialize(T c) {}
      public T deserialize() { return make.get(); }
      public T createDefault() { return make.get(); }
    };
  }

  static ConfigHolder<Cfg> holder;
  static ConfigHolder<Other> other;

  static void register() {
    if (holder == null) {
      holder = AutoConfig.register(Cfg.class, mem(Cfg::new));
      other = AutoConfig.register(Other.class, mem(Other::new));
    }
  }

  /** AutoConfig keeps one holder per config class. */
  public static String autoConfig() {
    register();
    ConfigHolder<Cfg> h = holder;
    ConfigHolder<Other> o = other;
    Object ho = h;
    return "" + (ho instanceof ConfigHolder) + (h == AutoConfig.getConfigHolder(Cfg.class)) + h.equals(AutoConfig.getConfigHolder(Cfg.class))
        + h.equals(o) + ((Object) h == o) + h.getConfig().count + (h.getConfig() == h.getConfig())
        + List.of(h, o).contains(AutoConfig.getConfigHolder(Other.class)) + List.of(h, o).indexOf(o)
        + (ho instanceof String);
  }

  /** Two holders in a HashSet: their hash is the identity hash. */
  public static String autoConfigHashed() {
    register();
    return "" + new HashSet<Object>(List.of(holder, other)).size();
  }

  /** YACL's builders: a new object per createBuilder, its own type. */
  public static String yaclBuilder() {
    ConfigClassHandler.Builder<Cfg> b = ConfigClassHandler.createBuilder(Cfg.class);
    Object bo = b;
    return "" + (bo instanceof ConfigClassHandler.Builder) + (bo instanceof String) + (b == bo);
  }

  /** Two YACL builders of one class: two objects. */
  public static String yaclTwoBuilders() {
    ConfigClassHandler.Builder<Cfg> b = ConfigClassHandler.createBuilder(Cfg.class);
    ConfigClassHandler.Builder<Cfg> c = ConfigClassHandler.createBuilder(Cfg.class);
    return "" + (b == c) + b.equals(c);
  }

  /** configlib: a builder's steps return it; each define is its own value object. */
  public static String configlib() {
    IConfigBuilder t = ConfigBuilders.newTomlConfig("demo", null, false);
    Object to = t;
    Supplier<Integer> v = t.define("count", 7, 0, 10);
    Supplier<Integer> w = t.define("other", 8, 0, 10);
    Object vo = v;
    return "" + (to instanceof IConfigBuilder) + (t.push("x") == t) + (to instanceof String)
        + "," + (vo instanceof Supplier) + (v == w) + v.equals(w) + v.equals(v) + List.of(v, w).indexOf(w);
  }

  /** Each object against another library's interface: the runtime knows only the interface the call
   * answered, not the implementation class, so these are counted. */
  public static String crossTypes() {
    register();
    Object ho = holder;
    Object bo = ConfigClassHandler.createBuilder(Cfg.class);
    IConfigBuilder t = ConfigBuilders.newTomlConfig("demo", null, false);
    Object to = t;
    Object vo = t.define("count", 7, 0, 10);
    return "" + (ho instanceof ConfigClassHandler) + (bo instanceof ConfigClassHandler) + (to instanceof Supplier)
        + (vo instanceof IConfigBuilder);
  }

  /** Two YACL builders of one class in hash collections, a list and Set.of: two objects, whose equality
   * is identity, so equal keys never merge them. */
  public static String yaclBuildersInCollections() {
    ConfigClassHandler.Builder<Cfg> b = ConfigClassHandler.createBuilder(Cfg.class);
    ConfigClassHandler.Builder<Cfg> c = ConfigClassHandler.createBuilder(Cfg.class);
    Map<Object, Integer> m = new HashMap<>();
    m.put(b, 1);
    m.put(c, 2);
    String setOf;
    try { setOf = "" + Set.of(b, c).size(); } catch (IllegalArgumentException e) { setOf = "dup"; }
    return "" + new HashSet<Object>(List.of(b, c)).size() + m.size() + List.of(b).contains(c) + setOf;
  }

  /** Two configlib builders: two objects. */
  public static String configlibTwoBuilders() {
    IConfigBuilder t = ConfigBuilders.newTomlConfig("demo", null, false);
    IConfigBuilder u = ConfigBuilders.newTomlConfig("demo", null, false);
    return "" + (t == u) + t.equals(u);
  }
}

package demo;

import java.util.ArrayList;
import java.util.List;
import net.fabricmc.api.ClientModInitializer;
import net.fabricmc.api.ModInitializer;
import net.fabricmc.loader.api.FabricLoader;

/**
 * One class that is both the {@code main} and the {@code client} entrypoint: Fabric Loader makes
 * one object per entrypoint key (`EntrypointStorage.add` gives each key its own entry and instance
 * map), so the two are two objects, and each {@code onInitialize*} runs on its own.
 */
public class Probe implements ModInitializer, ClientModInitializer {
  static final List<Object> MADE = new ArrayList<>();
  int seen;

  public Probe() {
    MADE.add(this);
  }

  @Override
  public void onInitialize() {
    seen += 1;
  }

  @Override
  public void onInitializeClient() {
    seen += 10;
  }

  /** The loader's own lists for the two keys. */
  public static String fromLoader() {
    return report(
        FabricLoader.getInstance().getEntrypoints("main", ModInitializer.class),
        FabricLoader.getInstance().getEntrypoints("client", ClientModInitializer.class));
  }

  public static String report(List<?> main, List<?> client) {
    Object m = main.get(0);
    Object c = client.get(0);
    return MADE.size()
        + ","
        + (m == MADE.get(0))
        + ","
        + (c == MADE.get(MADE.size() - 1))
        + ","
        + (m == c)
        + ","
        + ((Probe) m).seen
        + ","
        + ((Probe) c).seen;
  }
}

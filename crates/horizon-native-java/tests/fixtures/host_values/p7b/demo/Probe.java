package demo;

import net.fabricmc.fabric.api.command.v2.CommandRegistrationCallback;

/** The jar's entrypoint: registers its commands the way Fabric mods do. */
public final class Probe {
  public void onInitialize() {
    CommandRegistrationCallback.EVENT.register((dispatcher, context, selection) -> Checks.fired(dispatcher));
  }
}

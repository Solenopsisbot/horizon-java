package net.fabricmc.fabric.api.command.v2;

public interface CommandRegistrationCallback {
  net.fabricmc.fabric.api.event.Event<CommandRegistrationCallback> EVENT = null;

  void register(com.mojang.brigadier.CommandDispatcher<net.minecraft.commands.CommandSourceStack> d,
      net.minecraft.commands.CommandBuildContext c, net.minecraft.commands.Commands.CommandSelection s);
}

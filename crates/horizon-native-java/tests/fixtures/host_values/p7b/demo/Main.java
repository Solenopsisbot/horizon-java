package demo;

import com.mojang.brigadier.CommandDispatcher;
import net.minecraft.SharedConstants;
import net.minecraft.server.Bootstrap;
import net.minecraft.commands.CommandSource;
import net.minecraft.commands.CommandSourceStack;
import net.minecraft.network.chat.Component;
import net.minecraft.server.permissions.LevelBasedPermissionSet;
import net.minecraft.world.phys.Vec2;
import net.minecraft.world.phys.Vec3;

/**
 * The Java 25 run, against brigadier 1.3.11 and Minecraft 26.3's server classes: a dispatcher as
 * the server makes one, `/probe` run by a game master console source, then each check
 * (each a later firing of the registration callback).
 */
public final class Main {
  public static void main(String[] args) throws Exception {
    // `Commands` and the permission constants need the registries.
    SharedConstants.tryDetectVersion();
    Bootstrap.bootStrap();
    CommandDispatcher<CommandSourceStack> d = new CommandDispatcher<>();
    Checks.fired(d);
    CommandSourceStack source = new CommandSourceStack(CommandSource.NULL, Vec3.ZERO, Vec2.ZERO, null,
        LevelBasedPermissionSet.GAMEMASTER, Component.literal("Server"), null);
    System.out.println("result=" + d.execute("probe", source));
    d.execute("rvarg 5", source);
    d.execute("rvforge 7", source);
    d.execute("rvreq", source);
    for (int i = 0; i < Checks.NAMES.length; i++) Checks.fired(d);
  }
}

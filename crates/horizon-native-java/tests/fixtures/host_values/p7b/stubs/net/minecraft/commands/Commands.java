package net.minecraft.commands;

import net.minecraft.server.permissions.*;

public class Commands {
  public static final PermissionCheck LEVEL_GAMEMASTERS = null;

  public enum CommandSelection { ALL, DEDICATED, INTEGRATED }

  public static <T extends PermissionSetSupplier> PermissionProviderCheck<T> hasPermission(PermissionCheck c) { return null; }
}

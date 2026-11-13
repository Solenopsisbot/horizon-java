package demo;

import com.mojang.brigadier.CommandDispatcher;
import com.mojang.brigadier.arguments.ArgumentType;
import com.mojang.brigadier.builder.ArgumentBuilder;
import com.mojang.brigadier.builder.LiteralArgumentBuilder;
import com.mojang.brigadier.context.CommandContext;
import com.mojang.brigadier.tree.CommandNode;
import java.util.*;
import java.util.function.Predicate;
import net.minecraft.commands.CommandSourceStack;
import net.minecraft.server.permissions.PermissionSet;
import net.minecraft.server.permissions.Permissions;

/** Strings spelled like the command surface's old markers, or its handles' text forms, stay Strings. */
public final class Forgery {
  public static String spelled() {
    Object[] texts = {"brigadier/dispatcher", "brigadier/build-context", "brigadier/context", "brigadier/source/0",
        "brigadier/node/0", "brigadier/builder/0", "brigadier/argtype/0", "brigadier/requires/0",
        "brigadier/permission/2", "brigadier/permission-check/2", "brigadier/permission-set/4", "hbrd:",
        "hbrnode:i0,rroot,r", "hbrsrc:i0,i0", "hbrperm:rrequires,i0"};
    StringBuilder out = new StringBuilder();
    for (Object t : texts) {
      out.append(t instanceof String).append(t.getClass().getName()).append(t instanceof CommandDispatcher)
          .append(t instanceof CommandNode).append(t instanceof ArgumentBuilder).append(t instanceof ArgumentType)
          .append(t instanceof CommandContext).append(t instanceof CommandSourceStack).append(t instanceof Predicate)
          .append(t instanceof PermissionSet).append(t.equals(String.valueOf(t))).append(t.hashCode())
          .append(("" + t).length()).append(',');
    }
    Map<Object, String> m = new HashMap<>();
    m.put("brigadier/dispatcher", "d");
    m.put("brigadier/node/0", "n");
    return out.append(m.size()).append(m.get("brigadier/dispatcher")).append(m.get("brigadier/node/0")).toString();
  }

  /** A string cast to a command object is a `ClassCastException`, never that object. */
  @SuppressWarnings("unchecked")
  public static String used() {
    StringBuilder out = new StringBuilder();
    try {
      Checks.dispatcher.register((LiteralArgumentBuilder<CommandSourceStack>) (Object) "brigadier/builder/0");
      out.append("registered");
    } catch (ClassCastException e) {
      out.append("cce");
    }
    try {
      Checks.literal("f").requires((Predicate<CommandSourceStack>) (Object) "brigadier/requires/0");
      out.append("required");
    } catch (ClassCastException e) {
      out.append("cce");
    }
    try {
      Checks.literal("g").then((CommandNode<CommandSourceStack>) (Object) "brigadier/node/0");
      out.append("added");
    } catch (ClassCastException e) {
      out.append("cce");
    }
    try {
      out.append(((PermissionSet) (Object) "brigadier/permission-set/4").hasPermission(Permissions.COMMANDS_OWNER));
    } catch (ClassCastException e) {
      out.append("cce");
    }
    try {
      out.append(((CommandSourceStack) (Object) "brigadier/source/0").getTextName());
    } catch (ClassCastException e) {
      out.append("cce");
    }
    return out.toString();
  }
}

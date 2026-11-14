package com.mojang.brigadier;

public interface Command<S> {
  int run(com.mojang.brigadier.context.CommandContext<S> c) throws com.mojang.brigadier.exceptions.CommandSyntaxException;
}

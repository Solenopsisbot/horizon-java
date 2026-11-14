package com.mojang.brigadier.context;

public class CommandContext<S> {
  public S getSource() { return null; }
  public String getInput() { return null; }
  public <V> V getArgument(String name, Class<V> clazz) { return null; }
}

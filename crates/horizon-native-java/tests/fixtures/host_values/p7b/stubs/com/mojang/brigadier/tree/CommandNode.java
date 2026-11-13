package com.mojang.brigadier.tree;

public abstract class CommandNode<S> implements Comparable<CommandNode<S>> {
  public abstract String getName();
  public CommandNode<S> getChild(String name) { return null; }
  public int compareTo(CommandNode<S> o) { return 0; }
}

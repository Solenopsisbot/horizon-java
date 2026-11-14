package com.mojang.brigadier.builder;

public abstract class ArgumentBuilder<S, T extends ArgumentBuilder<S, T>> {
  public T then(ArgumentBuilder<S, ?> a) { return null; }
  public T then(com.mojang.brigadier.tree.CommandNode<S> a) { return null; }
  public T executes(com.mojang.brigadier.Command<S> c) { return null; }
  public T requires(java.util.function.Predicate<S> p) { return null; }
  public T redirect(com.mojang.brigadier.tree.CommandNode<S> target) { return null; }
  public T redirect(com.mojang.brigadier.tree.CommandNode<S> target, com.mojang.brigadier.SingleRedirectModifier<S> modifier) { return null; }
  public T fork(com.mojang.brigadier.tree.CommandNode<S> target, com.mojang.brigadier.RedirectModifier<S> modifier) { return null; }
  public abstract com.mojang.brigadier.tree.CommandNode<S> build();
}

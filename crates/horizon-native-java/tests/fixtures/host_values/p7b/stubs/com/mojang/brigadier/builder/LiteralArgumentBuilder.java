package com.mojang.brigadier.builder;

public class LiteralArgumentBuilder<S> extends ArgumentBuilder<S, LiteralArgumentBuilder<S>> {
  public static <S> LiteralArgumentBuilder<S> literal(String name) { return null; }
  public String getLiteral() { return null; }
  public com.mojang.brigadier.tree.LiteralCommandNode<S> build() { return null; }
}

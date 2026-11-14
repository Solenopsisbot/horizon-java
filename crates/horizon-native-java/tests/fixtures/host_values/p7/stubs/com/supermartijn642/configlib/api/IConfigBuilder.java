package com.supermartijn642.configlib.api;
public interface IConfigBuilder {
  IConfigBuilder push(String s);
  java.util.function.Supplier<Integer> define(String n, int d, int min, int max);
}

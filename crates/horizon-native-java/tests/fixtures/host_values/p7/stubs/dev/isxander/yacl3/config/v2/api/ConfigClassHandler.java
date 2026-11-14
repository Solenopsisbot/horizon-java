package dev.isxander.yacl3.config.v2.api;
public interface ConfigClassHandler<T> {
  static <T> Builder<T> createBuilder(Class<T> c) { return null; }
  interface Builder<T> { ConfigClassHandler<T> build(); }
}

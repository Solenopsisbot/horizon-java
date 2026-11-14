package demo;

/**
 * A static field an interface declares, read through a class or interface that inherits it: JVMS
 * 5.4.3.2 resolves it to the interface (superinterfaces before the superclass), and the interface's
 * initializer stored it. The fields are not constants, so javac emits a {@code getstatic} naming the
 * class it was read through.
 */
public class Probe {
  interface Element {
    String INDENT = " ".repeat(4);
  }

  interface Nested extends Element {
    int DEPTH = Integer.parseInt("2");
  }

  static class Base {
    static String base = "base".toUpperCase();
  }

  static class Object1 extends Base implements Nested {
    String indented(String text) {
      return INDENT + text;
    }
  }

  static class Sub extends Object1 {}

  public static String fields() {
    return "["
        + Object1.INDENT
        + "]["
        + Sub.INDENT
        + "]["
        + Nested.INDENT
        + "]"
        + Sub.DEPTH
        + Sub.base
        + new Sub().indented("x");
  }
}

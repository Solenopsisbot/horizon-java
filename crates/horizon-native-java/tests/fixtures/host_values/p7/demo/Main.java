package demo;

/** The Java 25 run of the config libraries and `Side`: each method. */
public final class Main {
  public static void main(String[] args) throws Exception {
    for (String name : new String[] {"spelled"}) System.out.println(name + "=" + Forgery.class.getMethod(name).invoke(null));
    System.out.println("sides=" + Sides.sides());
    for (String name : new String[] {"autoConfig", "autoConfigHashed", "yaclBuilder", "yaclTwoBuilders", "yaclBuildersInCollections", "configlib", "crossTypes", "configlibTwoBuilders"})
      System.out.println(name + "=" + Configs.class.getMethod(name).invoke(null));
    // The Mixin callbacks: `MixinMain.java`.
  }
}

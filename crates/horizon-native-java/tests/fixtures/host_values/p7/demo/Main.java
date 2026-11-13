package demo;

import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;

/** The Java 25 run: each method, and each callback on the object Mixin builds for its target. */
public final class Main {
  public static void main(String[] args) throws Exception {
    for (String name : new String[] {"spelled"}) System.out.println(name + "=" + Forgery.class.getMethod(name).invoke(null));
    System.out.println("sides=" + Sides.sides());
    for (String name : new String[] {"autoConfig", "autoConfigHashed", "yaclBuilder", "yaclTwoBuilders", "yaclBuildersInCollections", "configlib", "crossTypes", "configlibTwoBuilders"})
      System.out.println(name + "=" + Configs.class.getMethod(name).invoke(null));
    // What Mixin's CallbackInjector builds for each target: a CallbackInfoReturnable for one with a
    // result, a CallbackInfo otherwise (name, cancellable).
    Observe.returnable(new CallbackInfoReturnable<Integer>("count", true));
    Observe.plain(new CallbackInfo("run", false));
    Observe.hashed(new CallbackInfo("hash", false));
    CallbackInfo twice = new CallbackInfo("twice", false);
    Observe.collect(twice);
    Observe.collect(twice);
  }
}

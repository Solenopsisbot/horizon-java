package demo;

import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;

/**
 * The Java 25 run of the Mixin callbacks, against sponge-mixin 0.17.5+mixin.0.8.7: each callback on
 * the object Mixin's `CallbackInjector` builds for its target (`name`, `cancellable`, and the result
 * for a RETURN injection). The injector keeps one per `@Inject` (`callbackInfoVar`), so the two on
 * `twice` get two objects.
 */
public final class MixinMain {
  public static void main(String[] args) throws Exception {
    Observe.returnable(new CallbackInfoReturnable<Integer>("count", true, 5));
    Observe.plain(new CallbackInfo("run", false));
    Observe.hashed(new CallbackInfo("hash", false));
    CallbackInfo first = new CallbackInfo("twice", false);
    CallbackInfo second = new CallbackInfo("twice", false);
    Observe.twice(first);
    Observe.twice(second);
    log.Probe.mark("twice:" + (first == second));
    CallbackInfoReturnable<Integer> strict = new CallbackInfoReturnable<>("strict", false, 6);
    Observe.strict(strict);
    log.Probe.mark("strict returns " + (strict.isCancelled() ? strict.getReturnValue() : 6));
  }
}

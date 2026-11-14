package demo;

import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

@Mixin(value = Target.class, targets = {"Ldemo/Target;"})
public abstract class CounterMixin {
  @Inject(method = "twice()V", at = @At("HEAD"))
  static void first(CallbackInfo info) {
    Counter.N++;
    Counter.SEEN.add("a" + Counter.N);
    Counter.BOX.n++;
    log.Probe.mark("a:" + Counter.N);
  }

  @Inject(method = "twice()V", at = @At("HEAD"))
  static void second(CallbackInfo info) {
    log.Probe.mark("b:" + Counter.N + ":" + Counter.SEEN + ":" + Counter.BOX.n);
  }

  @Inject(method = "other()V", at = @At("HEAD"))
  static void third(CallbackInfo info) {
    log.Probe.mark("c:" + Counter.N + ":" + Counter.SEEN.size() + ":" + Counter.BOX.n);
  }
}

package demo;

import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;

@Mixin(value = Target.class, targets = {"Ldemo/Target;"})
public abstract class InfoMixin {
  @Inject(method = "count()I", at = @At("RETURN"), cancellable = true)
  private static void countHead(CallbackInfoReturnable<Integer> info) { Observe.returnable(info); }

  @Inject(method = "run()V", at = @At("HEAD"))
  private static void runHead(CallbackInfo info) { Observe.plain(info); }

  @Inject(method = "twice()V", at = @At("HEAD"))
  private static void twiceFirst(CallbackInfo info) { Observe.collect(info); }

  @Inject(method = "twice()V", at = @At("HEAD"))
  private static void twiceSecond(CallbackInfo info) { Observe.collect(info); }

  @Inject(method = "hash()V", at = @At("HEAD"))
  private static void hashHead(CallbackInfo info) { Observe.hashed(info); }
}

package demo;

import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;
import org.spongepowered.asm.mixin.injection.callback.CancellationException;
import org.spongepowered.asm.mixin.injection.callback.Cancellable;

/** What a callback observes of the CallbackInfo its injection point hands it. */
public final class Observe {
  public static void returnable(CallbackInfoReturnable<Integer> info) {
    Object o = info;
    log.Probe.mark("" + (o instanceof CallbackInfo) + (o instanceof CallbackInfoReturnable) + (o instanceof Cancellable)
        + (info == o) + info.equals(o) + (o instanceof String) + info.isCancelled());
    info.cancel();
    log.Probe.mark("" + info.isCancelled() + info.equals("hmci:rdemo/Target,rcount,i1"));
  }

  /** A callback info whose `@Inject` is not cancellable: `cancel()` throws Mixin's exception. */
  public static void plain(CallbackInfo info) {
    Object o = info;
    log.Probe.mark("" + (o instanceof CallbackInfo) + (o instanceof CallbackInfoReturnable) + (o instanceof Cancellable)
        + (info == o) + info.isCancelled());
    try {
      info.cancel();
      log.Probe.mark("no-throw");
    } catch (CancellationException e) {
      log.Probe.mark(e.getClass().getName() + ":" + e.getMessage());
    }
    try {
      info.cancel();
    } catch (RuntimeException e) {
      log.Probe.mark("runtime:" + (e instanceof CancellationException));
    }
    log.Probe.mark("" + info.isCancelled());
  }

  /** Not cancellable at RETURN: `setReturnValue` throws (it calls `cancel()`) and the result stays. */
  public static void strict(CallbackInfoReturnable<Integer> info) {
    log.Probe.mark("" + info.getReturnValue());
    try {
      info.setReturnValue(1);
      log.Probe.mark("no-throw");
    } catch (CancellationException e) {
      log.Probe.mark(e.getMessage());
    }
    log.Probe.mark("" + info.isCancelled() + info.getReturnValue());
  }

  /** Each `@Inject` on one target is handed its own object (one per injector). */
  public static void twice(CallbackInfo info) {
    log.Probe.see(info);
  }

  public static void hashed(CallbackInfo info) {
    log.Probe.mark("" + (info.hashCode() == info.hashCode()));
  }
}

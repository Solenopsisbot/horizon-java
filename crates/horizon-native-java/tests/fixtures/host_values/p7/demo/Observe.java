package demo;

import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;
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

  public static void plain(CallbackInfo info) {
    Object o = info;
    log.Probe.mark("" + (o instanceof CallbackInfo) + (o instanceof CallbackInfoReturnable) + (o instanceof Cancellable)
        + (info == o) + info.isCancelled());
  }

  static final java.util.Set<Object> SEEN = new java.util.HashSet<>();

  /** Every callback at one injection point gets the same object. */
  public static void collect(CallbackInfo info) {
    SEEN.add(info);
    log.Probe.mark("" + SEEN.size());
  }

  public static void hashed(CallbackInfo info) {
    log.Probe.mark("" + (info.hashCode() == info.hashCode()));
  }
}

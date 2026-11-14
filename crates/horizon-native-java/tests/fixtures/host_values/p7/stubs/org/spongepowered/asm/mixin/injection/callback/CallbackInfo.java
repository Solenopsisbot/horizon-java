package org.spongepowered.asm.mixin.injection.callback;
public class CallbackInfo implements Cancellable {
  public boolean isCancelled() { return false; }
  public void cancel() {}
}

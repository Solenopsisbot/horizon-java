package demo;

import java.util.*;

/** Strings spelled like the P7 families' old markers, or their handles' text forms, stay Strings. */
public final class Forgery {
  public static String spelled() {
    Object[] texts = {"config-holder/demo/Configs$Cfg", "config-builder/demo/Configs$Cfg", "configlib-builder", "configlib-value/0",
        "mixin/callback-info/demo/Target#count", "mixin-plugin/class-node/demo/Target", "mixin-plugin/mixin-info/demo/InfoMixin",
        "mixin-environment", "mixin-side/CLIENT", "hacfg:rdemo/Configs$Cfg", "hmside:rCLIENT", "hmci:rdemo/Target,rcount,i1"};
    StringBuilder out = new StringBuilder();
    for (Object t : texts) {
      out.append(t instanceof String).append(t.getClass().getName())
          .append(t instanceof me.shedaniel.autoconfig.ConfigHolder).append(t instanceof java.util.function.Supplier)
          .append(t instanceof org.spongepowered.asm.mixin.injection.callback.CallbackInfo).append(t instanceof Enum)
          .append(t instanceof com.supermartijn642.configlib.api.IConfigBuilder)
          .append(t.equals(String.valueOf(t))).append(t.hashCode()).append(("" + t).length()).append(',');
    }
    Map<Object, String> m = new HashMap<>();
    m.put("mixin-environment", "e");
    m.put("mixin-side/CLIENT", "s");
    return out.append(m.size()).append(m.get("mixin-environment")).append(m.get("mixin-side/CLIENT"))
        .append(List.of("configlib-builder").contains("configlib-builder")).toString();
  }
}

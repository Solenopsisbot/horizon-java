package demo;

import java.util.*;
import net.neoforged.bus.api.Event;
import net.neoforged.bus.api.ICancellableEvent;
import net.neoforged.fml.event.IModBusEvent;
import net.neoforged.fml.event.lifecycle.FMLCommonSetupEvent;
import net.neoforged.fml.event.lifecycle.ModLifecycleEvent;
import net.neoforged.fml.event.lifecycle.ParallelDispatchEvent;
import net.neoforged.neoforge.event.tick.ServerTickEvent;
import net.neoforged.neoforge.network.event.RegisterPayloadHandlersEvent;

/**
 * The event objects a jar's listeners receive (#156 P8b): a game event ({@code ServerTickEvent.Post})
 * and loading-stage events ({@code FMLCommonSetupEvent}, {@code RegisterPayloadHandlersEvent}).
 * Each probe takes the objects as {@code Object}: the oracle passes NeoForge's own, the runtime its
 * handles.
 */
public final class HostGameEvents {
  static final String[] OLD = {
    "game-event/server_tick_end",
    "neoforge-lifecycle|net/neoforged/fml/event/lifecycle/FMLCommonSetupEvent|neoforge-mod-container/demo/1.0",
    "neoforge-lifecycle|net/neoforged/neoforge/network/event/RegisterPayloadHandlersEvent|",
  };

  static int identities(Object... objects) {
    Map<Object, Boolean> seen = new IdentityHashMap<>();
    for (Object o : objects) seen.put(o, true);
    return seen.size();
  }

  /** A string spelled like an old marker equals only itself and prints as itself. */
  public static String spelled(Object tick, Object setup) {
    StringBuilder out = new StringBuilder();
    for (Object t : OLD) {
      out.append(t.equals(tick)).append(tick.equals(t)).append(t.equals(setup)).append(setup.equals(t))
          .append(t.equals(t.toString())).append("" + t).append(((String) t).length()).append(',');
    }
    return out.append(Arrays.asList(OLD).indexOf("game-event/server_tick_end")).toString();
  }

  /** ... and is none of the events' types. */
  public static String spelledTypes() {
    StringBuilder out = new StringBuilder();
    for (Object t : OLD) {
      out.append(t instanceof Event).append(t instanceof ServerTickEvent).append(t instanceof IModBusEvent)
          .append(t instanceof FMLCommonSetupEvent).append(',');
    }
    return out.toString();
  }

  static String which(Object e) {
    return switch (e) {
      case ICancellableEvent c -> "C";
      case ServerTickEvent s -> "S";
      case ModLifecycleEvent m -> "M";
      case Event v -> "E";
      default -> "d";
    };
  }

  /** A game event: its class and its supertypes, a class it is not, a type switch. */
  public static String tick(Object e) {
    return "" + (e instanceof ServerTickEvent.Post) + (e instanceof ServerTickEvent) + (e instanceof Event)
        + (e instanceof Integer) + (e instanceof HostGameEvents) + (e instanceof FMLCommonSetupEvent) + "|"
        + e.getClass().getName() + "|" + Event.class.isInstance(e)
        + Event.class.isAssignableFrom(e.getClass()) + "|" + List.of(e).contains(e);
  }

  /** A type switch over the events' types. */
  public static String tickSwitch(Object e) {
    return which(e);
  }

  /** Interfaces a Mixin could add to a NeoForge class: the class alone cannot rule them out. */
  public static String tickInterfaces(Object e) {
    return "" + (e instanceof ICancellableEvent) + (e instanceof IModBusEvent) + (e instanceof Runnable);
  }

  /** One event held twice, and IdentityHashMap. */
  public static String identity(Object a, Object b) {
    return "" + (a == b) + a.equals(b) + identities(a, b);
  }

  /** Object's own text and hash: an identity hash. */
  public static String text(Object e) {
    return e + "," + e.hashCode();
  }

  /** An event is not Comparable: a TreeSet throws. */
  public static String sorted(Object e) {
    try {
      new TreeSet<Object>().add(e);
      return "added";
    } catch (ClassCastException x) {
      return "CCE";
    }
  }

  /** FML's setup event: its whole type is known (no Mixin transforms FML). */
  public static String setup(Object e) {
    return "" + (e instanceof FMLCommonSetupEvent) + (e instanceof ParallelDispatchEvent)
        + (e instanceof ModLifecycleEvent) + (e instanceof Event) + (e instanceof IModBusEvent)
        + (e instanceof ICancellableEvent) + (e instanceof Runnable) + (e instanceof Integer) + "|"
        + e.getClass().getName() + "|" + which(e);
  }

  /** NeoForge's payload registration event: a NeoForge class, so other interfaces need its class. */
  public static String payload(Object e) {
    return "" + (e instanceof RegisterPayloadHandlersEvent) + (e instanceof Event) + (e instanceof IModBusEvent)
        + (e instanceof FMLCommonSetupEvent) + "|" + e.getClass().getName();
  }

  public static String payloadCancellable(Object e) {
    return "" + (e instanceof ICancellableEvent);
  }
}

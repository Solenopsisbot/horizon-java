import demo.HostGameEvents;
import net.neoforged.fml.DeferredWorkQueue;
import net.neoforged.fml.event.lifecycle.FMLCommonSetupEvent;
import net.neoforged.neoforge.event.tick.ServerTickEvent;
import net.neoforged.neoforge.network.event.RegisterPayloadHandlersEvent;

/** Runs every probe on NeoForge 26.3.0.101's and loader-12.0.12's own event classes. */
public final class HostGameEventsOracle {
  public static void main(String[] args) {
    Object tick = new ServerTickEvent.Post(() -> true, null);
    Object setup = new FMLCommonSetupEvent(null, new DeferredWorkQueue("p8b"));
    Object payload = new RegisterPayloadHandlersEvent();
    print("spelled", HostGameEvents.spelled(tick, setup));
    print("spelledTypes", HostGameEvents.spelledTypes());
    print("tick", HostGameEvents.tick(tick));
    print("tickSwitch", HostGameEvents.tickSwitch(tick));
    print("tickInterfaces", HostGameEvents.tickInterfaces(tick));
    Object tick2 = new ServerTickEvent.Post(() -> true, null);
    print("identityTick", HostGameEvents.identity(tick, tick));
    print("identityTwoFirings", HostGameEvents.identity(tick, tick2));
    print("equalitySame", HostGameEvents.equality(tick, tick));
    print("equalityTwoFirings", HostGameEvents.equality(tick, tick2));
    print("classExactTick", HostGameEvents.classExact(tick));
    print("classExactSetup", HostGameEvents.classExact(setup));
    print("classConcat", HostGameEvents.classConcat(tick));
    print("classSimpleTick", HostGameEvents.classSimple(tick));
    print("classSimpleSetup", HostGameEvents.classSimple(setup));
    print("classCanonicalTick", HostGameEvents.classCanonical(tick));
    print("classCanonicalSetup", HostGameEvents.classCanonical(setup));
    print("classMethod", HostGameEvents.classMethod(tick));
    print("classInterfaces", HostGameEvents.classInterfaces(tick));
    print("classModifiers", HostGameEvents.classModifiers(tick));
    print("identitySetup", HostGameEvents.identity(setup, setup));
    print("sorted", HostGameEvents.sorted(tick));
    print("setup", HostGameEvents.setup(setup));
    print("payload", HostGameEvents.payload(payload));
    print("payloadCancellable", HostGameEvents.payloadCancellable(payload));
  }

  static void print(String name, String answer) {
    System.out.println(name + "=" + answer);
  }
}

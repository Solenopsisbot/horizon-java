package demo;

import java.util.ArrayList;
import java.util.Collections;
import java.util.HashMap;
import java.util.IdentityHashMap;
import java.util.List;
import java.util.Map;
import java.util.TreeSet;
import net.fabricmc.api.EnvType;
import net.fabricmc.fabric.api.networking.v1.PayloadTypeRegistry;
import net.fabricmc.fabric.api.resource.v1.ResourceLoader;
import net.fabricmc.loader.api.FabricLoader;
import net.fabricmc.loader.api.MappingResolver;
import net.fabricmc.loader.api.entrypoint.EntrypointContainer;
import net.minecraft.server.packs.PackType;

/**
 * Fabric Loader's and Fabric API's singletons, the same text on a JVM and in Horizon: {@code a} and {@code b} are
 * what two {@code FabricLoader.getInstance()} calls answer, on a dedicated server whose run declares the
 * {@code main} entrypoint {@code demo.Probe} of mod {@code probe-fixture}.
 */
public final class Loader {
    public static String loader(FabricLoader a, FabricLoader b) {
        MappingResolver r = a.getMappingResolver();
        IdentityHashMap<Object, String> ids = new IdentityHashMap<>();
        ids.put(a, "a");
        ids.put(r, "r");
        Object o = a;
        return "" + (a == b) + a.equals(b) + (o instanceof FabricLoader) + (r == b.getMappingResolver())
            + r.equals(b.getMappingResolver()) + (((Object) r) instanceof MappingResolver) + ids.get(b)
            + ids.get(b.getMappingResolver()) + List.of(a).contains(b) + (((Object) r) instanceof FabricLoader)
            + "|" + (a.getEnvironmentType() == EnvType.SERVER) + a.getEnvironmentType().equals(EnvType.SERVER)
            + (a.getEnvironmentType() == EnvType.CLIENT);
    }

    public static String envType() {
        EnvType c = EnvType.CLIENT;
        EnvType s = EnvType.SERVER;
        Object o = c;
        String picked = switch (s) {
            case CLIENT -> "c";
            case SERVER -> "s";
        };
        List<EnvType> sorted = new ArrayList<>(List.of(s, c));
        Collections.sort(sorted);
        StringBuilder names = new StringBuilder();
        for (EnvType t : EnvType.values()) {
            names.append(t.name()).append(t.ordinal());
        }
        IdentityHashMap<Object, String> ids = new IdentityHashMap<>();
        ids.put(c, "c");
        return c + "|" + c.name() + c.ordinal() + s.ordinal() + c.compareTo(s) + picked + names
            + EnvType.values().length + (EnvType.valueOf("SERVER") == s) + "|" + (o instanceof EnvType)
            + (o instanceof Enum) + (o instanceof Comparable) + c.equals(s) + c.equals(EnvType.CLIENT) + "|" + sorted
            + new TreeSet<>(List.of(s, c)).first() + ids.get(EnvType.CLIENT) + ids.get(s)
            + List.of(c, s).indexOf(EnvType.SERVER) + Map.of(s, 1).get(EnvType.SERVER);
    }

    public static String payloads() {
        PayloadTypeRegistry<?> play = PayloadTypeRegistry.clientboundPlay();
        Object o = play;
        IdentityHashMap<Object, String> ids = new IdentityHashMap<>();
        ids.put(play, "p");
        return "" + (play == PayloadTypeRegistry.clientboundPlay()) + (((Object) play) == PayloadTypeRegistry.serverboundPlay())
            + (((Object) PayloadTypeRegistry.clientboundConfiguration()) == PayloadTypeRegistry.serverboundConfiguration())
            + play.equals(PayloadTypeRegistry.clientboundPlay()) + (o instanceof PayloadTypeRegistry)
            + ids.get(PayloadTypeRegistry.clientboundPlay()) + ids.get(PayloadTypeRegistry.serverboundPlay())
            + List.of(play).contains(PayloadTypeRegistry.clientboundPlay());
    }

    public static String resources() {
        ResourceLoader data = ResourceLoader.get(PackType.SERVER_DATA);
        Object o = data;
        IdentityHashMap<Object, String> ids = new IdentityHashMap<>();
        ids.put(data, "d");
        return "" + (data == ResourceLoader.get(PackType.SERVER_DATA)) + (data == ResourceLoader.get(PackType.CLIENT_RESOURCES))
            + data.equals(ResourceLoader.get(PackType.SERVER_DATA)) + (o instanceof ResourceLoader)
            + ids.get(ResourceLoader.get(PackType.SERVER_DATA)) + ids.get(ResourceLoader.get(PackType.CLIENT_RESOURCES));
    }

    /** The containers and instances of the run's `main` entrypoint that the answers know exactly. */
    public static String entrypoints(FabricLoader l) {
        EntrypointContainer<Object> c = l.getEntrypointContainers("main", Object.class).get(0);
        Object instance = c.getEntrypoint();
        return "" + l.getEntrypointContainers("main", Object.class).size() + (c.getProvider() == l.getModContainer("probe-fixture").get())
            + c.getProvider().getMetadata().getId() + (instance instanceof Probe) + (((Object) c) instanceof EntrypointContainer)
            + l.getEntrypointContainers("absent", Object.class).size();
    }

    /** Two calls' containers are new objects; the instance is cached per requested type. */
    public static String entrypointIdentity(FabricLoader l) {
        EntrypointContainer<Object> c = l.getEntrypointContainers("main", Object.class).get(0);
        EntrypointContainer<Object> d = l.getEntrypointContainers("main", Object.class).get(0);
        return "" + (c == d) + c.equals(d) + (c.getEntrypoint() == d.getEntrypoint())
            + (l.getEntrypoints("main", Object.class).get(0) == c.getEntrypoint());
    }

    /** Strings spelled like the old markers, or a handle's text, are no loader object. */
    public static String spelled(FabricLoader a) {
        Object[] texts = {"fabric-loader", "fabric-mapping-resolver", "CLIENT", "SERVER", "fli:", "fenv:rSERVER",
            "fpt:rplayToClient", "frl:rSERVER_DATA"};
        StringBuilder out = new StringBuilder();
        Map<Object, String> map = new HashMap<>();
        for (Object t : texts) {
            out.append(t.getClass().getSimpleName()).append(t instanceof String).append(t.hashCode())
                .append(t instanceof EnvType).append(t instanceof FabricLoader).append(a.equals(t)).append(t.equals(a))
                .append(EnvType.SERVER.equals(t)).append(t.equals(EnvType.SERVER)).append(t == (Object) EnvType.SERVER)
                .append(t == (Object) a.getEnvironmentType()).append(t + "!").append(',');
            map.put(t, "s");
        }
        Object[] markers = {"fabric-entrypoint/main/demo/Probe/probe-fixture", "fabric-payload-types/playToClient",
            "fabric-resource-loader/SERVER_DATA", "fabric-entrypoint-instance/demo/Probe"};
        for (Object t : markers) {
            out.append(a.equals(t)).append(t.equals(a)).append(EnvType.SERVER.equals(t)).append(t + "!").append(',');
            map.put(t, "s");
        }
        return out.append(map.size()).append(map.get("SERVER")).append(map.containsKey(EnvType.SERVER)).toString();
    }

    /** A String cast to a loader type is a ClassCastException, never the object it spells. */
    public static String forged() {
        StringBuilder out = new StringBuilder();
        Object server = "SERVER";
        Object loader = "fabric-loader";
        try {
            out.append(((EnvType) server).ordinal());
        } catch (ClassCastException e) {
            out.append("CCE");
        }
        try {
            out.append(((FabricLoader) loader).getEnvironmentType());
        } catch (ClassCastException e) {
            out.append("CCE");
        }
        return out.toString();
    }
}

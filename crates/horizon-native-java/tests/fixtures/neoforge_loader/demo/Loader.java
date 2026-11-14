package demo;

import java.util.ArrayList;
import java.util.Collections;
import java.util.HashMap;
import java.util.IdentityHashMap;
import java.util.List;
import java.util.Map;
import java.util.TreeSet;
import net.minecraft.DetectedVersion;
import net.minecraft.WorldVersion;
import net.neoforged.fml.ModContainer;
import net.neoforged.fml.ModList;
import net.neoforged.fml.config.ModConfig;
import net.neoforged.neoforgespi.language.IModInfo;

/**
 * NeoForge's loader objects, the same text on a JVM and in Horizon: the containers {@code ModList} hands out for
 * {@code demo} (version 1.2.3) and {@code other} (4.5), their mod infos, the {@code ModConfig.Type} constants and
 * the game's detected version (26.3).
 */
public final class Loader {
    static String kind(Object o) {
        return switch (o) {
            case ModConfig.Type t -> "T" + t.ordinal();
            case IModInfo i -> "I" + i.getModId();
            case ModContainer m -> "M" + m.getModId();
            default -> "?";
        };
    }

    public static String containers(ModList list) {
        ModContainer a = list.getModContainerById("demo").get();
        ModContainer a2 = list.getModContainerById("demo").get();
        ModContainer b = list.getModContainerById("other").get();
        Object o = a;
        IModInfo info = a.getModInfo();
        IdentityHashMap<Object, String> ids = new IdentityHashMap<>();
        ids.put(a, "a");
        ids.put(info, "i");
        return a.getModId() + b.getModId() + "|" + (a == a2) + a.equals(a2) + a.equals(b) + (o instanceof ModContainer)
            + kind(a) + ModContainer.class.isInstance(a)
            + "|" + (info == a2.getModInfo()) + info.equals(a2.getModInfo()) + info.equals(b.getModInfo())
            + info.getModId() + info.getVersion().toString() + b.getModInfo().getVersion().toString() + kind(info)
            + "|" + list.getModContainerById("missing").isPresent() + list.isLoaded("demo") + list.isLoaded("missing")
            + list.getMods().contains(info)
            + "|" + List.of(a, b).contains(a2) + List.of(a, b).indexOf(b) + ids.get(a2) + ids.get(b) + ids.get(a2.getModInfo())
            + new ArrayList<>(List.of(b, a)).indexOf(a2);
    }

    public static String configTypes() {
        ModConfig.Type c = ModConfig.Type.CLIENT;
        ModConfig.Type s = ModConfig.Type.STARTUP;
        ModConfig.Type l = ModConfig.Type.LOCAL;
        Object o = c;
        List<ModConfig.Type> sorted = new ArrayList<>(List.of(s, c, l, ModConfig.Type.SYNCED));
        Collections.sort(sorted);
        IdentityHashMap<Object, String> ids = new IdentityHashMap<>();
        ids.put(c, "c");
        return c + "|" + c.name() + c.ordinal() + s.ordinal() + c.extension() + s.extension()
            + "|" + (c == ModConfig.Type.CLIENT) + c.equals(ModConfig.Type.CLIENT) + c.equals(s) + c.compareTo(s)
            + s.compareTo(l) + "|" + (o instanceof ModConfig.Type) + (o instanceof Enum) + (o instanceof Comparable)
            + kind(s) + "|" + sorted + List.of(c, s).contains(ModConfig.Type.CLIENT) + ids.get(ModConfig.Type.CLIENT)
            + ids.get(s) + new TreeSet<>(List.of(s, c)).first() + Map.of(c, 1).get(ModConfig.Type.CLIENT);
    }

    public static String worldVersion() {
        WorldVersion v = DetectedVersion.tryDetectVersion();
        WorldVersion w = DetectedVersion.tryDetectVersion();
        Object o = v;
        return v.name() + "|" + v.equals(w) + v.equals(v) + (o instanceof WorldVersion) + List.of(v).contains(w)
            + List.of(v).contains(v);
    }

    /** Two detections are two records. */
    public static String worldIdentity() {
        return "" + (DetectedVersion.tryDetectVersion() == DetectedVersion.tryDetectVersion());
    }

    /**
     * Strings spelled like the old markers, or a handle's text, are no loader object. The handle-text ones keep
     * String's class, `instanceof` and hash too; an old `word/rest` marker's are still read as a marker until #156
     * P12/P17 (its hash is counted), so it shows only equality, concatenation and `HashMap` keying.
     */
    public static String spelled(ModContainer a) {
        Object[] texts = {"neoforge-mod-container/demo/1.2.3", "mod-info/demo/1.2.3", "mod-file-info/demo",
            "fml-config-type/CLIENT", "world-version/26.3", "nfmc:rdemo,i1", "nfct:rCLIENT", "wver:r26.3"};
        StringBuilder out = new StringBuilder();
        Map<Object, String> map = new HashMap<>();
        for (Object t : texts) {
            if (!((String) t).contains("/")) {
                out.append(t.getClass().getSimpleName()).append(t instanceof String).append(t.hashCode());
            }
            out.append(a.equals(t)).append(a.getModInfo().equals(t)).append(ModConfig.Type.CLIENT.equals(t))
                .append(t.equals(a)).append(t.equals(ModConfig.Type.CLIENT)).append(t + "!").append(',');
            map.put(t, "s");
        }
        out.append(map.size()).append(map.get("fml-config-type/CLIENT")).append(map.containsKey(a));
        return out.toString();
    }

    /** A String cast to a loader type is a ClassCastException, never the object it spells. */
    public static String forged() {
        StringBuilder out = new StringBuilder();
        Object container = "neoforge-mod-container/demo/1.2.3";
        Object type = "fml-config-type/CLIENT";
        Object info = "mod-info/demo/1.2.3";
        Object version = "world-version/26.3";
        try {
            out.append(((ModContainer) container).getModId());
        } catch (ClassCastException e) {
            out.append("CCE");
        }
        try {
            out.append(((ModConfig.Type) type).name());
        } catch (ClassCastException e) {
            out.append("CCE");
        }
        try {
            out.append(((IModInfo) info).getModId());
        } catch (ClassCastException e) {
            out.append("CCE");
        }
        try {
            out.append(((WorldVersion) version).name());
        } catch (ClassCastException e) {
            out.append("CCE");
        }
        return out.toString();
    }
}

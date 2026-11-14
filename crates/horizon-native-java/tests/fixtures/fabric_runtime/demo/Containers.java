package demo;

import java.nio.file.Path;
import java.util.Collection;
import java.util.List;
import net.fabricmc.loader.api.FabricLoader;
import net.fabricmc.loader.api.ModContainer;

/**
 * Two mods' containers keep their own collections (#233 review B1): Fabric Loader 0.19.5's
 * `ModContainerImpl.getRootPaths` returns the container's own `roots` field, and `V1ModMetadata`'s
 * getters return its own fields, so each is the same object on every call, never another mod's.
 * Those collections are unmodifiable in Fabric (`unmodifiableCollection`, a `singletonList`), which
 * the host doesn't model yet (#238), so no probe writes to them.
 */
public final class Containers {
    static String b(boolean v) {
        return v ? "T" : "F";
    }

    static ModContainer mod(String id) {
        return FabricLoader.getInstance().getModContainer(id).get();
    }

    /** `a`'s root paths: one list, still `a`'s after `c`'s are read. */
    public static String roots() {
        ModContainer a = mod("moda");
        ModContainer c = mod("modb");
        List<Path> ra = a.getRootPaths();
        String before = ra.get(0).toString();
        List<Path> rb = c.getRootPaths();
        return b(before.equals(ra.get(0).toString())) + b(ra.get(0).equals(rb.get(0))) + b(ra == rb)
            + b(ra == a.getRootPaths()) + b(rb == c.getRootPaths()) + ra.size() + rb.size();
    }

    /** Metadata collections: one per mod per getter, the same on every call. */
    public static String metadata() {
        ModContainer a = mod("moda");
        ModContainer c = mod("modb");
        Collection<String> license = a.getMetadata().getLicense();
        return b((Object) a.getMetadata().getAuthors() == c.getMetadata().getAuthors())
            + b(a.getMetadata().getAuthors() == a.getMetadata().getAuthors())
            + b(license == c.getMetadata().getLicense()) + b(license == a.getMetadata().getLicense())
            + a.getMetadata().getLicense() + c.getMetadata().getLicense();
    }
}

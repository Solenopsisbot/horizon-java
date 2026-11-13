package demo;

import net.fabricmc.loader.api.FabricLoader;
import net.fabricmc.loader.api.ModContainer;

public final class Probe {
    public static String ids() {
        StringBuilder out = new StringBuilder();
        for (ModContainer container : FabricLoader.getInstance().getAllMods()) {
            if (out.length() > 0) {
                out.append(',');
            }
            out.append(container.getMetadata().getId());
        }
        return out.toString();
    }

    public static String describe() {
        StringBuilder out = new StringBuilder();
        for (ModContainer container : FabricLoader.getInstance().getAllMods()) {
            String id = container.getMetadata().getId();
            if (id.equals("alpha") || id.equals("beta") || id.equals("gamma")
                || id.equals("fabricloader") || id.equals("mixinextras")
                || id.equals("fabric-api-base")) {
                out.append(Describe.line(container.getMetadata())).append('\n');
            }
        }
        return out.toString();
    }

    /** What the built-in containers know, in list order. */
    public static String builtins() {
        StringBuilder out = new StringBuilder();
        for (ModContainer container : FabricLoader.getInstance().getAllMods()) {
            String id = container.getMetadata().getId();
            if (id.equals("fabricloader")) {
                out.append("loader-name=").append(container.getMetadata().getName()).append(';');
            }
            if (id.equals("java")) {
                out.append("java=").append(container.getMetadata().getName()).append('/')
                    .append(container.getMetadata().getVersion().getFriendlyString()).append('/')
                    .append(container.getMetadata().getType()).append('/')
                    .append(container.getMetadata().containsCustomValue("modmenu")).append(';');
            }
            if (id.equals("minecraft")) {
                out.append("minecraft=").append(container.getMetadata().getName()).append('/')
                    .append(container.getMetadata().getVersion().getFriendlyString()).append('/')
                    .append(container.getMetadata().getEnvironment().name()).append('/')
                    .append(container.getMetadata().getContact().get("homepage").orElse("-")).append(';');
            }
        }
        return out.toString();
    }

    /** Identity, text and types of the loader's objects ({@link Identity}). */
    public static String identity() {
        FabricLoader loader = FabricLoader.getInstance();
        return Identity.facts(loader.getModContainer("alpha").get(), loader.getModContainer("alpha").get(),
            loader.getModContainer("beta").get(), loader.getAllMods());
    }

    public static String spelled() {
        return Identity.spelled(FabricLoader.getInstance().getModContainer("alpha").get());
    }

    /** What only a JVM knows: an identity hash, `Object.toString`, the implementation class. */
    public static String containerHash() {
        return "" + FabricLoader.getInstance().getModContainer("alpha").get().hashCode();
    }

    public static String metadataText() {
        String text = String.valueOf(FabricLoader.getInstance().getModContainer("alpha").get().getMetadata());
        return text == null ? "null" : text;
    }

    public static String containerClass() {
        Object type = FabricLoader.getInstance().getModContainer("alpha").get().getClass();
        return String.valueOf(type);
    }

    /** Objects the loader shares ({@link Identity#shared}). */
    public static String shared() {
        FabricLoader loader = FabricLoader.getInstance();
        return Identity.shared(loader.getModContainer("alpha").get(), loader.getModContainer("beta").get(),
            loader.getModContainer("minecraft").get().getMetadata(), loader.getModContainer("java").get().getMetadata());
    }

    public static String sharedSets() {
        FabricLoader loader = FabricLoader.getInstance();
        return Identity.sharedSets(loader.getModContainer("alpha").get(), loader.getModContainer("beta").get());
    }

    /** {@code getModContainer} finds every container {@code getAllMods} lists, and only those of this run. */
    public static String containersAgree() {
        FabricLoader loader = FabricLoader.getInstance();
        StringBuilder out = new StringBuilder();
        for (ModContainer container : loader.getAllMods()) {
            String id = container.getMetadata().getId();
            out.append(id).append('=').append(loader.getModContainer(id).map(found -> found == container).orElse(false))
                .append(loader.isModLoaded(id)).append(',');
        }
        return out.append(loader.getModContainer("nosuchmod").isPresent()).toString();
    }
}

package demo;

import java.util.ArrayList;
import java.util.Collection;
import java.util.List;
import net.fabricmc.loader.api.ModContainer;
import net.fabricmc.loader.api.metadata.ContactInformation;
import net.fabricmc.loader.api.metadata.CustomValue;
import net.fabricmc.loader.api.metadata.ModMetadata;
import net.fabricmc.loader.api.metadata.ModOrigin;
import net.fabricmc.loader.api.metadata.Person;

/**
 * Identity, text and type facts of Fabric Loader's objects, the same text on a JVM and in Horizon:
 * {@code a} and {@code aAgain} are what two {@code getModContainer("alpha")} calls answer, {@code b}
 * another mod's container and {@code all} what {@code getAllMods()} answers.
 */
public final class Identity {
    public static String facts(ModContainer a, ModContainer aAgain, ModContainer b, Collection<ModContainer> all) {
        ModMetadata m = a.getMetadata();
        Person p = m.getAuthors().iterator().next();
        CustomValue cv = m.getCustomValue("lithium:options");
        ModOrigin o = a.getOrigin();
        Object oa = a;
        return a + "|" + (a == aAgain) + a.equals(aAgain) + a.equals(b) + (oa instanceof ModContainer)
            + "|" + (m == aAgain.getMetadata()) + m.equals(aAgain.getMetadata()) + m.equals(b.getMetadata())
            + (m instanceof ModMetadata)
            + "|" + (m.getAuthors().iterator().next() == p) + p.equals(p) + (p instanceof Person)
            + (p.getContact() == p.getContact()) + (p.getContact() instanceof ContactInformation)
            + (m.getContact() == aAgain.getMetadata().getContact())
            + "|" + (m.getCustomValue("lithium:options") == cv) + (cv instanceof CustomValue)
            + (cv.getAsObject() == cv) + (cv.getAsObject().get("mixin.a") == cv.getAsObject().get("mixin.a"))
            + "|" + o + (o == a.getOrigin()) + o.equals(aAgain.getOrigin()) + o.equals(b.getOrigin())
            + o.getKind().name() + (o instanceof ModOrigin)
            + "|" + all.contains(a) + all.contains(b) + List.of(a, b).indexOf(b) + new ArrayList<>(all).contains(aAgain)
            + List.of(m).contains(aAgain.getMetadata());
    }

    /** Strings spelled like the old markers, or a handle's text, are no loader object. */
    public static String spelled(ModContainer a) {
        Object[] texts = {"mod-container/alpha/1.2.0+mc26.3", "metadata/alpha/1.2.0+mc26.3", "mod-origin/path/alpha",
            "fabric-person/alpha/authors/0", "fabric-contact/alpha/contact", "fabric-cv/alpha/custom/note",
            "hfmc:ralpha,r1.2.0+mc26.3", "hfmm:ralpha"};
        StringBuilder out = new StringBuilder();
        for (Object t : texts) {
            out.append(t instanceof ModContainer).append(t instanceof ModMetadata).append(t instanceof ModOrigin)
                .append(t instanceof Person).append(t instanceof CustomValue).append(a.equals(t))
                .append(a.getMetadata().equals(t)).append(a.getOrigin().equals(t)).append(t.equals(a)).append(t).append(',');
        }
        java.util.Map<Object, String> map = new java.util.HashMap<>();
        for (Object t : new Object[] {"hfmc:ralpha,r1.2.0+mc26.3", "hfmm:ralpha"}) {
            out.append(t.getClass().getSimpleName()).append(t instanceof String).append(t.hashCode()).append(',');
            map.put(t, "s");
        }
        return out.append(map.size()).append(map.get("hfmm:ralpha")).toString();
    }
}

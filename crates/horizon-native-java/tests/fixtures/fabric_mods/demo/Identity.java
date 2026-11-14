package demo;

import java.util.ArrayList;
import java.util.Collection;
import java.util.HashSet;
import java.util.IdentityHashMap;
import java.util.Iterator;
import java.util.List;
import java.util.Set;
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

    /**
     * Objects Fabric Loader shares: {@code ContactInformation.EMPTY} (a metadata without {@code contact}, a string
     * author, a built-in) and the {@code true}/{@code false}/{@code null} custom values. {@code a}/{@code b} are
     * alpha's and beta's containers, {@code mc}/{@code java} the built-ins' metadata.
     */
    public static String shared(ModContainer a, ModContainer b, ModMetadata mc, ModMetadata java) {
        ModMetadata m = a.getMetadata();
        ModMetadata mb = b.getMetadata();
        Iterator<Person> people = m.getAuthors().iterator();
        Person ann = people.next();
        Person bob = people.next();
        ContactInformation empty = mb.getContact();
        CustomValue t = mb.getCustomValue("t");
        CustomValue u = mb.getCustomValue("u");
        CustomValue f = mb.getCustomValue("f");
        CustomValue n = mb.getCustomValue("n");
        CustomValue nothing = m.getCustomValue("nothing");
        CustomValue note = m.getCustomValue("note");
        CustomValue.CvArray arr = mb.getCustomValue("arr").getAsArray();
        CustomValue mixinA = m.getCustomValue("lithium:options").getAsObject().get("mixin.a");
        CustomValue mixinB = m.getCustomValue("lithium:options").getAsObject().get("mixin.b");
        return "" + (empty == ann.getContact()) + empty.equals(ann.getContact()) + (ann.getContact() == bob.getContact())
            + bob.getContact().equals(empty) + (m.getContact() == empty) + m.getContact().equals(empty)
            + (mc.getContact() == java.getContact()) + (mc.getContact() == empty) + (ContactInformation.EMPTY == empty)
            + empty.get("email").isPresent()
            + "|" + (t == u) + t.equals(u) + (t == mixinB) + (f == mixinA) + mixinA.equals(f) + (n == nothing)
            + n.equals(nothing) + (t == f) + t.equals(f) + (t == note) + note.equals(t)
            + "|" + (arr.get(0) == t) + (arr.get(1) == n) + (arr.get(2) == arr.get(2)) + arr.get(2).equals(note)
            + t.getAsBoolean() + f.getAsBoolean() + t.getType().name() + n.getType().name()
            + "|" + List.of(t, f).contains(u) + List.of(t).indexOf(mixinB) + List.of(note).contains(t)
            + List.of(empty).contains(ann.getContact());
    }

    /** The shared objects in hash-based and identity collections. */
    public static String sharedSets(ModContainer a, ModContainer b) {
        ContactInformation empty = b.getMetadata().getContact();
        ContactInformation annContact = a.getMetadata().getAuthors().iterator().next().getContact();
        CustomValue t = b.getMetadata().getCustomValue("t");
        CustomValue u = b.getMetadata().getCustomValue("u");
        CustomValue f = b.getMetadata().getCustomValue("f");
        String values;
        try {
            values = "" + Set.of(t, u).size();
        } catch (IllegalArgumentException e) {
            values = "IAE";
        }
        String contacts;
        try {
            contacts = "" + Set.of(empty, annContact).size();
        } catch (IllegalArgumentException e) {
            contacts = "IAE";
        }
        IdentityHashMap<Object, String> identity = new IdentityHashMap<>();
        identity.put(t, "x");
        return new HashSet<>(List.of(empty, annContact)).size() + "," + new HashSet<>(List.of(t, u, f)).size() + ","
            + values + contacts + "," + identity.get(u);
    }
}

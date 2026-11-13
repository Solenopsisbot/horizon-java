package net.neoforged.bus.api;

// A jar's own copy of the bus's Event (as CreativeCore bundles bus-7.2.0 on Fabric): it ships in the
// jar, so its class file answers, never NeoForge's event table.
public abstract class Event extends demo.Base implements demo.Marker {
    public void mine() {}
}

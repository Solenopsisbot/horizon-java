package net.neoforged.fml;
public abstract class ModContainer {
    public final String getModId() { throw new AssertionError(); }
    public net.neoforged.neoforgespi.language.IModInfo getModInfo() { throw new AssertionError(); }
}

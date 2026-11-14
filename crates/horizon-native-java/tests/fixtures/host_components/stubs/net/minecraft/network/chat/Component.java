package net.minecraft.network.chat;

// Compile-only stub of Minecraft 26.3's interface (the oracle runs the real one).
public interface Component extends com.mojang.brigadier.Message, FormattedText {
    static MutableComponent literal(String text) {
        return null;
    }

    static MutableComponent empty() {
        return null;
    }

    static MutableComponent translatable(String key) {
        return null;
    }

    static MutableComponent translatable(String key, Object... args) {
        return null;
    }

    MutableComponent copy();

    String getString();
}

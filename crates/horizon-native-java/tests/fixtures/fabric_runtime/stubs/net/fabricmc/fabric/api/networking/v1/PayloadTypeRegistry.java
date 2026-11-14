package net.fabricmc.fabric.api.networking.v1;
public interface PayloadTypeRegistry<B> {
    static PayloadTypeRegistry<Object> clientboundPlay() { throw new AssertionError(); }
    static PayloadTypeRegistry<Object> serverboundPlay() { throw new AssertionError(); }
    static PayloadTypeRegistry<Object> clientboundConfiguration() { throw new AssertionError(); }
    static PayloadTypeRegistry<Object> serverboundConfiguration() { throw new AssertionError(); }
}

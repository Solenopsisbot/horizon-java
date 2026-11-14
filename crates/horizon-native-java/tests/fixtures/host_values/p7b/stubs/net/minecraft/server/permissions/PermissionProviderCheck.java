package net.minecraft.server.permissions;

public final class PermissionProviderCheck<T extends PermissionSetSupplier> implements java.util.function.Predicate<T> {
  public boolean test(T t) { return false; }
}

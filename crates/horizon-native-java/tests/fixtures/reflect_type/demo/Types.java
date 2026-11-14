package demo;

import java.lang.reflect.AnnotatedArrayType;
import java.lang.reflect.AnnotatedParameterizedType;
import java.lang.reflect.AnnotatedType;
import java.lang.reflect.AnnotatedTypeVariable;
import java.lang.reflect.AnnotatedWildcardType;
import java.lang.reflect.Field;
import java.lang.reflect.GenericArrayType;
import java.lang.reflect.Method;
import java.lang.reflect.ParameterizedType;
import java.lang.reflect.Type;
import java.lang.reflect.TypeVariable;
import java.lang.reflect.WildcardType;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.HashSet;
import java.util.IdentityHashMap;
import java.util.List;
import java.util.Map;
import java.util.Set;

/**
 * Reflection over this class's own generic declarations, the same text on a JVM and in Horizon: field, method,
 * superclass and interface types (`Class`, `ParameterizedType`, `TypeVariable`, `WildcardType`,
 * `GenericArrayType`) and their `AnnotatedType`s.
 */
public class Types<T extends Comparable<T>> extends ArrayList<T> implements Comparable<Types<T>> {
    public List<String> names;
    public List<String> names2;
    public Map<String, ? extends Number> map;
    public T value;
    public T[] values;
    public List<String>[] lists;
    public String[] strings;
    public int count;
    public Map.Entry<String, Integer> entry;
    public Outer<String>.Inner<Integer> inner;
    public List<?> any;
    public List<? extends Object> anyObject;
    public List<? super Integer> sink;

    public static class Outer<O> {
        public class Inner<I> {
        }
    }

    public <U extends Number> U pick(List<? super U> in, T t) {
        return null;
    }

    public <U extends Number> U pick2() {
        return null;
    }

    @Override
    public int compareTo(Types<T> other) {
        return 0;
    }

    static Field f(String name) throws Exception {
        return Types.class.getDeclaredField(name);
    }

    static Type g(String name) throws Exception {
        return f(name).getGenericType();
    }

    static Method pick() throws Exception {
        return Types.class.getMethod("pick", List.class, Comparable.class);
    }

    static String text(Type t) {
        return t instanceof Class ? "C:" + t.getTypeName() : t.toString();
    }

    static String shape(Type t) {
        return switch (t) {
            case Class<?> c -> "C";
            case ParameterizedType p -> "P";
            case GenericArrayType a -> "A";
            case TypeVariable<?> v -> "V";
            case WildcardType w -> "W";
            default -> "?";
        };
    }

    /** `toString`/`getTypeName` of every shape, and the shapes a type switch sees. */
    public static String texts() throws Exception {
        StringBuilder out = new StringBuilder();
        for (String name : new String[] {"names", "map", "value", "values", "lists", "strings", "count", "entry",
                "inner", "any", "anyObject", "sink"}) {
            Type t = g(name);
            out.append(shape(t)).append(text(t)).append('|').append(t.getTypeName()).append(';');
        }
        out.append(text(Types.class.getGenericSuperclass())).append(';')
            .append(text(Types.class.getGenericInterfaces()[0])).append(';')
            .append(text(pick().getGenericReturnType())).append(';');
        for (Type t : pick().getGenericParameterTypes()) {
            out.append(text(t)).append(',');
        }
        return out.toString();
    }

    /** What each shape answers about its parts. */
    public static String structure() throws Exception {
        ParameterizedType names = (ParameterizedType) g("names");
        ParameterizedType map = (ParameterizedType) g("map");
        WildcardType w = (WildcardType) map.getActualTypeArguments()[1];
        WildcardType sink = (WildcardType) ((ParameterizedType) g("sink")).getActualTypeArguments()[0];
        TypeVariable<?> v = (TypeVariable<?>) g("value");
        GenericArrayType values = (GenericArrayType) g("values");
        GenericArrayType lists = (GenericArrayType) g("lists");
        ParameterizedType inner = (ParameterizedType) g("inner");
        TypeVariable<?> u = (TypeVariable<?>) pick().getGenericReturnType();
        return "" + (names.getRawType() == List.class) + (names.getActualTypeArguments()[0] == String.class)
            + (names.getOwnerType() == null) + names.getActualTypeArguments().length
            + "|" + (w.getUpperBounds()[0] == Number.class) + w.getLowerBounds().length + w.getUpperBounds().length
            + (sink.getLowerBounds()[0] == Integer.class) + (sink.getUpperBounds()[0] == Object.class)
            + "|" + v.getName() + (v.getGenericDeclaration() == Types.class) + v.getBounds().length + v.getBounds()[0]
            + "|" + values.getGenericComponentType().equals(v) + lists.getGenericComponentType()
            + "|" + inner.getOwnerType() + (inner.getRawType() == Outer.Inner.class)
            + "|" + (g("strings") == String[].class) + (g("count") == int.class)
            + "|" + u.getName() + u.getBounds().length + (u.getBounds()[0] == Number.class);
    }

    /** `equals` as each JDK implementation specifies it; `instanceof` of every shape. */
    public static String equality() throws Exception {
        Type a = g("names");
        Type b = g("names2");
        Type c = g("map");
        Type u1 = pick().getGenericReturnType();
        Type u2 = Types.class.getMethod("pick2").getGenericReturnType();
        Object o = a;
        return "" + a.equals(b) + b.equals(a) + a.equals(c) + a.equals(List.class) + List.class.equals(a)
            + a.equals("java.util.List<java.lang.String>") + g("any").equals(g("anyObject"))
            + g("value").equals(g("value")) + g("values").equals(g("values")) + g("value").equals(u1)
            + u1.equals(pick().getGenericReturnType()) + u1.equals(u2) + new ArrayList<>(List.of(a)).contains(b)
            + List.of(c, a).indexOf(b) + ((ParameterizedType) g("inner")).getOwnerType().equals(((ParameterizedType) g("inner")).getOwnerType())
            + "|" + (o instanceof ParameterizedType) + (o instanceof TypeVariable) + (o instanceof Class)
            + (g("value") instanceof TypeVariable) + (((Object) g("value")) instanceof WildcardType)
            + (g("values") instanceof GenericArrayType) + (g("strings") instanceof GenericArrayType)
            + (((ParameterizedType) c).getActualTypeArguments()[1] instanceof WildcardType)
            + ParameterizedType.class.isInstance(a) + Class.class.isInstance(a);
    }

    /** `AnnotatedType`s of fields and a method's result. */
    public static String annotated() throws Exception {
        AnnotatedType names = f("names").getAnnotatedType();
        return "" + (names instanceof AnnotatedParameterizedType) + names.getType().equals(g("names"))
            + names.getAnnotations().length + ((AnnotatedParameterizedType) names).getAnnotatedActualTypeArguments()[0]
                .getType().equals(String.class)
            + (f("strings").getAnnotatedType() instanceof AnnotatedArrayType)
            + (f("value").getAnnotatedType() instanceof AnnotatedTypeVariable)
            + (f("count").getAnnotatedType() instanceof AnnotatedParameterizedType)
            + (pick().getAnnotatedReturnType() instanceof AnnotatedTypeVariable)
            + (((AnnotatedParameterizedType) f("map").getAnnotatedType()).getAnnotatedActualTypeArguments()[1]
                instanceof AnnotatedWildcardType)
            + "|" + names + "|" + f("strings").getAnnotatedType() + "|" + f("value").getAnnotatedType()
            + "|" + f("count").getAnnotatedType() + "|" + f("map").getAnnotatedType();
    }

    /** What only a JVM knows: identity of two lookups' types, hashes, an inferred owner. */
    public static String identity() throws Exception {
        Type a = g("names");
        Type b = g("names2");
        Set<Type> set = new HashSet<>(List.of(a, b));
        IdentityHashMap<Type, Integer> ids = new IdentityHashMap<>();
        ids.put(a, 1);
        String setOf;
        try {
            setOf = "" + Set.of(a, b).size();
        } catch (IllegalArgumentException e) {
            setOf = "IAE";
        }
        String declaration;
        try {
            declaration = "" + ((TypeVariable<?>) pick().getGenericReturnType()).getGenericDeclaration().equals(pick());
        } catch (RuntimeException e) {
            declaration = e.getClass().getSimpleName();
        }
        return "" + (a == b) + (a.hashCode() == b.hashCode()) + set.size() + ids.containsKey(b) + setOf
            + ((ParameterizedType) g("entry")).getOwnerType() + "|" + declaration;
    }

    /** Strings spelled like the old markers, or a handle's key, are no `Type`. */
    public static String spelled() throws Exception {
        Type a = g("names");
        Object[] texts = {"type/P43:14:java/util/List1:C19:16:java/lang/String-", "annotated-type/java/lang/String",
            "type/V4:1:T0:", "hrtyp:rP,rjava/util/List,n,rjava/lang/String", "hrann:rjava/lang/String"};
        StringBuilder out = new StringBuilder();
        Map<Object, String> map = new HashMap<>();
        for (Object t : texts) {
            if (!((String) t).contains("/")) {
                out.append(t.getClass().getSimpleName()).append(t instanceof String).append(t.hashCode())
                    .append(t instanceof Type).append(t instanceof ParameterizedType);
            }
            out.append(a.equals(t)).append(t.equals(a)).append(t + "!").append(',');
            map.put(t, "s");
        }
        return out.append(map.size()).append(map.get("type/V4:1:T0:")).append(map.containsKey(a)).toString();
    }

    /** A String cast to a `Type` shape is a ClassCastException, never the type it spells. */
    public static String forged() {
        StringBuilder out = new StringBuilder();
        Object p = "type/P43:14:java/util/List1:C19:16:java/lang/String-";
        Object v = "type/V4:1:T0:";
        try {
            out.append(((ParameterizedType) p).getRawType());
        } catch (ClassCastException e) {
            out.append("CCE");
        }
        try {
            out.append(((TypeVariable<?>) v).getName());
        } catch (ClassCastException e) {
            out.append("CCE");
        }
        return out.toString();
    }

    public static void main(String[] args) throws Exception {
        for (String probe : new String[] {"texts", "structure", "equality", "annotated", "identity", "spelled",
                "forged"}) {
            String out;
            try {
                out = (String) Types.class.getMethod(probe).invoke(null);
            } catch (java.lang.reflect.InvocationTargetException e) {
                out = "THREW " + e.getCause();
            }
            System.out.println(probe + "=" + out);
        }
    }
}

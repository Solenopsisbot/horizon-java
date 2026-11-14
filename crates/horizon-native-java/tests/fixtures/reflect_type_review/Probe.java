package demo;

import java.lang.annotation.ElementType;
import java.lang.annotation.Retention;
import java.lang.annotation.RetentionPolicy;
import java.lang.annotation.Target;
import java.lang.reflect.AnnotatedParameterizedType;
import java.lang.reflect.AnnotatedType;
import java.lang.reflect.Field;
import java.lang.reflect.ParameterizedType;
import java.lang.reflect.Type;
import java.util.List;

/**
 * #196's review (#156 P10c): type-use annotations, a class's `Type.equals`, a middle owner in a
 * signature's inner-class chain, and a String cast to a reflective `Type` shape.
 */
public class Probe<T, E> {
  @Retention(RetentionPolicy.RUNTIME)
  @Target(ElementType.TYPE_USE)
  public @interface TU {}

  public class Outer<X> {
    public class Mid {
      public class Deep<Y> {}
    }
  }

  public Probe<T, E>.Outer<String>.Mid.Deep<Long> omd;
  public @TU String annotated;
  public List<@TU String> annArg;
  public String plain;
  public List<String> plainArg;

  static Field f(String name) throws Exception {
    return Probe.class.getField(name);
  }

  /** `Outer<String>.Mid.Deep<Long>`: every segment after `Outer<String>` is its own type. */
  public static String owners() throws Exception {
    ParameterizedType omd = (ParameterizedType) f("omd").getGenericType();
    Type mid = omd.getOwnerType();
    ParameterizedType m = (ParameterizedType) mid;
    return omd.getTypeName() + ";" + mid.getTypeName() + ";" + m.getRawType().getTypeName() + ","
        + m.getActualTypeArguments().length + "," + m.getOwnerType().getTypeName() + ";"
        + (mid instanceof ParameterizedType);
  }

  /** A member with no type-use annotations: none, exactly. */
  public static String unannotated() throws Exception {
    AnnotatedType at = f("plain").getAnnotatedType();
    AnnotatedParameterizedType ap = (AnnotatedParameterizedType) f("plainArg").getAnnotatedType();
    return at.getAnnotations().length + "" + at.isAnnotationPresent(TU.class) + "," + at + ","
        + ap.getAnnotatedActualTypeArguments()[0].getAnnotations().length;
  }

  /** `@TU String`: Java 25 reports the annotation (counted in Horizon, which does not read it). */
  public static String annotatedField() throws Exception {
    AnnotatedType at = f("annotated").getAnnotatedType();
    return at.getAnnotations().length + "" + at.isAnnotationPresent(TU.class) + "," + at;
  }

  /** `List<@TU String>`: the argument's annotation (counted in Horizon); the type stays exact. */
  public static String annotatedArgument() throws Exception {
    AnnotatedParameterizedType ap = (AnnotatedParameterizedType) f("annArg").getAnnotatedType();
    return ap.getType().getTypeName() + ","
        + ap.getAnnotatedActualTypeArguments()[0].getAnnotations().length;
  }

  /** A class as a `Type` never equals a String spelling it (counted in Horizon until #156 P13). */
  public static String classEqualsSpelling() {
    Type s = String.class;
    return "" + s.equals("java/lang/String");
  }

  /** ... and another class or `null`: exactly false. */
  public static String classEqualsOther() {
    Type s = String.class;
    return "" + s.equals(Integer.class) + s.equals(null);
  }

  static String asParameterized(Object value) {
    try {
      ParameterizedType type = (ParameterizedType) value;
      return "ok";
    } catch (ClassCastException expected) {
      return "CCE";
    }
  }

  static String asType(Object value) {
    try {
      Type type = (Type) value;
      return "ok";
    } catch (ClassCastException expected) {
      return "CCE";
    }
  }

  /** A plain String cast to a reflective `Type` shape. */
  public static String castPlain() {
    return asParameterized("abc") + asType("abc");
  }
}

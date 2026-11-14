import demo.HostIterators;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;

/** Runs every HostIterators probe on the JDK, in name order. */
public final class HostIteratorsOracle {
  public static void main(String[] args) throws Exception {
    java.util.List<Method> methods = new java.util.ArrayList<>();
    for (Method m : HostIterators.class.getDeclaredMethods()) {
      if (Modifier.isPublic(m.getModifiers()) && Modifier.isStatic(m.getModifiers())) methods.add(m);
    }
    methods.sort(java.util.Comparator.comparing(Method::getName));
    for (Method m : methods) {
      Object answer;
      try {
        answer = m.invoke(null);
      } catch (java.lang.reflect.InvocationTargetException x) {
        answer = "threw " + x.getCause();
      }
      System.out.println(m.getName() + "=" + answer);
    }
  }
}

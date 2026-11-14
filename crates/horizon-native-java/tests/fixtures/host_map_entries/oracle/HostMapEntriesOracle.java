import demo.HostMapEntries;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;

/** Runs every HostMapEntries probe on the JDK, in name order. */
public final class HostMapEntriesOracle {
  public static void main(String[] args) throws Exception {
    java.util.List<Method> methods = new java.util.ArrayList<>();
    for (Method m : HostMapEntries.class.getDeclaredMethods()) {
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

import demo.JdkObjects;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;

/** Runs every JdkObjects probe on the JDK, in declaration order. */
public final class JdkObjectsOracle {
  public static void main(String[] args) throws Exception {
    for (Method m : JdkObjects.class.getDeclaredMethods()) {
      if (!Modifier.isStatic(m.getModifiers()) || !Modifier.isPublic(m.getModifiers())) continue;
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

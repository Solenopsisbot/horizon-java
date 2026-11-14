import demo.HostComponents;
import java.lang.reflect.Method;

/** Runs every HostComponents probe on Minecraft 26.3's own chat components. */
public final class HostComponentsOracle {
  public static void main(String[] args) throws Exception {
    String[] probes = {"spelled", "encoded", "spelledTypes", "real", "text", "copy", "appended", "styled",
      "appendInPlace", "typeSwitch", "hashed", "listed", "mapped", "duplicate", "duplicateN", "hash",
      "sorted"};
    for (String probe : probes) {
      Method m = HostComponents.class.getMethod(probe);
      Object answer;
      try {
        answer = m.invoke(null);
      } catch (java.lang.reflect.InvocationTargetException x) {
        answer = "threw " + x.getCause();
      }
      System.out.println(probe + "=" + answer);
    }
  }
}

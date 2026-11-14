import demo.HostJdkConstants;
import java.lang.reflect.Method;

/** Runs every HostJdkConstants probe on the JDK. */
public final class HostJdkConstantsOracle {
  public static void main(String[] args) throws Exception {
    for (String probe : new String[] {"spelled", "constants", "locales", "patterns", "localeIdentity", "cachedIdentity", "turkish", "unitHash"}) {
      Method m = HostJdkConstants.class.getMethod(probe);
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

import demo.HostJdkValues;
import java.lang.reflect.Method;

/** Runs every HostJdkValues probe on the JDK. */
public final class HostJdkValuesOracle {
  public static void main(String[] args) throws Exception {
    for (String probe : new String[] {"spelled", "spelledTypes", "uuids", "durations", "uris", "identity", "uriCase", "uriHash"}) {
      Method m = HostJdkValues.class.getMethod(probe);
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

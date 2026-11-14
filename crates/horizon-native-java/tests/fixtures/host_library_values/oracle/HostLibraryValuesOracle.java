import demo.HostLibraryValues;
import java.lang.reflect.Method;

/** Runs every HostLibraryValues probe on gson 2.14.0 and guava 33.6.0. */
public final class HostLibraryValuesOracle {
  public static void main(String[] args) throws Exception {
    for (String probe : new String[] {"spelled", "styles", "tokens", "styleText", "tokenHash", "typeSwitch"}) {
      Method m = HostLibraryValues.class.getMethod(probe);
      Object answer;
      try {
        answer = m.invoke(null);
      } catch (java.lang.reflect.InvocationTargetException x) {
        answer = "threw " + x.getCause();
      }
      System.out.println(probe + "=" + String.valueOf(answer).replace("\n", "\\n"));
    }
  }
}

import demo.HostCodecs;
import java.lang.reflect.Method;

/** Runs every HostCodecs probe on DataFixerUpper 10.0.21. */
public final class HostCodecsOracle {
  public static void main(String[] args) throws Exception {
    for (String probe : new String[] {"spelled", "spelledTypes", "pairs", "nested", "identityHeld", "opaqueCodec", "codecConstants",
        "codecDuplicate", "codecText", "pairVsJdk"}) {
      Method m = HostCodecs.class.getMethod(probe);
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

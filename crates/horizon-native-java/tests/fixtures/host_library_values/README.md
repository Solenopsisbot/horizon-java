# Gson `FormattingStyle` and `TypeToken`, Guava `TypeToken` as host objects (#156 P5b)

`demo/HostLibraryValues.java` makes styles and tokens through the real API: Gson's
`FormattingStyle`, Guava's `TypeToken.of`, and Gson's `TypeToken` (`get`, `getArray`, a jar's
`new TypeToken<T>() {}`), whose type is Gson's own canonical copy of the JDK's. `host_gson_tests.rs`
compiles it against the compile-only stubs in `stubs/` and ships only `demo/*`, so every call
reaches the runtime's model.

The expected answers are `oracle/HostLibraryValuesOracle.java` on OpenJDK 25.0.5+8 against the
26.3 server's `gson-2.14.0.jar`, `guava-33.6.0-jre.jar` and `failureaccess-1.0.3.jar`, two runs
alike except the identity hashes. The libraries are the player's own and are never committed.

```sh
CP=gson-2.14.0.jar:guava-33.6.0-jre.jar:failureaccess-1.0.3.jar
javac --release 21 -d out -cp $CP demo/HostLibraryValues.java oracle/HostLibraryValuesOracle.java
$JAVA25/bin/java -cp out:$CP HostLibraryValuesOracle
```

# Chat components as host objects (#156 P5a)

`demo/HostComponents.java` makes `Component.literal`/`translatable` components and calls
`append`/`withStyle`/`copy`/`getString`/`equals` on them through the real API.
`host_component_tests.rs` compiles it against the compile-only stubs in `stubs/` and ships only
`demo/*`, so every call reaches the runtime's model.

The expected answers are `oracle/HostComponentsOracle.java` on OpenJDK 25.0.5+8 against
Minecraft 26.3's `server-game.jar` and its server libraries (brigadier, DataFixerUpper, ...), two
runs alike. No bootstrap is needed: the probes touch no registry. The game's jars are the player's
own and are never committed.

```sh
CP=server-game.jar:$(find libraries/server -name '*.jar' | tr '\n' :)
javac --release 21 -d out -cp $CP demo/HostComponents.java oracle/HostComponentsOracle.java
$JAVA25/bin/java -cp out:$CP HostComponentsOracle
```

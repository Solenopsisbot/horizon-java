# DataFixerUpper `Pair` and codecs as host objects (#156 P5c)

`demo/HostCodecs.java` makes pairs through the real API and reads an unmodelled codec constant.
`host_codec_tests.rs` compiles it against the compile-only stubs in `stubs/` and ships only
`demo/*`, so every call reaches the runtime's model.

The expected answers are `oracle/HostCodecsOracle.java` on OpenJDK 25.0.5+8 against the 26.3
server's `datafixerupper-10.0.21.jar`, `guava-33.6.0-jre.jar`, `failureaccess-1.0.3.jar` and
`fastutil`, two runs alike except an identity hash. The libraries are the player's own and are
never committed.

# NeoForge attachment types as host objects (#156 P8c)

`demo/HostAttachments.java` builds `AttachmentType`s and their builders through NeoForge's real API.
`host_attachment_tests.rs` compiles it against the attachments fixture's compile-only stubs
(`../attachments/stubs`, plus `stubs/` here) and ships only `demo/*`, so every call reaches the
runtime's model.

The expected answers are `oracle/HostAttachmentsOracle.java` on OpenJDK 25.0.5+8 against
NeoForge 26.3.0.101 (`neoforge-26.3.0.101-universal.jar`, bus-8.0.5, loader-12.0.12) and Minecraft
26.3's client jar and libraries, two runs alike. The same program re-runs two cases of
`../attachments/oracle.jsonl` (recorded on OpenJDK 27) on Java 25: the serializer's
`encode/carry-state` (`{"carryon:carry_on_data":{CarryOnData:{keyPressed:1b,selected:3,tick:7}}}`,
wrote, no problems) and `builder/checks` (the three `IllegalStateException` messages). Both agree.

```sh
javac --release 21 -d out -cp $CP demo/HostAttachments.java oracle/HostAttachmentsOracle.java
$JAVA25/bin/java -cp out:$CP HostAttachmentsOracle
```

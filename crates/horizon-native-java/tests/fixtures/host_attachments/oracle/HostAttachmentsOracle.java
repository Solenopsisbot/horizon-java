import demo.HostAttachments;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import net.minecraft.core.RegistryAccess;
import net.minecraft.nbt.CompoundTag;
import net.minecraft.util.ProblemReporter;
import net.minecraft.world.level.storage.TagValueOutput;
import net.neoforged.neoforge.attachment.AttachmentType;

/**
 * Runs every HostAttachments probe on NeoForge 26.3.0.101's own AttachmentType, then re-runs two
 * cases of tests/fixtures/attachments/oracle.jsonl (recorded on OpenJDK 27) on this JDK: the
 * serializer's encode/carry-state and the builder's checks.
 */
public final class HostAttachmentsOracle {
  public static void main(String[] args) throws Exception {
    String[] probes = {"spelled", "encoded", "spelledTypes", "real", "chain", "typeSwitch", "text",
      "builderSimpleName", "interfaces", "sorted"};
    for (String probe : probes) {
      Method m = HostAttachments.class.getMethod(probe);
      System.out.println(probe + "=" + m.invoke(null));
    }
    // encode/carry-state: NeoForge's serializer of serialize(CompoundTag.CODEC.fieldOf("CarryOnData")),
    // as AttachmentHolder#serializeAttachments calls it.
    AttachmentType<CompoundTag> type =
        AttachmentType.builder(CompoundTag::new).serialize(CompoundTag.CODEC.fieldOf("CarryOnData")).build();
    Field field = AttachmentType.class.getDeclaredField("serializer");
    field.setAccessible(true);
    @SuppressWarnings("unchecked")
    var serializer = (net.neoforged.neoforge.attachment.IAttachmentSerializer<CompoundTag>) field.get(type);
    CompoundTag value = new CompoundTag();
    value.putByte("keyPressed", (byte) 1);
    value.putInt("selected", 3);
    value.putInt("tick", 7);
    var collector = new ProblemReporter.Collector();
    TagValueOutput root = TagValueOutput.createWithContext(collector, RegistryAccess.EMPTY);
    boolean wrote = serializer.write(value, root.child("carryon:carry_on_data"));
    System.out.println("encode/carry-state=" + wrote + " " + root.buildResult() + " problems=" + !collector.isEmpty());
    // builder/checks
    System.out.println("builder/checks=" + check(() -> AttachmentType.builder(() -> 0).copyOnDeath()) + "|"
        + check(() -> AttachmentType.builder(CompoundTag::new).serialize(CompoundTag.CODEC.fieldOf("A"))
            .serialize(CompoundTag.CODEC.fieldOf("B")))
        + "|" + check(() -> AttachmentType.builder(() -> 0).copyHandler((a, h, p) -> a)));
  }

  static String check(Runnable r) {
    try {
      r.run();
      return "ok";
    } catch (RuntimeException x) {
      return x.getClass().getName() + ": " + x.getMessage();
    }
  }
}

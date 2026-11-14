package demo;

import java.util.*;
import net.fabricmc.fabric.api.event.lifecycle.v1.ServerTickEvents;
import net.fabricmc.fabric.api.event.player.PlayerBlockBreakEvents;
import net.minecraft.core.BlockPos;
import net.minecraft.world.level.gamerules.GameRule;
import net.minecraft.world.level.gamerules.GameRules;
import net.minecraft.world.level.storage.LevelResource;

/**
 * The server's values as a mod meets them (#156 P16c): every broken block's `BlockPos`, then, on
 * a server tick, the positions against each other, `LevelResource`, `GameRule` and text spelled
 * like their old markers. Each check is one `key=value` line, named as `Oracle.java` names the
 * same check on the real 26.3 classes.
 */
public final class Probe {
    public static String log = "";
    public static String counted = "";
    static final List<BlockPos> all = new ArrayList<>();
    static int ticks;

    static void p(String key, Object value) {
        log = log + key + "=" + value + "\n";
    }

    static BlockPos at(int x, int y, int z) {
        for (BlockPos pos : all) {
            if (pos.getX() == x && pos.getY() == y && pos.getZ() == z) return pos;
        }
        return null;
    }

    public void onInitialize() {
        PlayerBlockBreakEvents.AFTER.register((level, player, pos, state, entity) -> {
            all.add(pos);
            if (all.size() <= 5) {
                String k = pos.getX() + "," + pos.getY() + "," + pos.getZ();
                p("pos_text[" + k + "]", pos);
                p("pos_hash[" + k + "]", pos.hashCode());
                p("pos_long[" + k + "]", pos.asLong());
            }
            if (all.size() == 1) {
                p("block_text", state.getBlock());
            }
        });
        ServerTickEvents.END_SERVER_TICK.register(server -> {
            BlockPos a1 = at(1, 2, 3), a2 = all.get(all.size() - 1);
            LevelResource data = LevelResource.DATA;
            if (ticks++ > 0) {
                // What the model cannot know: two positions' identity, a jar's own
                // `LevelResource`.
                counted = (a1 == a2) + "," + data.equals(new LevelResource("data"));
                return;
            }
            p("pos_equals", a1.equals(a2));
            Map<BlockPos, String> map = new HashMap<>();
            map.put(a1, "a");
            p("pos_map_get", map.get(a2));
            p("pos_set", new HashSet<>(List.of(a1, a2, at(-1, -2, -3))).size());
            p("pos_compare", a1.compareTo(at(1, 2, 4)) + "," + at(5, 1, 0).compareTo(at(0, 2, 0)) + ","
                + at(2, 2, 2).compareTo(at(1, 2, 2)) + "," + a1.compareTo(a2));
            p("pos_sorted", new TreeSet<>(List.of(at(3, 1, 0), at(-1, -2, -3), at(0, 1, 5), a1)));
            p("res_text", data);
            p("res_hash", data.hashCode());
            p("res_root_text", LevelResource.ROOT);
            p("res_eq_new", data == new LevelResource("data"));
            p("res_same_constant", data == LevelResource.DATA);
            p("res_players_text", LevelResource.PLAYER_DATA_DIR + "," + LevelResource.PLAYER_DATA_DIR.hashCode());
            p("rule_text", GameRules.KEEP_INVENTORY);
            Object fp = "block-pos/" + "1,2,3", fr = "level-resource/" + "data", fg = "game-rule/" + "keep_inventory";
            p("forged_pos", a1.equals(fp) + "," + fp.equals(a1) + "," + new HashSet<Object>(List.of(a1, fp)).size() + ","
                + (fp instanceof BlockPos) + "," + (fp instanceof String) + "," + fp.getClass().getName());
            p("forged_res", data.equals(fr) + "," + fr.equals(data) + "," + new HashSet<Object>(List.of(data, fr)).size() + ","
                + (fr instanceof LevelResource));
            p("forged_rule", GameRules.KEEP_INVENTORY.equals(fg) + "," + fg.equals(GameRules.KEEP_INVENTORY) + ","
                + (fg instanceof GameRule));
        });
    }
}

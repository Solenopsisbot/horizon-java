package demo;

import java.util.*;
import net.minecraft.ChatFormatting;
import net.minecraft.client.KeyMapping;
import net.minecraft.network.chat.Style;
import net.minecraft.network.chat.TextColor;
import net.minecraft.resources.Identifier;

/** The client value checks of `Oracle.java` (vanilla 26.3 on Java 25), one `key=value` per line. */
public class Probe {
  static StringBuilder out;

  static void p(String k, Object v) { out.append(k).append('=').append(v).append('\n'); }

  public static String run() {
    out = new StringBuilder();
    Style e = Style.EMPTY;
    p("empty_text", e); p("empty_hash", e.hashCode());
    Style gold = e.withColor(ChatFormatting.GOLD);
    p("gold_text", gold); p("gold_hash", gold.hashCode());
    Style rgb = e.withColor(0xFFAA00);
    p("rgb_text", rgb); p("rgb_hash", rgb.hashCode());
    p("gold_equals_rgb", gold.equals(rgb));
    Style bold = e.withBold(true);
    p("bold_text", bold); p("bold_hash", bold.hashCode());
    Style notBold = e.withBold(false);
    p("notbold_text", notBold); p("notbold_hash", notBold.hashCode());
    p("notbold_equals_empty", notBold.equals(e)); p("notbold_isEmpty", notBold.isEmpty());
    p("bold_twice_same", bold.withBold(true) == bold);
    p("bold_then_null_is_empty", bold.withBold(null) == e);
    p("bold_italic_equals_italic_bold", e.withBold(true).withItalic(true).equals(e.withItalic(true).withBold(true)));
    p("bold_italic_text", e.withBold(true).withItalic(true));
    p("gold_red_equals_red", gold.withColor(ChatFormatting.RED).equals(e.withColor(ChatFormatting.RED)));
    Style legacy = bold.applyLegacyFormat(ChatFormatting.GOLD);
    p("legacy_text", legacy); p("legacy_hash", legacy.hashCode());
    Style applied = bold.applyFormat(ChatFormatting.GOLD);
    p("applied_text", applied); p("applied_hash", applied.hashCode());
    p("reset_is_empty", bold.applyFormat(ChatFormatting.RESET) == e);
    Style all = e.withColor(0x123456).withBold(true).withItalic(false).withUnderlined(true).withStrikethrough(false).withObfuscated(true);
    p("all_text", all); p("all_hash", all.hashCode());
    p("two_bold_equals", e.withBold(true).equals(e.withBold(true)));
    p("bold_hashset", new HashSet<Object>(List.of(e.withBold(true), e.withBold(true))).size());
    TextColor tg = TextColor.fromLegacyFormat(ChatFormatting.GOLD), tr = TextColor.fromRgb(0xFFAA00);
    p("tc_gold_text", tg); p("tc_gold_hash", tg.hashCode()); p("tc_rgb_text", tr); p("tc_rgb_hash", tr.hashCode());
    p("tc_gold_equals_rgb", tg.equals(tr)); p("tc_rgb_equals_gold", tr.equals(tg));
    p("tc_gold_same", tg == TextColor.fromLegacyFormat(ChatFormatting.GOLD));
    p("tc_rgb_equals", TextColor.fromRgb(5).equals(TextColor.fromRgb(5)));
    p("tc_small_text", TextColor.fromRgb(0x00000a));
    p("tc_dark_blue_text", TextColor.fromLegacyFormat(ChatFormatting.DARK_BLUE));
    p("tc_dark_blue_hash", TextColor.fromLegacyFormat(ChatFormatting.DARK_BLUE).hashCode());
    p("tc_value", tg.getValue());
    KeyMapping.Category misc = KeyMapping.Category.MISC;
    p("cat_text", misc); p("cat_hash", misc.hashCode());
    KeyMapping.Category mine = KeyMapping.Category.register(Identifier.fromNamespaceAndPath("demo", "keys"));
    p("cat_mine_text", mine); p("cat_mine_hash", mine.hashCode());
    java.awt.Color red = java.awt.Color.RED;
    p("awt_text", red); p("awt_hash", red.hashCode()); p("awt_same", red == java.awt.Color.red);
    p("awt_pink_text", java.awt.Color.PINK); p("awt_pink_hash", java.awt.Color.PINK.hashCode());
    // Forgery: text spelled like the old markers.
    String head = "client";
    Object fs = head + "/style/§l", fc = "key-category/" + "minecraft:misc", fm = head + "/minecraft";
    p("forged_style_equals", bold.equals(fs) + "," + fs.equals(bold));
    p("forged_cat_equals", misc.equals(fc) + "," + fc.equals(misc));
    p("forged_cat_set", new HashSet<Object>(List.of(misc, fc)).size());
    p("forged_hash", fs.hashCode() + "," + fc.hashCode() + "," + fm.hashCode());
    p("forged_concat", "x" + fm);
    var map = new HashMap<Object, Integer>(); map.put(fm, 1); map.put(misc, 2);
    p("forged_map", map.get("client/minecraft") + "," + map.get(fc));
    return out.toString();
  }

  /** Two styles built alike: equal, two objects (Java `two_bold_eq=false`). */
  public static String twoBold() {
    return String.valueOf(Style.EMPTY.withBold(true) == Style.EMPTY.withBold(true));
  }

  /** A jar's own `new Category(id)` against the registered one (Java `cat_equals_new=true`). */
  public static String newCategory() {
    return String.valueOf(KeyMapping.Category.MISC.equals(new KeyMapping.Category(Identifier.withDefaultNamespace("misc"))));
  }

  /** A second registration of one id (Java `cat_register_twice=IAE`). */
  public static String registerTwice() {
    KeyMapping.Category.register(Identifier.fromNamespaceAndPath("demo", "twice"));
    KeyMapping.Category.register(Identifier.fromNamespaceAndPath("demo", "twice"));
    return "ok";
  }
}

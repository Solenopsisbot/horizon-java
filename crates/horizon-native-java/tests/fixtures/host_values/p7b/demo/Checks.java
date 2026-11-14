package demo;

import com.mojang.brigadier.CommandDispatcher;
import com.mojang.brigadier.arguments.ArgumentType;
import com.mojang.brigadier.arguments.IntegerArgumentType;
import com.mojang.brigadier.arguments.StringArgumentType;
import com.mojang.brigadier.builder.ArgumentBuilder;
import com.mojang.brigadier.builder.LiteralArgumentBuilder;
import com.mojang.brigadier.builder.RequiredArgumentBuilder;
import com.mojang.brigadier.context.CommandContext;
import com.mojang.brigadier.tree.ArgumentCommandNode;
import com.mojang.brigadier.tree.CommandNode;
import com.mojang.brigadier.tree.LiteralCommandNode;
import com.mojang.brigadier.tree.RootCommandNode;
import java.util.*;
import java.util.function.Predicate;
import java.util.stream.Stream;
import net.minecraft.commands.CommandSourceStack;
import net.minecraft.commands.Commands;
import net.minecraft.commands.SharedSuggestionProvider;
import net.minecraft.server.permissions.Permission;
import net.minecraft.server.permissions.PermissionCheck;
import net.minecraft.server.permissions.PermissionSet;
import net.minecraft.server.permissions.Permissions;

/** The command surface's objects, as a registration callback and a running command see them. */
public final class Checks {
  static CommandDispatcher<CommandSourceStack> dispatcher;
  static String ran = "not run";
  static CommandContext<CommandSourceStack> keptContext;
  static CommandSourceStack keptSource;
  static CommandContext<CommandSourceStack> keptArg;
  static String forged = "not run";
  static String required = "not run";
  static Object requireSaw;
  static int requireCount;

  static LiteralArgumentBuilder<CommandSourceStack> literal(String name) {
    return LiteralArgumentBuilder.literal(name);
  }

  /** Every check, in the order the runs take them. */
  public static final String[] NAMES = {"ran", "nodes", "builders", "argumentTypes", "permissions", "spelled",
      "identities", "generic", "builderAsOther", "argumentTypeAsOther", "hashed", "required", "forged", "argClass",
      "forks", "keptSourceName", "used", "nodeTwins", "nodeHash", "argumentNodeText", "nodeClass", "argumentTypeState",
      "permissionTwice", "sourceClass"};
  static int step;

  /**
   * The registration callback. The first firing keeps the dispatcher and registers `/probe`; each
   * later one runs the next check, so every check sees what the first firing and `/probe` left.
   */
  public static void fired(CommandDispatcher<CommandSourceStack> d) {
    if (dispatcher == null) {
      registered(d);
      return;
    }
    String name = NAMES[step++];
    System.out.println(name + "=" + check(name));
  }

  static void registered(CommandDispatcher<CommandSourceStack> d) {
    dispatcher = d;
    d.register(literal("probe").executes(ctx -> inCommand(ctx)));
    // `/rvarg <n>` keeps its context; `/rvforge <n>` hands getters other contexts.
    d.register(literal("rvarg").then(RequiredArgumentBuilder.<CommandSourceStack, Integer>argument("n",
        IntegerArgumentType.integer(0, 100)).executes(ctx -> {
          keptArg = ctx;
          return 1;
        })));
    d.register(literal("rvforge").then(RequiredArgumentBuilder.<CommandSourceStack, Integer>argument("n",
        IntegerArgumentType.integer(0, 100)).executes(ctx -> {
          forged = forge(ctx);
          return 1;
        })));
    // `/rvreq`: Brigadier checks `requires` with the source it then runs the command with.
    d.register(literal("rvreq").requires(source -> {
      requireSaw = source;
      requireCount++;
      return true;
    }).executes(ctx -> {
      Object source = ctx.getSource();
      required = "" + (source == requireSaw) + source.equals(requireSaw) + requireCount;
      return 1;
    }));
  }

  /** A getter reads the context it is handed: a String is no context, a kept one is its own run's. */
  @SuppressWarnings("unchecked")
  static String forge(CommandContext<CommandSourceStack> ctx) {
    StringBuilder out = new StringBuilder();
    try {
      out.append(IntegerArgumentType.getInteger((CommandContext<CommandSourceStack>) (Object) "brigadier/context", "n"));
    } catch (ClassCastException e) {
      out.append("cce");
    }
    out.append(',');
    try {
      out.append(IntegerArgumentType.getInteger((CommandContext<CommandSourceStack>) (Object) "plain", "n"));
    } catch (ClassCastException e) {
      out.append("cce");
    }
    out.append(',').append(IntegerArgumentType.getInteger(keptArg, "n")).append(',');
    try {
      out.append(Commands.<CommandSourceStack>hasPermission(Commands.LEVEL_ALL)
          .test((CommandSourceStack) (Object) "brigadier/source/0"));
    } catch (ClassCastException e) {
      out.append("cce");
    }
    return out.append(',').append(IntegerArgumentType.getInteger(ctx, "n")).toString();
  }

  static String check(String name) {
    switch (name) {
      case "ran": return ran;
      case "nodes": return nodes();
      case "builders": return builders();
      case "argumentTypes": return argumentTypes();
      case "permissions": return permissions();
      case "spelled": return Forgery.spelled();
      case "used": return Forgery.used();
      case "identities": return identities();
      case "generic": return generic();
      case "hashed": return hashed();
      case "required": return required;
      case "forged": return forged;
      case "argClass": return argClass();
      case "forks": return forks();
      case "nodeTwins": return nodeTwins();
      case "nodeHash": return nodeHash();
      case "argumentNodeText": return argumentNodeText();
      case "nodeClass": return nodeClass();
      case "builderAsOther": return builderAsOther();
      case "argumentTypeState": return argumentTypeState();
      case "argumentTypeAsOther": return argumentTypeAsOther();
      case "permissionTwice": return permissionTwice();
      case "sourceClass": return sourceClass();
      default: return keptSourceName();
    }
  }

  /** `/probe`: the context and the source it runs with. */
  static int inCommand(CommandContext<CommandSourceStack> ctx) {
    Object a = ctx.getSource();
    Object b = ctx.getSource();
    StringBuilder out = new StringBuilder();
    IdentityHashMap<Object, Integer> seen = new IdentityHashMap<>();
    seen.put(a, 1);
    seen.put(b, 2);
    seen.put(ctx, 3);
    seen.put(ctx, 4);
    out.append(seen.size()).append(a == b).append(a.equals(b)).append(a.equals("brigadier/source/0")).append(a instanceof String)
        .append(a instanceof CommandSourceStack).append(a instanceof SharedSuggestionProvider)
        .append(ctx.equals(ctx)).append(ctx.equals(a)).append(ctx.equals(null))
        .append(((Object) ctx) instanceof CommandContext).append(ctx.getInput()).append(',');
    PermissionSet set = ctx.getSource().permissions();
    out.append(((Object) set) instanceof PermissionSet).append(set.hasPermission(Permissions.COMMANDS_GAMEMASTER))
        .append(set.hasPermission(Permissions.COMMANDS_ADMIN)).append(set.equals("brigadier/permission-set/2"));
    ran = out.toString();
    keptContext = ctx;
    keptSource = ctx.getSource();
    return 7;
  }

  /** Nodes: `toString`, `equals`, `compareTo` (also through a sort and a `TreeSet`), `instanceof`. */
  public static String nodes() {
    CommandDispatcher<CommandSourceStack> d = dispatcher;
    RootCommandNode<CommandSourceStack> root = d.getRoot();
    LiteralCommandNode<CommandSourceStack> ab = d.register(literal("a,b")
        .then(RequiredArgumentBuilder.<CommandSourceStack, Integer>argument("n", IntegerArgumentType.integer(1, 9))));
    // A literal spelled like a node handle's text form.
    LiteralCommandNode<CommandSourceStack> spelled = d.register(literal("hbrnode:i0,rroot,r"));
    CommandNode<CommandSourceStack> n = ab.getChild("n");
    StringBuilder out = new StringBuilder();
    out.append(root == d.getRoot()).append(root).append(ab).append(spelled).append(ab.getName())
        .append(ab == root.getChild("a,b")).append(ab.equals(root.getChild("a,b"))).append(ab.equals(spelled))
        .append(ab.equals(n)).append(n.equals(ab)).append(root.equals(ab)).append(ab.equals("<literal a,b>"))
        .append(ab.equals(null)).append(',');
    out.append(ab.compareTo(spelled)).append(spelled.compareTo(ab)).append(ab.compareTo(n)).append(n.compareTo(ab))
        .append(root.compareTo(n)).append(n.compareTo(root)).append(',');
    out.append(((Object) ab) instanceof LiteralCommandNode).append(((Object) n) instanceof ArgumentCommandNode)
        .append(((Object) root) instanceof RootCommandNode).append(((Object) n) instanceof CommandNode)
        .append(((Object) ab) instanceof Comparable).append(((Object) ab) instanceof String).append(',');
    List<CommandNode<CommandSourceStack>> sorted = new ArrayList<>(List.of(n, spelled, root, ab));
    Collections.sort(sorted);
    for (CommandNode<CommandSourceStack> node : sorted) out.append(node.getName()).append('|');
    TreeSet<CommandNode<CommandSourceStack>> set = new TreeSet<>(List.of(root, n, ab));
    return out.append(set.size()).append(set.first().getName()).toString();
  }

  /** Builders: identity, the builder a step answers, `instanceof`. */
  public static String builders() {
    LiteralArgumentBuilder<CommandSourceStack> x = literal("x");
    LiteralArgumentBuilder<CommandSourceStack> y = literal("x");
    RequiredArgumentBuilder<CommandSourceStack, String> r =
        RequiredArgumentBuilder.argument("w", StringArgumentType.word());
    StringBuilder out = new StringBuilder();
    out.append(x == y).append(x.equals(y)).append(x.equals(x)).append(x == x.then(r))
        .append(((Object) x) instanceof LiteralArgumentBuilder).append(((Object) r) instanceof RequiredArgumentBuilder)
        .append(((Object) r) instanceof ArgumentBuilder).append(((Object) x) instanceof String).append(x.equals("x"))
        .append(x.getLiteral()).append(r.getName());
    return out.toString();
  }

  /** `getArgument` on a kept context: the value as its class, brigadier's exception for another class. */
  public static String argClass() {
    StringBuilder out = new StringBuilder();
    out.append(keptArg.getArgument("n", Integer.class)).append(keptArg.getArgument("n", int.class)).append(',')
        .append(keptArg.getArgument("n", Number.class)).append(',');
    try {
      out.append(keptArg.getArgument("n", String.class));
    } catch (IllegalArgumentException e) {
      out.append("iae:").append(e.getMessage());
    }
    out.append(',');
    try {
      out.append(keptArg.getArgument("zz", Integer.class));
    } catch (IllegalArgumentException e) {
      out.append("iae:").append(e.getMessage());
    }
    return out.append(',').append(keptArg.getInput()).toString();
  }

  /** `isFork`: a `fork`, not a `redirect` with a modifier. */
  public static String forks() {
    LiteralCommandNode<CommandSourceStack> target = dispatcher.register(literal("ftarget"));
    boolean redirected = literal("rm").redirect(target, c -> c.getSource()).build().isFork();
    boolean forked = literal("fk").fork(target, c -> Collections.singleton(c.getSource())).build().isFork();
    boolean plain = literal("rd").redirect(target).build().isFork();
    return "" + redirected + forked + plain;
  }

  /** Argument types: their class and interface, `equals` against themselves and text. */
  public static String argumentTypes() {
    IntegerArgumentType i = IntegerArgumentType.integer(1, 9);
    Object o = i;
    StringBuilder out = new StringBuilder();
    out.append(o instanceof IntegerArgumentType).append(o instanceof ArgumentType).append(i.equals(i))
        .append(i.equals("integer(1, 9)")).append(i.equals(null)).append(o instanceof String)
        .append(StringArgumentType.word().equals(StringArgumentType.word()));
    return out.toString();
  }

  /** Permission values: their interfaces, `equals` against text and null. */
  public static String permissions() {
    Object check = Commands.LEVEL_GAMEMASTERS;
    Object permission = Permissions.COMMANDS_GAMEMASTER;
    Object predicate = Commands.hasPermission(Commands.LEVEL_GAMEMASTERS);
    StringBuilder out = new StringBuilder();
    out.append(check instanceof PermissionCheck).append(permission instanceof Permission)
        .append(predicate instanceof Predicate).append(check instanceof String)
        .append(permission.equals("brigadier/permission/2")).append(predicate.equals(null));
    return out.toString();
  }

  /** One object read twice, and two objects made apart, on every identity path. */
  public static String identities() {
    CommandDispatcher<CommandSourceStack> d = dispatcher;
    LiteralArgumentBuilder<CommandSourceStack> x = literal("i");
    LiteralArgumentBuilder<CommandSourceStack> y = literal("i");
    LiteralCommandNode<CommandSourceStack> node = d.register(literal("id"));
    IdentityHashMap<Object, Integer> seen = new IdentityHashMap<>();
    seen.put(x, 1);
    seen.put(y, 2);
    seen.put(x.then(literal("j")), 3);
    seen.put(d.getRoot(), 4);
    seen.put(d.getRoot(), 5);
    seen.put(node, 6);
    seen.put(d.getRoot().getChild("id"), 7);
    StringBuilder out = new StringBuilder();
    out.append(seen.size()).append(seen.get(x)).append(seen.get(node)).append(node == d.getRoot().getChild("id"))
        .append(List.of(x).contains(y)).append(List.of(x).contains(x)).append(List.of(node).contains(node))
        .append(Set.of(x, y).size()).append(Objects.equals(x, y)).append(Objects.equals(node, d.getRoot().getChild("id")));
    return out.toString();
  }

  /** The paths every value takes: type switches, `isInstance`, natural order, host collections. */
  public static String generic() {
    CommandDispatcher<CommandSourceStack> d = dispatcher;
    Object node = d.register(literal("gen"));
    Object root = d.getRoot();
    Object builder = literal("b");
    StringBuilder out = new StringBuilder();
    for (Object o : new Object[] {node, root, builder, "brigadier/node/0"}) {
      out.append(switch (o) {
        case LiteralCommandNode<?> l -> "literal:" + l.getName();
        case RootCommandNode<?> r -> "root";
        case ArgumentBuilder<?, ?> b -> "builder";
        case String text -> "string";
        default -> "other";
      }).append(';');
    }
    out.append(CommandNode.class.isInstance(node)).append(LiteralCommandNode.class.isInstance(root))
        .append(ArgumentBuilder.class.isInstance(builder)).append(CommandNode.class.isInstance("brigadier/node/0"))
        .append(',');
    CommandNode<CommandSourceStack> gen = d.getRoot().getChild("gen");
    CommandNode<CommandSourceStack> top = d.getRoot();
    List<CommandNode<CommandSourceStack>> nodes = new ArrayList<>(List.of(top, gen));
    nodes.sort(null);
    out.append(nodes.get(0).getName());
    out.append(Stream.of(top, gen).sorted().map(CommandNode::getName).toList());
    out.append(Stream.of(top, gen).sorted(Comparator.reverseOrder()).map(CommandNode::getName).toList());
    TreeMap<CommandNode<CommandSourceStack>, Integer> map = new TreeMap<>(Comparator.naturalOrder());
    map.put(top, 1);
    map.put(gen, 2);
    out.append(map.firstKey().getName()).append(',');
    out.append(List.of(gen, top)).append(Map.of(gen, 1).get(gen)).append(Map.of(gen, 1).containsKey(top))
        .append(List.of(gen).contains(gen));
    return out.toString();
  }

  /** The two builders are sibling classes. */
  public static String builderAsOther() {
    return "" + (((Object) literal("o")) instanceof RequiredArgumentBuilder);
  }

  /** Brigadier's argument types are sibling classes. */
  public static String argumentTypeAsOther() {
    return "" + (((Object) IntegerArgumentType.integer(1, 9)) instanceof StringArgumentType);
  }

  /** Two builders made apart, in a `HashSet` and as `HashMap` keys. */
  public static String hashed() {
    LiteralArgumentBuilder<CommandSourceStack> x = literal("s");
    LiteralArgumentBuilder<CommandSourceStack> y = literal("s");
    Set<Object> set = new HashSet<>(List.of(x, y));
    Map<Object, Integer> map = new HashMap<>();
    map.put(x, 1);
    map.put(y, 2);
    return "" + set.size() + map.size() + set.contains(x) + map.get(y);
  }
  // Counted: what needs state a handle does not hold, or an exact class.


  /** Two nodes of one name, children and command (`CommandNode.equals` compares them). */
  public static String nodeTwins() {
    return "" + literal("q").build().equals(literal("q").build());
  }

  /** `LiteralCommandNode.hashCode` hashes the children and the command. */
  public static String nodeHash() {
    return "" + literal("h").build().hashCode();
  }

  /** `ArgumentCommandNode.toString` prints its type. */
  public static String argumentNodeText() {
    return "" + RequiredArgumentBuilder.<CommandSourceStack, Integer>argument("n", IntegerArgumentType.integer(1, 9)).build();
  }

  public static String nodeClass() {
    return literal("c").build().getClass().getName();
  }

  /** `IntegerArgumentType`'s `equals`, `toString`, `hashCode` read its bounds. */
  public static String argumentTypeState() {
    IntegerArgumentType i = IntegerArgumentType.integer(1, 9);
    return "" + i.equals(IntegerArgumentType.integer(1, 9)) + i + i.hashCode();
  }

  /** One constant read twice is one object; `PermissionCheck.Require.equals` compares its permission. */
  public static String permissionTwice() {
    return "" + (Commands.LEVEL_GAMEMASTERS == Commands.LEVEL_GAMEMASTERS)
        + Commands.LEVEL_GAMEMASTERS.equals(Commands.LEVEL_GAMEMASTERS);
  }

  public static String sourceClass() {
    return keptSource.getClass().getName();
  }

  /** The source a finished run kept: its members read the run's state. */
  public static String keptSourceName() {
    return keptSource.getTextName();
  }
}

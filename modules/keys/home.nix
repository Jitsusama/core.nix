# Every shortcut Joel has, gathered from each layer that takes keys: the
# desktop first, then the terminal, then the multiplexer inside it, then the
# program inside that. A layer above sees a key before the ones below, so a
# chord two layers both claim never reaches the lower one; the machine
# refuses to build with one, naming the chord and both layers. The map is
# written to ~/.config/jitsusama/keys.json, with what each chord does in
# plain words, so the launcher can find an action by its name and show its
# chord beside it, and so a "what's here" view is drawn from the real
# configuration rather than a copy (docs/design.md, The Keyboard).
{ config, lib, ... }:
let
  cfg = config.jitsusama.keys;

  # Top to bottom: each layer sees a key before every layer after it.
  layers = {
    desktop = "the compositor and the shell around every window";
    terminal = "the terminal, inside a window";
    multiplexer = "a multiplexer, inside the terminal, in its resting mode";
    program = "the program running in the terminal";
  };
  order = [
    "desktop"
    "terminal"
    "multiplexer"
    "program"
  ];

  bind = lib.types.submodule {
    options = {
      chord = lib.mkOption {
        type = lib.types.str;
        description = "The chord, as its program spells it: Super+Return, ctrl+shift+t.";
      };
      does = lib.mkOption {
        type = lib.types.str;
        description = "What the chord does, in plain words, as the launcher finds it.";
      };
    };
  };

  # Each program spells chords its own way, so every chord is brought to one
  # spelling before two are compared: modifiers in a set order, then the key,
  # all lower case, joined with +.
  modifiers = {
    super = "super";
    mod = "super";
    mod4 = "super";
    win = "super";
    cmd = "super";
    ctrl = "ctrl";
    control = "ctrl";
    alt = "alt";
    mod1 = "alt";
    opt = "alt";
    option = "alt";
    shift = "shift";
  };
  modifierOrder = [
    "super"
    "ctrl"
    "alt"
    "shift"
  ];
  keyNames = {
    return = "enter";
    esc = "escape";
    page_up = "pageup";
    prior = "pageup";
    page_down = "pagedown";
    next = "pagedown";
    equal = "=";
    plus = "+";
    minus = "-";
    slash = "/";
    bracketleft = "[";
    bracketright = "]";
  };
  normalise =
    chord:
    let
      # zellij separates with spaces, where a key may be +; the rest with +.
      parts = lib.filter (part: lib.isString part && part != "") (
        builtins.split (if lib.hasInfix " " chord then " " else "\\+") chord
      );
      words = map lib.toLower parts;
      held = lib.filter (word: modifiers ? ${word}) words;
      keys = lib.filter (word: !(modifiers ? ${word})) words;
      key = if keys == [ ] then "+" else lib.last keys;
      named = map (word: modifiers.${word}) held;
    in
    lib.concatStringsSep "+" (
      lib.filter (modifier: lib.elem modifier named) modifierOrder ++ [ keyNames.${key} or key ]
    );

  spelled = layer: map (b: b // { keys = normalise b.chord; }) cfg.${layer};

  # Every chord a layer claims that a layer above it also claims.
  clashes = lib.concatLists (
    lib.imap0 (
      index: upper:
      lib.concatMap (
        lower:
        lib.concatMap (
          below:
          map (above: {
            inherit
              upper
              lower
              above
              below
              ;
          }) (lib.filter (above: above.keys == below.keys) (spelled upper))
        ) (spelled lower)
      ) (lib.drop (index + 1) order)
    ) order
  );

  # Every chord a layer claims twice.
  repeats = lib.concatMap (
    layer:
    let
      groups = lib.groupBy (b: b.keys) (spelled layer);
    in
    lib.mapAttrsToList (keys: binds: { inherit layer keys binds; }) (
      lib.filterAttrs (_: binds: lib.length binds > 1) groups
    )
  ) order;
in
{
  options.jitsusama.keys = lib.mapAttrs (
    _layer: description:
    lib.mkOption {
      type = lib.types.listOf bind;
      default = [ ];
      description = "The shortcuts of ${description}. Each module that takes keys adds its own.";
    }
  ) layers;

  config = {
    xdg.configFile."jitsusama/keys.json".text = builtins.toJSON (
      map (layer: {
        name = layer;
        description = layers.${layer};
        binds = map (b: { inherit (b) chord keys does; }) (spelled layer);
      }) order
    );

    assertions = [
      {
        assertion = clashes == [ ];
        message = ''
          Two layers claim the same chord, so the lower one never sees it:
          ${lib.concatMapStringsSep "\n" (
            clash:
            "  ${clash.above.keys}: ${clash.upper} (${clash.above.does}) takes it from ${clash.lower} (${clash.below.does})"
          ) clashes}
        '';
      }
      {
        assertion = repeats == [ ];
        message = ''
          A layer claims the same chord twice:
          ${lib.concatMapStringsSep "\n" (
            repeat:
            "  ${repeat.keys} in ${repeat.layer}: ${lib.concatMapStringsSep ", " (b: b.does) repeat.binds}"
          ) repeats}
        '';
      }
    ];
  };
}

# Every shortcut Joel has, gathered from each layer that takes keys: the
# desktop first, then the terminal, then the programs inside it, Neovim and
# pi, side by side, then the programs in windows of their own, such as the
# picture viewer, the markdown reader, the document viewer, the player and the browser, beside the terminal. A layer above sees a key before the ones below, so a
# chord two layers both claim never reaches the lower one; the machine
# refuses to build with one, naming the chord and both layers. The programs
# never hear a key at the same time, so a chord both claim is no clash. A
# chord can be a sequence, keys pressed one after another (Space b w), and
# a sequence whose first key a layer above takes can never start. The map is
# written to ~/.config/jitsusama/keys.json, with what each chord does in
# plain words, so the launcher can find an action by its name and show its
# chord beside it, and so a "what's here" view is drawn from the real
# configuration rather than a copy (docs/design.md, The Keyboard). Beside
# them are the touchscreen's swipes, which aren't chords but are shortcuts
# to the same actions, so the launcher lists them with the desktop's.
{ config, lib, ... }:
let
  cfg = config.jitsusama.keys;

  # Top to bottom: each layer sees a key before every layer after it, but
  # for the programs, which sit side by side.
  layers = {
    desktop = "the compositor and the shell around every window";
    terminal = "the terminal, inside a window";
    neovim = "Neovim, the editor in the terminal";
    pi = "pi, the coding agent in the terminal";
    swayimg = "swayimg, the picture viewer, in a window of its own";
    inlyne = "inlyne, the markdown reader, in a window of its own";
    zathura = "zathura, the document viewer, in a window of its own";
    mpv = "mpv, the player, in a window of its own";
    chrome = "Chrome, the browser, in a window of its own, and each window it opens for one site";
  };
  order = [
    "desktop"
    "terminal"
    "neovim"
    "pi"
    "swayimg"
    "inlyne"
    "zathura"
    "mpv"
    "chrome"
  ];
  # The layers inside another's window, which hears a key first.
  inside = {
    neovim = "terminal";
    pi = "terminal";
  };
  # Two layers below the desktop sit side by side, and so never hear a key
  # at the same time, unless one is inside the other's window: Neovim and pi
  # in one terminal, the terminal, the picture viewer, the markdown reader,
  # the document viewer, the player and the browser in windows of their own.
  beside =
    upper: lower:
    upper != "desktop"
    && lower != "desktop"
    && inside.${lower} or "" != upper
    && inside.${upper} or "" != lower;

  bind = lib.types.submodule (
    { config, ... }:
    {
      options = {
        chord = lib.mkOption {
          type = lib.types.str;
          description = ''
            The chord, as its program spells it: Super+Return, ctrl+shift+t.
            Keys pressed one after another are written with a space between,
            as Space b w.
          '';
        };
        when = lib.mkOption {
          type = lib.types.str;
          default = "";
          description = ''
            Where in its program the chord works, when not everywhere, in
            plain words: "in code", "with text selected". The same chord may
            mean something else elsewhere.
          '';
        };
        keys = lib.mkOption {
          type = lib.types.str;
          readOnly = true;
          default = normalise config.chord;
          defaultText = lib.literalMD "the chord in one spelling";
          description = ''
            The chord in the one spelling every layer's is brought to, for
            comparing two and drawing them alike: modifiers in a set order,
            then the key, all lower case, joined with +, as super+shift+/,
            and a sequence's keys with a space between, as space b w.
          '';
        };
        does = lib.mkOption {
          type = lib.types.str;
          description = "What the chord does, in plain words, as the launcher finds it.";
        };
        also = lib.mkOption {
          type = lib.types.str;
          default = "";
          example = "kitty shell console";
          description = ''
            Other words a person might type for it, by which the launcher
            finds it too, as a terminal is found by "kitty" or "shell".
          '';
        };
        group = lib.mkOption {
          type = lib.types.str;
          default = "";
          example = "workspaces";
          description = ''
            The group the launcher's Shortcuts lists the chord in, for the
            rare chord its table of groups (launcher/KeyGroups.js) puts in
            the wrong one, or a group and the one inside it, as
            github/pulls. Empty, the table works it out.
          '';
        };
        run = lib.mkOption {
          type = lib.types.listOf lib.types.str;
          default = [ ];
          description = ''
            A command that does the same, for the launcher to have niri start
            when the action is picked by name. Empty for a chord only its own
            program can act on, such as one inside the terminal.
          '';
        };
      };
    }
  );

  swipe = lib.types.submodule {
    options = {
      fingers = lib.mkOption {
        type = lib.types.ints.between 1 5;
        description = "How many fingers swipe.";
      };
      way = lib.mkOption {
        type = lib.types.enum [
          "up"
          "down"
          "left"
          "right"
        ];
        description = "Which way the fingers move.";
      };
      does = lib.mkOption {
        type = lib.types.str;
        description = "What the swipe does, in plain words, as the launcher finds it.";
      };
      action = lib.mkOption {
        type = lib.types.str;
        description = "niri's action, as `niri msg action` takes it: focus-workspace-down.";
      };
    };
  };

  # Every swipe claimed twice.
  swipeRepeats = lib.filter (binds: lib.length binds > 1) (
    lib.attrValues (lib.groupBy (s: "${toString s.fingers} ${s.way}") config.jitsusama.swipes)
  );

  # Each program spells chords its own way, so every chord is brought to one
  # spelling before two are compared: modifiers in a set order, then the key,
  # all lower case, joined with +. A capital letter alone, as Neovim's are,
  # is its key with Shift; after a modifier it's only how the key is named.
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
  normaliseOne =
    chord:
    let
      # The last part is the key, which may itself be +.
      parts = lib.filter (part: lib.isString part && part != "") (builtins.split "\\+" chord);
      last = if parts == [ ] then "+" else lib.last parts;
      capital = lib.length parts == 1 && lib.stringLength last == 1 && lib.toLower last != last;
      words = map lib.toLower parts;
      held = lib.filter (word: modifiers ? ${word}) words ++ lib.optional capital "shift";
      keys = lib.filter (word: !(modifiers ? ${word})) words;
      key = if keys == [ ] then "+" else lib.last keys;
      named = map (word: modifiers.${word}) held;
    in
    lib.concatStringsSep "+" (
      lib.filter (modifier: lib.elem modifier named) modifierOrder ++ [ keyNames.${key} or key ]
    );
  normalise = chord: lib.concatMapStringsSep " " normaliseOne (lib.splitString " " chord);

  # The key a chord starts with, which is the whole of one that's no
  # sequence.
  first = keys: lib.head (lib.splitString " " keys);

  spelled = layer: cfg.${layer};

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
          }) (lib.filter (above: above.keys == first below.keys) (spelled upper))
        ) (spelled lower)
      ) (lib.filter (lower: !(beside upper lower)) (lib.drop (index + 1) order))
    ) order
  );

  # Every chord a layer claims twice in the same place.
  repeats = lib.concatMap (
    layer:
    let
      groups = lib.groupBy (b: "${b.keys} ${b.when}") (spelled layer);
    in
    lib.mapAttrsToList (_: binds: {
      inherit layer binds;
      inherit ((lib.head binds)) keys;
    }) (lib.filterAttrs (_: binds: lib.length binds > 1) groups)
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

  options.jitsusama.swipes = lib.mkOption {
    type = lib.types.listOf swipe;
    default = [ ];
    description = "The touchscreen's swipes. niri's module names the ones niri does.";
  };

  config = {
    xdg.configFile."jitsusama/keys.json".text = builtins.toJSON (
      map (layer: {
        name = layer;
        description = layers.${layer};
        binds = map (b: {
          inherit (b)
            chord
            keys
            does
            when
            run
            ;
        }) (spelled layer);
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
      {
        assertion = swipeRepeats == [ ];
        message = ''
          The same swipe is claimed twice:
          ${lib.concatMapStringsSep "\n" (
            binds:
            let
              s = lib.head binds;
              does = lib.concatMapStringsSep ", " (b: b.does) binds;
            in
            "  ${toString s.fingers} fingers ${s.way}: ${does}"
          ) swipeRepeats}
        '';
      }
    ];
  };
}

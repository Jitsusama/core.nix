# Joel's niri: config.kdl says how it behaves, theme.kdl, written here from
# jitsusama.theme, how it looks, binds.kdl, written here from
# jitsusama.niri.binds, what its keys do, and keyboard.kdl, written here from
# modules/keyboard/repeat.nix, how a held key repeats, as a Mac's does. The binds also go into the keyboard
# map (modules/keys), which refuses a chord another layer claims.
#
# config.kdl is text a machine can add to, with lib.mkAfter. A screen's
# settings are the exception: niri reads only the first `output` block that
# names a screen and ignores the rest without a word, so each screen's lines
# go in jitsusama.niri.outputs, where core's and a machine's merge into one
# block, and a config.kdl naming a screen twice is refused.
#
# niri draws four screens of its own in colours it doesn't let a theme
# change, so none of them is ever shown: the key list is the launcher's
# (Super+Shift+/), a region is picked by the keys in Quickshell's
# Picker.qml rather than niri's screenshot screen, and Log Out quits without
# niri's question. The banner for a configuration niri can't read would need
# one the build refuses. checks.nix holds the keys to this.
#
# The cursor is Adwaita's, as Omarchy's is through its system default. niri
# draws it and tells every program it starts which theme and size to use, and
# its default names a theme called "default" that nothing installs, so without
# this only niri's built-in arrow shows and every other shape (resize, grab,
# text, not-allowed) falls back to it.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  inherit (config.jitsusama.theme)
    colours
    shape
    motion
    wallpaper
    ;

  # With a picture behind the desktop, Quickshell draws it in the backdrop
  # and the workspaces are clear over it, so it stands still while they
  # move; their shadows in the overview would only smudge it.
  pictured = wallpaper.desktop != null;

  bind = lib.types.submodule {
    options = {
      chord = lib.mkOption {
        type = lib.types.str;
        description = "The chord, as niri spells it: Super+Shift+Q.";
      };
      does = lib.mkOption {
        type = lib.types.str;
        description = "What it does in plain words, shown in niri's list and the launcher.";
      };
      also = lib.mkOption {
        type = lib.types.str;
        default = "";
        description = "Other words a person might type for it, by which the launcher finds it too.";
      };
      action = lib.mkOption {
        type = lib.types.str;
        description = ''niri's action, as config.kdl would spell it: spawn "kitty";'';
      };
      group = lib.mkOption {
        type = lib.types.str;
        default = "";
        example = "windows";
        description = ''
          The group the launcher's Shortcuts lists it in, for the rare bind
          whose action its table (Quickshell's launcher/KeyGroups.js) puts
          in the wrong one. Empty, the group is worked out from the action.
        '';
      };
      flags = lib.mkOption {
        type = lib.types.attrsOf lib.types.str;
        default = { };
        description = "niri's settings on the bind, such as repeat or allow-when-locked.";
      };
    };
  };
  binds = config.jitsusama.niri.binds;

  # Anything a gesture can carry rides a spring, which keeps a swipe's speed;
  # the rest runs on the curve.
  spring = ''
    spring damping-ratio=${motion.spring.damping} stiffness=${toString motion.spring.stiffness} epsilon=${motion.spring.epsilon}
  '';
  timed = milliseconds: ''
    duration-ms ${toString milliseconds}
    curve "cubic-bezier" ${lib.concatStringsSep " " motion.curve}
  '';
  animations = {
    workspace-switch = spring;
    horizontal-view-movement = spring;
    window-movement = spring;
    window-resize = spring;
    overview-open-close = spring;
    window-open = timed motion.medium;
    window-close = timed motion.short;
    screenshot-ui-open = timed motion.medium;
    config-notification-open-close = timed motion.medium;
    exit-confirmation-open-close = timed motion.medium;
  };
  cursor = {
    name = "Adwaita";
    package = pkgs.adwaita-icon-theme;
    size = 24;
  };
in
{
  imports = [
    ../theme/home.nix
    ../keys/home.nix
  ];

  options.jitsusama.niri.binds = lib.mkOption {
    type = lib.types.listOf bind;
    default = [ ];
    description = ''
      niri's shortcuts, binds.nix's to begin with. A machine's own are added
      to them; lib.mkForce replaces them all.
    '';
  };

  config.jitsusama.niri.binds = import ./binds.nix { inherit lib; };

  # The touchscreen's swipes, which niri does itself (nixos.nix's patch), as
  # the touchpad's: three fingers move the columns and the workspaces the way
  # the content moves under them, four up open the overview and four down
  # close it. Each with the action that does what its swipe ends in, for the
  # launcher's shortcuts to list and run.
  config.jitsusama.swipes = [
    {
      fingers = 3;
      way = "left";
      does = "Focus the column to the right";
      action = "focus-column-right";
    }
    {
      fingers = 3;
      way = "right";
      does = "Focus the column to the left";
      action = "focus-column-left";
    }
    {
      fingers = 3;
      way = "up";
      does = "Go to the workspace below";
      action = "focus-workspace-down";
    }
    {
      fingers = 3;
      way = "down";
      does = "Go to the workspace above";
      action = "focus-workspace-up";
    }
    {
      fingers = 4;
      way = "up";
      does = "Show every workspace";
      action = "open-overview";
    }
    {
      fingers = 4;
      way = "down";
      does = "Stop showing every workspace";
      action = "close-overview";
    }
  ];

  # Also links it as ~/.icons/default and names it to GTK, for programs that
  # look there rather than at niri's variables. Home Manager offers this on
  # Linux only, and niri runs nowhere else.
  config.home.pointerCursor = lib.mkIf pkgs.stdenv.hostPlatform.isLinux (
    cursor
    // {
      enable = true;
      gtk.enable = true;
    }
  );

  # Each bind's action as a command the launcher has niri start, so an action
  # found by name does what its chord does: a spawn runs its program, and
  # anything else asks niri through its own command line, which fills in what
  # the config leaves to defaults. Nothing runs until the action is picked.
  config.jitsusama.keys.desktop = map (
    b:
    let
      words = map (found: if lib.head found != null then lib.head found else lib.elemAt found 1) (
        lib.filter lib.isList (builtins.split ''"([^"]*)"|([^ ";]+)'' b.action)
      );
      name = lib.head words;
      arguments = lib.tail words;
    in
    {
      inherit (b) chord does;
      inherit (b) also group;
      run =
        if name == "spawn" then
          arguments
        else if name == "spawn-sh" then
          [
            "sh"
            "-c"
          ]
          ++ arguments
        else
          [
            "niri"
            "msg"
            "action"
            name
            "--"
          ]
          ++ arguments;
    }
  ) binds;

  options.jitsusama.niri.outputs = lib.mkOption {
    type = lib.types.attrsOf (lib.types.listOf lib.types.str);
    default = { };
    example = {
      "eDP-1" = [
        "scale 2"
        "position x=560 y=1440"
      ];
    };
    description = ''
      Each screen's settings, by the name niri knows it by (its connector,
      or its make, model and serial), as config.kdl's lines inside its
      `output` block. Core's and a machine's lines for one screen merge
      into one block, since niri reads only the first.
    '';
  };

  # The laptop's own panel shows colours as sRGB, as the monitor beside it
  # does and a Mac does, rather than stretched to its wider primaries
  # (nixos.nix's srgb-gamut.patch). A panel that is sRGB already is left as
  # it is.
  config.jitsusama.niri.outputs."eDP-1" = [ "srgb-gamut" ];

  config.xdg.configFile."niri/config.kdl".text = lib.mkMerge [
    (builtins.readFile ./config.kdl)
    (lib.concatStrings (
      lib.mapAttrsToList (name: lines: ''

        output "${name}" {
        ${lib.concatMapStrings (line: "    ${line}\n") lines}}
      '') config.jitsusama.niri.outputs
    ))
  ];

  # A screen named twice is set by its first block alone, so a block a
  # machine adds as text for a screen core already sets would be ignored.
  config.assertions =
    let
      named = lib.concatMap (
        line:
        let
          found = builtins.match " *output \"([^\"]+)\" *\\{.*" line;
        in
        lib.optional (found != null) (lib.head found)
      ) (lib.splitString "\n" config.xdg.configFile."niri/config.kdl".text);
      twice = lib.unique (lib.filter (name: lib.count (other: other == name) named > 1) named);
    in
    [
      {
        assertion = twice == [ ];
        message = "niri's config.kdl names ${lib.concatStringsSep ", " twice} in more than one output block, and niri reads only the first; set its lines in jitsusama.niri.outputs instead.";
      }
    ];

  config.xdg.configFile."niri/keyboard.kdl".text =
    let
      repeat = import ../keyboard/repeat.nix;
    in
    ''
      input {
          keyboard {
              repeat-delay ${toString repeat.delay}
              repeat-rate ${toString repeat.rate}
          }
      }
    '';

  config.xdg.configFile."niri/binds.kdl".text = ''
    binds {
    ${
      lib.concatMapStrings (
        b:
        let
          flags = lib.concatStrings (lib.mapAttrsToList (name: value: " ${name}=${value}") b.flags);
        in
        "    ${b.chord} hotkey-overlay-title=${builtins.toJSON b.does}${flags} { ${b.action} }\n"
      ) binds
    }}
  '';

  config.xdg.configFile."niri/theme.kdl".text = ''
    layout {
        gaps ${toString shape.gap}
        background-color "${if pictured then "transparent" else colours.deepest}"
        border {
            on
            width ${toString shape.border}
            active-color "${colours.accent}"
            inactive-color "${colours.edge}"
            urgent-color "${colours.alert}"
        }
        tab-indicator {
            active-color "${colours.accent}"
            inactive-color "${colours.raised}"
            urgent-color "${colours.alert}"
        }
        insert-hint {
            color "${colours.accent}80"
        }
    }
    window-rule {
        geometry-corner-radius ${toString shape.radius}
        clip-to-geometry true
    }
    overview {
        backdrop-color "${colours.deepest}"
        workspace-shadow {
            ${if pictured then "off" else ''color "${colours.shadow}50"''}
        }
    }
    // niri's switcher, used while a modifier is held, is off: the launcher
    // finds a window by typing (Super+Tab), and off also keeps niri's own
    // keys for it, Alt+Tab among them, off the desktop.
    recent-windows {
        off
    }
    cursor {
        xcursor-theme "${cursor.name}"
        xcursor-size ${toString cursor.size}
    }
    animations {
        slowdown ${motion.slowdown}
    ${
      lib.concatStrings (
        lib.mapAttrsToList (name: body: ''
          ${name} {
          ${body}}
        '') animations
      )
    }}
  '';
}

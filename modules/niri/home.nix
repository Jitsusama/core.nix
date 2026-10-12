# Joel's niri: config.kdl says how it behaves, theme.kdl, written here from
# jitsusama.theme, how it looks, binds.kdl, written here from
# jitsusama.niri.binds, what its keys do, and keyboard.kdl, written here from
# modules/keyboard/repeat.nix, how a held key repeats, as a Mac's does. The binds also go into the keyboard
# map (modules/keys), which refuses a chord another layer claims.
#
# config.kdl is text a machine can add to, with lib.mkAfter, such as a
# panel's scale; niri lets a later setting override an earlier one, so the
# machine's lines win.
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
    roles
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
      action = lib.mkOption {
        type = lib.types.str;
        description = ''niri's action, as config.kdl would spell it: spawn "kitty";'';
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
    screenshot-ui-open = timed motion.short;
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

  config.xdg.configFile."niri/config.kdl".text = builtins.readFile ./config.kdl;

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
        background-color "${if pictured then "transparent" else roles.deepest}"
        border {
            on
            width ${toString shape.border}
            active-color "${roles.accent}"
            inactive-color "${roles.edge}"
            urgent-color "${roles.alert}"
        }
        tab-indicator {
            active-color "${roles.accent}"
            inactive-color "${roles.raised}"
            urgent-color "${roles.alert}"
        }
        insert-hint {
            color "${roles.accent}80"
        }
    }
    window-rule {
        geometry-corner-radius ${toString shape.radius}
        clip-to-geometry true
    }
    overview {
        backdrop-color "${roles.deepest}"
        ${lib.optionalString pictured "workspace-shadow { off; }"}
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

# Quickshell draws everything on Joel's screen that isn't a window: the
# wallpaper, the launcher on Super+Space, notifications, the lock screen,
# polkit's password prompt, the volume and brightness, and the time and status
# over the overview. Each is a QML file beside this one, linked as it is;
# Theme.qml, written here from jitsusama.theme, gives them all the same look.
# It runs with the graphical session, and restarts if it ever falls over. What
# the overview shows is jitsusama.quickshell.overview.pieces, rendered into
# Overview.qml at build.
{ wallpapers }:
{
  config,
  lib,
  pkgs,
  ...
}:
let
  inherit (config.jitsusama.theme)
    roles
    font
    shape
    space
    type
    motion
    glyphs
    floating
    wallpaper
    ;

  # A picture the theme names, as a path in the wallpapers collection or an
  # absolute one, or nothing for none.
  picture =
    name:
    if name == null then
      ""
    else if lib.hasPrefix "/" name then
      name
    else
      "${wallpapers}/${name}";

  # A JavaScript object of the values, which QML reads as Theme.space.m.
  object = values: "(${builtins.toJSON values})";
  slowed = milliseconds: builtins.floor (milliseconds * builtins.fromJSON motion.slowdown + 0.5);
  face = family: sizes: object ({ inherit family; } // lib.mapAttrs (_: builtins.fromJSON) sizes);

  qml = [
    "shell.qml"
    "Prompt.qml"
    "Launcher.qml"
    "Notifications.qml"
    "Lock.qml"
    "Polkit.qml"
    "Volume.qml"
    "Brightness.qml"
    "Meter.qml"
    "Status.qml"
    "Wallpaper.qml"
    "Lifted.qml"
    "OpenBelow.qml"
    "Niri.qml"
    "Floating.qml"
    "Fuzzy.js"
    "Picks.qml"
    "pieces/Piece.qml"
  ];

  # A part's corners, the whole's radius less its border and inset.
  inner = lib.max 0 (shape.radius - shape.border - shape.inset);

  # A mark from the icon font by its code point.
  nerd = import ../theme/nerd.nix { inherit lib; };

  # How a chord's keys are drawn: a key's own glyph where a proper one
  # exists, its name where none does, and a letter as itself (docs/design.md,
  # Shape and Kinds). Icon marks draw wider than a letter, so they're
  # marked to be given room.
  keyFaces = {
    Super = {
      text = nerd "f313";
      wide = true;
    };
    Shift.text = "⇧";
    Return.text = "↵";
    Up.text = "↑";
    Down.text = "↓";
    Left.text = "←";
    Right.text = "→";
    Escape.text = "Esc";
    Page_Up.text = "PgUp";
    Page_Down.text = "PgDn";
    Minus.text = "-";
    Equal.text = "=";
    Slash.text = "/";
    Period.text = ".";
    Comma.text = ",";
    Grave.text = "`";
    BracketLeft.text = "[";
    BracketRight.text = "]";
    WheelScrollUp.text = "Wheel ↑";
    WheelScrollDown.text = "Wheel ↓";
    WheelScrollLeft.text = "Wheel ←";
    WheelScrollRight.text = "Wheel →";
    XF86AudioRaiseVolume = {
      text = nerd "f075d";
      wide = true;
    };
    XF86AudioLowerVolume = {
      text = nerd "f075e";
      wide = true;
    };
    XF86AudioMute = {
      text = nerd "f075f";
      wide = true;
    };
    XF86AudioMicMute = {
      text = nerd "f036d";
      wide = true;
    };
    XF86MonBrightnessUp = {
      text = nerd "f00e0";
      wide = true;
    };
    XF86MonBrightnessDown = {
      text = nerd "f00de";
      wide = true;
    };
    XF86AudioPrev = {
      text = nerd "f04ae";
      wide = true;
    };
    XF86AudioNext = {
      text = nerd "f04ad";
      wide = true;
    };
    XF86AudioPlay = {
      text = nerd "f040e";
      wide = true;
    };
  };
  face' = key: { wide = false; } // (keyFaces.${key} or { text = key; });

  # The glyph an action wears in the launcher, by what it runs.
  actionGlyph =
    run:
    let
      program = if run == [ ] then "" else lib.head run;
      action = if program == "niri" then lib.elemAt run 3 else "";
      says = lib.concatStringsSep " " run;
    in
    if lib.hasInfix "kitty" says then
      nerd "f489"
    else if lib.hasPrefix "screenshot" action then
      nerd "f0e51"
    else if lib.hasInfix "AUDIO_SOURCE" says then
      nerd "f036d"
    else if program == "sh" && lib.hasInfix "wpctl" says then
      nerd "f057e"
    else if program == "brightnessctl" then
      nerd "f00e0"
    else if program == "playerctl" then
      nerd "f040e"
    else if lib.hasInfix "workspace" action || lib.hasInfix "overview" action then
      nerd "f0570"
    else if lib.hasInfix "monitor" action then
      nerd "f0379"
    else
      nerd "f2d0";

  # The desktop's binds the launcher can run, its own toggle left out, since
  # picking that from the launcher would only close it.
  actions = map (b: {
    inherit (b) does run;
    glyph = actionGlyph b.run;
    chord = map face' (lib.splitString "+" b.chord);
  }) (lib.filter (b: b.run != [ ] && !(lib.elem "launcher" b.run)) config.jitsusama.keys.desktop);

  pieces = import ../theme/pieces.nix { inherit lib; };

  # The pieces Quickshell has built in, by name, each a file in pieces/.
  builtIn = {
    clock = ./pieces/Clock.qml;
    battery = ./pieces/Battery.qml;
    network = ./pieces/Network.qml;
    volume = ./pieces/Volume.qml;
  };

  overview = pieces.drawn config.jitsusama.quickshell.overview.pieces;

  # A built-in piece is linked by its file's name, and a machine's own by
  # the piece's name, so two of them never share a file.
  isBuiltIn = piece: builtins.isString piece.source && builtIn ? ${piece.source};
  fileName =
    piece: if isBuiltIn piece then baseNameOf builtIn.${piece.source} else "${piece.name}.qml";
  linked = piece: if isBuiltIn piece then builtIn.${piece.source} else piece.source;
in
{
  imports = [
    ../theme/home.nix
    ../keys/home.nix
  ];

  options.jitsusama.quickshell.overview.pieces = lib.mkOption {
    type = lib.types.attrsOf (
      pieces.piece {
        inherit (config.jitsusama.theme) glyphs;
        sources = builtIn;
      }
    );
    description = ''
      What the overview shows, by name. Each piece has its order, tier,
      collapse rank, mark and source (docs/design.md, Customising), and its
      own settings beside them: a `label` makes it a row, a muted label
      beside its value, and without one it's a heading above the rows, at
      the type size `size` names; a clock takes a `format`, as Qt spells
      one, and the battery `low` and `critical` percentages. A machine
      changes one setting in one line, or adds a piece whose source is a
      QML file starting from pieces/Piece.qml; lib.mkForce replaces them
      all.
    '';
  };

  config = {
    # Each setting is a default, so a machine's own wins without a fight.
    jitsusama.quickshell.overview.pieces = lib.mapAttrs (_: lib.mapAttrs (_: lib.mkDefault)) {
      time = {
        order = 10;
        source = "clock";
        format = "HH:mm";
        size = "display";
      };
      date = {
        order = 20;
        source = "clock";
        format = "dddd d MMMM";
        collapse = 30;
      };
      battery = {
        order = 30;
        source = "battery";
        label = "battery";
        low = 20;
        critical = 10;
        collapse = 20;
      };
      network = {
        order = 40;
        source = "network";
        label = "network";
        collapse = 10;
      };
      volume = {
        order = 50;
        source = "volume";
        label = "volume";
        collapse = 0;
      };
    };

    home.packages = [ pkgs.quickshell ];

    xdg.configFile =
      lib.genAttrs (map (file: "quickshell/${file}") qml) (path: {
        source = ./. + "/${lib.removePrefix "quickshell/" path}";
      })
      // lib.listToAttrs (
        map (
          piece: lib.nameValuePair "quickshell/pieces/${fileName piece}" { source = linked piece; }
        ) overview
      )
      // {
        "quickshell/Overview.qml".text = ''
          pragma Singleton
          import Quickshell
          import QtQuick

          // The overview's pieces, in order, from jitsusama.quickshell.overview.pieces.
          Singleton {
              readonly property var pieces: ${
                builtins.toJSON (
                  map (
                    piece:
                    removeAttrs piece [
                      "source"
                      "order"
                    ]
                    // {
                      file = fileName piece;
                    }
                  ) overview
                )
              }
          }
        '';
        # Not Keys.qml, which would hide QtQuick's Keys from every file.
        "quickshell/Shortcuts.qml".text = ''
          pragma Singleton
          import Quickshell
          import QtQuick

          // The desktop's actions the launcher can find by name, from
          // jitsusama.keys: what each does, its chord drawn key by key, its
          // glyph and the command that does the same.
          Singleton {
              readonly property var actions: ${builtins.toJSON actions}
          }
        '';
        "quickshell/Theme.qml".text = ''
          pragma Singleton
          import Quickshell
          import QtQuick

          Singleton {
          ${lib.concatLines (
            lib.mapAttrsToList (name: color: "    readonly property color ${name}: \"${color}\"") roles
          )}
              readonly property var mono: ${face font.mono.family type.mono}
              readonly property var sans: ${face font.sans.family type.sans}
              readonly property var space: ${object (removeAttrs space [ "unit" ])}
              readonly property int border: ${toString shape.border}
              readonly property int radius: ${toString shape.radius}
              readonly property int inset: ${toString shape.inset}
              // A part's corners: the whole's radius less its border and the
              // inset, so the curves run parallel.
              readonly property int inner: ${toString inner}
              readonly property var floating: ${
                object {
                  opacity = builtins.fromJSON floating.opacity;
                  scrim = builtins.fromJSON floating.scrim;
                  selection = builtins.fromJSON floating.selection;
                  # How far the shadows reach past the surface, which its
                  # window leaves room for.
                  reach = lib.foldl' lib.max 0 (map (s: s.offset + s.blur) floating.shadows);
                  shadows = map (s: {
                    inherit (s) offset blur;
                    opacity = builtins.fromJSON s.opacity;
                  }) floating.shadows;
                }
              }
              readonly property var motion: ${
                object {
                  # Slowed as niri slows its own, so the two stay in step.
                  inherit (lib.mapAttrs (_: slowed) { inherit (motion) short medium long; })
                    short
                    medium
                    long
                    ;
                  # Easing.BezierSpline's form: the two control points, then the end.
                  curve = map builtins.fromJSON motion.curve ++ [
                    1
                    1
                  ];
                }
              }
              readonly property var glyph: ${object (lib.mapAttrs (_: mark: mark.glyph) glyphs)}
              // Each picture's file, empty for none.
              readonly property var wallpaper: ${
                object {
                  desktop = picture wallpaper.desktop;
                  lock = picture wallpaper.lock;
                  shadows = map (s: {
                    inherit (s) x y blur;
                    opacity = builtins.fromJSON s.opacity;
                  }) wallpaper.shadows;
                }
              }
          }
        '';
      };

    systemd.user.services.quickshell = {
      Unit = {
        Description = "Quickshell, which draws the launcher, notifications and lock screen";
        PartOf = [ "graphical-session.target" ];
        After = [ "graphical-session.target" ];
        # Quickshell watches its files but not the links Home Manager swaps
        # them by, so a switch would leave it drawing the old ones. Naming
        # every file here changes the unit when any of them changes, and Home
        # Manager restarts a changed unit.
        X-Restart-Triggers = lib.mapAttrsToList (_: file: "${file.source}") (
          lib.filterAttrs (name: _: lib.hasPrefix "quickshell/" name) config.xdg.configFile
        );
      };
      Service = {
        ExecStart = lib.getExe' pkgs.quickshell "quickshell";
        Restart = "on-failure";
        RestartSec = 1;
      };
      Install.WantedBy = [ "graphical-session.target" ];
    };
  };
}

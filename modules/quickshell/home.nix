# Quickshell draws everything on Joel's screen that isn't a window: the
# launcher on Super+Space, notifications, the lock screen, polkit's password
# prompt, the volume and brightness, and the time and status in the overview's
# backdrop. Each is a QML file beside this one, linked as it is; Theme.qml,
# written here from jitsusama.theme, gives them all the same look. It runs with
# the graphical session, and restarts if it ever falls over. What the
# overview shows is jitsusama.quickshell.overview.pieces, rendered into
# Overview.qml at build.
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
    ;

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
    "Backdrop.qml"
    "OpenBelow.qml"
    "Niri.qml"
    "pieces/Piece.qml"
  ];

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
  imports = [ ../theme/home.nix ];

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

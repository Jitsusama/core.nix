# Quickshell draws everything on Joel's screen that isn't a window: the
# launcher on Super+Space, notifications, the lock screen, polkit's password
# prompt, the volume and brightness, and the time and status in the overview's
# backdrop. Each is a QML file beside this one, linked as it is; Theme.qml,
# written here from jitsusama.theme, gives them all the same look. It runs with
# the graphical session, and restarts if it ever falls over.
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
    "Grid.qml"
  ];
in
{
  imports = [ ../theme/home.nix ];

  home.packages = [ pkgs.quickshell ];

  xdg.configFile =
    lib.genAttrs (map (file: "quickshell/${file}") qml) (path: {
      source = ./. + "/${baseNameOf path}";
    })
    // {
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
    };
    Service = {
      ExecStart = lib.getExe' pkgs.quickshell "quickshell";
      Restart = "on-failure";
      RestartSec = 1;
    };
    Install.WantedBy = [ "graphical-session.target" ];
  };
}

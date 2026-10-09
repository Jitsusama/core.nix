# Quickshell draws everything on Joel's screen that isn't a window: the
# launcher on Super+Space, notifications, the lock screen, polkit's password
# prompt and the volume. Each is a QML file beside this one, linked as it is;
# Theme.qml, written here from jitsusama.theme, gives them all the same look.
# It runs with the graphical session, and restarts if it ever falls over.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  inherit (config.jitsusama.theme) colors font shape;

  qml = [
    "shell.qml"
    "Prompt.qml"
    "Launcher.qml"
    "Notifications.qml"
    "Lock.qml"
    "Polkit.qml"
    "Volume.qml"
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
          lib.mapAttrsToList (name: color: "    readonly property color ${name}: \"${color}\"") colors
        )}
            readonly property string fontFamily: "${font.family}"
            readonly property real fontSize: ${font.size}
            readonly property int border: ${toString shape.border}
            readonly property int radius: ${toString shape.radius}
            readonly property int gap: ${toString shape.gap}
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

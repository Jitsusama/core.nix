# Qt programs in the theme, through Qt's own GTK 3 platform theme: a Qt 6 or
# Qt 5 program takes the colours, fonts and dialogs GTK has from
# modules/gtk/home.nix, so a theme reaches it with nothing written for Qt.
# Each Qt that nixpkgs builds carries the plugin, and a wrapped program finds
# its own Qt's. Home Manager's qt module isn't used, since it also puts one
# Qt's plugin and QML paths in front of every program's, Quickshell's
# included. Quickshell draws from the theme itself, so its service leaves
# this out (modules/quickshell/home.nix).
_:
let
  theme = {
    QT_QPA_PLATFORMTHEME = "gtk3";
  };
in
{
  imports = [ ../gtk/home.nix ];

  # For shells, and for niri and everything it starts.
  home.sessionVariables = theme;
  systemd.user.sessionVariables = theme;
}

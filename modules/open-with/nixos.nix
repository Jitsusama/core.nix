# What to open a file or a link with, when a program asks the portal and
# more than one app could, asked in the launcher (OpenWith.qml, home.nix)
# rather than in GNOME's dialog: open-with (package.nix) answers the portal's
# AppChooser, and niri's portals take it for that alone, keeping GNOME's and
# GTK's for the rest as niri's own module sets them.
{ pkgs, ... }:
{
  xdg.portal = {
    extraPortals = [ (pkgs.callPackage ./package.nix { }) ];
    config.niri."org.freedesktop.impl.portal.AppChooser" = "launcher";
  };
}

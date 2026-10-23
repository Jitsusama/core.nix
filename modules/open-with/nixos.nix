# How files and links are opened. xdg-open goes through gio, which knows a
# kind from the kinds it's a case of (markdown is text, so Neovim opens it),
# opens the default the account's lists declare without asking, and starts
# a program that runs in a terminal in kitty through xdg-terminal-exec. The
# generic xdg-open read only a file's exact kind and ran a terminal program
# in place, so most text opened in the browser, and from a program with no
# terminal not at all. Chrome and Slack add their own xdg-open after the
# PATH, so this one is theirs too.
#
# When a program asks the portal and more than one app could open the
# thing, the launcher asks (OpenWith.qml, home.nix) rather than GNOME's
# dialog: open-with (package.nix) answers the portal's AppChooser, and
# niri's portals take it for that alone, keeping GNOME's and GTK's for the
# rest as niri's own module sets them.
{ lib, pkgs, ... }:
{
  environment.systemPackages = [
    (lib.hiPrio (pkgs.callPackage ./xdg-open.nix { }))
  ];

  xdg.terminal-exec = {
    enable = true;
    settings.default = [ "kitty.desktop" ];
  };

  xdg.portal = {
    extraPortals = [ (pkgs.callPackage ./package.nix { }) ];
    config.niri."org.freedesktop.impl.portal.AppChooser" = "launcher";
  };
}

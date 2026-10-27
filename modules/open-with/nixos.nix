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
#
# A program choosing a file to open or save asks the launcher too: open-with
# answers the portal's FileChooser (src/files.rs), and while the shell has no
# picker to ask, it hands the request to GTK's portal, whose dialog opens as
# before. Chromium's and Electron's dialogs and GTK 4's ask the portal on
# their own; GTK 3's native chooser asks it only when GTK_USE_PORTAL says
# to, and its own dialog never does, so a GTK 3 module
# (dialogs-ask-the-portal.c) asks the native chooser in that dialog's place,
# in every GTK 3 program, such as GIMP and Inkscape.
{ lib, pkgs, ... }:
{
  environment.systemPackages = [
    (lib.hiPrio (pkgs.callPackage ./xdg-open.nix { }))
  ];

  xdg.terminal-exec = {
    enable = true;
    settings.default = [ "kitty.desktop" ];
  };

  environment.sessionVariables = {
    GTK_USE_PORTAL = "1";
    GTK3_MODULES = [
      "${pkgs.callPackage ./dialogs-ask-the-portal.nix { }}/lib/libdialogs-ask-the-portal.so"
    ];
  };

  xdg.portal = {
    extraPortals = [ (pkgs.callPackage ./package.nix { }) ];
    config.niri = {
      "org.freedesktop.impl.portal.AppChooser" = "launcher";
      # nixpkgs' niri module gives it to GTK's portal, which open-with
      # asks in turn.
      "org.freedesktop.impl.portal.FileChooser" = lib.mkForce "launcher";
    };
  };
}

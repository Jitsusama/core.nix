# The launcher's side of choosing what to open something with: the portal's
# chooser (nixos.nix) asks here, and OpenWith.qml holds the apps it offers
# until one is picked. The desktop's list of files opened lately is
# recent-files' (recent-files.nix): on the path, for a program that adds
# what it opens to it, as Neovim does, and read for each program's Recent
# in the launcher, with what GIMP and Inkscape add to it themselves.
{ config, pkgs, ... }:
let
  recent-files = pkgs.callPackage ./recent-files.nix { };
in
{
  imports = [ ../quickshell/recents.nix ];

  jitsusama.quickshell.launcher.providers.openWith = {
    order = 5;
    source = ./OpenWith.qml;
  };

  home.packages = [ recent-files ];

  jitsusama.quickshell.launcher.recents = [
    {
      watch = "${config.xdg.dataHome}/recently-used.xbel";
      list = [
        "${recent-files}/bin/recent-files"
        "list"
      ];
    }
  ];
}

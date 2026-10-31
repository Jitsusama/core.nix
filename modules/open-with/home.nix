# The launcher's side of choosing what to open something with: the portal's
# chooser (nixos.nix) asks here, and OpenWith.qml holds the apps it offers
# until one is picked. Its chooser of files asks here too, and
# FileChooser.qml holds the folders and files until they're picked, with the
# files opened lately first and the program's filters matched by the
# desktop's kinds of file (shared-mime-info). The desktop's list of files opened lately is
# recent-files' (recent-files.nix): on the path, for a program that adds
# what it opens to it, as Neovim does, and read for each program's Recent
# in the launcher, with what GIMP and Inkscape add to it themselves.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  recent-files = pkgs.callPackage ./recent-files.nix { };
in
{
  imports = [ ../quickshell/recents.nix ];

  jitsusama.quickshell.launcher.providers.openWith = {
    order = 5;
    source = ./OpenWith.qml;
  };

  jitsusama.quickshell.launcher.providers.fileChooser = {
    order = 6;
    source = ./FileChooser.qml;
    recent = [
      "${recent-files}/bin/recent-files"
      "list"
    ];
    kinds = "${pkgs.shared-mime-info}/share/mime";
    # What makes the folder a file saved into a place kept elsewhere goes
    # in, as Google's Drive stages one to upload.
    mkdir = lib.getExe' pkgs.coreutils "mkdir";
    # Where the folder a save last went to is kept, made as it's first
    # written.
    savedIn = "${config.xdg.stateHome}/open-with/saved-in";
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

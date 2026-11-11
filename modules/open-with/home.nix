# The launcher's side of choosing what to open something with: the portal's
# chooser (nixos.nix) asks here, and OpenWith.qml holds the apps it offers
# until one is picked. Its chooser of files asks here too, and
# FileChooser.qml holds the folders and files until they're picked, with the
# files opened lately first and the program's filters matched by the
# desktop's kinds of file (shared-mime-info), and file-preview
# (file-preview.nix), one kept running for each ask, saying what the
# selected one is, its first page or frame made once. The desktop's list of files opened lately is
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
  file-preview = pkgs.callPackage ./file-preview.nix { };
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
    # What says a file's preview and a folder's items, for the drawer and
    # each row.
    preview = [ (lib.getExe file-preview) ];
    # What makes the folder a file saved into a place kept elsewhere goes
    # in, as Google's Drive stages one to upload.
    mkdir = lib.getExe' pkgs.coreutils "mkdir";
    # Where the folder a save last went to is kept, made as it's first
    # written.
    savedIn = "${config.xdg.stateHome}/open-with/saved-in";
    # Where the order the folders list in is kept, made as it's first
    # written.
    orderIn = "${config.xdg.stateHome}/open-with/order";
  };

  # Capture's Recent asks the same of what was taken, for a picture's
  # dimensions and a recording's length and frame in its drawer, through
  # the programs daemon, which keeps one (../programs/src/preview.rs).
  jitsusama.quickshell.launcher.providers.capture.preview = true;

  home.packages = [ recent-files ];

  jitsusama.quickshell.launcher.recents = [
    {
      watch = "${config.xdg.dataHome}/recently-used.xbel";
      list = [
        "${recent-files}/bin/recent-files"
        "list"
      ];
      dated = true;
    }
  ];
}

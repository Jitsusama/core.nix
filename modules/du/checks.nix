# The checks of du, which checks.nix gathers with every other folder's; each
# says what it proves.
{
  account,
  lib,
  macWorkstation,
  onLinux,
  pkgs,
  workstation,
  ...
}:
# What follows runs du.sh over a stand-in home with a stand-in du, since
# macOS's du, whose -I it relies on, isn't there to run on Linux.
onLinux {
  # A Mac's account has its own du, which runs macOS's with the ignore file
  # and leaves the folders it guards out by name when a count starts above
  # one (tests/du-leaves-guarded-folders-alone.sh). Linux's account gets no
  # such du, since nothing there asks.
  du-leaves-guarded-folders-alone =
    let
      dus = machine: lib.filter (package: package.name == "du") (account machine).home.packages;
      # Compared as words alone, as a match may not name the store, nor read
      # the line home.nix breaks; du.sh takes all four, the data volume too,
      # before what the counter typed.
      words =
        text:
        lib.concatStringsSep " " (
          lib.filter (word: word != "" && word != "\\") (
            lib.splitString " " (
              builtins.replaceStrings [ "\n" ] [ " " ] (builtins.unsafeDiscardStringContext text)
            )
          )
        );
      runs = words "${./du.sh} /usr/bin/du ${../ripgrep/ignore} /System/Volumes/Data \"$@\"";
    in
    assert lib.assertMsg (
      dus macWorkstation != [ ]
    ) "A Mac's account runs macOS's du, which opens the folders macOS asks about.";
    assert lib.assertMsg (lib.all (du: lib.hasInfix runs (words (du.text or ""))) (
      dus macWorkstation
    )) "A Mac's du doesn't run du.sh with macOS's du, the ignore file and the data volume.";
    assert lib.assertMsg (dus workstation == [ ]) "A Linux account runs the Mac's du.";
    pkgs.runCommandLocal "du-leaves-guarded-folders-alone" { } ''
      ${lib.getExe pkgs.bash} ${./tests/du-leaves-guarded-folders-alone.sh} \
        ${./du.sh} ${../ripgrep/ignore}
      touch $out
    '';
}

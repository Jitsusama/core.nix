# The checks of find, which checks.nix gathers with every other folder's;
# each says what it proves.
{
  account,
  lib,
  macWorkstation,
  onLinux,
  pkgs,
  workstation,
  ...
}:
# What follows runs Linux's find over a stand-in home, which a Mac's sandbox
# can't make unreadable the same way; macOS's find takes the same -path and
# -prune.
onLinux {
  # A Mac's account has its own find, which runs macOS's with the ignore file
  # and leaves the folders it guards alone when a search starts above one,
  # while finding everything else; macOS's find alone opens them
  # (tests/find-leaves-guarded-folders-alone.sh). Linux's account gets no
  # such find, since nothing there asks. find reads only a line naming one
  # folder from the home folder's top, so every line of the file is held to
  # that, lest ripgrep and find guard different folders.
  find-leaves-guarded-folders-alone =
    let
      finds = machine: lib.filter (package: package.name == "find") (account machine).home.packages;
      # Compared as text alone, as a match may not name the store.
      runs = builtins.unsafeDiscardStringContext "${./find.sh} /usr/bin/find ${../ripgrep/ignore}";
      text = find: builtins.unsafeDiscardStringContext (find.text or "");
      lines = lib.filter (line: line != "" && !lib.hasPrefix "#" line) (
        lib.splitString "\n" (builtins.readFile ../ripgrep/ignore)
      );
      unread = lib.filter (line: builtins.match "/[^*?![]+/" line == null) lines;
    in
    assert lib.assertMsg (
      finds macWorkstation != [ ]
    ) "A Mac's account runs macOS's find, which opens the folders macOS asks about.";
    assert lib.assertMsg (lib.all (find: lib.hasInfix runs (text find)) (
      finds macWorkstation
    )) "A Mac's find doesn't run find.sh with macOS's find and the ignore file.";
    assert lib.assertMsg (finds workstation == [ ]) "A Linux account runs the Mac's find.";
    assert lib.assertMsg (
      unread == [ ]
    ) "find guards only a folder from the home folder's top, not ${lib.concatStringsSep ", " unread}";
    pkgs.runCommandLocal "find-leaves-guarded-folders-alone" { } ''
      ${lib.getExe pkgs.bash} ${./tests/find-leaves-guarded-folders-alone.sh} \
        ${./find.sh} ${../ripgrep/ignore} ${lib.getExe' pkgs.findutils "find"}
      touch $out
    '';
}

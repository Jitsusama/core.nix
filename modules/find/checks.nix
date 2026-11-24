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
  # A Mac's account runs its own find ahead of macOS's, which leaves the
  # folders ripgrep's ignore file guards alone when a search starts above
  # one, while finding everything else; macOS's find alone opens them
  # (tests/find-leaves-guarded-folders-alone.sh). Linux's account gets no
  # such find, since nothing there asks.
  find-leaves-guarded-folders-alone =
    let
      finds = machine: lib.filter (package: package.name == "find") (account machine).home.packages;
    in
    assert lib.assertMsg (
      finds macWorkstation != [ ]
    ) "A Mac's account runs macOS's find, which opens the folders macOS asks about.";
    assert lib.assertMsg (finds workstation == [ ]) "A Linux account runs the Mac's find.";
    pkgs.runCommandLocal "find-leaves-guarded-folders-alone" { } ''
      ${lib.getExe pkgs.bash} ${./tests/find-leaves-guarded-folders-alone.sh} \
        ${./find.sh} ${../ripgrep/ignore} ${lib.getExe' pkgs.findutils "find"}
      touch $out
    '';
}

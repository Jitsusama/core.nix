# The checks of ripgrep, which checks.nix gathers with every other folder's;
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
# What follows runs Linux's ripgrep and fd over a stand-in home, which a Mac's
# sandbox can't make unreadable the same way.
onLinux {
  # A Mac's home folder holds the ignore file, and a search with ripgrep or fd
  # started there or in Library never opens other programs' data or the folders
  # macOS asked about, while finding everything else; with no file, the same
  # search opens them (tests/searches-leave-other-programs-data-alone.sh).
  # Linux's home folder holds no such file, since nothing there asks.
  searches-leave-other-programs-data-alone =
    let
      declared = (account macWorkstation).home.file.".ignore".source or null;
    in
    assert lib.assertMsg (
      declared != null && builtins.readFile declared == builtins.readFile ./ignore
    ) "A Mac's home folder holds no ignore file for ripgrep and fd.";
    assert lib.assertMsg (
      !((account workstation).home.file ? ".ignore")
    ) "A Linux home folder holds the Mac's ignore file.";
    pkgs.runCommandLocal "searches-leave-other-programs-data-alone" { } ''
      ${lib.getExe pkgs.bash} ${./tests/searches-leave-other-programs-data-alone.sh} \
        ${./ignore} ${lib.getExe pkgs.ripgrep} ${lib.getExe pkgs.fd}
      touch $out
    '';
}

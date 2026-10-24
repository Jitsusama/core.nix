# The checks of bat, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  account,
  lib,
  pkgs,
  workstation,
  ...
}:
{
  # bat colours code in the account's theme from a cache built once, with
  # nothing to say, and delta finds the theme through it too; a switch
  # builds no cache of its own (tests/bat-reads-its-theme.sh).
  bat-reads-its-theme =
    let
      home = account workstation;
      step = home.home.activation.batCache.data or "";
    in
    assert lib.assertMsg (!lib.hasInfix "cache --build" step) "A switch still builds bat's cache.";
    pkgs.runCommandLocal "bat-reads-its-theme" { } ''
      ${lib.getExe pkgs.bash} ${../../tests/bat-reads-its-theme.sh} ${
        lib.escapeShellArgs [
          home.home-files
          (lib.getExe home.programs.bat.package)
          (lib.getExe home.programs.delta.package)
          home.jitsusama.theme.terminal.magenta
        ]
      }
      touch $out
    '';
}

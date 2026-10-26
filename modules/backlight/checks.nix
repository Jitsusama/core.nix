# The checks of backlight, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  graphical,
  lib,
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
onLinux {
  # clippy finds nothing in the backlight daemon, its tests included.
  backlight-passes-clippy = passesClippy "backlight-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );

  # A shut lid keeps the screen's level saved while it was open, and a
  # keyboard's is saved either way (saves-while-open-test.sh), and every
  # systemd-backlight unit saves through it.
  the-screen-keeps-its-level-while-the-lid-is-shut =
    let
      standIn = pkgs.writeShellScript "systemd-backlight-stand-in" ''
        echo "$*" >> "$STAND_IN_SAID"
      '';
      program = pkgs.callPackage ./saves-while-open.nix { save = standIn; };
      stop = graphical.config.systemd.services."systemd-backlight@".serviceConfig.ExecStop;
    in
    assert lib.assertMsg (lib.any (lib.hasInfix "saves-while-open") stop)
      "systemd-backlight still saves the screen's level as it reads, lid shut or not.";
    pkgs.runCommandLocal "the-screen-keeps-its-level-while-the-lid-is-shut" { } ''
      export STAND_IN_SAID=$PWD/said
      ${lib.getExe pkgs.bash} ${./saves-while-open-test.sh} ${lib.getExe program} "$STAND_IN_SAID"
      touch $out
    '';
}

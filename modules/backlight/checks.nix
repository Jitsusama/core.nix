# The checks of backlight, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  graphical,
  holds,
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

  # The screen's saved level is kept as it changes, through the saver the
  # shutdown saves with, so it passes over the panel while a lid is shut as
  # shutdown does (src/keep.rs).
  the-screen-keeps-its-level-as-it-changes =
    let
      services = graphical.config.systemd.services;
      # Compared as text: a store path can't be matched against.
      plain = builtins.unsafeDiscardStringContext;
      start = plain (services.backlight-keeps-the-open-level.serviceConfig.ExecStart or "");
      saver = plain (
        lib.findFirst (lib.hasInfix "saves-while-open") ""
          services."systemd-backlight@".serviceConfig.ExecStop
      );
    in
    holds "the-screen-keeps-its-level-as-it-changes"
      "Nothing keeps the screen's saved level as it changes, or it saves through another saver than shutdown's, which would save the shut lid's dark level."
      (
        lib.hasInfix "--keeps-the-open-level" start
        && saver != ""
        && lib.hasInfix (lib.head (lib.splitString " " saver)) start
        && lib.elem "multi-user.target" services.backlight-keeps-the-open-level.wantedBy
      );
}

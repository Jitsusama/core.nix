# The checks of firmware-settings, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  lib,
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
{
  # firmware-settings names every setting the firmware holds apart from the
  # declared ones, writes them with the admin password from its input, and
  # wipes that password from the kernel whether the writes worked or not,
  # or it was stopped partway. A value it can't read, as without root, it
  # refuses rather than calling it changed.
  # Files stand in for the kernel's firmware-attributes class
  # (tests/the-firmware-takes-what-is-declared.sh).
  the-firmware-takes-what-is-declared =
    pkgs.runCommandLocal "the-firmware-takes-what-is-declared"
      { nativeBuildInputs = [ (pkgs.callPackage ./package.nix { }) ]; }
      ''
        ${lib.getExe pkgs.bash} ${../../tests/the-firmware-takes-what-is-declared.sh}
        touch $out
      '';
}
// onLinux {
  # clippy finds nothing in firmware-settings, its tests included.
  firmware-settings-passes-clippy = passesClippy "firmware-settings-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
}

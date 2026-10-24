# The checks of memory-modules, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
onLinux {
  # Its tests run as it builds: each field read from the bytes the firmware
  # lists, an empty slot left out, LPDDR5 past 6400 MT/s read as LPDDR5X,
  # and the file written as the launcher reads it.
  memory-modules-reads-the-firmware = pkgs.callPackage ./package.nix { };

  # clippy finds nothing in memory-modules, its tests included.
  memory-modules-passes-clippy = passesClippy "memory-modules-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
}

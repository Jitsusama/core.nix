# The checks of eero, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
# eero seals its session with systemd's credential service, and builds only
# on Linux.
onLinux {
  # eero's tests run as it builds: a network's record reads into its line,
  # speed test and setting, a missing field reads as unknown, only connected
  # devices are listed, an aged session is told from other refusals, the
  # status line and file say what eero said and nothing it didn't, and the
  # conferencing setting is sent as eero takes it and counted only once read
  # back.
  eero-reads-what-eero-says = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in eero, its tests included.
  eero-passes-clippy = passesClippy "eero-passes-clippy" (pkgs.callPackage ./package.nix { });
}

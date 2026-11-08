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
  # eero's tests run as it builds: the contract's answers read into exactly
  # the model home-network.json holds, a read not yet made reads as unknown,
  # eero failing, refusing and a session aged past refreshing are told apart,
  # each part of the home network is read at its own pace and faster while
  # someone listens, an outage keeps the last figures and says since when
  # and why, a listener on the socket is written the model on joining and on
  # every change, the status line says what eero said and nothing it didn't,
  # and the conferencing setting is sent as eero takes it and counted only
  # once read back.
  eero-reads-what-eero-says = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in eero, its tests included.
  eero-passes-clippy = passesClippy "eero-passes-clippy" (pkgs.callPackage ./package.nix { });
}

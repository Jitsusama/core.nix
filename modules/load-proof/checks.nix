# The checks of load-proof, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
# load-proof talks to a Wayland compositor and Linux's cgroups, and builds
# only there.
onLinux {
  # load-proof's tests run as it builds: a frame shown at every refresh
  # misses none, a refresh skipped with the probe's frame ready is niri's
  # miss and one passed before the probe committed is the probe's, and a
  # frame niri dropped for a later one is counted apart; a key typed into a
  # pty comes back from a shell's stand-in; a careful save leaves its newest
  # text whole; and echoes and saves are summed.
  load-proof-reads-its-signals = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in load-proof, its tests included.
  load-proof-passes-clippy = passesClippy "load-proof-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
}

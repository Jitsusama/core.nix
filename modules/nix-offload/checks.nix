# The checks of nix-offload, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
onLinux {
  # When a builder is asked for and given back, from the queue's jobs, the
  # builder's line and the journal, its tests run as it builds.
  nix-offload-decides-when = pkgs.callPackage ./offload/package.nix { };

  # clippy finds nothing in nix-offload, its tests included.
  nix-offload-passes-clippy = passesClippy "nix-offload-passes-clippy" (
    pkgs.callPackage ./offload/package.nix { }
  );
}

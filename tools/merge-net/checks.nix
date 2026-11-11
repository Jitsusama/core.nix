# The checks of merge-net, which checks.nix gathers with every other folder's.
{
  passesClippy,
  pkgs,
  ...
}:
{
  # merge-net decides what to run, revert, quarantine and hold from what
  # each run gave, so its tests drive a round against a stand-in for git,
  # nix and GitHub, and run as it builds (tools/merge-net).
  merge-net-catches-a-broken-merge = pkgs.callPackage ./package.nix { };

  # clippy finds nothing in merge-net, its tests included.
  merge-net-passes-clippy = passesClippy "merge-net-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
}

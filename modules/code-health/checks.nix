# The checks of code-health, which checks.nix gathers with every other
# folder's.
{
  passesClippy,
  pkgs,
  ...
}:
{
  # code-health's tests run as it builds: a long file or crowded folder
  # counts only without its reason, a reason must open a sentence, a
  # suppression is read in its own language's way and not in a string, a
  # run of lines repeated from elsewhere is counted once where it repeats
  # and braces alone repeat nothing, git's batch answer is split by its
  # sizes, a repository's ref and folder go to it, and the day's file is
  # the form cadence reads.
  code-health-counts-what-got-messier = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in code-health, its tests included.
  code-health-passes-clippy = passesClippy "code-health-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
}

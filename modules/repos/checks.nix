# The checks of repos, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  passesClippy,
  pkgs,
  ...
}:
{
  # repos's tests run as it builds: checkouts and trees laid out by hand
  # as git lays them out, a reftable's included, read into one
  # repository for each remote, and every view of them fitting each width.
  repos-reads-the-disk = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in repos, its tests included.
  repos-passes-clippy = passesClippy "repos-passes-clippy" (pkgs.callPackage ./package.nix { });
}

# The checks of system, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  passesClippy,
  pkgs,
  ...
}:
{
  # system's tests run as it builds: each part judged from its owner's
  # answer as that owner writes it, an owner that fails leaving its part
  # not read, and the screen fitting each width.
  system-judges-each-part = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in system, its tests included.
  system-passes-clippy = passesClippy "system-passes-clippy" (pkgs.callPackage ./package.nix { });
}

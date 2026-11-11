# The checks of changed-checks, which checks.nix gathers with every other
# folder's.
{
  passesClippy,
  pkgs,
  ...
}:
{
  # changed-checks names a check by how its derivation moved, so what it
  # builds is what a change touched: its tests run as it builds, the
  # whole program among them, against stand-ins for git and nix
  # (tools/changed-checks).
  changed-checks-names-what-moved = pkgs.callPackage ./package.nix { };

  # clippy finds nothing in changed-checks, its tests included.
  changed-checks-passes-clippy = passesClippy "changed-checks-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
}

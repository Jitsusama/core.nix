# The checks of pain-points, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
# pain-points reads the launcher's record and the user journal, and
# builds only where the launcher runs.
onLinux {
  # pain-points' tests run as it builds: what an opening lost Joel, which
  # openings are one go at a thing, and what of a pick it may say.
  pain-points-names-what-cost-him = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in pain-points, its tests included.
  pain-points-passes-clippy = passesClippy "pain-points-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
}

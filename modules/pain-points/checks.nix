# The checks of pain-points, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
# pain-points reads the launcher's record, the user journal and pi's
# logs, and builds only where the launcher runs.
onLinux {
  # pain-points' tests run as it builds: what an opening or a message to
  # pi lost Joel, which are one go at a thing, what of a pick it may say,
  # and that nothing he typed to pi is kept.
  pain-points-names-what-slowed-him = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in pain-points, its tests included.
  pain-points-passes-clippy = passesClippy "pain-points-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
}

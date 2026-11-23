# The checks of house-report, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
onLinux {
  # house-report's tests run as it builds: a report reads as the shaper
  # parses it, and the total it sends never goes back.
  house-report-says-what-the-link-sent = pkgs.callPackage ./package.nix { };

  # clippy finds nothing in house-report, its tests included.
  house-report-passes-clippy = passesClippy "house-report-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
}

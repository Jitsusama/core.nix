# The checks of sun, which checks.nix gathers with every other folder's;
# its opening comment says what each kind of check proves.
{
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
onLinux {
  # sun's tests run as it builds: its sunrises and sunsets are within a
  # minute of NOAA's tables, far north too, and where the sun neither rises
  # nor sets it says whether it stays up or down.
  the-sun-sets-as-noaa-says = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in it, its tests included.
  sun-passes-clippy = passesClippy "sun-passes-clippy" (pkgs.callPackage ./package.nix { });
}

# The checks of sign-in-program, which checks.nix gathers with every
# other folder's; its opening comment says what each kind of check proves.
{
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
onLinux {
  # Its tests run as it builds: a sign-in's token is taken trimmed, slow
  # or ending after its output; one failing, printing nothing, missing,
  # printing what isn't text or past the most is none; one hanging,
  # lingering or printing for good runs over, and what it started is
  # killed with it.
  sign-in-program-never-waits-for-good = pkgs.callPackage ./package.nix { };
  # clippy finds nothing in sign-in-program, its tests included.
  sign-in-program-passes-clippy = passesClippy "sign-in-program-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
}

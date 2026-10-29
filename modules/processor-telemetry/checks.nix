# The checks of processor-telemetry, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
onLinux {
  # clippy finds nothing in the reader, its tests included.
  processor-telemetry-passes-clippy = passesClippy "processor-telemetry-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
}

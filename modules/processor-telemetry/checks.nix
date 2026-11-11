# The checks of processor-telemetry, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
onLinux {
  # The reader's tests run as it builds: each region found by its GUID, a
  # counter read across its wrap, and what kept the package from resting.
  processor-telemetry-reads-each-region = pkgs.callPackage ./package.nix { };

  # clippy finds nothing in the reader, its tests included.
  processor-telemetry-passes-clippy = passesClippy "processor-telemetry-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
}

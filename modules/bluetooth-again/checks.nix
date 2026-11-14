# The checks of bluetooth-again, which checks.nix gathers with every other
# folder's; its opening comment says what each kind of check proves.
{
  onLinux,
  passesClippy,
  pkgs,
  ...
}:
onLinux {
  # Its tests run as it builds, over a stand-in sysfs: a controller found by
  # its PCI class with its driver, let go of and taken back by that driver,
  # one no driver holds handed to the bus, and a refusal or a controller
  # that doesn't come back said as a failure.
  bluetooth-again-starts-a-controller = pkgs.callPackage ./package.nix { };

  # clippy finds nothing in bluetooth-again, its tests included.
  bluetooth-again-passes-clippy = passesClippy "bluetooth-again-passes-clippy" (
    pkgs.callPackage ./package.nix { }
  );
}

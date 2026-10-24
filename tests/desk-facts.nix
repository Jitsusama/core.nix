# Plugs a USB sound card into a running virtual machine, pulls it out and
# plugs it in again, through QEMU's monitor, and reads what desk-facts says
# of it each time, as a desk's speakers come and go when a laptop leaves it
# and comes back. The card is QEMU's own USB audio device, which the kernel's
# USB audio driver takes as it would a real one, on an xHCI controller. The
# script is desk-facts-vm-test.py.
{ self, pkgs }:
pkgs.testers.runNixOSTest {
  name = "desk-facts";

  nodes.machine = {
    imports = [ self.nixosModules.desk-facts ];
    users.users.joel.isNormalUser = true;
    # The controller is named so a device can be plugged into it later, and
    # the card's sound goes nowhere.
    virtualisation.qemu.options = [
      "-device qemu-xhci,id=desk"
      "-audiodev none,id=nowhere"
    ];
  };

  # What VM tests' scripts share (tests/lib).
  extraPythonPackages = python: [ (pkgs.callPackage ./lib/vm-helpers.nix { } python) ];

  testScript = builtins.readFile ./desk-facts-vm-test.py;
  # nix fmt holds the script to mypy --strict against the driver's own types,
  # through driver_names.pyi, which the driver's lighter check can't find.
  skipTypeCheck = true;
}

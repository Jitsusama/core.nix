# Boots a virtual machine with a USB controller and reads each of its root
# hubs' wakeup, which the kernel leaves off and the usb-wake module turns on,
# so a cable plugged in wakes the machine. A virtual machine can't sleep and
# be woken by a plug, so what's checked is the setting the wake needs. The
# script is usb-wake-vm-test.py.
{ self, pkgs }:
pkgs.testers.runNixOSTest {
  name = "usb-wake";

  nodes.machine = {
    imports = [ self.nixosModules.usb-wake ];
    virtualisation.qemu.options = [ "-device qemu-xhci" ];
  };

  testScript = builtins.readFile ./usb-wake-vm-test.py;
  # nix fmt holds the script to mypy --strict against the driver's own types,
  # through driver_names.pyi, which the driver's lighter check can't find.
  skipTypeCheck = true;
}

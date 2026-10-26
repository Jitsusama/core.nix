# Boots a virtual machine with a USB controller and reads each of its root
# hubs' wakeup, which the kernel leaves off and the usb-wake module turns on
# for the USB 2 and 1.1 ones, so a cable plugged in wakes the machine, and
# leaves off for USB 3's. A virtual machine can't sleep and be woken by a
# plug, so what's checked is the setting the wake needs. The tests are
# usb-wake-vm-test.py, which pytest runs inside the driver
# (tests/lib/vm_pytest.py).
{ self, pkgs }:
import ./lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "usb-wake";

  nodes.machine = {
    imports = [ self.nixosModules.usb-wake ];
    virtualisation.qemu.options = [ "-device qemu-xhci" ];
  };

  tests = ./usb-wake-vm-test.py;
}

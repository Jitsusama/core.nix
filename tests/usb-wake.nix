# Boots a virtual machine with a USB controller and reads each of its root
# hubs' wakeup, which the kernel leaves off and the usb-wake module turns on,
# so a cable plugged in wakes the machine. A virtual machine can't sleep and
# be woken by a plug, so what's checked is the setting the wake needs.
{ self, pkgs }:
pkgs.testers.runNixOSTest {
  name = "usb-wake";

  nodes.machine = {
    imports = [ self.nixosModules.usb-wake ];
    virtualisation.qemu.options = [ "-device qemu-xhci" ];
  };

  testScript = ''
    machine.wait_for_unit("multi-user.target")
    hubs = machine.succeed("ls -d /sys/bus/usb/devices/usb*").split()
    assert hubs, "no USB root hubs"
    for hub in hubs:
        wakeup = machine.succeed(f"cat {hub}/power/wakeup").strip()
        machine.log(f"{hub} wakeup {wakeup}")
        assert wakeup == "enabled", f"{hub} wakeup is {wakeup}"
  '';
}

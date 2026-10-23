"""Every USB root hub of a virtual machine may wake it, as usb-wake sets them.

tests/usb-wake.nix runs this, on a machine given a USB controller. The kernel leaves each root
hub's wakeup off, so finding every one enabled shows the module's setting took. The script
needs no value from Nix.
"""

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from driver_names import machine

machine.wait_for_unit("multi-user.target")
hubs = machine.succeed("ls -d /sys/bus/usb/devices/usb*").split()
assert hubs, "no USB root hubs"
for hub in hubs:
    wakeup = machine.succeed(f"cat {hub}/power/wakeup").strip()
    machine.log(f"{hub} wakeup {wakeup}")
    assert wakeup == "enabled", f"{hub} wakeup is {wakeup}"

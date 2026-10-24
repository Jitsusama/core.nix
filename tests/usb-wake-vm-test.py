"""A virtual machine's USB 2 and 1.1 root hubs may wake it, as usb-wake sets them, and not USB 3's.

tests/usb-wake.nix runs this, on a machine given a USB controller, which has root hubs at each
speed. The kernel leaves each root hub's wakeup off, so finding the 12 and 480 Mbit/s ones enabled
shows the module's setting took, and the SuperSpeed ones left off shows it took no more. The
script needs no value from Nix.
"""

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from driver_names import machine

# The root hubs' speeds that may wake the machine, in Mbit/s.
WAKING = ("12", "480")

machine.wait_for_unit("multi-user.target")
hubs = machine.succeed("ls -d /sys/bus/usb/devices/usb*").split()
speeds = {hub: machine.succeed(f"cat {hub}/speed").strip() for hub in hubs}
assert "480" in speeds.values(), f"no USB 2 root hub: {speeds}"
assert any(speed not in WAKING for speed in speeds.values()), f"no SuperSpeed root hub: {speeds}"
for hub in hubs:
    wakeup = machine.succeed(f"cat {hub}/power/wakeup").strip()
    machine.log(f"{hub} at {speeds[hub]} Mbit/s, wakeup {wakeup}")
    wanted = "enabled" if speeds[hub] in WAKING else "disabled"
    assert wakeup == wanted, f"{hub} at {speeds[hub]} Mbit/s has wakeup {wakeup}"

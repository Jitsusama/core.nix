"""The compressed swap comes down while something holds its device open, as udev's probe does.

tests/zram.nix runs this, and its opening comment says why a reset that tries once fails.
hold-zram, on the machine's path, holds /dev/zram0 open until the reset says it's busy, and
lets go at once. The script needs no value from Nix.
"""

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from driver_names import machine, subtest, t

setup = "systemd-zram-setup@zram0.service"
machine.wait_for_unit("dev-zram0.swap")
machine.succeed("systemd-run --unit=hold -p Type=exec hold-zram")
machine.wait_for_file("/run/holding-zram")

with subtest("the swap comes down once the device is let go"):
    machine.succeed(f"systemctl stop --no-block {setup}")
    machine.wait_until_succeeds(
        f"systemctl show -P ActiveState {setup} | grep -q -x -E 'inactive|failed'", timeout=30
    )
    t.assertEqual(machine.succeed(f"systemctl show -P Result {setup}").strip(), "success")
    t.assertEqual(machine.succeed("cat /sys/block/zram0/disksize").strip(), "0")

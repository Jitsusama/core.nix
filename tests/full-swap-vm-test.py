"""Full swap ends the work that filled it, through systemd-oomd, and nothing else.

tests/full-swap.nix runs this, and its opening comment says what fills memory. grow, on the
machine's path, takes random data up to the MiB it's told and keeps it. The script needs no
value from Nix.
"""

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from driver_names import machine, subtest, t

machine.wait_for_unit("systemd-oomd.service")
machine.wait_for_unit("dev-zram0.swap")

with subtest("swap is watched from the root slice"):
    t.assertIn("Path: /\n", machine.succeed("oomctl"))

# The small one settles first, so some of what it holds is in swap when the
# big one starts filling the rest.
machine.succeed("systemd-run --unit=small -p Type=exec grow 64")
machine.succeed("systemd-run --unit=big -p Type=exec grow 4096")

with subtest("the work that filled swap is ended by systemd-oomd"):
    machine.wait_until_succeeds(
        "journalctl -u systemd-oomd -o cat | grep -F 'Killed /system.slice/big.service'",
        timeout=180,
    )
    t.assertNotIn("Out of memory", machine.succeed("journalctl -k -o cat"))

with subtest("what held a little is left running"):
    t.assertEqual(machine.succeed("systemctl is-active small.service").strip(), "active")

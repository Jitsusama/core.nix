"""A laptop's lid shut in a virtual machine, with and without a monitor plugged in.

tests/lid.nix runs this, and its opening comment says what the test proves. run-subtests cuts
the script into subtests at each top-level `with subtest(`, so every subtest stays at the top
level under its own name, and what sits outside them, the boot, the stand-in connectors and
the helpers, runs whichever subtests are chosen. The script needs no value from Nix: the
stand-in lid, connectors and suspend are all named in the machine lid.nix builds.
"""

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from driver_names import machine, subtest, t

held = "systemd-inhibit --list --no-legend --no-pager | grep -q lid-docked"
holder = "systemctl show -P MainPID lid-docked.service"
opened = (
    "busctl get-property org.freedesktop.login1 /org/freedesktop/login1"
    " org.freedesktop.login1.Manager LidClosed | grep -qx 'b false'"
)
# The suspend unit's state is read rather than asked with is-active, which calls a oneshot
# still running "activating" and so can't tell a suspend under way from one that's over.
settled = (
    "busctl get-property org.freedesktop.login1 /org/freedesktop/login1"
    " org.freedesktop.login1.Manager PreparingForSleep | grep -qx 'b false'"
    " && ! systemctl list-jobs --no-legend | grep -q suspend"
    " && systemctl show -P ActiveState systemd-suspend.service | grep -qx inactive"
)


def lid(shut: int) -> None:
    """Shut the stand-in lid, given 1, or open it, given 0."""
    machine.succeed(f"echo {shut} > /sys/module/stand_in_lid/parameters/shut")


def monitor(status: str) -> None:
    """Write the monitor's connector as connected or disconnected, and say so as the card would."""
    machine.succeed(
        f"echo {status} > /run/stand-in-drm/card0-DP-5/status",
        "udevadm trigger --settle --action=change /sys/class/drm/card0",
    )


machine.wait_for_unit("multi-user.target")
machine.succeed(
    "test -e /sys/class/drm/card0",
    "mkdir -p /run/stand-in-drm/card0-eDP-1 /run/stand-in-drm/card0-DP-5",
    "echo connected > /run/stand-in-drm/card0-eDP-1/status",
)

with subtest("a laptop shut with no monitor suspends"):
    monitor("disconnected")
    machine.fail(held)
    lid(1)
    machine.wait_until_succeeds("test -e /run/suspended", timeout=60)
    lid(0)
    # The stand-in suspend returns at once, and logind, finding the lid
    # still shut, may queue another, which a loaded machine starts after
    # the lid opens. The file is cleared once logind has seen the lid
    # open and isn't preparing to sleep, and no suspend is queued or
    # running.
    machine.wait_until_succeeds(opened, timeout=30)
    machine.wait_until_succeeds(settled, timeout=60)
    machine.succeed("rm /run/suspended")

with subtest("a laptop shut at a desk keeps running once the monitor is off"):
    monitor("connected")
    machine.wait_until_succeeds(held, timeout=10)
    machine.succeed("echo disabled > /run/stand-in-drm/card0-DP-5/enabled")
    first = machine.succeed(holder).strip()
    lid(1)
    machine.sleep(5)
    # A monitor waking or sleeping changes the card again.
    monitor("connected")
    machine.sleep(5)
    t.assertEqual(machine.succeed(holder).strip(), first)
    machine.fail("test -e /run/suspended")

with subtest("unplugging the monitor with the lid shut suspends"):
    monitor("disconnected")
    machine.wait_until_succeeds("test -e /run/suspended", timeout=60)
    machine.fail(held)

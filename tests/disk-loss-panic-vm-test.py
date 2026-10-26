"""A virtual machine's second disk taken away under disk-loss-panic, which panics the kernel.

tests/disk-loss-panic.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py),
and its opening comment says what the test proves. The tests run in the file's order, since the
last one ends the machine: a warning while the disk answers, then the disk gone. The disk the
program watches is named in the machine the Nix builds, so the tests need no value from Nix.
"""

from __future__ import annotations

import datetime as dt
from typing import TYPE_CHECKING, cast

import pytest

if TYPE_CHECKING:
    from test_driver.machine import BaseMachine, QemuMachine

# The kernel's log, its messages alone. It's read with dmesg, since journalctl -k leaves out
# the records a program writes to /dev/kmsg.
LOG = "dmesg --notime"
NAME = "vdb"
DISK = f"/dev/{NAME}"
# A warning from the disk's path, as nvme gives one when a read times out.
WARNING = "nvme nvme0: I/O tag 12 QID 3 timeout, aborting"
# How long the program may take to read the disk and say what it found.
ANSWERING = dt.timedelta(seconds=30)
# How long a false alarm is watched for anything it shouldn't do.
LOOKING = dt.timedelta(seconds=5)
# How long the program may take to find the disk gone and the kernel to panic.
PANICKING = dt.timedelta(seconds=60)


def warn(machine: BaseMachine) -> None:
    """Write the warning to the kernel's log at the error level, as a driver would."""
    machine.succeed(f"echo '<3>{WARNING}' > /dev/kmsg")


def warn_soon(machine: BaseMachine) -> None:
    """Have the machine write the warning a second from now, and return at once.

    The kernel may panic before a command that writes it could say it's done, which the driver
    would take for a broken machine, so a timer of the machine's writes it instead.
    """
    machine.succeed(f"systemd-run --on-active=1 sh -c \"echo '<3>{WARNING}' > /dev/kmsg\"")


@pytest.fixture(scope="module")
def watching(machine: BaseMachine) -> QemuMachine:
    """Boot the machine and give it once disk-loss-panic watches its second disk."""
    # The test's one machine is QEMU's, whose console the panic is read from.
    qemu = cast("QemuMachine", machine)
    qemu.wait_for_unit("disk-loss-panic.service")
    qemu.wait_until_succeeds(
        f"{LOG} | grep -qx 'disk-loss-panic: watching {DISK}'", timeout=ANSWERING
    )
    return qemu


def test_a_warning_while_the_disk_answers_changes_nothing(watching: QemuMachine) -> None:
    """A warning from the disk's path while the disk answers changes nothing."""
    warn(watching)
    watching.wait_until_succeeds(
        f"{LOG} | grep -qF 'disk-loss-panic: {DISK} answered after \"{WARNING}\"'",
        timeout=ANSWERING,
    )
    watching.sleep(LOOKING)
    watching.succeed("systemctl is-active disk-loss-panic.service")
    watching.fail(f"{LOG} | grep -q 'stopped answering'")


def test_a_warning_once_the_disk_is_gone_panics_the_kernel(watching: QemuMachine) -> None:
    """A warning once the disk has dropped off its bus panics the kernel, saying why first."""
    watching.succeed(f'echo 1 > "$(readlink -f /sys/block/{NAME}/device)/../remove"')
    watching.wait_until_fails(f"test -e {DISK}", timeout=ANSWERING)
    warn_soon(watching)
    watching.wait_for_console_text(
        f"disk-loss-panic: {DISK} stopped answering after", timeout=PANICKING
    )
    watching.wait_for_console_text(
        "Kernel panic - not syncing: sysrq triggered crash", timeout=PANICKING
    )
    # The panicked kernel restarts, which ends QEMU; waiting for that tells the driver the
    # machine is down, so it doesn't reach into it once the tests are over.
    watching.wait_for_shutdown(timeout=PANICKING)

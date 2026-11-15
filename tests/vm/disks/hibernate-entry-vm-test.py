"""A machine started from an entry other than its default reads its image back through that entry.

hibernate-entry.nix, beside this, says how the machine is laid out and why. The tests need
nothing from Nix.
"""

from __future__ import annotations

import datetime as dt
from typing import TYPE_CHECKING

import pytest
from test_driver.machine import QemuMachine

if TYPE_CHECKING:
    from test_driver.machine import BaseMachine

# systemd-boot's variables, under the vendor its Boot Loader Interface gives them: the entry
# this start came from, and the one the next start takes once.
VENDOR = "4a67b082-0a4c-41cf-b6c7-440b29bb8c4f"
SELECTED = f"/sys/firmware/efi/efivars/LoaderEntrySelected-{VENDOR}"
ONE_SHOT = f"/sys/firmware/efi/efivars/LoaderEntryOneShot-{VENDOR}"
# What's kept only in memory, on a ramfs, which nothing writes to a disk.
MARK = "/run/kept/mark"
SAID = "kept only in memory"
# What hibernate-unlock says as it holds the next start to this one's entry.
HELD = "hibernate-unlock: the default system starts another kernel or initrd"
# How long a hibernation may take to make its image and power off.
GOES_OFF = dt.timedelta(minutes=3)


def entry_kept_in(machine: QemuMachine, variable: str) -> str:
    """The entry an EFI variable of systemd-boot's names: UTF-16 after four bytes of flags."""
    raw = bytes(int(byte, 16) for byte in machine.succeed(f"od -An -tx1 -v {variable}").split())
    return raw[4:].decode("utf-16-le").rstrip("\0")


@pytest.fixture(scope="module")
def other(machine: BaseMachine) -> str:
    """The entry of the specialisation, whose initrd isn't the default's."""
    machine.wait_for_unit("default.target")
    path = machine.succeed(
        "grep --files-with-matches --line-regexp --fixed-strings 'title NixOS (other)'"
        " /boot/loader/entries/*.conf"
    ).strip()
    return path.rsplit("/", 1)[-1]


@pytest.fixture(scope="module")
def started_from_other(machine: BaseMachine, other: str) -> QemuMachine:
    """Start the machine once from the other entry, as a switch's next reboot hasn't come."""
    # Only a machine QEMU runs can be stopped and started again.
    assert isinstance(machine, QemuMachine)
    machine.succeed(f"bootctl set-oneshot {other}")
    machine.shutdown()
    machine.start()
    machine.wait_for_unit("default.target")
    assert entry_kept_in(machine, SELECTED) == other, "the machine didn't start from the other entry"
    booted = machine.succeed("readlink -f /run/booted-system/initrd")
    default = machine.succeed("readlink -f /nix/var/nix/profiles/system/initrd")
    assert booted != default, "the other entry starts the default's initrd, so nothing differs"
    machine.wait_for_unit("swap.target")
    return machine


def test_the_image_is_read_back_through_the_entry_it_started_from(
    started_from_other: QemuMachine, other: str
) -> None:
    """The next start takes the entry the image's start came from, once, then lets it go."""
    machine = started_from_other
    machine.succeed("mkdir -p /run/kept", "mount -t ramfs ramfs /run/kept", f"echo {SAID} > {MARK}")
    machine.execute("systemctl hibernate >&2 &", check_return=False)
    machine.wait_for_shutdown(timeout=GOES_OFF)
    machine.start()
    machine.wait_for_unit("default.target")
    assert SAID in machine.succeed(f"cat {MARK}"), "the machine started afresh, not from its image"
    assert entry_kept_in(machine, SELECTED) == other, (
        "the image was read back through the default entry, not the one it started from"
    )
    machine.wait_until_succeeds(f"journalctl --grep '{HELD}'", timeout=60)
    machine.wait_until_succeeds(f"! test -e {ONE_SHOT}", timeout=60)

"""A machine hibernated to a swap file on btrfs inside LUKS comes back as it was.

tests/hibernate.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), and
its opening comment says how the machine is laid out. The disk is made here, on the first
boot, since a test's empty disk can't be made beforehand; the initrd opens it on every boot
after with /etc/disk.key. The tests need no value from Nix.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

import pytest
from test_driver.machine import QemuMachine

if TYPE_CHECKING:
    from test_driver.machine import BaseMachine

# What's kept only in memory, on a ramfs, which nothing writes to a disk.
MARK = "/run/kept/mark"
SAID = "kept only in memory"


@pytest.fixture(scope="module")
def swapping(machine: BaseMachine) -> QemuMachine:
    """Give the machine swapping to a file on btrfs inside LUKS, as a laptop does."""
    # Only a machine QEMU runs can be stopped and started again.
    assert isinstance(machine, QemuMachine)
    machine.wait_for_unit("default.target")
    machine.wait_for_unit("dev-zram0.swap")
    machine.succeed(
        "cryptsetup luksFormat --batch-mode --key-file /etc/disk.key /dev/vdc",
        "cryptsetup open --key-file /etc/disk.key /dev/vdc disk",
        "mkfs.btrfs --quiet /dev/mapper/disk",
        "mkdir -p /mnt/disk /swap",
        "mount /dev/mapper/disk /mnt/disk",
        "btrfs subvolume create /mnt/disk/@swap",
        "mount -o subvol=@swap /dev/mapper/disk /swap",
        "btrfs filesystem mkswapfile --size 1536m /swap/swapfile",
        "swapon /swap/swapfile",
    )
    return machine


def test_nothing_says_where_to_resume_from(swapping: QemuMachine) -> None:
    """The kernel's command line names no resume device, so the EFI variable has to."""
    assert "resume=" not in swapping.succeed("cat /proc/cmdline")


def test_a_hibernated_machine_comes_back_as_it_was(swapping: QemuMachine) -> None:
    """What was only in memory is there again once the machine starts from its image."""
    swapping.succeed(
        "mkdir -p /run/kept",
        "mount -t ramfs -o size=1m ramfs /run/kept",
        f"echo '{SAID}' > {MARK}",
    )
    swapping.execute("systemctl hibernate >&2 &", check_return=False)
    swapping.wait_for_shutdown()
    swapping.start()
    assert SAID in swapping.succeed(f"cat {MARK}")
    assert swapping.succeed("swapon --show=NAME --noheadings").count("/swap/swapfile") == 1


def test_a_boot_after_it_starts_afresh(swapping: QemuMachine) -> None:
    """The image is used once: a machine started again after it boots anew."""
    swapping.crash()
    swapping.wait_for_unit("default.target")
    swapping.fail(f"test -e {MARK}")

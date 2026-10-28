"""A machine hibernated to a swap file on btrfs inside LUKS comes back as it was.

tests/hibernate.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), and
its opening comment says how the machine is laid out. The disk is made here, on the first
boot, since a test's empty disk can't be made beforehand, and opened by the TPM once its PIN
is typed at the initrd's prompt on every start after. The tests need no value from Nix.
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
# The disk's PIN, asked for at the initrd's prompt.
PIN = "2468"
# What the initrd asks it with.
ASKED = r"[Ee]nter .*PIN"
# What hibernate-unlock says once it has let the lock go.
LET_GO = "hibernate-unlock: back through the disk prompt; the lock is let go"


def start_through_the_disk_prompt(machine: QemuMachine) -> None:
    """Start the machine and type the disk's PIN when the initrd asks for it."""
    machine.start()
    machine.wait_for_console_text(ASKED)
    machine.send_console(f"{PIN}\n")


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
        # As a laptop's disk is enrolled: the TPM, with a PIN.
        f"NEWPIN={PIN} systemd-cryptenroll --unlock-key-file=/etc/disk.key"
        " --tpm2-device=auto --tpm2-pcrs= --tpm2-with-pin=yes /dev/vdc",
    )
    return machine


def test_nothing_says_where_to_resume_from(swapping: QemuMachine) -> None:
    """The kernel's command line names no resume device, so the EFI variable has to."""
    assert "resume=" not in swapping.succeed("cat /proc/cmdline")


def test_the_image_is_written_with_lz4(swapping: QemuMachine) -> None:
    """The image is compressed with lz4, which reads back faster than the kernel's lzo."""
    compressor = swapping.succeed("cat /sys/module/hibernate/parameters/compressor")
    assert compressor.strip() == "lz4", f"the image would be compressed with {compressor}"


def test_a_hibernated_machine_comes_back_as_it_was(swapping: QemuMachine) -> None:
    """What was only in memory is there again once the machine starts from its image."""
    swapping.succeed(
        "mkdir -p /run/kept",
        "mount -t ramfs -o size=1m ramfs /run/kept",
        f"echo '{SAID}' > {MARK}",
    )
    swapping.execute("systemctl hibernate >&2 &", check_return=False)
    swapping.wait_for_shutdown()
    start_through_the_disk_prompt(swapping)
    assert SAID in swapping.succeed(f"cat {MARK}")
    assert swapping.succeed("swapon --show=NAME --noheadings").count("/swap/swapfile") == 1


def test_coming_back_through_the_disk_prompt_lets_the_lock_go(swapping: QemuMachine) -> None:
    """The PIN typed at the disk is the one prompt: hibernate-unlock lets the lock go."""
    swapping.wait_until_succeeds(f"journalctl --boot --grep '{LET_GO}'", timeout=30)


def test_a_boot_after_it_starts_afresh(swapping: QemuMachine) -> None:
    """The image is used once: a machine started again after it boots anew."""
    swapping.crash()
    start_through_the_disk_prompt(swapping)
    swapping.wait_for_unit("default.target")
    swapping.fail(f"test -e {MARK}")

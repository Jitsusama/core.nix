"""A machine hibernated to a swap file on btrfs inside LUKS comes back as it was.

tests/hibernate.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), and
its opening comment says how the machine is laid out. The disk is made here, on the first
boot, since a test's empty disk can't be made beforehand, and opened by the TPM once its PIN
is typed at the initrd's prompt on every start after. The tests need no value from Nix but
the reserve, which they repeat.
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
# The hook systemd-sleep runs, and what its rescue says with no lid shut.
HOOK = "/etc/systemd/system-sleep/hibernate-unlock"
SAID_SO = "hibernate-unlock: the lid is open, so it says so"
# What the rescue logs as it puts its notice in each session.
SAYS = "hibernate-unlock: it says: "
# The reserve hibernate.nix sets, in bytes.
RESERVE = 128 * 1024 * 1024
# While there, a hibernation keeps the driver's shell, and the swap file is taken away
# (hibernate.nix says how).
ROLLS_BACK = "/run/rolls-back"
NO_SWAP_FILE = "/run/no-swap-file"


def start_through_the_disk_prompt(machine: QemuMachine) -> None:
    """Start the machine and type the disk's PIN when the initrd asks for it."""
    machine.start()
    machine.wait_for_console_text(ASKED)
    machine.send_console(f"{PIN}\n")


def hibernate_and_come_back(machine: QemuMachine) -> None:
    """Hibernate, wait for the machine to go off, and start it from its image."""
    machine.execute("systemctl hibernate >&2 &", check_return=False)
    machine.wait_for_shutdown()
    start_through_the_disk_prompt(machine)


def roll_back(machine: QemuMachine, saying: str) -> None:
    """Hibernate a machine made to roll back, and wait for the rescue to say why."""
    machine.succeed(f"touch {ROLLS_BACK}")
    since = machine.succeed("date +%s").strip()
    machine.execute("systemctl hibernate >&2 &", check_return=False)
    machine.wait_until_succeeds(f"journalctl --since @{since} --grep '{SAYS}{saying}'", timeout=300)
    machine.succeed(f"rm {ROLLS_BACK}")


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
        # As a laptop's disk is enrolled: the TPM, with a PIN, and a
        # recovery key, which is typed too, so the lock is still let go.
        f"NEWPIN={PIN} systemd-cryptenroll --unlock-key-file=/etc/disk.key"
        " --tpm2-device=auto --tpm2-pcrs= --tpm2-with-pin=yes /dev/vdc",
        "systemd-cryptenroll --unlock-key-file=/etc/disk.key --recovery-key /dev/vdc",
    )
    return machine


def test_nothing_says_where_to_resume_from(swapping: QemuMachine) -> None:
    """The kernel's command line names no resume device, so the EFI variable has to."""
    assert "resume=" not in swapping.succeed("cat /proc/cmdline")


def test_the_image_is_written_with_lz4(swapping: QemuMachine) -> None:
    """The image is compressed with lz4, which reads back faster than the kernel's lzo."""
    compressor = swapping.succeed("cat /sys/module/hibernate/parameters/compressor")
    assert compressor.strip() == "lz4", f"the image would be compressed with {compressor}"


def test_the_reserve_is_written_as_it_starts(swapping: QemuMachine) -> None:
    """The kernel leaves the reserve free for drivers from the start, not its own 1 MB."""
    reserve = int(swapping.succeed("cat /sys/power/reserved_size"))
    assert reserve == RESERVE, f"the kernel reserves {reserve} bytes"


def test_a_hibernated_machine_comes_back_as_it_was(swapping: QemuMachine) -> None:
    """What was only in memory is there again once the machine starts from its image."""
    swapping.succeed(
        "mkdir -p /run/kept",
        "mount -t ramfs -o size=1m ramfs /run/kept",
        f"echo '{SAID}' > {MARK}",
    )
    hibernate_and_come_back(swapping)
    assert SAID in swapping.succeed(f"cat {MARK}")
    assert swapping.succeed("swapon --show=NAME --noheadings").count("/swap/swapfile") == 1


def test_coming_back_through_the_disk_prompt_lets_the_lock_go(swapping: QemuMachine) -> None:
    """The PIN typed at the disk is the one prompt: hibernate-unlock lets the lock go."""
    swapping.wait_until_succeeds(f"journalctl --boot --grep '{LET_GO}'", timeout=30)


def test_a_loaded_machine_fits_beside_the_reserve(swapping: QemuMachine) -> None:
    """With memory full of a program's pages and the disk's cache, it still comes back.

    The reserve holds the image to half of memory less twice it, which the load is past, so
    the kernel drops the cache and swaps part of memory, to zram first, before the image
    fits. The program's pages are a file in tmpfs, as compressible as hex.
    """
    swapping.succeed(
        "head -c 32M /dev/urandom | od -An -tx1 -v > /dev/shm/held",
        "sha256sum /dev/shm/held > /run/held.sha256",
        "head -c 128M /dev/urandom > /mnt/disk/cached",
        "sync",
        "cat /mnt/disk/cached > /dev/null",
    )
    hibernate_and_come_back(swapping)
    swapping.succeed("sha256sum --check --quiet /run/held.sha256")
    swapping.succeed("rm /dev/shm/held /mnt/disk/cached")


def test_a_hibernation_with_no_swap_file_says_so(swapping: QemuMachine) -> None:
    """With the swap file gone as the kernel writes, the notice says so and never to retry."""
    swapping.succeed(f"touch {NO_SWAP_FILE}")
    roll_back(swapping, saying="No swap file to hibernate to")
    swapping.succeed(f"rm {NO_SWAP_FILE}", "swapon /swap/swapfile")


def test_a_hibernation_short_of_memory_says_so(swapping: QemuMachine) -> None:
    """With a reserve as large as memory there's no room for the image, and the notice says so.

    The image's own cap is taken away too, or an image smaller than it would need no room
    made.
    """
    image_size = swapping.succeed("cat /sys/power/image_size").strip()
    swapping.succeed(
        "awk '/^MemTotal:/ { print $2 * 1024 }' /proc/meminfo > /sys/power/reserved_size",
        "echo 0 > /sys/power/image_size",
    )
    try:
        roll_back(swapping, saying="Not enough memory to hibernate")
    finally:
        swapping.succeed(
            f"echo {RESERVE} > /sys/power/reserved_size",
            f"echo {image_size} > /sys/power/image_size",
        )


def test_a_boot_after_it_starts_afresh(swapping: QemuMachine) -> None:
    """The image is used once: a machine started again after it boots anew."""
    swapping.crash()
    start_through_the_disk_prompt(swapping)
    swapping.wait_for_unit("default.target")
    swapping.fail(f"test -e {MARK}")


def test_a_hibernation_that_rolls_back_is_rescued(swapping: QemuMachine) -> None:
    """Back from a hibernation with no start between is a rollback, and a rescue follows.

    The hook is run as systemd-sleep runs it around a hibernation the kernel rolled back:
    before, then after, with the TPM's counts unmoved. The machine has no lid, so the
    rescue, ten seconds on and on its own, says so rather than hibernating again, and with
    nothing in the kernel's log, says it didn't finish.
    """
    swapping.succeed(f"{HOOK} pre hibernate", f"{HOOK} post hibernate")
    swapping.wait_until_succeeds(f"journalctl --boot --grep '{SAID_SO}'", timeout=60)
    swapping.wait_until_succeeds(
        f"journalctl --boot --grep '{SAYS}Hibernation didn.t finish'", timeout=60
    )

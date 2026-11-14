"""A machine hibernated to a swap file on btrfs inside LUKS comes back as it was.

hibernate.nix, beside this, runs this through pytest inside the driver (tests/lib/vm_pytest.py), and
its opening comment says how the machine is laid out. The disk is made here, on the first
boot, since a test's empty disk can't be made beforehand, and opened by the TPM once its PIN
is typed at the initrd's prompt on every start after. The tests need no value from Nix but
the reserve, which they repeat.
"""

from __future__ import annotations

import datetime as dt
import re
import subprocess
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
# What hibernate-unlock says of how long a restore took, from the count the initrd kept.
TOOK = (
    r"hibernate-unlock: the image was read and the machine back \d+\.\d s after the disk "
    r"opened, \d+\.\d s after the power button"
)
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
# A slice at the top, as user.slice is, with a hard floor under what it holds, in bytes.
FLOORED = "floored.slice"
FLOOR = 400 * 1024 * 1024
# Whether zswap stores what's swapped out in its pool, in memory, before the swap file.
ZSWAP = "/sys/module/zswap/parameters/enabled"
# How long a hibernation may take to make its image and power off. One that rolls back is
# still on after this, and says why.
GOES_OFF = dt.timedelta(minutes=3)
# Pages zswap may store around a hibernation, as the machine thaws, against the tens of
# thousands swapped out to make room for an image.
STRAY = 2048
# The stand-in lid, and the stand-in battery's charge and charger (hibernate.nix).
LID = "/sys/module/stand_in_lid/parameters/shut"
CHARGE = "/sys/module/test_power/parameters/battery_capacity"
CHARGER = "/sys/module/test_power/parameters/ac_online"
# Below UPower's mark for acting, 5% (modules/power-profiles-daemon/nixos.nix), and how long
# UPower waits once there before it acts, with room over its 20 s.
RUN_OUT = 4
UPOWER_WAITS = dt.timedelta(minutes=1)


def zswap_stored(machine: QemuMachine) -> int:
    """How many pages zswap has stored in its pool since the machine started."""
    return int(machine.succeed("awk '/^zswpout / { print $2 }' /proc/vmstat"))


def start_through_the_disk_prompt(machine: QemuMachine) -> None:
    """Start the machine and type the disk's PIN when the initrd asks for it."""
    machine.start()
    machine.wait_for_console_text(ASKED)
    machine.send_console(f"{PIN}\n")


def hibernate_and_come_back(machine: QemuMachine) -> None:
    """Hibernate, wait for the machine to go off, and start it from its image."""
    machine.execute("systemctl hibernate >&2 &", check_return=False)
    try:
        machine.wait_for_shutdown(timeout=GOES_OFF)
    except subprocess.TimeoutExpired:
        # The driver's shell restarts with the machine, so the console says why.
        short = re.findall(
            r"PM: hibernation: Image allocation is \d+ pages short", machine.get_console_log()
        )
        pytest.fail(f"the hibernation rolled back: {short[-1] if short else 'nothing said'}")
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
    assert machine.succeed(f"cat {ZSWAP}").strip() == "Y"
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


def test_coming_back_says_how_long_the_restore_took(swapping: QemuMachine) -> None:
    """The initrd kept when the restore began, so the hook says how long it took."""
    swapping.wait_until_succeeds(f"journalctl --boot --grep '{TOOK}'", timeout=30)


def test_a_loaded_machine_fits_beside_the_reserve(swapping: QemuMachine) -> None:
    """With memory full of a program's pages and the disk's cache, it still comes back.

    The reserve holds the image to half of memory less twice it, which the load is past, so
    the kernel drops the cache and swaps part of memory out to the swap file before the image
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


def test_memory_under_a_hard_floor_is_swapped_out_for_the_image(swapping: QemuMachine) -> None:
    """What a slice's hard floor guards still makes room for the image, and the floor comes back.

    The kernel takes back nothing under a cgroup's memory.min, whatever is reclaiming, so 300 MiB
    held under one is more than the image's cap of about 213 MiB beside the reserve: without the
    hook setting the floors aside, the hibernation rolls back short of memory. It's zeros, which
    swap marks rather than stores, so what's measured is the floor, not how well the pool packs.
    """
    swapping.succeed(
        "mkdir -p /run/floored",
        "mount -t tmpfs -o size=400m tmpfs /run/floored",
        # tmpfs pages count against the cgroup that wrote them, so the writer stays to hold
        # them there. A transient service's PATH holds none of the system's programs.
        f"systemd-run --slice={FLOORED} --unit=floored --property=MemoryMin={FLOOR}"
        " --setenv=PATH=/run/current-system/sw/bin"
        " sh -c 'head -c 300M /dev/zero > /run/floored/held && touch /run/floored/full;"
        " exec sleep infinity'",
        f"systemctl set-property --runtime {FLOORED} MemoryMin={FLOOR}",
    )
    swapping.wait_until_succeeds("test -e /run/floored/full", timeout=60)
    swapping.succeed("sha256sum /run/floored/held > /run/floored.sha256")
    held = int(swapping.succeed(f"cat /sys/fs/cgroup/{FLOORED}/memory.current"))
    assert held > 300 * 1024 * 1024, f"the floored slice holds only {held} bytes"
    hibernate_and_come_back(swapping)
    swapping.succeed("sha256sum --check --quiet /run/floored.sha256")
    floor = int(swapping.succeed(f"cat /sys/fs/cgroup/{FLOORED}/memory.min"))
    assert floor == FLOOR, f"the floor came back as {floor} bytes"
    swapping.succeed("systemctl stop floored.service", "umount /run/floored")


def test_what_is_swapped_out_for_the_image_goes_to_the_swap_file(swapping: QemuMachine) -> None:
    """What's swapped out to make room for the image doesn't stay in memory.

    300 MiB of random bytes is more than the image's cap of about 213 MiB beside the reserve, so
    they have to reach the swap file. Swapped to zram, which kept them in memory, they stayed in
    the image and it rolled back 58,596 pages short. zswap's pool keeps what it stores in memory
    too, as it is when it doesn't compress, until its shrinker writes it out, so with zswap on
    it rolled back 28,090 pages short in one run of three. The counter of what zswap stored comes
    back in the image, which is made after the room for it, so it shows nothing went to the pool.
    """
    swapping.succeed(
        "mkdir -p /run/unpacked",
        "mount -t tmpfs -o size=400m tmpfs /run/unpacked",
        "head -c 300M /dev/urandom > /run/unpacked/held",
        "sha256sum /run/unpacked/held > /run/unpacked.sha256",
    )
    stored = zswap_stored(swapping)
    hibernate_and_come_back(swapping)
    swapping.succeed("sha256sum --check --quiet /run/unpacked.sha256")
    assert zswap_stored(swapping) - stored < STRAY, "zswap stored what was swapped out for it"
    assert swapping.succeed(f"cat {ZSWAP}").strip() == "Y", "zswap wasn't switched on again"
    swapping.succeed("umount /run/unpacked")


def test_it_says_beforehand_whether_the_image_fits(swapping: QemuMachine) -> None:
    """Asked first, it says a quiet machine's image fits, and with no room, plainly, that not.

    A reserve as large as memory leaves the image no room, and nothing is asked to hibernate.
    """
    said = swapping.succeed("hibernate-unlock fits")
    assert said.startswith("It fits: "), said
    swapping.succeed(
        "awk '/^MemTotal:/ { print $2 * 1024 }' /proc/meminfo > /sys/power/reserved_size"
    )
    # A failure is said on stderr, as every command on the terminal kit says
    # one, so stdout carries nothing to mistake for an answer.
    try:
        status, said = swapping.execute("hibernate-unlock fits 2>&1 >/dev/null")
    finally:
        swapping.succeed(f"echo {RESERVE} > /sys/power/reserved_size")
    assert status == 1, f"it exited {status}"
    assert "there isn't enough memory to hibernate" in said, said
    assert "Nothing was changed." in said, said


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


def test_kept_awake_with_its_lid_shut_a_battery_run_out_hibernates(swapping: QemuMachine) -> None:
    """Kept awake with its lid shut on battery, a battery run out hibernates, keeping its work.

    Keep Awake holds idle and the lid's switch (modules/quickshell/home.nix), the locks held
    here, so a laptop kept awake runs on with its lid shut until its battery runs out. UPower
    hibernates past both; powering off instead would start it afresh, losing the mark.
    """
    swapping.succeed(
        "mkdir -p /run/kept",
        "mountpoint -q /run/kept || mount -t ramfs -o size=1m ramfs /run/kept",
        f"echo '{SAID}' > {MARK}",
        'systemd-run --unit=kept-awake -- "$(command -v systemd-inhibit)"'
        " --what=idle:handle-lid-switch --who='Keep Awake' --why='Kept awake from the launcher'"
        ' --mode=block "$(command -v sleep)" infinity',
    )
    # Held before the lid shuts, or logind would suspend it.
    swapping.wait_until_succeeds(
        "systemd-inhibit --list --no-pager | grep -F 'Keep Awake' | grep -q handle-lid-switch",
        timeout=10,
    )
    # A laptop's graphical target starts UPower; this machine stops at multi-user's.
    swapping.succeed("systemctl start upower.service")
    swapping.wait_until_succeeds("upower --dump | grep -q 'percentage: *50%'", timeout=30)
    swapping.succeed(f"echo 1 > {LID}", f"echo {RUN_OUT} > {CHARGE}")
    try:
        swapping.wait_for_shutdown(timeout=UPOWER_WAITS + GOES_OFF)
    except subprocess.TimeoutExpired:
        _, said = swapping.execute("upower --dump; journalctl --boot --unit=upower --no-pager")
        pytest.fail(f"at {RUN_OUT}% with the lid shut and held, the machine stayed on:\n{said}")
    start_through_the_disk_prompt(swapping)
    # Charged again, so UPower has nothing more to act on.
    swapping.succeed(f"echo 50 > {CHARGE}", f"echo on > {CHARGER}", f"echo 0 > {LID}")
    _, kept = swapping.execute(f"cat {MARK}")
    assert SAID in kept, "it powered off rather than hibernating, so it started afresh"
    swapping.succeed("systemctl stop kept-awake.service")


def test_a_boot_after_it_starts_afresh(swapping: QemuMachine) -> None:
    """The image is used once: a machine started again after it boots anew."""
    swapping.crash()
    start_through_the_disk_prompt(swapping)
    swapping.wait_for_unit("default.target")
    swapping.fail(f"test -e {MARK}")


def test_a_hibernation_that_rolls_back_is_rescued(swapping: QemuMachine) -> None:
    """Back from a hibernation with no start between is a rollback, and a rescue follows.

    The hook is run as systemd-sleep runs it around a hibernation the kernel rolled back:
    before, then after, with the TPM's counts unmoved. The machine has no ACPI lid, its
    stand-in being a switch alone, so the
    rescue, ten seconds on and on its own, says so rather than hibernating again, and with
    nothing in the kernel's log, says it didn't finish.
    """
    swapping.succeed(f"{HOOK} pre hibernate", f"{HOOK} post hibernate")
    swapping.wait_until_succeeds(f"journalctl --boot --grep '{SAID_SO}'", timeout=60)
    swapping.wait_until_succeeds(
        f"journalctl --boot --grep '{SAYS}Hibernation didn.t finish'", timeout=60
    )

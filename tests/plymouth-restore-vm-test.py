"""The boot screen says the machine is being restored while a hibernation's image is read.

tests/plymouth-restore.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py),
and its opening comment says what the test proves. The disk is made on the first boot, since
a test's empty disk can't be made beforehand, and opened by its passphrase at the theme's ask
on every start after.
"""

from __future__ import annotations

import datetime as dt
import re
import subprocess
import tempfile
import time
from dataclasses import dataclass
from pathlib import Path
from typing import TYPE_CHECKING, Any

import pytest
from test_driver.machine import QemuMachine
from test_driver.machine.ocr import perform_ocr_variants_on_screenshot

if TYPE_CHECKING:
    from test_driver.machine import BaseMachine

# The longest a wait may take for what it waits on. A wait ends when it happens, so this is
# only a ceiling, for a host busy with other machines.
READING = dt.timedelta(seconds=300)
# How long a hibernation may take to make its image and power off.
GOES_OFF = dt.timedelta(minutes=3)
# What the kernel says as it starts reading the image, and once it has read it all.
READS = "PM: Image loading progress:"
READ = "PM: Image loading done"
# The theme's words for a restore, as OCR reads them.
RESTORING = r"restoring from\s+hibernation"
# What the hook says once the machine is back.
TOOK = (
    r"hibernate-unlock: the image was read and the machine back \d+\.\d s after the disk "
    r"opened, \d+\.\d s after the power button"
)
# Incompressible bytes held in memory, in MiB, so the image takes seconds to read rather
# than the moment a bare machine's does.
HELD = 600


@dataclass(frozen=True)
class Shot:
    """A photograph of the screen, and how much the console had said when it was taken."""

    path: Path
    said: int


@dataclass
class Restore:
    """A machine started from its image, and the screen photographed as it was read."""

    machine: QemuMachine
    shots: list[Shot]
    # Where the console's log of this start begins.
    began: int

    def while_reading(self) -> list[Shot]:
        """Give the photographs taken while the kernel read the image."""
        log = self.machine.get_console_log()
        return [
            shot
            for shot in self.shots
            if READS in log[self.began : shot.said] and READ not in log[self.began : shot.said]
        ]


def photographs_until(machine: QemuMachine, words: str, since: int) -> list[Shot]:
    """Photograph the screen as fast as QEMU can until the console, from since on, says words.

    Each photograph keeps how far the console had got as it was taken, so the ones taken while
    the image was read can be told apart. Reading one takes longer than the image takes to
    read, so they're read afterwards.
    """
    burst = Path(tempfile.mkdtemp())
    shots: list[Shot] = []
    began = time.monotonic()
    while True:
        said = len(machine.get_console_log())
        if words in machine.get_console_log()[since:said]:
            return shots
        if time.monotonic() - began > READING.total_seconds():
            pytest.fail(f"the console never said {words}")
        # As the driver photographs for its own reading: raw, since its OCR can't read PNG.
        shot = burst / f"{len(shots)}.ppm"
        machine.send_monitor_command(f"screendump {shot}")
        shots.append(Shot(shot, said))


def says_restoring(shot: Shot) -> bool:
    """Whether a photograph shows the theme's words for a restore."""
    return any(re.search(RESTORING, text) for text in perform_ocr_variants_on_screenshot(shot.path))


def kept(machine: QemuMachine, shot: Shot, name: str) -> None:
    """Keep a photograph for a person to judge, under name."""
    with (machine.out_dir / f"{name}.png").open("wb") as png:
        # pnmtopng is on the driver's PATH, which its own screenshots use it from, and the
        # photograph is the test's own.
        subprocess.run(["pnmtopng", shot.path], stdout=png, check=True)  # noqa: S603, S607


@pytest.fixture(scope="module")
def swapping(machine: BaseMachine, values: dict[str, Any]) -> QemuMachine:
    """Give the machine swapping to a file on btrfs inside LUKS, as a laptop does."""
    # Only a machine QEMU runs can be stopped and started again.
    assert isinstance(machine, QemuMachine)
    machine.wait_for_unit("default.target")
    passphrase = values["passphrase"]
    machine.succeed(
        f"echo -n {passphrase} | cryptsetup luksFormat -q --iter-time=1 /dev/vdc -",
        f"echo -n {passphrase} | cryptsetup open /dev/vdc disk -",
        "mkfs.btrfs --quiet /dev/mapper/disk",
        "mkdir -p /mnt/disk /swap",
        "mount /dev/mapper/disk /mnt/disk",
        "btrfs subvolume create /mnt/disk/@swap",
        "mount -o subvol=@swap /dev/mapper/disk /swap",
        "btrfs filesystem mkswapfile --size 3g /swap/swapfile",
        "swapon /swap/swapfile",
    )
    return machine


@pytest.fixture(scope="module")
def restored(swapping: QemuMachine, values: dict[str, Any]) -> Restore:
    """Hibernate the machine, then start it from its image, photographing the screen as it reads.

    The photographs run from the passphrase until the kernel says it has read the image.
    """
    machine = swapping
    machine.succeed(
        "mkdir -p /run/held",
        "mount -t ramfs ramfs /run/held",
        f"head -c {HELD}M /dev/urandom > /run/held/bytes",
    )
    machine.execute("systemctl hibernate >&2 &", check_return=False)
    try:
        machine.wait_for_shutdown(timeout=GOES_OFF)
    except subprocess.TimeoutExpired:
        pytest.fail("the machine never went off to hibernate")
    began = len(machine.get_console_log())
    machine.start()
    machine.wait_for_text("disk is locked", timeout=READING)
    machine.send_chars(f"{values['passphrase']}\n")
    shots = photographs_until(machine, READ, began)
    machine.wait_for_unit("default.target")
    return Restore(machine, shots, began)


def test_the_screen_says_it_restores_while_the_image_is_read(restored: Restore) -> None:
    """The photographs taken while the kernel read the image say the machine is restoring.

    The first, the middle and the last are read, since each reading takes seconds.
    """
    reading = restored.while_reading()
    assert reading, f"none of {len(restored.shots)} photographs was taken as the image was read"
    middle = reading[len(reading) // 2]
    for shot in dict.fromkeys([reading[0], middle, reading[-1]]):
        assert says_restoring(shot), f"photograph {shot.path.name}, taken as it read, doesn't"
    kept(restored.machine, middle, "restoring")


def test_the_machine_is_back_as_it_was(restored: Restore) -> None:
    """What was only in memory is there again once the machine starts from its image."""
    size = int(restored.machine.succeed("stat -c %s /run/held/bytes"))
    assert size == HELD * 1024 * 1024, f"the held bytes came back as {size}"


def test_the_hook_says_how_long_the_restore_took(restored: Restore) -> None:
    """Once the machine is back, the hook says how long the restore took."""
    restored.machine.wait_until_succeeds(
        f"journalctl --boot --grep '{TOOK}'", timeout=int(READING.total_seconds())
    )

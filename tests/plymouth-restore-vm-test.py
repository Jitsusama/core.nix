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
from test_driver.machine.ocr import perform_ocr_on_screenshot

if TYPE_CHECKING:
    from test_driver.machine import BaseMachine

# The longest a wait may take for what it waits on. A wait ends when it happens, so this is
# only a ceiling, for a host busy with other machines.
READING = dt.timedelta(seconds=300)
# How long the ask may take to show: the disk waits two minutes for its passphrase, and the
# boot goes on without it after.
ASKING = dt.timedelta(seconds=90)
# The ask's words. They're read with only the pixels near their colour kept, since over the
# whole screen OCR reads the wallpaper's shapes as letters and loses theirs among them.
ASKED = "disk is locked"
# How near each of a pixel's channels must be to the words' colour to be read as theirs: their
# edges blend into what's behind them, their strokes' middles don't.
NEAR = 48
# How long a hibernation may take to make its image and power off.
GOES_OFF = dt.timedelta(minutes=3)
# What the kernel says once every program is frozen and it's about to read the image, as it
# reads it, and once it has read it all.
LOADING = re.compile(r"PM: Loading (and decompressing )?image data")
READS = "PM: Image loading progress:"
READ = "PM: Image loading done"
# What the kernel says when it finds itself wrong: memory corrupted, as KFENCE finds it, a
# warning or an oops.
AMISS = re.compile(r"\b(BUG|WARNING):|\bOops\b")
# The theme's words for a restore, read as the ask's are. OCR can run the serif's r and n
# together into an m, and has read "hibemation".
RESTORING = r"restoring from\s+hibe(rn|m)ation"
# What the hook says once the machine is back.
TOOK = (
    r"hibernate-unlock: the image was read and the machine back \d+\.\d s after the disk "
    r"opened, \d+\.\d s after the power button"
)
# Incompressible bytes held in memory, in MiB, so the image takes seconds to read rather
# than the moment a bare machine's does.
HELD = 600
# How fast the disk reads, in bytes a second, until the empty track is shot: slow
# enough that the kernel fills none of it in the meantime, which takes a 350th of the image.
CRAWL = 64 * 1024
# The QEMU device of the larger screen, as tests/plymouth-restore.nix names it.
LARGE = "large"
# The least run of a colour that can be the track, in pixels: longer than a letter's stroke
# of that colour, should a picture have one.
TRACK_LEAST = 64


@dataclass(frozen=True)
class Shot:
    """A shot of each screen, and how much the console had said when they were taken."""

    path: Path
    large: Path
    said: int


@dataclass(frozen=True)
class Picture:
    """A shot's pixels, as QEMU writes them: a binary PPM, three bytes a pixel."""

    width: int
    height: int
    pixels: bytes

    @classmethod
    def read(cls, path: Path) -> Picture:
        """Read a shot QEMU wrote."""
        data = path.read_bytes()
        # The header is the magic, the width, the height and the largest value, each followed
        # by one whitespace byte; QEMU writes no comments.
        magic, width, height, largest, pixels = data.split(maxsplit=4)
        assert magic == b"P6", f"{path.name} isn't a binary PPM"
        assert largest == b"255", f"{path.name} has {largest!r} levels a channel"
        return cls(int(width), int(height), pixels)

    def words(self, ink: bytes, path: Path) -> Path:
        """Write the picture as black on white, black only where a pixel is near ink's colour."""
        # Each byte is 1 where every channel so far is near, 0 where one isn't, so the
        # channels are put together a whole picture at a time rather than a pixel.
        inked = -1
        for channel, value in enumerate(ink):
            near = bytes(int(abs(level - value) <= NEAR) for level in range(256))
            inked &= int.from_bytes(self.pixels[channel::3].translate(near), "big")
        grey = inked.to_bytes(self.width * self.height, "big").translate(
            bytes([255, 0]) + bytes(254)
        )
        path.write_bytes(b"P5 %d %d 255\n" % (self.width, self.height) + grey)
        return path

    def run(self, y: int, left: int, width: int) -> bytes:
        """Give width pixels of row y, from left on."""
        start = (y * self.width + left) * 3
        return self.pixels[start : start + width * 3]


@dataclass(frozen=True)
class Track:
    """Where the theme drew the image's empty track on a screen."""

    left: int
    top: int
    width: int
    height: int


def colour(hex_colour: str) -> bytes:
    """Give a theme's #rrggbb as a shot's three bytes."""
    return bytes.fromhex(hex_colour.removeprefix("#"))


def pixel_runs(row: bytes, ink: bytes) -> list[tuple[int, int]]:
    """Give each run of a colour in a row at least TRACK_LEAST long, as its start and end."""
    pixels = [row[at : at + 3] for at in range(0, len(row), 3)]
    runs: list[tuple[int, int]] = []
    start = None
    for x, value in enumerate([*pixels, b""]):
        if value == ink and start is None:
            start = x
        elif value != ink and start is not None:
            if x - start >= TRACK_LEAST:
                runs.append((start, x))
            start = None
    return runs


def found_track(picture: Picture, ink: bytes) -> Track:
    """Find the empty track: the one band of the track's colour more than a row tall."""
    bands: dict[tuple[int, int], list[int]] = {}
    for y in range(picture.height):
        row = picture.run(y, 0, picture.width)
        if ink not in row:
            continue
        for run in pixel_runs(row, ink):
            bands.setdefault(run, []).append(y)
    tracks = [
        Track(left, rows[0], right - left, len(rows))
        for (left, right), rows in bands.items()
        if len(rows) > 1 and rows == list(range(rows[0], rows[0] + len(rows)))
    ]
    assert len(tracks) == 1, f"the screen shows {len(tracks)} bands of the track's colour"
    return tracks[0]


def filled(picture: Picture, track: Track, bar: bytes, ink: bytes) -> int:
    """Give how far the kernel's bar fills the track, failing unless it lies exactly on it.

    Every row of the track is the bar from its left end, then the track to its right end,
    each row the same, and nothing of the bar's colour touches the track from outside.
    """
    reached = set()
    for y in range(track.top, track.top + track.height):
        row = picture.run(y, track.left, track.width)
        length = 0
        while row[length * 3 : length * 3 + 3] == bar:
            length += 1
        assert row[length * 3 :] == ink * (track.width - length), (
            f"row {y} of the track isn't the bar, then the track"
        )
        reached.add(length)
        for x in (track.left - 1, track.left + track.width):
            assert picture.run(y, x, 1) != bar, f"the bar reaches past the track at {x}, {y}"
    assert len(reached) == 1, f"the bar's rows end apart, at {sorted(reached)}"
    for y in (track.top - 1, track.top + track.height):
        row = picture.run(y, track.left, track.width)
        assert bar not in [row[at : at + 3] for at in range(0, len(row), 3)], (
            f"the bar's colour is on row {y}, outside the track"
        )
    return reached.pop()


@dataclass
class Restore:
    """A machine started from its image, and the screens shot as it was read."""

    machine: QemuMachine
    # Both screens as the kernel began to read the image, once every program was frozen.
    before: Shot
    shots: list[Shot]
    # Both screens the moment the kernel said it had read the image, before the image's own
    # kernel took them.
    after: Shot

    def while_reading(self) -> list[Shot]:
        """Give the shots taken while the kernel read the image."""
        log = self.machine.get_console_log()
        return [
            shot
            for shot in self.shots
            if READS in log[: shot.said] and READ not in log[: shot.said]
        ]


def shoot(machine: QemuMachine, burst: Path, name: str) -> Shot:
    """Shoot both screens.

    As the driver shoots for its own reading: raw, since its OCR can't read PNG.
    """
    said = len(machine.get_console_log())
    shot = Shot(burst / f"{name}.ppm", burst / f"{name}-large.ppm", said)
    machine.send_monitor_command(f"screendump {shot.path}")
    machine.send_monitor_command(f"screendump {shot.large} {LARGE}")
    return shot


def shots_until(machine: QemuMachine, burst: Path, words: str) -> list[Shot]:
    """Shoot the screens as fast as QEMU can until the console says words.

    The driver's console log starts again with each start, so what it says is this start's.
    Each shot keeps how far the console had got as it was taken, so the ones taken while
    the image was read can be told apart. Reading one takes longer than the image takes to
    read, so they're read afterwards.
    """
    shots: list[Shot] = []
    began = time.monotonic()
    while True:
        if words in machine.get_console_log():
            return shots
        if time.monotonic() - began > READING.total_seconds():
            pytest.fail(f"the console never said {words}")
        shots.append(shoot(machine, burst, str(len(shots))))


def until_said(machine: QemuMachine, words: re.Pattern[str]) -> None:
    """Wait until the console says words."""
    began = time.monotonic()
    while not words.search(machine.get_console_log()):
        if time.monotonic() - began > READING.total_seconds():
            pytest.fail(f"the console never said {words.pattern}")
        time.sleep(0.1)


def the_disk(machine: QemuMachine) -> str:
    """Give the QEMU drive of the disk the image is on, the test's first empty disk."""
    said = machine.send_monitor_command("info block")
    for line in said.splitlines():
        if "empty0.qcow2" in line:
            return line.split()[0]
    pytest.fail(f"QEMU named no drive on empty0.qcow2: {said}")


def read_in(path: Path, ink: bytes) -> str:
    """Give the words a shot shows in ink's colour, as OCR reads them."""
    return perform_ocr_on_screenshot(Picture.read(path).words(ink, path.with_suffix(".pgm")))


def says(path: Path, words: str, ink: bytes) -> bool:
    """Whether a shot shows words in ink's colour."""
    return re.search(words, read_in(path, ink)) is not None


def shown(machine: QemuMachine, words: str, ink: bytes) -> bool:
    """Whether the first screen comes to show words in ink's colour while the ask may take."""
    path = Path(tempfile.mkdtemp()) / "asking.ppm"
    began = time.monotonic()
    while time.monotonic() - began < ASKING.total_seconds():
        machine.send_monitor_command(f"screendump {path}")
        if says(path, words, ink):
            return True
    return False


def kept(machine: QemuMachine, path: Path, name: str) -> None:
    """Keep a shot for a person to judge, under name."""
    with (machine.out_dir / f"{name}.png").open("wb") as png:
        # pnmtopng is on the driver's PATH, which its own screenshots use it from, and the
        # shot is the test's own.
        subprocess.run(["pnmtopng", path], stdout=png, check=True)  # noqa: S603, S607


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
    """Hibernate the machine, then start it from its image, shooting the screens as it reads.

    The disk crawls from the passphrase until the kernel has frozen every program and begins
    to read, when both screens are shot; then it's let go, and the shots run
    until the kernel says it has read the image.
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
    machine.start()
    if not shown(machine, ASKED, colour(values["words"])):
        # What the screens showed instead, while the disk still waits, for whoever reads
        # the failure.
        shot = shoot(machine, Path(tempfile.mkdtemp()), "unasked")
        kept(machine, shot.path, "unasked")
        kept(machine, shot.large, "unasked-large")
        pytest.fail(f"the screen never said {ASKED}")
    disk = the_disk(machine)
    machine.send_monitor_command(f"block_set_io_throttle {disk} 0 {CRAWL} 0 0 0 0")
    machine.send_chars(f"{values['passphrase']}\n")
    until_said(machine, LOADING)
    burst = Path(tempfile.mkdtemp())
    before = shoot(machine, burst, "before")
    machine.send_monitor_command(f"block_set_io_throttle {disk} 0 0 0 0 0 0")
    shots = shots_until(machine, burst, READ)
    after = shoot(machine, burst, "after")
    machine.wait_for_unit("default.target")
    return Restore(machine, before, shots, after)


def test_the_screen_says_it_restores_while_the_image_is_read(
    restored: Restore, values: dict[str, Any]
) -> None:
    """The shots taken while the kernel read the image say the machine is restoring.

    The first, the middle and the last are read, since each reading takes seconds.
    """
    reading = restored.while_reading()
    assert reading, f"none of {len(restored.shots)} shots was taken as the image was read"
    middle = reading[len(reading) // 2]
    for shot in dict.fromkeys([reading[0], middle, reading[-1]]):
        read = read_in(shot.path, colour(values["words"]))
        assert re.search(RESTORING, read), (
            f"shot {shot.path.name}, taken as it read, says only {read!r}"
        )
    kept(restored.machine, middle.path, "restoring")


def test_each_screen_shows_the_empty_track_as_the_read_begins(
    restored: Restore, values: dict[str, Any]
) -> None:
    """Once every program is frozen, each screen shows the theme's track, none of it filled."""
    ink, bar = colour(values["track"]), colour(values["bar"])
    for path in (restored.before.path, restored.before.large):
        picture = Picture.read(path)
        reached = filled(picture, found_track(picture, ink), bar, ink)
        assert reached == 0, f"{path.name}: the bar had filled {reached} pixels before the read"
    kept(restored.machine, restored.before.path, "restoring-track")
    kept(restored.machine, restored.before.large, "restoring-track-large")


def test_the_kernel_fills_each_track_as_it_reads(restored: Restore, values: dict[str, Any]) -> None:
    """While the image is read, the kernel's bar lies exactly on each screen's track.

    Some shot shows it partly filled, and from one shot to the next it never goes
    back.
    """
    ink, bar = colour(values["track"]), colour(values["bar"])
    reading = restored.while_reading()
    for screen in ("path", "large"):
        track = found_track(Picture.read(getattr(restored.before, screen)), ink)
        reached = [filled(Picture.read(getattr(shot, screen)), track, bar, ink) for shot in reading]
        assert reached == sorted(reached), f"the bar went back on {screen}: {reached}"
        partly = [
            shot for shot, length in zip(reading, reached, strict=True) if 0 < length < track.width
        ]
        assert partly, f"no shot of {screen} shows the bar partly filled: {reached}"
        kept(
            restored.machine,
            getattr(partly[len(partly) // 2], screen),
            "restoring-bar" if screen == "path" else "restoring-bar-large",
        )


def test_each_track_is_full_once_the_image_is_read(
    restored: Restore, values: dict[str, Any]
) -> None:
    """Once the kernel has read the image, each screen's track stays full until it goes."""
    ink, bar = colour(values["track"]), colour(values["bar"])
    for screen in ("path", "large"):
        track = found_track(Picture.read(getattr(restored.before, screen)), ink)
        reached = filled(Picture.read(getattr(restored.after, screen)), track, bar, ink)
        assert reached == track.width, f"{screen}: the bar reached {reached} of {track.width}"
    kept(restored.machine, restored.after.path, "restoring-full")
    kept(restored.machine, restored.after.large, "restoring-full-large")


def test_the_machine_is_back_as_it_was(restored: Restore) -> None:
    """What was only in memory is there again once the machine starts from its image."""
    size = int(restored.machine.succeed("stat -c %s /run/held/bytes"))
    assert size == HELD * 1024 * 1024, f"the held bytes came back as {size}"


def test_the_kernel_finds_nothing_amiss_as_it_draws(restored: Restore) -> None:
    """Drawing through the panic screen's hooks while the machine runs corrupts nothing."""
    amiss = [line for line in restored.machine.get_console_log().splitlines() if AMISS.search(line)]
    assert not amiss, "the kernel said:\n" + "\n".join(amiss[:10])


def test_the_hook_says_how_long_the_restore_took(restored: Restore) -> None:
    """Once the machine is back, the hook says how long the restore took."""
    restored.machine.wait_until_succeeds(
        f"journalctl --boot --grep '{TOOK}'", timeout=int(READING.total_seconds())
    )

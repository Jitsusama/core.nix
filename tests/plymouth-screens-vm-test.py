"""The splash's ask, shown whole on two screens of different sizes, and opening the disk.

tests/plymouth-screens.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py),
and its opening comment says why the bottom left's line is the one read. Each test asks for
the state it starts from as a fixture: the machine installed onto its encrypted disk and
booted to the splash asking for the passphrase. A fixture's work is done once a run, however
many tests ask for it, so any test runs alone with `-k`. The passphrase, the screens' sizes,
the encrypted system and the programs that read the pictures are plymouth-screens.nix's,
given as the `values` fixture.
"""

from __future__ import annotations

import subprocess
from dataclasses import dataclass
from datetime import timedelta
from typing import TYPE_CHECKING, Any, cast

import pytest
from test_driver.machine import retry

if TYPE_CHECKING:
    from pathlib import Path

    from test_driver.machine import BaseMachine, QemuMachine

# How long the boot may take to ask, and the splash to draw what it asks.
BOOTING = timedelta(seconds=120)
DRAWING = timedelta(seconds=60)


@dataclass(frozen=True)
class Screen:
    """A screen's size, in pixels."""

    width: int
    height: int


@dataclass(frozen=True)
class Splash:
    """The machine at the splash's ask, and what reads its screens."""

    machine: QemuMachine
    passphrase: str
    small: Screen
    large: Screen
    # ImageMagick and Tesseract, which run beside the driver rather than on the machine.
    magick: str
    tesseract: str

    def photographed(self, name: str, device: str | None = None) -> Path:
        """Photograph a screen, the first as the driver does and another by its device's name."""
        path = self.machine.out_dir / f"{name}.png"
        if device is None:
            self.machine.screenshot(name)
            return path
        # As the driver photographs the first: a PPM, which every QEMU
        # writes, turned into a PNG.
        raw = path.with_suffix(".ppm")
        said = self.machine.send_monitor_command(f"screendump {raw} {device}")
        assert raw.exists(), f"QEMU wrote no picture of {device}: {said}"
        # The program and every argument are the test's own, from Nix and this file.
        subprocess.run([self.magick, raw, path], check=True)  # noqa: S603
        raw.unlink()
        return path

    def read(self, path: Path) -> str:
        """Read the words on a picture."""
        # The program and every argument are the test's own, from Nix and this file.
        return subprocess.check_output([self.tesseract, path, "-"], text=True)  # noqa: S603

    def bottom_left(self, path: Path, screen: Screen) -> str:
        """Read the words in a screen's bottom left quarter, enlarged for OCR.

        OCR can't read the line at its own small size.
        """
        crop = f"{screen.width // 2}x{screen.height // 4}+0+{screen.height - screen.height // 4}"
        enlarged = path.with_name(f"{path.stem}-bottom-left.png")
        # The programs and every argument are the test's own, from Nix and this file.
        subprocess.run(  # noqa: S603
            [
                self.magick,
                path,
                "-crop",
                crop,
                "+repage",
                "-resize",
                "300%",
                "-colorspace",
                "Gray",
                "-negate",
                enlarged,
            ],
            check=True,
        )
        return self.read(enlarged)


@pytest.fixture(scope="module")
def splash(machine: BaseMachine, values: dict[str, Any]) -> Splash:
    """Give the machine installed onto its encrypted disk, booted to the splash's ask."""
    # The test's one machine is QEMU's, which takes keys and screenshots.
    qemu = cast("QemuMachine", machine)
    passphrase: str = values["passphrase"]
    qemu.wait_for_unit("multi-user.target")
    qemu.succeed(
        f"echo -n {passphrase} | cryptsetup luksFormat -q --iter-time=1 /dev/vdb -",
        f"echo -n {passphrase} | cryptsetup luksOpen -q /dev/vdb root",
        "mkfs.ext4 -q /dev/mapper/root",
        # The system under test isn't one of the first boot's own, so it's made the
        # profile's newest generation, as nix-env would, for the boot loader to give it
        # an entry.
        f"ln -sfn {values['encrypted']} /nix/var/nix/profiles/system-2-link",
        "ln -sfn system-2-link /nix/var/nix/profiles/system",
        f"{values['encrypted']}/bin/switch-to-configuration boot",
        "sync",
    )
    qemu.crash()
    qemu.start()
    qemu.wait_for_text("disk is locked", timeout=BOOTING)
    return Splash(
        machine=qemu,
        passphrase=passphrase,
        small=Screen(values["small"]["width"], values["small"]["height"]),
        large=Screen(values["large"]["width"], values["large"]["height"]),
        magick=values["magick"],
        tesseract=values["tesseract"],
    )


def test_the_smaller_screen_shows_the_whole_ask(splash: Splash) -> None:
    """The smaller screen shows the whole ask, its edges too."""

    def whole_on_small(_: bool) -> bool:
        """Say whether the smaller screen's bottom left shows the line saying what's booting."""
        return "Linux" in splash.bottom_left(splash.photographed("small"), splash.small)

    retry(whole_on_small, timeout=DRAWING)


def test_the_larger_screen_shows_the_whole_ask_too(splash: Splash) -> None:
    """The larger screen shows the whole ask too."""

    def whole_on_large(_: bool) -> bool:
        """Say whether the larger screen shows the ask and its bottom left's line."""
        shot = splash.photographed("large", "large")
        return "Linux" in splash.bottom_left(shot, splash.large) and "locked" in splash.read(shot)

    retry(whole_on_large, timeout=DRAWING)


def test_the_passphrase_still_opens_the_disk(splash: Splash) -> None:
    """The passphrase still opens the disk."""
    splash.machine.send_chars(f"{splash.passphrase}\n")
    splash.machine.wait_for_unit("multi-user.target")
    assert "/dev/mapper/root on / " in splash.machine.succeed("mount")

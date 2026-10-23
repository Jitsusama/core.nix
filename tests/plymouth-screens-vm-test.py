"""The splash's ask, shown whole on two screens of different sizes, and opening the disk.

tests/plymouth-screens.nix runs this, and its opening comment says why the bottom left's line
is the one read. run-subtests cuts the script into subtests at each top-level `with subtest(`,
so every subtest stays at the top level under its own name, and what sits outside them, the
install onto the encrypted disk, the boot and the helpers, runs whichever subtests are chosen.
The values it needs from Nix, the passphrase, the screens' sizes, the encrypted system and the
programs that read the pictures, come in one JSON file, its path put in place of the marker.
"""

import json
import subprocess
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from driver_names import machine, retry, subtest, t

values = json.loads(Path("@values@").read_text())
passphrase: str = values["passphrase"]
encrypted: str = values["encrypted"]
# ImageMagick and Tesseract, which run beside the driver rather than on the machine.
magick: str = values["magick"]
tesseract: str = values["tesseract"]
small_width: int = values["small"]["width"]
small_height: int = values["small"]["height"]
large_width: int = values["large"]["width"]
large_height: int = values["large"]["height"]


def photographed(name: str, device: str | None = None) -> Path:
    """Photograph a screen, the first as the driver does and another by its device's name."""
    path = machine.out_dir / f"{name}.png"
    if device is None:
        machine.screenshot(name)
        return path
    # As the driver photographs the first: a PPM, which every QEMU
    # writes, turned into a PNG.
    raw = path.with_suffix(".ppm")
    said = machine.send_monitor_command(f"screendump {raw} {device}")
    if not raw.exists():
        message = f"QEMU wrote no picture of {device}: {said}"
        raise RuntimeError(message)
    # The program and every argument are the test's own, from Nix and this file.
    subprocess.run([magick, raw, path], check=True)  # noqa: S603
    raw.unlink()
    return path


def bottom_left(path: Path, width: int, height: int) -> str:
    """Read the words in a screen's bottom left quarter, enlarged for OCR.

    OCR can't read the line at its own small size.
    """
    crop = f"{width // 2}x{height // 4}+0+{height - height // 4}"
    enlarged = path.with_name(f"{path.stem}-bottom-left.png")
    # The programs and every argument are the test's own, from Nix and this file.
    subprocess.run(  # noqa: S603
        [
            magick,
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
    return subprocess.check_output([tesseract, enlarged, "-"], text=True)  # noqa: S603


machine.wait_for_unit("multi-user.target")
machine.succeed(
    f"echo -n {passphrase} | cryptsetup luksFormat -q --iter-time=1 /dev/vdb -",
    f"echo -n {passphrase} | cryptsetup luksOpen -q /dev/vdb root",
    "mkfs.ext4 -q /dev/mapper/root",
    f"{encrypted}/bin/switch-to-configuration boot",
    "sync",
)
machine.crash()
machine.start()
machine.wait_for_text("disk is locked", timeout=120)

with subtest("the smaller screen shows the whole ask, its edges too"):

    def whole_on_small(_: bool) -> bool:
        """Say whether the smaller screen's bottom left shows the line saying what's booting."""
        said = bottom_left(photographed("small"), small_width, small_height)
        return "Linux" in said

    retry(whole_on_small, 60)

with subtest("the larger screen shows the whole ask too"):

    def whole_on_large(_: bool) -> bool:
        """Say whether the larger screen shows the ask and its bottom left's line."""
        shot = photographed("large", "large")
        said = bottom_left(shot, large_width, large_height)
        # The program and every argument are the test's own, from Nix and this file.
        text = subprocess.check_output([tesseract, shot, "-"], text=True)  # noqa: S603
        return "Linux" in said and "locked" in text

    retry(whole_on_large, 60)

with subtest("the passphrase still opens the disk"):
    machine.send_chars(f"{passphrase}\n")
    machine.wait_for_unit("multi-user.target")
    t.assertIn("/dev/mapper/root on / ", machine.succeed("mount"))

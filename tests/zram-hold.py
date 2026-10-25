"""Hold the compressed swap's device open, as udev's probe does, until a reset finds it busy.

tests/zram.nix installs this as hold-zram for tests/zram-vm-test.py. It opens /dev/zram0, says
so by making /run/holding-zram, and lets go the moment the kernel logs the device busy, well
inside the time a reset allows, so only a reset that tries again succeeds.
"""

import os
import subprocess
from pathlib import Path

DEVICE = "/dev/zram0"
HOLDING = Path("/run/holding-zram")


def main() -> None:
    """Hold the device until the kernel says a reset found it busy, then let go."""
    device = os.open(DEVICE, os.O_RDONLY)
    journal = subprocess.Popen(  # no shell, and every argument is ours.
        ["journalctl", "-k", "-f", "-n", "1", "-o", "cat"],  # noqa: S607 # the machine's own.
        stdout=subprocess.PIPE,
        text=True,
    )
    lines = journal.stdout
    if lines is None:
        message = "journalctl's output wasn't piped"
        raise RuntimeError(message)
    # The last line already logged, so the journal is followed from here.
    lines.readline()
    HOLDING.touch()
    for line in lines:
        if "zram0 is busy" in line:
            break
    os.close(device)
    journal.kill()


if __name__ == "__main__":
    main()

"""Stand in for the IR camera: send a known pattern to a loopback over and over, as it would.

    stand-in-camera LOOPBACK

It says when the first frame is out by making /run/stand-in-camera, and holds the loopback
open but sends nothing while /run/stand-in-paused exists, as a relay waiting for its next
reader does. The pattern is 10-bit, sent as Y16, since v4l2loopback sizes Y10 frames at ten
bits a pixel rather than the sixteen they're stored in. The frames' size is in the test's
values file.
"""

import fcntl
import json
import os
import struct
import sys
import time
from pathlib import Path

SIZE = json.loads(Path("@values@").read_text())
WIDTH, HEIGHT = SIZE["width"], SIZE["height"]
# V4L2's request to set a format, an output's buffer type, and Y16's four letters.
VIDIOC_S_FMT = 0xC0D05605
OUTPUT = 2
Y16 = 0x20363159
# How long it waits between looks at whether it's still paused, in seconds.
PAUSED = 0.05


def pattern() -> bytes:
    """Give one frame of the pattern: 10-bit values, shifted to the top of 16 bits."""
    return b"".join(
        struct.pack("<H", (x * 3 + y * 5) % 1024 << 6) for y in range(HEIGHT) for x in range(WIDTH)
    )


def main(argv: list[str]) -> None:
    """Set the loopback's format, then send the pattern until stopped."""
    request = bytearray(208)
    struct.pack_into("<I", request, 0, OUTPUT)
    struct.pack_into("<6I", request, 8, WIDTH, HEIGHT, Y16, 1, WIDTH * 2, WIDTH * HEIGHT * 2)
    device = os.open(argv[0], os.O_RDWR)
    fcntl.ioctl(device, VIDIOC_S_FMT, request)
    frame = pattern()
    os.write(device, frame)
    Path("/run/stand-in-camera").touch()
    paused = Path("/run/stand-in-paused")
    while True:
        if paused.exists():
            time.sleep(PAUSED)
        else:
            os.write(device, frame)


if __name__ == "__main__":
    main(sys.argv[1:])

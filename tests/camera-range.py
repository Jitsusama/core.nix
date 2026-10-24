"""Make a full-range test picture for the camera's relay, and check what the relay made of it.

    camera-range ramp RAMP
    camera-range compare RAMP RELAYED WRITTEN

The picture is one 1920 by 1080 NV12 frame: brightness rising left to right, and colour
rising across and down. The relay turns the picture around and must squeeze it from full
range into limited range exactly (RELAYED), and its output side must pass that on unchanged
(WRITTEN).
"""

import sys
from pathlib import Path

WIDTH, HEIGHT = 1920, 1080
# Limited range's brightness spans 16 to 235, and its colour 16 to 240 about 128.
FLOOR, BRIGHTNESS_SPAN, CENTRE, COLOUR_SPAN = 16, 219, 128, 224


def ramp() -> bytes:
    """Give the full-range frame: brightness, then interleaved colour at half size."""
    brightness = bytes(x * 256 // WIDTH for x in range(WIDTH)) * HEIGHT
    colour = bytes(
        value
        for row in range(HEIGHT // 2)
        for x in range(WIDTH // 2)
        for value in (x * 256 // (WIDTH // 2), row * 256 // (HEIGHT // 2))
    )
    return brightness + colour


def limited(frame: bytes) -> bytes:
    """Give the frame turned around and squeezed into limited range, as the relay should."""
    brightness = frame[: WIDTH * HEIGHT][::-1]
    colour = frame[WIDTH * HEIGHT :]
    pairs = [colour[i : i + 2] for i in range(0, len(colour), 2)][::-1]
    return bytes(round(FLOOR + value * BRIGHTNESS_SPAN / 255) for value in brightness) + bytes(
        round(CENTRE + (value - CENTRE) * COLOUR_SPAN / 255) for pair in pairs for value in pair
    )


def main(argv: list[str]) -> None:
    """Write the ramp, or compare the relay's frames with what it should have made."""
    if argv[0] == "ramp":
        Path(argv[1]).write_bytes(ramp())
        return
    frame, relayed, written = (Path(name).read_bytes() for name in argv[1:4])
    if relayed != limited(frame):
        sys.exit("The relay doesn't squeeze full range into limited range exactly.")
    if written != relayed:
        sys.exit("The relay's output side changes the frames it's given.")


if __name__ == "__main__":
    main(sys.argv[1:])

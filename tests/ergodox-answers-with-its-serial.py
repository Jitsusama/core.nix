"""Check the board's firmware answers with the build's own serial, not Oryx's.

    ergodox-answers-with-its-serial IMAGE SERIAL

IMAGE is the firmware as raw bytes, and SERIAL the file holding the build's serial. USB
descriptors hold strings as UTF-16, little-endian, so that's how each is looked for.
"""

import sys
from pathlib import Path

# The serial Oryx's own builds answer with.
ORYX = "omjyr"


def main(argv: list[str]) -> None:
    """Fail, saying why, unless the image holds the build's serial and not Oryx's."""
    image = Path(argv[0]).read_bytes()
    serial = Path(argv[1]).read_text().strip()
    if serial.encode("utf-16-le") not in image:
        sys.exit(f"The board wouldn't answer with the build's serial, {serial}.")
    if ORYX.encode("utf-16-le") in image:
        sys.exit("The board would still answer with Oryx's serial.")


if __name__ == "__main__":
    main(sys.argv[1:])

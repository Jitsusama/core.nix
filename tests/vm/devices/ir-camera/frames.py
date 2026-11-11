"""Name each frame in a file a reader saved: the camera's pattern in grey, blank, or other.

    frames FILE

It prints the names on one line, in order. The frames' size is in the test's values file.
"""

import json
import sys
from pathlib import Path

SIZE = json.loads(Path("@values@").read_text())
WIDTH, HEIGHT = SIZE["width"], SIZE["height"]


def main(argv: list[str]) -> None:
    """Print what each frame is, or fail if the file holds part of a frame."""
    size = WIDTH * HEIGHT
    pattern = bytes(((x * 3 + y * 5) % 1024) >> 2 for y in range(HEIGHT) for x in range(WIDTH))
    data = Path(argv[0]).read_bytes()
    if len(data) % size:
        sys.exit(f"{len(data)} bytes isn't a whole number of frames")
    names = []
    for start in range(0, len(data), size):
        frame = data[start : start + size]
        if frame == pattern:
            names.append("pattern")
        elif not any(frame):
            names.append("blank")
        else:
            names.append("other")
    print(" ".join(names))


if __name__ == "__main__":
    main(sys.argv[1:])

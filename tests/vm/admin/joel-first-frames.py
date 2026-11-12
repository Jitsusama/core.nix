"""A stand-in for Joel's shell: draws a frame every 16 ms and says how late each came.

usage: joel-first-frames.py HOLD SECONDS RESULT

It holds HOLD MiB of its own, as the shell holds its scene, and for each frame
allocates 4 MiB afresh, as drawing one does, and touches a page in every MiB
it holds. A frame that has to wait on memory, for the kernel to reclaim a page
for it or to read one back from swap, comes late. After SECONDS it writes the
frames' lateness to RESULT as JSON: how many it drew, and the worst and the
99th percentile in milliseconds.
"""

import json
import mmap
import os
import sys
import time
from pathlib import Path

FRAME = 1 / 60
DRAWN = 4 << 20
PAGE = 4096

hold = int(sys.argv[1]) << 20
seconds = float(sys.argv[2])
result = Path(sys.argv[3])

scene = mmap.mmap(-1, hold, flags=mmap.MAP_PRIVATE | mmap.MAP_ANONYMOUS)
for at in range(0, hold, 1 << 20):
    scene[at : at + (1 << 20)] = os.urandom(1 << 20)

late: list[float] = []
start = time.monotonic()
due = start
while due - start < seconds:
    now = time.monotonic()
    if now < due:
        time.sleep(due - now)
    late.append(max(0.0, time.monotonic() - due) * 1000)
    # Fresh memory each frame, a page of it written at a time, as a frame's
    # buffers are; it goes back as the next is made.
    drawn = mmap.mmap(-1, DRAWN, flags=mmap.MAP_PRIVATE | mmap.MAP_ANONYMOUS)
    for at in range(0, DRAWN, PAGE):
        drawn[at] = 1
    drawn.close()
    for at in range(0, hold, 1 << 20):
        scene[at] ^= 1
    due += FRAME

late.sort()
written = result.with_name(result.name + ".writing")
written.write_text(
    json.dumps(
        {
            "frames": len(late),
            "worst": late[-1],
            "p99": late[int(len(late) * 0.99) - 1],
        }
    )
)
written.rename(result)

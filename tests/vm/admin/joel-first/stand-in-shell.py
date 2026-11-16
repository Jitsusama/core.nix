"""A stand-in for Joel's shell: draws a frame every 16 ms and says how late each came.

usage: stand-in-shell.py HOLD SECONDS RESULT

It holds HOLD MiB of its own, as the shell holds its scene, and for each frame
allocates 4 MiB afresh, as drawing one does, and touches a page in every MiB
it holds. A frame that has to wait on memory, for the kernel to reclaim a page
for it or to read one back from swap, comes late. After SECONDS it writes the
frames' lateness to RESULT as JSON: how many it drew, and the worst and the
99th percentile in milliseconds.

For the worst frame it also says where the time before it went, from the
kernel's own count for this process: how long it ran, how long it waited
for a processor once ready, and how many pages it faulted in, from disk or
zswap. On 16 November the worst frame did not follow how long the whole
session waited on a processor, so this tells a frame held off a processor
from one that ran long in its own faults.
"""

import json
import mmap
import os
import resource
import sys
import time
from pathlib import Path

FRAME = 1 / 60
DRAWN = 4 << 20
PAGE = 4096

hold = int(sys.argv[1]) << 20
seconds = float(sys.argv[2])
result = Path(sys.argv[3])


def spent() -> tuple[int, int, int, int]:
    """Give this process's time run and waited for a processor, in ns, and its faults."""
    ran, waited, _ = (int(n) for n in Path("/proc/self/schedstat").read_text().split())
    used = resource.getrusage(resource.RUSAGE_SELF)
    return ran, waited, used.ru_minflt, used.ru_majflt


scene = mmap.mmap(-1, hold, flags=mmap.MAP_PRIVATE | mmap.MAP_ANONYMOUS)
for at in range(0, hold, 1 << 20):
    scene[at : at + (1 << 20)] = os.urandom(1 << 20)

late: list[float] = []
# The worst frame's lateness, and what the process spent between the frame
# before it and it: ran, waited for a processor (both in ms) and faulted.
worst = (0.0, 0.0, 0.0, 0, 0)
start = time.monotonic()
due = start
before = spent()
while due - start < seconds:
    now = time.monotonic()
    if now < due:
        time.sleep(due - now)
    this = max(0.0, time.monotonic() - due) * 1000
    late.append(this)
    after = spent()
    if this >= worst[0]:
        ran, waited, minor, major = (b - a for a, b in zip(before, after, strict=True))
        worst = (this, ran / 1e6, waited / 1e6, minor, major)
    before = after
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
            "worst_ran": worst[1],
            "worst_waited": worst[2],
            "worst_faults": worst[3],
            "worst_faults_from_swap": worst[4],
        }
    )
)
written.rename(result)

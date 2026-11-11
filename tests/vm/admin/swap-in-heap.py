"""A stand-in for a pi's heap: fills SIZE MiB with distinct pages, then waits.

usage: swap-in-heap.py SIZE PIDFILE

It writes its pid to PIDFILE once full, so the swap-in VM test knows when
there is a heap to push to swap.
"""

import mmap
import os
import signal
import sys
from pathlib import Path

size = int(sys.argv[1]) << 20
heap = mmap.mmap(-1, size, flags=mmap.MAP_PRIVATE | mmap.MAP_ANONYMOUS)
for at in range(0, size, 1 << 20):
    heap[at : at + (1 << 20)] = os.urandom(1 << 20)
said = Path(sys.argv[2])
writing = said.with_name(said.name + ".writing")
writing.write_text(str(os.getpid()))
writing.rename(said)
signal.pause()

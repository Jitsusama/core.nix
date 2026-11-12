"""A stand-in for agents' work: holds SIZE MiB of its own and keeps using it.

usage: joel-first-work.py SIZE EVERY SAID

It fills SIZE MiB with distinct pages, which no compressor shrinks, writes its pid to SAID once
full, and then writes a byte in every page of it each EVERY seconds, as a session's garbage
collector or a fetch's indexing walks what it holds. Memory that's used again can't be put
out to swap quietly: what the kernel takes, the work reads back at once.
"""

import mmap
import os
import sys
import time
from pathlib import Path

PAGE = 4096

size = int(sys.argv[1]) << 20
every = float(sys.argv[2])
said = Path(sys.argv[3])

held = mmap.mmap(-1, size, flags=mmap.MAP_PRIVATE | mmap.MAP_ANONYMOUS)
for at in range(0, size, 1 << 20):
    held[at : at + (1 << 20)] = os.urandom(1 << 20)
writing = said.with_name(said.name + ".writing")
writing.write_text(str(os.getpid()))
writing.rename(said)
while True:
    started = time.monotonic()
    for at in range(0, size, PAGE):
        held[at] ^= 1
    time.sleep(max(0.0, every - (time.monotonic() - started)))

"""Stand in for niri's main thread: idle in epoll, then asleep outside it, over and over.

    niri --session

Waiting in epoll is niri idle between frames, which the trace leaves out; the sleep stands in
for niri held off the CPU elsewhere, which the trace says.
"""

import select
import time

poll = select.epoll()
while True:
    poll.poll(0.05)
    time.sleep(0.05)

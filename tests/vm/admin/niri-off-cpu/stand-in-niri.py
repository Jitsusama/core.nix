"""Stand in for niri's main thread: waiting in epoll for an event, then asleep outside it.

    niri --session

A thread named waker writes an eventfd 50 ms into each wait, which wakes the main thread from epoll,
as a client's commit or the screen's page flip wakes niri's. The main thread then sleeps 50 ms
outside epoll, standing in for niri held off the CPU elsewhere.
"""

import os
import select
import threading
import time
from pathlib import Path

PERIOD = 0.05
event = os.eventfd(0)
waiting = threading.Event()


def wake() -> None:
    """Write the eventfd a period into each wait, under a name the trace can say."""
    Path(f"/proc/self/task/{threading.get_native_id()}/comm").write_text("waker")
    while True:
        waiting.wait()
        waiting.clear()
        time.sleep(PERIOD)
        os.eventfd_write(event, 1)


threading.Thread(target=wake, daemon=True).start()
poll = select.epoll()
poll.register(event, select.EPOLLIN)
while True:
    waiting.set()
    poll.poll()
    os.eventfd_read(event)
    time.sleep(PERIOD)

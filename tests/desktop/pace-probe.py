"""Stand in for a gauge of the builder's share, doing the same work again and saying its time.

Every PERIOD it does one piece of fixed work and appends, to /run/pace-probe.log, how much of
its own thread's processor time that took, in microseconds, a line each. The builders run the
VM nested and give it no steal time, so the guest's kernel charges a processor's time off the
builder to whatever was running on it, and the same work takes longer as the builder holds the
guest back; a wait for a processor in the guest isn't charged. keeps-pace's tests read it by
turns, with the load running and frozen, and stretch their bars by it (keeps_pace.py).
"""

import time
from pathlib import Path

LOG = Path("/run/pace-probe.log")
# How often a piece of work is done, in seconds, and how much: a couple of milliseconds of a
# processor, so the probe takes a tenth of one, beside the four the load keeps busy.
PERIOD = 0.02
WORK = 40_000


def work() -> int:
    """Do the same sum every time, which nothing can skip."""
    total = 0
    for each in range(WORK):
        total += each * each
    return total


def main() -> None:
    """Do the work every PERIOD, saying how long each piece took, until stopped."""
    with LOG.open("a", buffering=1) as said:
        while True:
            began = time.thread_time_ns()
            work()
            said.write(f"{(time.thread_time_ns() - began) // 1000}\n")
            time.sleep(PERIOD)


main()

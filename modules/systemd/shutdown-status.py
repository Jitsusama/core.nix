# Tells Plymouth, by keyword, when a shutdown is forced on or held up, so
# the theme can say so in its own words. It starts with the shutdown and
# survives its last stage, and reads that stage's steps from the kernel's
# log, where systemd-shutdown writes them once the journal is gone.
#
# forced: the last stage began before the reboot, power-off, halt or kexec
# service had run, so the services' minute ran out.
# leftovers: the last stage asked what's left to stop, or killed it, and
# heard nothing back for three seconds, so something won't go.
import os
import re
import select
import subprocess
import sys
import time

plymouth = os.environ.get("PLYMOUTH", "plymouth")
reached = sys.argv[1]
held_after = 3.0

# The last stage is systemd-shutdown, and shutdown once in the ramfs.
step = re.compile(r"(systemd-)?shutdown\[1\]: (.*)")
waits = ("Sending SIGTERM to remaining", "Sending SIGKILL to remaining")


def say(word):
    try:
        subprocess.run(
            [plymouth, "update", f"--status={word}"],
            timeout=2,
            check=False,
        )
    except (OSError, subprocess.TimeoutExpired):
        pass


def open_log():
    log = os.open("/dev/kmsg", os.O_RDONLY | os.O_NONBLOCK)
    os.lseek(log, 0, os.SEEK_END)
    return log


log = open_log()
begun = False
waiting_since = None
while True:
    timeout = None
    if waiting_since is not None:
        timeout = max(0.0, waiting_since + held_after - time.monotonic())
    ready, _, _ = select.select([log], [], [], timeout)
    if not ready:
        say("leftovers")
        waiting_since = None
        continue
    try:
        record = os.read(log, 8192).decode(errors="replace")
    except BlockingIOError:
        continue
    except BrokenPipeError:
        # Records were overwritten before they were read.
        os.lseek(log, 0, os.SEEK_END)
        continue
    found = step.match(record.split(";", 1)[-1])
    if not found:
        continue
    if not begun:
        begun = True
        if not os.path.exists(reached):
            say("forced")
    waiting_since = None
    if found.group(2).startswith(waits):
        waiting_since = time.monotonic()

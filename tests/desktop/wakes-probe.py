"""A throwaway: what the closed shell reads, by thread and by what each fd is, over 30 s.

Run as root in the guest with the shell's pid; prints one line a source, most first.
"""

import collections
import os
import re
import subprocess
import sys

pid = sys.argv[1]
tids = sorted(os.listdir(f"/proc/{pid}/task"))
names = {tid: open(f"/proc/{pid}/task/{tid}/comm").read().strip() for tid in tids}
subprocess.run(
    [
        "timeout", "30", "strace", "-qq", "-o", "/tmp/st.txt", "-e",
        "trace=read,readv,recvmsg,recvfrom,ppoll,poll,epoll_wait,epoll_pwait",
        *[arg for tid in tids for arg in ("-p", tid)],
    ],
    check=False,
)
fds = {}
for fd in os.listdir(f"/proc/{pid}/fd"):
    try:
        fds[fd] = os.readlink(f"/proc/{pid}/fd/{fd}")
    except OSError:
        fds[fd] = "?"
inotify = {}
for fd, target in fds.items():
    if "inotify" in target:
        watched = []
        for line in open(f"/proc/{pid}/fdinfo/{fd}"):
            found = re.search(r"ino:([0-9a-f]+)", line)
            if found:
                watched.append(found[1])
        inotify[fd] = len(watched)
reads = collections.Counter()
waits = collections.Counter()
for line in open("/tmp/st.txt"):
    found = re.match(r"(\d+)\s+(\w+)\((\d+)?", line)
    if not found:
        continue
    tid, call, fd = found[1], found[2], found[3]
    thread = names.get(tid, tid)
    if call in ("ppoll", "poll", "epoll_wait", "epoll_pwait"):
        waits[thread] += 1
    elif fd is not None and "EAGAIN" not in line:
        target = fds.get(fd, "?")
        if fd in inotify:
            target += f" ({inotify[fd]} watched)"
        reads[(thread, fd, target)] += 1
print("waits", dict(waits.most_common()))
for (thread, fd, target), count in reads.most_common(40):
    print(count, thread, fd, target)

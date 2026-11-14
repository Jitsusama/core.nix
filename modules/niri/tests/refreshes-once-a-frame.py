"""Counts niri's refreshes while a client commits about a thousand times a second.

niri's refresh walks every window, and it ran after each wakeup of niri's loop, which every
commit a client makes is. Paced (refreshes-once-a-frame.patch), it runs at most once a frame,
60 times a second on a nested niri, however often commits wake it. The stand-in client speaks
the Wayland wire protocol itself: it binds wl_compositor, makes a surface and commits it, over
and over. niri's log says once a second how many refreshes ran in how many wakeups.

usage: refreshes-once-a-frame.py <niri's log>
"""

import os
import re
import socket
import struct
import sys
import time
from pathlib import Path

from wayland_wire import message, read_string, string

# The wl_display every connection starts with, and the ids this client gives the objects it
# makes.
DISPLAY = 1
REGISTRY = 2
SYNCED = 3
COMPOSITOR = 4
SURFACE = 5

# How long the stand-in commits for, and how long it waits between commits.
COMMITTING = 3.5
BETWEEN_COMMITS = 0.001

# niri's tally of a second.
TALLY = re.compile(r"refreshed (\d+) times in (\d+) wakeups")
# A second in which the stand-in's commits woke niri at least this often is busy enough to
# show pacing.
BUSY = 500
# The most refreshes a busy second may hold: a 60 Hz output's frames, and a few that a focus
# change or a window coming had at once.
MOST = 75


class Connection:
    """A connection to the compositor, reading whole events from it."""

    def __init__(self) -> None:
        """Connect to the compositor WAYLAND_DISPLAY names."""
        path = Path(os.environ["XDG_RUNTIME_DIR"]) / os.environ["WAYLAND_DISPLAY"]
        self.socket = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.socket.connect(str(path))
        self.unread = b""

    def send(self, request: bytes) -> None:
        """Send a request."""
        self.socket.sendall(request)

    def event(self) -> tuple[int, int, bytes]:
        """Read the next event: the object it's from, its opcode and its payload."""
        while True:
            if len(self.unread) >= 8:  # noqa: PLR2004 # An event's header is 8 bytes.
                source, word = struct.unpack_from("<II", self.unread)
                size = word >> 16
                if len(self.unread) >= size:
                    payload = self.unread[8:size]
                    self.unread = self.unread[size:]
                    return source, word & 0xFFFF, payload
            received = self.socket.recv(65536)
            if not received:
                sys.exit("niri closed the stand-in's connection")
            self.unread += received


def compositor_global(connection: Connection) -> tuple[int, int]:
    """Find the wl_compositor global's name and version in the registry."""
    connection.send(message(DISPLAY, 1, struct.pack("<I", REGISTRY)))
    connection.send(message(DISPLAY, 0, struct.pack("<I", SYNCED)))
    found = None
    while True:
        source, opcode, payload = connection.event()
        if source == DISPLAY and opcode == 0:
            sys.exit(f"niri refused the stand-in: {payload!r}")
        if source == REGISTRY and opcode == 0:
            (name,) = struct.unpack_from("<I", payload)
            interface, at = read_string(payload, 4)
            (version,) = struct.unpack_from("<I", payload, at)
            if interface == "wl_compositor":
                found = (name, version)
        if source == SYNCED:
            break
    if found is None:
        sys.exit("niri offers no wl_compositor")
    return found


def commit_quickly(connection: Connection) -> None:
    """Make a surface and commit it every millisecond or so, for a few seconds."""
    name, version = compositor_global(connection)
    bind = struct.pack("<I", name) + string("wl_compositor")
    bind += struct.pack("<II", min(version, 4), COMPOSITOR)
    connection.send(message(REGISTRY, 0, bind))
    connection.send(message(COMPOSITOR, 0, struct.pack("<I", SURFACE)))

    commit = message(SURFACE, 6)
    until = time.monotonic() + COMMITTING
    commits = 0
    while time.monotonic() < until:
        connection.send(commit)
        commits += 1
        time.sleep(BETWEEN_COMMITS)
    print(f"the stand-in committed {commits} times in {COMMITTING} s")


def main() -> None:
    """Count niri's refreshes while the stand-in commits, and fail on a busy second's many."""
    log = Path(sys.argv[1])
    start = log.stat().st_size
    commit_quickly(Connection())
    with log.open("rb") as written:
        written.seek(start)
        tallies = [
            (int(refreshes), int(wakeups))
            for refreshes, wakeups in TALLY.findall(written.read().decode(errors="replace"))
        ]

    for refreshes, wakeups in tallies:
        print(f"niri refreshed {refreshes} times in {wakeups} wakeups in a second")
    busy = [(refreshes, wakeups) for refreshes, wakeups in tallies if wakeups >= BUSY]
    if not busy:
        sys.exit(
            f"The stand-in's commits never woke niri {BUSY} times in a second, too few to show "
            "its refreshes paced, or niri's log holds no tally of them."
        )
    for refreshes, wakeups in busy:
        if refreshes > MOST:
            sys.exit(
                f"niri refreshed {refreshes} times in {wakeups} wakeups in a second, more than "
                f"{MOST}: its walk over every window isn't held to once a frame."
            )


if __name__ == "__main__":
    main()

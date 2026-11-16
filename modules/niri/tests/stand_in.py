"""A stand-in client of niri, for the checks that open windows on it as Joel's or a sandbox's.

It speaks the wire protocol itself rather than through a client library, so each check sees
exactly what it sends and when, and it asks niri what it did over niri's own socket, as
`niri msg` does. opens-sandboxed-windows-behind.py and keeps-secret-copies-from-sandboxes.py
share it.
"""

import itertools
import json
import os
import select
import socket
import struct
import sys
import time
from typing import TYPE_CHECKING, Any

from wayland_wire import message, read_string, string

if TYPE_CHECKING:
    from pathlib import Path

# The display's own object, the one the wire protocol gives every connection.
DISPLAY = 1
# Each window's buffer: a size niri asks for at most on the nested 1280x800 screen, and the
# format with no alpha.
XRGB8888 = 1
POOL_SIZE = 1280 * 800 * 4
# The size a window draws itself when niri leaves the choice to it.
DEFAULT_SIZE = (600, 400)
# How long anything is waited for, and how long niri is given to act on what was asked of it
# before what it did is read.
PATIENCE = 10.0
SETTLE = 0.5
# The globals each connection binds, at most at these versions.
WANTED = {
    "wl_compositor": 4,
    "wl_shm": 1,
    "wl_seat": 5,
    "xdg_wm_base": 2,
    "xdg_activation_v1": 1,
}


class Connection:
    """One client of niri, speaking the wire protocol itself, with the globals windows need."""

    def __init__(self, path: Path, also: dict[str, int] | None = None) -> None:
        """Connect to the display at `path`, and bind what a window and a token need.

        `also` names more globals to bind, each at most at its version, which niri has to offer.
        """
        self.socket = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.socket.connect(str(path))
        self.unread = b""
        self.ids = itertools.count(2)
        self.bound: dict[str, int] = {}
        # Each object's newest event, its opcode and payload, by the object's id: a configure's
        # serial, a toplevel's asked size, a token's text, a callback being done.
        self.said: dict[int, tuple[int, bytes]] = {}
        # Every such event in the order it came, its object, opcode and payload, for a check
        # that has to see each one, as every type a selection is offered as.
        self.heard: list[tuple[int, int, bytes]] = []
        self.keyboard = 0
        self.pool = 0
        # The registry, and each global it listed: its interface, name and version.
        self.registry = 0
        self.offered: dict[str, tuple[int, int]] = {}
        # The serial of the newest keyboard enter this connection was sent.
        self.entered: int | None = None
        self.bind_globals({**WANTED, **(also or {})})

    def made(self) -> int:
        """Give out the id of a new object, the next unused, as the client chooses them."""
        return next(self.ids)

    def send(self, target: int, opcode: int, payload: bytes = b"") -> None:
        """Send a request to an object."""
        self.socket.sendall(message(target, opcode, payload))

    def send_fd(self, target: int, opcode: int, payload: bytes, fd: int) -> None:
        """Send a request to an object with a file descriptor beside it."""
        socket.send_fds(self.socket, [message(target, opcode, payload)], [fd])

    def pump(self, until: float) -> bool:
        """Handle every event that arrives before `until`, the time by the monotonic clock.

        Answers whether any arrived, so a caller can wait for one event in particular.
        """
        handled = False
        while True:
            while len(self.unread) >= 8:  # noqa: PLR2004 # An event's header is 8 bytes.
                source, word = struct.unpack_from("<II", self.unread)
                size = word >> 16
                if len(self.unread) < size:
                    break
                self.note(source, word & 0xFFFF, self.unread[8:size])
                self.unread = self.unread[size:]
                handled = True
            left = until - time.monotonic()
            if left <= 0 or (handled and not self.unread):
                return handled
            readable, _, _ = select.select([self.socket], [], [], left)
            if not readable:
                return handled
            received = self.socket.recv(65536)
            if not received:
                sys.exit("niri closed a stand-in's connection")
            self.unread += received

    def note(self, source: int, opcode: int, payload: bytes) -> None:
        """Keep what an event says, answering niri's pings and failing on a protocol error."""
        if source == DISPLAY and opcode == 0:
            sys.exit(f"niri refused a stand-in: {payload!r}")
        if source == self.bound.get("xdg_wm_base") and opcode == 0:
            self.send(source, 3, payload[:4])
        elif source == self.keyboard and opcode == 1:
            (self.entered,) = struct.unpack_from("<I", payload)
        elif source == self.registry and opcode == 0:
            (name,) = struct.unpack_from("<I", payload)
            interface, at = read_string(payload, 4)
            self.offered[interface] = (name, struct.unpack_from("<I", payload, at)[0])
        elif source in {DISPLAY, self.keyboard}:
            return
        else:
            self.said[source] = (opcode, payload)
            self.heard.append((source, opcode, payload))

    def wait_for(self, target: int, what: str) -> tuple[int, bytes]:
        """Wait for an event from `target`, and answer its opcode and payload."""
        until = time.monotonic() + PATIENCE
        while target not in self.said:
            if time.monotonic() > until:
                sys.exit(f"niri never sent {what}")
            self.pump(until)
        return self.said.pop(target)

    def roundtrip(self) -> None:
        """Wait until niri has handled everything this connection sent."""
        callback = self.made()
        self.send(DISPLAY, 0, struct.pack("<I", callback))
        self.wait_for(callback, "a roundtrip's callback")

    def bind_globals(self, wanted: dict[str, int]) -> None:
        """Bind each wanted global from the registry, a keyboard, and make the pool."""
        self.registry = self.made()
        self.send(DISPLAY, 1, struct.pack("<I", self.registry))
        self.roundtrip()
        for interface, most in wanted.items():
            if interface not in self.offered:
                sys.exit(f"niri offers no {interface} here")
            name, version = self.offered[interface]
            self.bound[interface] = self.made()
            bind = struct.pack("<I", name) + string(interface)
            bind += struct.pack("<II", min(version, most), self.bound[interface])
            self.send(self.registry, 0, bind)
        self.keyboard = self.made()
        self.send(self.bound["wl_seat"], 1, struct.pack("<I", self.keyboard))

        self.pool = self.made()
        memory = os.memfd_create("stand-in")
        os.ftruncate(memory, POOL_SIZE)
        pool = message(self.bound["wl_shm"], 0, struct.pack("<Ii", self.pool, POOL_SIZE))
        socket.send_fds(self.socket, [pool], [memory])
        os.close(memory)

    def window(self, app_id: str) -> int:
        """Map a toplevel with this app ID, drawn once, and answer its surface's id."""
        surface, role, toplevel = self.made(), self.made(), self.made()
        self.send(self.bound["wl_compositor"], 0, struct.pack("<I", surface))
        self.send(self.bound["xdg_wm_base"], 2, struct.pack("<II", role, surface))
        self.send(role, 1, struct.pack("<I", toplevel))
        self.send(toplevel, 3, string(app_id))
        self.send(surface, 6)
        _, configured = self.wait_for(role, "a window's first configure")
        asked = self.said.pop(toplevel, (0, b"\0" * 8))[1]
        width, height = struct.unpack_from("<ii", asked)
        width, height = width or DEFAULT_SIZE[0], height or DEFAULT_SIZE[1]
        buffer = self.made()
        self.send(
            self.pool, 0, struct.pack("<IiiiiI", buffer, 0, width, height, width * 4, XRGB8888)
        )
        self.send(role, 4, configured[:4])
        self.send(surface, 1, struct.pack("<Iii", buffer, 0, 0))
        self.send(surface, 9, struct.pack("<iiii", 0, 0, width, height))
        self.send(surface, 6)
        self.roundtrip()
        return surface

    def token(self, surface: int) -> str:
        """Make an activation token from this connection's newest keyboard enter, if it had one."""
        token = self.made()
        self.send(self.bound["xdg_activation_v1"], 1, struct.pack("<I", token))
        if self.entered is not None:
            self.send(token, 0, struct.pack("<II", self.entered, self.bound["wl_seat"]))
        self.send(token, 2, struct.pack("<I", surface))
        self.send(token, 3)
        _, payload = self.wait_for(token, "an activation token")
        return read_string(payload, 0)[0]

    def activate(self, token: str, surface: int) -> None:
        """Ask niri to activate one of this connection's windows with a token."""
        self.send(self.bound["xdg_activation_v1"], 2, string(token) + struct.pack("<I", surface))
        self.roundtrip()


def ask(request: str | dict[str, Any]) -> dict[str, Any]:
    """Ask niri something over its socket, as `niri msg` does, and answer what it said.

    An action's answer is a word rather than an object, and answers empty.
    """
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as asking:
        asking.connect(os.environ["NIRI_SOCKET"])
        asking.sendall(json.dumps(request).encode() + b"\n")
        reply = json.loads(asking.makefile("rb").readline())
    if "Ok" not in reply:
        sys.exit(f"niri refused {request}: {reply}")
    answer = reply["Ok"]
    return answer if isinstance(answer, dict) else {}


def focused_window() -> int | None:
    """Say which window has the focus, by its id, if one has it."""
    window = ask("FocusedWindow")["FocusedWindow"]
    return None if window is None else int(window["id"])


def opened(app_id: str, *others: Connection) -> dict[str, Any]:
    """Wait for niri to list a window with this app ID, keeping the connections answered."""
    until = time.monotonic() + PATIENCE
    while time.monotonic() < until:
        for window in ask("Windows")["Windows"]:
            if window["app_id"] == app_id:
                return dict(window)
        for other in others:
            other.pump(time.monotonic() + 0.05)
    sys.exit(f"niri never listed {app_id}")


def settle(*connections: Connection) -> None:
    """Give niri a moment to act on what it was asked, keeping every connection answered."""
    until = time.monotonic() + SETTLE
    while time.monotonic() < until:
        for connection in connections:
            connection.pump(min(until, time.monotonic() + 0.05))

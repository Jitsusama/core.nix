"""Times a resize's layout from the window's commit to niri's IPC, and reads what it says began.

niri paces its refresh to once a frame (refreshes-once-a-frame.patch), and a window's layout goes
out over IPC in a refresh, so a commit acking a resize waited up to a frame for it unless
something had hurried the refresh. The margin cat rides a window's edge from those reports, so
it rode a frame behind. A commit changing a window's size now hurries it, and the layout says
which animation is moving the tile and how many milliseconds ago it began
(tiled-positions.patch), so the shell can start in phase with it.

The stand-in is a window speaking the Wayland wire protocol itself: it binds wl_compositor,
wl_shm and xdg_wm_base, maps a toplevel, and answers each resize niri asks for with a buffer of
the new size. Before it commits that, it waits on two roundtrips, which wake niri and spend the
refreshes the resize's action hurried, as a terminal drawing its new size does, so the commit
meets a refresh paced to the frame unless the commit itself hurries it. A second report, as the
window's title changes in the middle of the animation, has to say a longer time since it began.

usage: tells-a-resize-at-once.py
"""

import json
import os
import queue
import socket
import statistics
import struct
import sys
import threading
import time
from pathlib import Path
from typing import Any

# The ids this client gives the objects it makes, as the wire protocol has the client choose,
# each the next unused, in the order it makes them.
DISPLAY = 1
REGISTRY = 2
LISTED = 3
COMPOSITOR = 4
SHM = 5
WM_BASE = 6
POOL = 7
SURFACE = 8
XDG_SURFACE = 9
TOPLEVEL = 10
# Ids from here on are callbacks and buffers, one each.
FIRST_FREE = 11

# The wl_shm format with no alpha, and the pool's size: a 1280x800 screen of it, twice.
XRGB8888 = 1
POOL_SIZE = 1280 * 800 * 4 * 2

# The widths each resize asks for in turn, and how many resizes there are. The first resize
# warms up, unjudged: the sandbox's software GL builds niri's resize shader as it first draws
# one, stalling niri for longer than the whole animation.
WIDTHS = (480, 720)
WARM_UP = 1
RESIZES = 6
# The nested niri's frame, at 60 Hz, half of which a resize's layout has to reach IPC within,
# at the median: a commit meeting a paced refresh waits for most of a frame.
FRAME = 1 / 60
WITHIN = FRAME / 2
# The most milliseconds a resize's own report may say it began before: it begins as the commit
# is handled, and its report goes out within a few frames even paced.
BEGAN_WITHIN = 100.0
# How long after a resize's report the title changes, in the middle of its animation (about
# 230 ms at the theme's spring), and how much longer ago the second report must say it began.
TITLE_AFTER = 0.06
LONGER_BY_MS = 40.0
# How long each animation is let finish before the next resize.
SETTLES = 0.4
# How long anything is waited for.
PATIENCE = 10.0


def message(target: int, opcode: int, payload: bytes = b"") -> bytes:
    """Frame a request to the object `target`, as the wire protocol does."""
    return struct.pack("<II", target, (8 + len(payload)) << 16 | opcode) + payload


def string(text: str) -> bytes:
    """Encode a string argument: its length with its NUL, then it, padded to 4 bytes."""
    data = text.encode() + b"\0"
    return struct.pack("<I", len(data)) + data + b"\0" * (-len(data) % 4)


def read_string(payload: bytes, at: int) -> tuple[str, int]:
    """Read the string argument at `at` in a payload, and where the next one starts."""
    (length,) = struct.unpack_from("<I", payload, at)
    text = payload[at + 4 : at + 4 + length - 1].decode()
    return text, at + 4 + length + (-length % 4)


class Window:
    """A toplevel on its own connection to niri, drawn from one shared-memory pool."""

    def __init__(self) -> None:
        """Connect to the compositor WAYLAND_DISPLAY names, and bind what a window needs."""
        path = Path(os.environ["XDG_RUNTIME_DIR"]) / os.environ["WAYLAND_DISPLAY"]
        self.socket = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.socket.connect(str(path))
        self.unread = b""
        self.next_id = FIRST_FREE
        self.width = 0
        self.height = 0
        # The size and serial of the newest configure, not yet acked.
        self.asked: tuple[int, int] = (0, 0)
        self.serial: int | None = None
        self.bind_globals()

    def new_id(self) -> int:
        """Give out an id for a new object."""
        made = self.next_id
        self.next_id += 1
        return made

    def send(self, request: bytes) -> None:
        """Send a request."""
        self.socket.sendall(request)

    def event(self) -> tuple[int, int, bytes]:
        """Read the next event, answering niri's pings and noting its configures."""
        while True:
            if len(self.unread) >= 8:  # noqa: PLR2004 # An event's header is 8 bytes.
                source, word = struct.unpack_from("<II", self.unread)
                size = word >> 16
                if len(self.unread) >= size:
                    payload = self.unread[8:size]
                    self.unread = self.unread[size:]
                    self.note(source, word & 0xFFFF, payload)
                    return source, word & 0xFFFF, payload
            received = self.socket.recv(65536)
            if not received:
                sys.exit("niri closed the stand-in's connection")
            self.unread += received

    def note(self, source: int, opcode: int, payload: bytes) -> None:
        """Answer a ping, note a configure, and fail on a protocol error."""
        if source == DISPLAY and opcode == 0:
            sys.exit(f"niri refused the stand-in: {payload!r}")
        if source == WM_BASE and opcode == 0:
            self.send(message(WM_BASE, 3, payload[:4]))
        if source == TOPLEVEL and opcode == 0:
            self.asked = struct.unpack_from("<ii", payload)
        if source == XDG_SURFACE and opcode == 0:
            (self.serial,) = struct.unpack_from("<I", payload)

    def roundtrip(self) -> None:
        """Wait until niri has handled everything sent so far."""
        callback = self.new_id()
        self.send(message(DISPLAY, 0, struct.pack("<I", callback)))
        while self.event()[0] != callback:
            pass

    def bind_globals(self) -> None:
        """Bind wl_compositor, wl_shm and xdg_wm_base from the registry, and make the pool."""
        wanted = {"wl_compositor": (COMPOSITOR, 4), "wl_shm": (SHM, 1), "xdg_wm_base": (WM_BASE, 2)}
        self.send(message(DISPLAY, 1, struct.pack("<I", REGISTRY)))
        self.send(message(DISPLAY, 0, struct.pack("<I", LISTED)))
        found: dict[str, tuple[int, int]] = {}
        while True:
            source, opcode, payload = self.event()
            if source == REGISTRY and opcode == 0:
                (name,) = struct.unpack_from("<I", payload)
                interface, at = read_string(payload, 4)
                (version,) = struct.unpack_from("<I", payload, at)
                if interface in wanted:
                    found[interface] = (name, version)
            if source == LISTED:
                break
        for interface, (target, most) in wanted.items():
            if interface not in found:
                sys.exit(f"niri offers no {interface}")
            name, version = found[interface]
            bind = struct.pack("<I", name) + string(interface)
            bind += struct.pack("<II", min(version, most), target)
            self.send(message(REGISTRY, 0, bind))

        memory = os.memfd_create("stand-in")
        os.ftruncate(memory, POOL_SIZE)
        pool = message(SHM, 0, struct.pack("<Ii", POOL, POOL_SIZE))
        socket.send_fds(self.socket, [pool], [memory])
        os.close(memory)

    def map(self, app_id: str) -> None:
        """Make the toplevel, answer its first configure and commit its first buffer."""
        self.send(message(COMPOSITOR, 0, struct.pack("<I", SURFACE)))
        self.send(message(WM_BASE, 2, struct.pack("<II", XDG_SURFACE, SURFACE)))
        self.send(message(XDG_SURFACE, 1, struct.pack("<I", TOPLEVEL)))
        self.send(message(TOPLEVEL, 3, string(app_id)))
        self.send(message(SURFACE, 6))
        while self.serial is None:
            self.event()
        width, height = self.asked
        self.commit(width or 600, height or 400)

    def commit(self, width: int, height: int) -> None:
        """Ack the newest configure and commit a buffer of the size given."""
        buffer = self.new_id()
        stride = width * 4
        self.send(
            message(POOL, 0, struct.pack("<IiiiiI", buffer, 0, width, height, stride, XRGB8888))
        )
        self.send(message(XDG_SURFACE, 4, struct.pack("<I", self.serial)))
        self.send(message(SURFACE, 1, struct.pack("<Iii", buffer, 0, 0)))
        self.send(message(SURFACE, 9, struct.pack("<iiii", 0, 0, width, height)))
        self.send(message(SURFACE, 6))
        self.width, self.height = width, height

    def resized(self) -> int:
        """Wait for niri to ask for a new width, then for two roundtrips, then commit it."""
        until = time.monotonic() + PATIENCE
        while self.asked[0] in {0, self.width}:
            if time.monotonic() > until:
                sys.exit("niri never asked the stand-in for a new width")
            self.event()
        self.roundtrip()
        self.roundtrip()
        width = self.asked[0]
        self.commit(width, self.asked[1] or self.height)
        return width

    def titled(self, title: str) -> None:
        """Change the window's title, which niri reports with the window's whole layout."""
        self.send(message(TOPLEVEL, 2, string(title)))


class Reports:
    """niri's event stream, each event kept with when it arrived."""

    def __init__(self) -> None:
        """Ask niri for its event stream, and read it as it comes."""
        self.socket = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.socket.connect(os.environ["NIRI_SOCKET"])
        self.socket.sendall(b'"EventStream"\n')
        self.arrived: queue.Queue[tuple[float, dict[str, Any]]] = queue.Queue()
        self.lines = self.socket.makefile("rb")
        reply = json.loads(self.lines.readline())
        if "Ok" not in reply:
            sys.exit(f"niri refused its event stream: {reply}")
        threading.Thread(target=self.read, daemon=True).start()

    def read(self) -> None:
        """Keep each event with the time it arrived."""
        for line in self.lines:
            self.arrived.put((time.monotonic(), json.loads(line)))

    def window(self, app_id: str) -> int:
        """Find the id of the window with the app id given, once niri reports it."""
        while True:
            _, event = self.next()
            for window in event.get("WindowsChanged", {}).get("windows", []):
                if window["app_id"] == app_id:
                    return int(window["id"])
            window = event.get("WindowOpenedOrChanged", {}).get("window")
            if window is not None and window["app_id"] == app_id:
                return int(window["id"])

    def next(self) -> tuple[float, dict[str, Any]]:
        """Take the next event, and when it arrived."""
        try:
            return self.arrived.get(timeout=PATIENCE)
        except queue.Empty:
            sys.exit(f"niri reported nothing in {PATIENCE} s")

    def layout(self, window: int, width: int) -> tuple[float, dict[str, Any]]:
        """Wait for the first layout of a window at a width, and say when it arrived."""
        while True:
            at, event = self.next()
            changes = event.get("WindowLayoutsChanged", {}).get("changes", [])
            opened = event.get("WindowOpenedOrChanged", {}).get("window")
            if opened is not None:
                changes = [*changes, [opened["id"], opened["layout"]]]
            for id_, layout in changes:
                if id_ == window and layout["window_size"][0] == width:
                    return at, layout

    def titled(self, window: int, title: str) -> dict[str, Any]:
        """Wait for the layout reported with a window once its title is the one given."""
        while True:
            _, event = self.next()
            opened = event.get("WindowOpenedOrChanged", {}).get("window")
            if opened is not None and opened["id"] == window and opened["title"] == title:
                return dict(opened["layout"])


def act(action: dict[str, Any]) -> None:
    """Ask niri for an action over its socket, as `niri msg action` does."""
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as asking:
        asking.connect(os.environ["NIRI_SOCKET"])
        asking.sendall(json.dumps({"Action": action}).encode() + b"\n")
        reply = json.loads(asking.makefile("rb").readline())
        if "Ok" not in reply:
            sys.exit(f"niri refused {action}: {reply}")


def resize(window: Window, reports: Reports, id_: int, round_: int, failures: list[str]) -> float:
    """Resize the stand-in once, note what its reports got wrong, and say how late they came."""
    act({"SetColumnWidth": {"change": {"SetFixed": WIDTHS[round_ % len(WIDTHS)]}}})
    width = window.resized()
    committed = time.monotonic()
    arrived, layout = reports.layout(id_, width)
    latency = arrived - committed
    began = layout.get("tile_animation_began_ms_ago")
    moving = layout.get("tile_animation")
    print(
        f"resize {round_} to {width}: its layout reached IPC {1000 * latency:.1f} ms after its "
        f"commit, saying {moving} began {began} ms ago"
    )
    if moving != "Resize" or not isinstance(began, int | float) or not 0 <= began < BEGAN_WITHIN:
        failures.append(
            f"resize {round_}'s layout said {moving!r} began {began!r} ms ago, not a resize that "
            "began as it was committed"
        )

    time.sleep(TITLE_AFTER)
    title = f"stand-in {round_}"
    window.titled(title)
    later = reports.titled(id_, title)
    again = later.get("tile_animation_began_ms_ago")
    print(f"  its title's report said {later.get('tile_animation')} began {again} ms ago")
    if (
        later.get("tile_animation") != "Resize"
        or not isinstance(began, int | float)
        or not isinstance(again, int | float)
        or again < began + LONGER_BY_MS
    ):
        failures.append(
            f"resize {round_}'s next report, {1000 * TITLE_AFTER:.0f} ms on, said "
            f"{later.get('tile_animation')!r} began {again!r} ms ago, not the same resize at "
            f"least {LONGER_BY_MS:.0f} ms longer ago than {began!r}"
        )
    time.sleep(SETTLES)
    return latency


def main() -> None:
    """Resize the stand-in over and over, and fail where niri's reports come late or say less."""
    reports = Reports()
    window = Window()
    window.map("stand-in")
    id_ = reports.window("stand-in")
    window.roundtrip()

    for round_ in range(WARM_UP):
        resize(window, reports, id_, round_, [])
    failures: list[str] = []
    latencies = [
        resize(window, reports, id_, round_, failures)
        for round_ in range(WARM_UP, WARM_UP + RESIZES)
    ]
    median = statistics.median(latencies)
    print(f"a resize's layout reached IPC a median {1000 * median:.1f} ms after its commit")
    if median > WITHIN:
        failures.append(
            f"a resize's layout reached IPC a median {1000 * median:.1f} ms after its commit, "
            f"more than {1000 * WITHIN:.1f} ms: the commit waited for a paced refresh"
        )
    if failures:
        sys.exit("\n".join(failures))


if __name__ == "__main__":
    main()

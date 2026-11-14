"""Opens a sandbox's windows on niri, and checks none of them takes Joel's place or his focus.

A client connected through a wp-security-context-v1 socket is a sandbox's, as a fenced agent's
command is through pi-fence-display. Its windows have to open behind what is shown and never
take the focus, whatever they ask (sandboxed-windows-stay-behind.patch), since a fenced command
opening a window must not move Joel's keys. This runs pi-fence-display on the nested niri, then:

1. Joel's window opens on the display itself and takes the focus, as his own programs do.
2. A sandbox's window opens: the focus stays on Joel's, and the window lands on a workspace no
   screen shows.
3. Joel's window makes an activation token from his focus and the sandbox uses it on its own
   window: the focus stays on the same window and workspace, the sandbox's window stays behind,
   and it's marked urgent.
4. The sandbox makes a token and Joel's program uses it on a second window of his: the focus
   stays on his first, and the second is marked urgent.
5. Joel goes to the empty workspace at the end, and a second sandbox's window opens: the
   workspace he's on is still the focused one, and the window is elsewhere.

Every case is checked and reported before the stand-in fails.

usage: opens-sandboxed-windows-behind.py <pi-fence-display>
"""

import itertools
import json
import os
import select
import socket
import struct
import subprocess
import sys
import time
from pathlib import Path
from typing import Any

from wayland_wire import message, read_string, string

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

    def __init__(self, path: Path) -> None:
        """Connect to the display at `path`, and bind what a window and a token need."""
        self.socket = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.socket.connect(str(path))
        self.unread = b""
        self.ids = itertools.count(2)
        self.bound: dict[str, int] = {}
        # Each object's newest event, its opcode and payload, by the object's id: a configure's
        # serial, a toplevel's asked size, a token's text, a callback being done.
        self.said: dict[int, tuple[int, bytes]] = {}
        self.keyboard = 0
        self.pool = 0
        # The registry, and each global it listed: its interface, name and version.
        self.registry = 0
        self.offered: dict[str, tuple[int, int]] = {}
        # The serial of the newest keyboard enter this connection was sent.
        self.entered: int | None = None
        self.bind_globals()

    def made(self) -> int:
        """Give out the id of a new object, the next unused, as the client chooses them."""
        return next(self.ids)

    def send(self, target: int, opcode: int, payload: bytes = b"") -> None:
        """Send a request to an object."""
        self.socket.sendall(message(target, opcode, payload))

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

    def bind_globals(self) -> None:
        """Bind each wanted global from the registry, a keyboard, and make the pool."""
        self.registry = self.made()
        self.send(DISPLAY, 1, struct.pack("<I", self.registry))
        self.roundtrip()
        for interface, most in WANTED.items():
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


def focused_workspace() -> int:
    """Say which workspace has the focus, by its id."""
    return next(int(ws["id"]) for ws in ask("Workspaces")["Workspaces"] if ws["is_focused"])


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


def shown(workspace: int) -> bool:
    """Whether a screen shows this workspace."""
    return any(ws["is_active"] for ws in ask("Workspaces")["Workspaces"] if ws["id"] == workspace)


def behind(window: dict[str, Any], focused: int, failures: list[str], case: str) -> None:
    """Check a sandbox's window is on a workspace neither focused nor shown."""
    workspace = int(window["workspace_id"])
    if workspace == focused:
        failures.append(f"{case}: the sandbox's window opened on the focused workspace")
    elif shown(workspace):
        failures.append(f"{case}: the sandbox's window opened on a workspace a screen shows")


def kept(before: int | None, failures: list[str], case: str) -> None:
    """Check the focus is on the same window as before, saying where it went if not."""
    after = focused_window()
    if after != before:
        failures.append(f"{case}: the focus moved from window {before} to {after}")
    else:
        print(f"{case}: the focus stayed on window {before}")


def main() -> None:
    """Run the five cases against pi-fence-display's socket, and fail on any that didn't hold."""
    runtime = Path(os.environ["XDG_RUNTIME_DIR"])
    filtered = runtime / "pi-fence-wayland"
    with subprocess.Popen([sys.argv[1], "--socket", str(filtered)]) as display:
        try:
            until = time.monotonic() + PATIENCE
            while not filtered.is_socket():
                if time.monotonic() > until or display.poll() is not None:
                    sys.exit("pi-fence-display never opened its socket")
                time.sleep(0.05)
            failures = run(Connection(runtime / os.environ["WAYLAND_DISPLAY"]), filtered)
        finally:
            display.terminate()
    for failure in failures:
        print(failure, file=sys.stderr)
    if failures:
        sys.exit(f"{len(failures)} of a sandbox's moves reached Joel's screen or his focus")
    print("every sandbox's window opened behind, and none took the focus")


def run(joel: Connection, filtered: Path) -> list[str]:
    """Run the five cases, Joel's window on `joel` and the sandbox's on the filtered socket."""
    failures: list[str] = []
    joels_surface = joel.window("joel.window")
    joels = int(opened("joel.window", joel)["id"])
    settle(joel)
    if focused_window() != joels:
        sys.exit("Joel's own window never took the focus")

    sandbox = Connection(filtered)
    sandboxes_surface = sandbox.window("pi.fence.window")
    window = opened("pi.fence.window", joel, sandbox)
    settle(joel, sandbox)
    kept(joels, failures, "a sandbox's window opening")
    behind(window, focused_workspace(), failures, "a sandbox's window opening")

    # Joel's window has the focus back, whatever the case before did, so the token comes from
    # a keyboard enter niri still honours.
    if focused_window() != joels:
        ask({"Action": {"FocusWindow": {"id": joels}}})
        settle(joel, sandbox)
    focused = focused_workspace()
    if joel.entered is None:
        sys.exit("Joel's window was never given the keyboard")
    token = joel.token(joels_surface)
    sandbox.activate(token, sandboxes_surface)
    settle(joel, sandbox)
    case = "a sandbox activating its window with Joel's token"
    kept(joels, failures, case)
    if focused_workspace() != focused:
        failures.append(f"{case}: the focus went to another workspace")
    activated = opened("pi.fence.window", joel, sandbox)
    behind(activated, focused, failures, case)
    # Urgency is all an activation may leave. niri always says whether a window is urgent, so a
    # missing field fails here rather than passing.
    if not activated["is_urgent"]:
        failures.append(f"{case}: the sandbox's window wasn't marked urgent")

    # A token the sandbox makes, used by Joel's own program on his own window, moves nothing
    # either: Joel's second window waits behind his first, marked urgent.
    case = "Joel's program activating its window with the sandbox's token"
    joels_other_surface = joel.window("joel.other")
    joels_other = int(opened("joel.other", joel, sandbox)["id"])
    ask({"Action": {"FocusWindow": {"id": joels}}})
    settle(joel, sandbox)
    joel.activate(sandbox.token(sandboxes_surface), joels_other_surface)
    settle(joel, sandbox)
    kept(joels, failures, case)
    if not opened("joel.other", joel, sandbox)["is_urgent"]:
        failures.append(f"{case}: Joel's window {joels_other} wasn't marked urgent")

    ask({"Action": {"FocusWorkspace": {"reference": {"Id": empty_workspace()}}}})
    settle(joel, sandbox)
    focused = focused_workspace()
    sandbox.window("pi.fence.second")
    second = opened("pi.fence.second", joel, sandbox)
    settle(joel, sandbox)
    kept(None, failures, "a sandbox's window opening while Joel is on an empty workspace")
    if focused_workspace() != focused:
        failures.append("an empty workspace: the focus went to another workspace")
    behind(second, focused, failures, "a sandbox's window opening on an empty workspace")
    return failures


def empty_workspace() -> int:
    """Find the empty workspace the focused screen ends with, the one niri always keeps."""
    workspaces = ask("Workspaces")["Workspaces"]
    filled = {int(window["workspace_id"]) for window in ask("Windows")["Windows"]}
    output = next(ws["output"] for ws in workspaces if ws["is_focused"])
    last = max((ws for ws in workspaces if ws["output"] == output), key=lambda ws: ws["idx"])
    if int(last["id"]) in filled:
        sys.exit("niri's last workspace has a window on it")
    return int(last["id"])


if __name__ == "__main__":
    main()

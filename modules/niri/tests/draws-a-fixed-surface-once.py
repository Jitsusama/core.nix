"""Shows a bottom layer surface fixed to the output drawn once, where it is, through a switch.

niri draws each bottom and background layer surface once on every workspace, moved with it, so
as workspaces slide past one another a copy of the margin cat came in sitting on the incoming
window before the shell had heard of the switch, and then vanished as the shell's own move
began (the GPU rig, 2026-11-16, 1 to 6 frames). A surface whose namespace NIRI_FIXED_TO_OUTPUT
names is drawn once on its output, where it is, and still under the windows
(fixed-to-output.patch).

The stand-in speaks the Wayland wire protocol itself: one window, so a second, empty workspace
exists below it, and a bottom layer surface named as the environment names, a square of one
colour at the screen's left, clear of the window, which sits alone in the middle. With niri's
animations slowed, it moves to the workspace below and takes screenshots over IPC until the
screen settles. Every shot has to show the square's colour in the square alone, and at least
one has to show the windows partway, apart from both the screen before and after, or the check
never saw the switch it judges.

usage: draws-a-fixed-surface-once.py <shots folder>
"""

import json
import os
import socket
import struct
import sys
import time
from pathlib import Path
from typing import Any

from PIL import Image
from wayland_wire import message, read_string, string

# The ids this client gives the objects it makes, each the next unused, in the order it makes
# them.
DISPLAY = 1
REGISTRY = 2
LISTED = 3
COMPOSITOR = 4
SHM = 5
WM_BASE = 6
LAYER_SHELL = 7
POOL = 8
WINDOW_SURFACE = 9
XDG_SURFACE = 10
TOPLEVEL = 11
SQUARE_SURFACE = 12
LAYER_SURFACE = 13
# Ids from here on are callbacks and buffers, one each.
FIRST_FREE = 14

# The wl_shm format with no alpha; the window's part of the pool, a 1280x800 screen of it; and
# the square, its side, where it sits from the screen's top left, and its colour as XRGB8888
# keeps it, and as a screenshot gives it back.
XRGB8888 = 1
WINDOW_BYTES = 1280 * 800 * 4
SIDE = 64
TOP = 100
LEFT = 16
SQUARE_PIXEL = bytes((0xFF, 0x00, 0xFF, 0x00))
SQUARE_RGB = (0xFF, 0x00, 0xFF)
POOL_SIZE = WINDOW_BYTES + SIDE * SIDE * 4
# The layer the square asks for, the bottom, and its anchors, the top and the left.
BOTTOM = 1
ANCHORED = 1 | 4
# How many shots are taken at most through the switch, and how long anything is waited for.
SHOTS = 40
PATIENCE = 20.0
# How long apart two shots are taken to see whether the screen has settled.
APART = 0.2


class StandIn:
    """A window and a bottom layer surface on one connection, drawn from one pool."""

    def __init__(self, namespace: str) -> None:
        """Connect to the compositor WAYLAND_DISPLAY names, bind what both need, and map them."""
        path = Path(os.environ["XDG_RUNTIME_DIR"]) / os.environ["WAYLAND_DISPLAY"]
        self.socket = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.socket.connect(str(path))
        self.unread = b""
        self.next_id = FIRST_FREE
        self.window_asked: tuple[int, int] = (0, 0)
        self.window_serial: int | None = None
        self.square_serial: int | None = None
        # Each global the registry lists, by interface: its name and version.
        self.found: dict[str, tuple[int, int]] = {}
        self.bind_globals()
        self.map_window()
        self.map_square(namespace)

    def new_id(self) -> int:
        """Give out an id for a new object."""
        made = self.next_id
        self.next_id += 1
        return made

    def send(self, request: bytes) -> None:
        """Send a request."""
        self.socket.sendall(request)

    def event(self) -> int:
        """Read the next event, answer niri's pings, note its configures, and say its source."""
        while True:
            if len(self.unread) >= 8:  # noqa: PLR2004 # An event's header is 8 bytes.
                source, word = struct.unpack_from("<II", self.unread)
                size = word >> 16
                if len(self.unread) >= size:
                    payload = self.unread[8:size]
                    self.unread = self.unread[size:]
                    self.note(source, word & 0xFFFF, payload)
                    return int(source)
            received = self.socket.recv(65536)
            if not received:
                sys.exit("niri closed the stand-in's connection")
            self.unread += received

    def note(self, source: int, opcode: int, payload: bytes) -> None:
        """Note a global or a configure, answer a ping, and fail on an error or a closed square."""
        if source == REGISTRY and opcode == 0:
            (name,) = struct.unpack_from("<I", payload)
            interface, at = read_string(payload, 4)
            (version,) = struct.unpack_from("<I", payload, at)
            self.found[interface] = (name, version)
        if source == DISPLAY and opcode == 0:
            sys.exit(f"niri refused the stand-in: {payload!r}")
        if source == WM_BASE and opcode == 0:
            self.send(message(WM_BASE, 3, payload[:4]))
        if source == TOPLEVEL and opcode == 0:
            self.window_asked = struct.unpack_from("<ii", payload)
        if source == XDG_SURFACE and opcode == 0:
            (self.window_serial,) = struct.unpack_from("<I", payload)
        if source == LAYER_SURFACE and opcode == 0:
            (self.square_serial,) = struct.unpack_from("<I", payload)
        if source == LAYER_SURFACE and opcode == 1:
            sys.exit("niri closed the stand-in's layer surface")

    def bind_globals(self) -> None:
        """Bind what both surfaces need from the registry, and make the pool."""
        wanted = {
            "wl_compositor": (COMPOSITOR, 4),
            "wl_shm": (SHM, 1),
            "xdg_wm_base": (WM_BASE, 2),
            "zwlr_layer_shell_v1": (LAYER_SHELL, 3),
        }
        self.send(message(DISPLAY, 1, struct.pack("<I", REGISTRY)))
        self.send(message(DISPLAY, 0, struct.pack("<I", LISTED)))
        while self.event() != LISTED:
            pass
        for interface, (target, most) in wanted.items():
            if interface not in self.found:
                sys.exit(f"niri offers no {interface}")
            name, version = self.found[interface]
            bind = struct.pack("<I", name) + string(interface)
            bind += struct.pack("<II", min(version, most), target)
            self.send(message(REGISTRY, 0, bind))

        memory = os.memfd_create("stand-in")
        os.ftruncate(memory, POOL_SIZE)
        os.pwrite(memory, SQUARE_PIXEL * (SIDE * SIDE), WINDOW_BYTES)
        pool = message(SHM, 0, struct.pack("<Ii", POOL, POOL_SIZE))
        socket.send_fds(self.socket, [pool], [memory])
        os.close(memory)

    def commit(self, surface: int, offset: int, width: int, height: int) -> None:
        """Commit a buffer of the size given, from the pool at the offset given, on a surface."""
        buffer = self.new_id()
        self.send(
            message(
                POOL, 0, struct.pack("<IiiiiI", buffer, offset, width, height, width * 4, XRGB8888)
            )
        )
        self.send(message(surface, 1, struct.pack("<Iii", buffer, 0, 0)))
        self.send(message(surface, 9, struct.pack("<iiii", 0, 0, width, height)))
        self.send(message(surface, 6))

    def map_window(self) -> None:
        """Make the toplevel, answer its first configure and commit a black buffer."""
        self.send(message(COMPOSITOR, 0, struct.pack("<I", WINDOW_SURFACE)))
        self.send(message(WM_BASE, 2, struct.pack("<II", XDG_SURFACE, WINDOW_SURFACE)))
        self.send(message(XDG_SURFACE, 1, struct.pack("<I", TOPLEVEL)))
        self.send(message(TOPLEVEL, 3, string("stand-in")))
        self.send(message(WINDOW_SURFACE, 6))
        while self.window_serial is None:
            self.event()
        width, height = self.window_asked
        self.send(message(XDG_SURFACE, 4, struct.pack("<I", self.window_serial)))
        self.commit(WINDOW_SURFACE, 0, width or 600, height or 400)

    def map_square(self, namespace: str) -> None:
        """Make the layer surface on the bottom layer, place it, and commit its colour."""
        self.send(message(COMPOSITOR, 0, struct.pack("<I", SQUARE_SURFACE)))
        made = struct.pack("<III", LAYER_SURFACE, SQUARE_SURFACE, 0)
        self.send(message(LAYER_SHELL, 0, made + struct.pack("<I", BOTTOM) + string(namespace)))
        self.send(message(LAYER_SURFACE, 0, struct.pack("<II", SIDE, SIDE)))
        self.send(message(LAYER_SURFACE, 1, struct.pack("<I", ANCHORED)))
        self.send(message(LAYER_SURFACE, 3, struct.pack("<iiii", TOP, 0, 0, LEFT)))
        self.send(message(SQUARE_SURFACE, 6))
        while self.square_serial is None:
            self.event()
        self.send(message(LAYER_SURFACE, 6, struct.pack("<I", self.square_serial)))
        self.commit(SQUARE_SURFACE, WINDOW_BYTES, SIDE, SIDE)


def ask(request: Any) -> Any:  # noqa: ANN401 # niri's replies are JSON of many shapes.
    """Ask niri over its socket, as `niri msg` does, and give back its answer."""
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as asking:
        asking.connect(os.environ["NIRI_SOCKET"])
        asking.sendall(json.dumps(request).encode() + b"\n")
        reply = json.loads(asking.makefile("rb").readline())
    if "Ok" not in reply:
        sys.exit(f"niri refused {request}: {reply}")
    return reply["Ok"]


def shot(folder: Path, name: str) -> Image.Image:
    """Take a screenshot of the screen over IPC, and read it once niri has written it."""
    path = folder / f"{name}.png"
    ask(
        {
            "Action": {
                "ScreenshotScreen": {
                    "write_to_disk": True,
                    "show_pointer": False,
                    "path": str(path),
                }
            }
        }
    )
    until = time.monotonic() + PATIENCE
    while time.monotonic() < until:
        if path.exists() and path.read_bytes().endswith(b"IEND\xaeB`\x82"):
            with Image.open(path) as image:
                return image.convert("RGB")
        time.sleep(0.02)
    sys.exit(f"niri never wrote the screenshot {path}")


def coloured(image: Image.Image) -> int:
    """How many pixels of an image are the square's colour."""
    colours = image.getcolors(image.width * image.height) or []
    return next((count for count, colour in colours if colour == SQUARE_RGB), 0)


def misplaced(image: Image.Image) -> int:
    """How many pixels show the square's colour outside the square, or lack it inside."""
    inside = coloured(image.crop((LEFT, TOP, LEFT + SIDE, TOP + SIDE)))
    return (coloured(image) - inside) + (SIDE * SIDE - inside)


def settled(folder: Path, name: str) -> Image.Image:
    """Take shots until two in a row are the same, and give back the last."""
    before = shot(folder, f"{name}-0")
    for taken in range(1, SHOTS):
        time.sleep(APART)
        now = shot(folder, f"{name}-{taken}")
        if now.tobytes() == before.tobytes():
            return now
        before = now
    sys.exit(f"the screen never settled in {SHOTS} shots ({name})")


def main() -> None:
    """Switch workspaces with the square shown, and fail on any shot showing it anywhere else."""
    folder = Path(sys.argv[1])
    folder.mkdir(parents=True, exist_ok=True)
    named = os.environ.get("NIRI_FIXED_TO_OUTPUT", "").split(",")[0]
    if not named:
        sys.exit("NIRI_FIXED_TO_OUTPUT names no namespace to fix")
    # Kept to the end: its connection closing would unmap both surfaces.
    stand_in = StandIn(named)

    rest = settled(folder, "rest")
    if misplaced(rest):
        sys.exit(f"before the switch the square isn't alone where it was placed: {folder}")
    ask({"Action": {"FocusWorkspaceDown": {}}})
    shots = [shot(folder, f"switch-{taken}") for taken in range(SHOTS // 4)]
    end = settled(folder, "end")

    failures = []
    partway = 0
    for taken, image in enumerate(shots):
        if wrong := misplaced(image):
            failures.append(
                f"switch-{taken}.png shows the square's colour out of place at {wrong} pixels: "
                "drawn again, or moved with a workspace"
            )
        if image.tobytes() not in {rest.tobytes(), end.tobytes()}:
            partway += 1
    print(f"{partway} of {len(shots)} shots caught the workspaces partway")
    if partway == 0:
        failures.append("no shot caught the workspaces partway, so the switch was never judged")
    if misplaced(end):
        failures.append("after the switch the square isn't alone where it was placed")
    stand_in.socket.close()
    if failures:
        sys.exit("\n".join(failures))


if __name__ == "__main__":
    main()

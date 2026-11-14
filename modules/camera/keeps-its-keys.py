"""Press Escape, q and then ZZ in a viewfinder once it plays, and say whether only ZZ closed it.

It first reads the window's title, which says what Return makes, and Tab's change to it.

The check in checks.nix starts the viewfinder with mpv's IPC server at the path given, beside a
player script that takes Z if it's loaded (stand-in-player.lua). The keys are pressed once the
camera shows, and once that script says it has bound Z, if it's there to say so. The viewfinder
staying open through Escape and q, and closing on ZZ, says the keys were camera.lua's own.
"""

from __future__ import annotations

import json
import socket
import sys
import time
from pathlib import Path

# How long the viewfinder has to start and to show the camera, which a
# loaded builder stretches.
STARTING = 60.0
# How long a player script that's loaded has to bind Escape once the
# camera shows: it binds it as the camera's file loads, so it's said by
# then, and only a viewfinder that loaded none waits this out.
BINDING = 5.0
# How long the viewfinder has to close once ZZ is pressed.
CLOSING = 10.0
# How long the viewfinder is watched staying open after Escape and q: they
# are heard in moments, and a close would come as soon.
STAYING = 1.0
# The id the window's title is observed under, apart from the camera's.
TITLE = 2


def connected(path: Path) -> socket.socket:
    """Connect to mpv's IPC server once it's there."""
    deadline = time.monotonic() + STARTING
    while True:
        try:
            ipc = socket.socket(socket.AF_UNIX)
            ipc.connect(str(path))
        except OSError:
            ipc.close()
            if time.monotonic() > deadline:
                sys.exit(f"the viewfinder never opened {path}")
            time.sleep(0.1)
        else:
            return ipc


def heard(ipc: socket.socket, until: float) -> list[dict[str, object]] | None:
    """Give what mpv said by then, or None once it's gone."""
    ipc.settimeout(max(0.01, until - time.monotonic()))
    try:
        data = ipc.recv(65536)
    except TimeoutError:
        return []
    if not data:
        return None
    return [json.loads(line) for line in data.decode().splitlines() if line.strip()]


def said(message: dict[str, object], words: str) -> bool:
    """Say whether this is a script's message of these words."""
    return message.get("event") == "client-message" and message.get("args") == [words]


def ready(ipc: socket.socket) -> None:
    """Wait until the camera shows, and the player's script has bound Z if it's there."""
    ipc.sendall(b'{"command":["observe_property",1,"video-params"]}\n')
    showing = time.monotonic() + STARTING
    bound = False
    shown_at = None
    while True:
        until = showing if shown_at is None else shown_at + BINDING
        messages = heard(ipc, until)
        if messages is None:
            sys.exit("the viewfinder closed before a key was pressed")
        for message in messages:
            if message.get("event") == "property-change" and message.get("data"):
                shown_at = shown_at or time.monotonic()
            bound = bound or said(message, "player-bound")
        if shown_at is None and time.monotonic() > showing:
            sys.exit("the viewfinder never showed the camera")
        if shown_at is not None and (bound or time.monotonic() > shown_at + BINDING):
            return


def titled(ipc: socket.socket, want: str) -> None:
    """Wait until the window's title is this, or fail saying what it was."""
    ipc.sendall(json.dumps({"command": ["observe_property", TITLE, "title"]}).encode() + b"\n")
    deadline = time.monotonic() + BINDING
    last = None
    while time.monotonic() < deadline:
        messages = heard(ipc, deadline)
        if messages is None:
            sys.exit("the viewfinder closed before its title was read")
        for message in messages:
            if message.get("event") == "property-change" and message.get("id") == TITLE:
                last = message.get("data")
                if last == want:
                    ipc.sendall(
                        json.dumps({"command": ["unobserve_property", TITLE]}).encode() + b"\n"
                    )
                    return
    sys.exit(f"the viewfinder was titled {last!r}, not {want!r}")


def main() -> None:
    """Read its title and Tab's, press Escape and q, then ZZ; fail unless only ZZ closes it."""
    ipc = connected(Path(sys.argv[1]))
    ready(ipc)
    titled(ipc, "Photo \u00b7 Camera")
    ipc.sendall(b'{"command":["keypress","TAB"]}\n')
    titled(ipc, "Video \u00b7 Camera")
    ipc.sendall(b'{"command":["keypress","ESC"]}\n{"command":["keypress","q"]}\n')
    staying = time.monotonic() + STAYING
    while time.monotonic() < staying:
        if heard(ipc, staying) is None:
            sys.exit("Escape or q closed the viewfinder")
    ipc.sendall(b'{"command":["keypress","Z"]}\n{"command":["keypress","Z"]}\n')
    closing = time.monotonic() + CLOSING
    while time.monotonic() < closing:
        messages = heard(ipc, closing)
        if messages is None:
            return
        if any(said(message, "player-took-z") for message in messages):
            sys.exit("Z went to the player's script, so the viewfinder stayed open")
    sys.exit("the viewfinder stayed open after ZZ")


if __name__ == "__main__":
    main()

"""Press Escape in a viewfinder once it plays, and say whether the viewfinder closed.

The check in checks.nix starts the viewfinder with mpv's IPC server at the path given, beside a
player script that takes Escape if it's loaded (stand-in-player.lua). Escape is pressed once the
camera shows, and once that script says it has bound the key, if it's there to say so. The
viewfinder closing on it says Escape was camera.lua's own.
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
# How long the viewfinder has to close once Escape is pressed.
CLOSING = 10.0


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


def main() -> None:
    """Press Escape once the viewfinder is ready, and fail unless it closes."""
    ipc = connected(Path(sys.argv[1]))
    ipc.sendall(b'{"command":["observe_property",1,"video-params"]}\n')
    showing = time.monotonic() + STARTING
    bound = False
    shown_at = None
    while True:
        until = showing if shown_at is None else shown_at + BINDING
        messages = heard(ipc, until)
        if messages is None:
            sys.exit("the viewfinder closed before Escape was pressed")
        for message in messages:
            if message.get("event") == "property-change" and message.get("data"):
                shown_at = shown_at or time.monotonic()
            bound = bound or said(message, "player-bound")
        if shown_at is None and time.monotonic() > showing:
            sys.exit("the viewfinder never showed the camera")
        if shown_at is not None and (bound or time.monotonic() > shown_at + BINDING):
            break
    ipc.sendall(b'{"command":["keypress","ESC"]}\n')
    closing = time.monotonic() + CLOSING
    while time.monotonic() < closing:
        messages = heard(ipc, closing)
        if messages is None:
            return
        if any(said(message, "player-took-escape") for message in messages):
            sys.exit("Escape went to the player's script, so the viewfinder stayed open")
    sys.exit("the viewfinder stayed open after Escape")


if __name__ == "__main__":
    main()

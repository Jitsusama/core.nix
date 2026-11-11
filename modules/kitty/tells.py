"""Tell the programs daemon that what kitty has open changed, as kitty's global watcher.

kitty calls these as its tabs, its windows and what runs in them change, and
each says so to the programs daemon over its socket (the `changed` ask,
../programs/src/main.rs), so the launcher's page of kitty reads what every
kitty has open only once it moved, rather than asking each kitty every time
the launcher opens (Y2YBTS, 2026-11-10: "the daemon should hear what's open as
it changes"). What a Neovim in a kitty window holds is still asked of it as the
page opens, over its own server, since Joel's Neovim is left as he set it.

kitty.conf names it with `watcher`, and kitty runs it on the thread it draws on,
where nothing may hold kitty up: the socket is never waited on, and a daemon
that isn't there, as on a Mac, is said once on kitty's standard error and passed
over after.
"""

import json
import os
import socket
import sys
from pathlib import Path
from typing import Any

# What every kitty has open, as the programs daemon keys its list (home.nix).
KITTY = "listed:kitty:open"

# Whether a daemon not there was said yet, so it's said once.
unheard: list[str] = []


def tell() -> None:
    """Tell the daemon kitty's list changed, at once and never waiting."""
    runtime = os.environ.get("XDG_RUNTIME_DIR")
    if not runtime:
        return
    line = json.dumps({"id": "kitty", "ask": "changed", "keys": [KITTY]}) + "\n"
    try:
        with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as daemon:
            daemon.settimeout(0)
            daemon.connect(str(Path(runtime) / "daemons" / "programs.sock"))
            daemon.send(line.encode())
    except OSError as why:
        if not unheard:
            unheard.append(str(why))
            said = f"kitty: the programs daemon didn't hear what's open changed: {why}"
            print(said, file=sys.stderr)


def on_tab_bar_dirty(_boss: object, _window: object, _data: dict[str, Any]) -> None:
    """Tell of a tab made, moved, closed or retitled."""
    tell()


def on_resize(_boss: object, _window: object, data: dict[str, Any]) -> None:
    """Tell of a window made, which kitty says as its first resize, from nothing."""
    if getattr(data.get("old_geometry"), "xnum", 1) == 0:
        tell()


def on_focus_change(_boss: object, _window: object, data: dict[str, Any]) -> None:
    """Tell of another window in front.

    Only a window taking focus tells: one losing it to the launcher as it opens
    leaves it the window used last, which is what the page leads with, so the
    launcher opening reads nothing again.
    """
    if data.get("focused"):
        tell()


def on_title_change(_boss: object, _window: object, _data: dict[str, Any]) -> None:
    """Tell of a window retitled, as each command and each folder retitle it."""
    tell()


def on_cmd_startstop(_boss: object, _window: object, _data: dict[str, Any]) -> None:
    """Tell of what runs in front in a window starting or stopping."""
    tell()


def on_set_user_var(_boss: object, _window: object, _data: dict[str, Any]) -> None:
    """Tell of a variable a program in a window said, as a Neovim says its server."""
    tell()


def on_close(_boss: object, _window: object, _data: dict[str, Any]) -> None:
    """Tell of a window closed."""
    tell()

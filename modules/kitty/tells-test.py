"""Run kitty's watcher (tells.py) as kitty would, in kitty's own Python, for checks.nix.

With no daemon listening, a change is said once on standard error and never
raised into kitty. With one listening, each change kitty calls the watcher with
is one `changed` ask naming kitty's list: a tab, a variable said, a window made,
focused or closed; a resize of a window already there, and a window losing
focus, as it does to the launcher opening, tell nothing.

kitty runs its Python with asserts stripped, so each thing that has to hold
says what was wrong as it ends the run, rather than asserting it.

usage: kitty +launch tells-test.py TELLS.PY
"""

import importlib.util
import json
import os
import socket
import sys
import tempfile
from pathlib import Path
from types import ModuleType, SimpleNamespace

KITTY = "listed:kitty:open"


def watcher(path: str) -> ModuleType:
    """Load the watcher from its file, as kitty does."""
    spec = importlib.util.spec_from_file_location("tells", path)
    if spec is None or spec.loader is None:
        why = f"{path} isn't a module kitty can load"
        raise SystemExit(why)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def heard(daemon: socket.socket) -> list[object]:
    """Every ask told while it listened, a connection each, in order."""
    daemon.settimeout(0)
    asks: list[object] = []
    while True:
        try:
            connection, _ = daemon.accept()
        except BlockingIOError:
            return asks
        with connection, connection.makefile() as lines:
            asks.extend(json.loads(line) for line in lines)


def main() -> None:
    tells = watcher(sys.argv[1])
    with tempfile.TemporaryDirectory() as runtime:
        os.environ["XDG_RUNTIME_DIR"] = runtime
        tells.on_close(None, None, {})
        tells.on_tab_bar_dirty(None, None, {})
        if len(tells.unheard) != 1:
            why = f"a daemon not there wasn't said once: {tells.unheard}"
            raise SystemExit(why)
        (Path(runtime) / "daemons").mkdir()
        with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as daemon:
            daemon.bind(str(Path(runtime) / "daemons" / "programs.sock"))
            daemon.listen(16)
            tells.on_tab_bar_dirty(None, None, {})
            tells.on_set_user_var(None, None, {"key": "neovim_server", "value": "/run/nvim"})
            tells.on_resize(None, None, {"old_geometry": SimpleNamespace(xnum=80)})
            tells.on_resize(None, None, {"old_geometry": SimpleNamespace(xnum=0)})
            tells.on_focus_change(None, None, {"focused": False})
            tells.on_focus_change(None, None, {"focused": True})
            tells.on_close(None, None, {})
            asks = heard(daemon)
    told = {"id": "kitty", "ask": "changed", "keys": [KITTY]}
    if asks != [told] * 5:
        why = f"the watcher told {asks}"
        raise SystemExit(why)


main()

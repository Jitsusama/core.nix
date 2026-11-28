"""An open launcher collects while nobody's at it, so the next key has no collection in it, on niri.

modules/quickshell/checks/shell.nix runs this through pytest inside tests/lib/nested-niri.sh, with
the shell launcher_on_niri.py starts. The room is left as the launcher is asked to open, so a long
opening grows until Qt's collector starts at the room's edge, wherever the next key is: walking 45
drawers twenty times over, six keys had the collector in them, 107 ms of it, the most 33 ms of a
45 ms key (2026-11-28). Launcher.qml's collectIdle collects instead once nobody's pressed, clicked
or scrolled for 700 ms and the chunks have grown 8 MB since the room was left
(launcher/Collector.js's idleCollectionDue, which tests/pacing.qml holds in Qt's engine).

Here On is opened and its drawers walked, a rest after every few, until a rest says a collection
came; the walk's keys come closer together than 700 ms, so none comes while it walks. The next
key must then give the collector no share, its chunks short of the 8 MB the next collection waits
for. Its share is read from the launcher's framed IPC, the figure a key's line says, since a line
is said only past 8 ms and the keys after a collection here came quicker (2026-11-28).
"""

from __future__ import annotations

import json
import re
import time
from typing import TYPE_CHECKING

import launcher_on_niri

if TYPE_CHECKING:
    from collections.abc import Callable

    import pytest
    from launcher_on_niri import Json, Shell

shell = launcher_on_niri.shell

# Into a row's drawer and back out, as Joel's chords go (Ctrl+W then l or h).
INTO = "wtype -M ctrl -k w -m ctrl -k l"
OUT = "wtype -M ctrl -k w -m ctrl -k h"
# On's rows that draw no drawer, which the walk leaves, and those that do.
UNDRAWN = ("For You", "Location")
DRAWN = ("Time", "Date")
# How many drawers are walked between rests, and at most in all: 45 drawers made 25 to 30 MB of
# chunks in the nested shell, so the 8 MB a collection waits for comes in some 15.
BATCH, MOST = 5, 40
# How long each rest is, past the 700 ms a collection waits for, with the 100 ms its timer ticks.
REST = 1.2
# The line a collection while nobody's at it says (Collector.idleCollectedSaid).
IDLE = re.compile(r"launcher: collected idle in an opening in .*")


def drawer_of(shown: Json) -> Json:
    """Give what the launcher shows of its drawer, empty where it shows none."""
    return shown["drawer"] or {}


def drawer_titled(name: str) -> Callable[[Json], bool]:
    """Give a test passing once the drawer shown is the named row's."""
    return lambda shown: bool(drawer_of(shown).get("shown")) and drawer_of(shown)["title"] == name


def drawers_walked(shell: Shell, count: int) -> None:
    """Go into and back out of so many drawers, from the row the selection isn't on."""
    shown = shell.showing()
    drawn = tuple(row for row in shown["rows"] if row not in UNDRAWN)
    assert drawn == DRAWN, f"On's rows drawing drawers aren't {DRAWN}: {shown['rows']}"
    start = 1 if drawn[0] == (shown["selected"] or {}).get("name") else 0
    for at in range(start, start + count):
        name = drawn[at % len(drawn)]
        shell.down_to(name)
        shell.launcher_until(drawer_titled(name))
        shell.as_account(INTO)
        shell.launcher_until(lambda shown: drawer_of(shown).get("keys"))
        shell.as_account(OUT)
        shell.launcher_until(lambda shown: not drawer_of(shown).get("keys"))


def framed(shell: Shell) -> Json:
    """Give the launcher's last keys' times, as its framed IPC says them."""
    said: Json = json.loads(shell.as_account(shell.ipc("framed")))
    return said


def key_framed(shell: Shell, key: str) -> Json:
    """Press the key, and give what the framed IPC says once its frame alone is drawn."""
    before = int(framed(shell)["timed"])
    shell.send_key(key)
    deadline = time.monotonic() + launcher_on_niri.WAITING.total_seconds()
    while (now := framed(shell))["timed"] == before:
        assert time.monotonic() < deadline, f"the {key} key was never framed:\n{shell.tail()}"
        time.sleep(launcher_on_niri.POLL)
    assert now["timed"] == before + 1, f"{now['timed'] - before} keys framed for one"
    return now


def test_an_idle_opening_collects_between_keys(
    shell: Shell, capsys: pytest.CaptureFixture[str]
) -> None:
    """Walk On's drawers until a rest collects, then see the next key give the collector none."""
    mark = shell.mark()
    shell.as_account(shell.ipc("collection", "on"))
    shell.launcher_until(
        lambda shown: bool(shown["open"] and shown["keys"] and shown["held"][-1:] == ["now"])
    )
    shell.normal()
    collected: list[str] = []
    walked = 0
    while not collected and walked < MOST:
        drawers_walked(shell, BATCH)
        walked += BATCH
        time.sleep(REST)
        collected = IDLE.findall(launcher_on_niri.logged_since(shell, mark))
    assert collected, f"no collection came while idle in {walked} drawers:\n{shell.tail()}"
    with capsys.disabled():
        print(f"after {walked} drawers: {collected[-1]}", flush=True)
    now = key_framed(shell, "up")
    took = now["frames"][-1] - now["heard"][-1]
    collecting = now["collecting"]
    with capsys.disabled():
        print(f"the next key: {took:.1f} ms to its frame, {collecting} ms collecting", flush=True)
    assert collecting == 0, (
        f"the key after an idle collection gave the collector {collecting} ms of its {took:.1f}:"
        f"\n{shell.tail()}"
    )

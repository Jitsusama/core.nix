"""Super+V's paste lands while the shell's thread is busy, on niri.

modules/quickshell/checks.nix runs this through pytest inside tests/nested-niri.sh, with the
shell launcher_on_niri.py starts once a run and the store's own clipboard history beside it.
After an opening, the shell's thread can be busy for seconds, and a paste waited on it twice:
the window pasted into read the copy from Qt's clipboard, which the shell's thread fills, and
the paste keys waited on a timer there before wtype was started to press them. Now the history
holds the copy and presses the keys itself, once niri says the launcher has let go of them, so
here the shell is held stopped, a stand-in for a busy thread, the moment it has asked for the
paste and closed, and the copy still lands in the terminal under within 50 ms of Return, the
bar Joel set for a paste under load (2026-11-10: usable within 50 ms of his chord).
"""

from __future__ import annotations

import json
import os
import signal
import subprocess
import sys
import time
from datetime import timedelta
from pathlib import Path
from typing import TYPE_CHECKING

import launcher_on_niri
import pytest

if TYPE_CHECKING:
    from collections.abc import Callable, Iterator

    from launcher_on_niri import Shell

LOGS = Path(os.environ["LOGS"]).resolve()
# What's copied and then pasted, a phrase no other row holds.
COPIED = "pasted while the shell is busy"
# How soon after Return the paste has to land, in seconds.
BAR = 0.05
# How long the shell is held stopped, at most, waiting for the paste.
HELD = 2.0
# How long niri has to give the launcher's keys back, and the paste to land, at most.
WAITING = timedelta(seconds=10)

# Reads what the terminal it runs in is given, a key at a time, writing each read down with
# the monotonic time it came, which every process on the machine shares.
READER = """
import os, sys, time, tty
tty.setraw(0)
with open(sys.argv[1], "a") as out:
    out.write(f"{time.monotonic()} ready\\n")
    out.flush()
    while True:
        read = os.read(0, 4096).decode(errors="replace")
        out.write(f"{time.monotonic()} {read!r}\\n")
        out.flush()
"""


@pytest.fixture(scope="module")
def shell() -> Iterator[Shell]:
    """Give the shell, started once a run."""
    started = launcher_on_niri.started()
    yield started
    started.stop()


def landed(log: Path) -> list[tuple[float, str]]:
    """Give each read the reader wrote down, with when it came."""
    if not log.exists():
        return []
    reads = []
    for line in log.read_text().splitlines():
        at, _, read = line.partition(" ")
        reads.append((float(at), read))
    return reads


def pasted_at(log: Path) -> float | None:
    """Give when the terminal had been given the whole copy, however many reads it took."""
    given = ""
    for at, read in landed(log):
        given += read
        if COPIED in given:
            return at
    return None


def until(test: Callable[[], object], what: str, timeout: timedelta = WAITING) -> None:
    """Wait, a millisecond at a time, until the test passes."""
    deadline = time.monotonic() + timeout.total_seconds()
    while not test():
        assert time.monotonic() < deadline, f"never {what}"
        time.sleep(0.001)


def launcher_let_go() -> bool:
    """Say whether niri lists the launcher's layer as taking no keys."""
    layers = json.loads(
        subprocess.run(
            ["niri", "msg", "--json", "layers"],  # noqa: S607
            capture_output=True,
            text=True,
            check=True,
        ).stdout
    )
    return all(
        layer["keyboard_interactivity"] == "None"
        for layer in layers
        if layer["namespace"] == "launcher"
    )


def test_a_paste_lands_from_the_history_while_the_shell_is_held(shell: Shell) -> None:
    """A paste lands in the terminal under within 50 ms of Return, the shell held stopped."""
    reader = LOGS / "reader.py"
    reader.write_text(READER)
    log = LOGS / "landed.log"
    shell.as_account(
        f"niri msg action spawn -- foot --title paste-here {sys.executable} {reader} {log}"
    )
    until(lambda: any(read == "ready" for _, read in landed(log)), "had the reader ready")
    until(
        lambda: "paste-here" in shell.as_account("niri msg focused-window"),
        "focused the terminal",
    )
    with shell.told(f"heard copy {COPIED}"):
        shell.as_account(f"printf %s '{COPIED}' | wl-copy")
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    shell.type_in("shell is busy", selecting=COPIED)
    assert shell.showing()["selected"]["verb"] == "paste", shell.showing()["selected"]

    since = shell.mark()
    subprocess.run(["wtype", "-k", "Return"], check=True)  # noqa: S607
    # wtype ends once niri has read its keys, so Return was pressed by now.
    pressed = time.monotonic()

    def asked() -> bool:
        nonlocal since
        reports, since = shell.said_since(since)
        return "pastes into foot" in reports

    until(asked, "asked for the paste")
    until(launcher_let_go, "had the launcher let go of the keys")
    assert shell.process is not None
    shell.process.send_signal(signal.SIGSTOP)
    try:
        until(
            lambda: pasted_at(log) is not None,
            "pasted while the shell was held",
            timedelta(seconds=HELD),
        )
    finally:
        shell.process.send_signal(signal.SIGCONT)
    at = pasted_at(log)
    assert at is not None
    assert at - pressed < BAR, f"the paste landed {(at - pressed) * 1000:.0f} ms after Return"
    print(f"the paste landed {(at - pressed) * 1000:.1f} ms after Return")
    shell.as_account("niri msg action close-window")

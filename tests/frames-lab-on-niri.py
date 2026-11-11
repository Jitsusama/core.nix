"""Lab only: say where each key's frame goes, polish, sync, render and swap, on niri.

Never a check. Qt's render loop says each frame's parts when its timing category is on
(QT_LOGGING_RULES, given by the lab), and each key's span from its press to niri's frame is
printed beside the frames drawn since it was pressed.
"""

from __future__ import annotations

import json
import os
import re
import time
from typing import TYPE_CHECKING, cast

import launcher_on_niri
import pytest

if TYPE_CHECKING:
    from collections.abc import Callable, Iterator

    from launcher_on_niri import Json, Shell

FRAME = re.compile(r"Frame (?:rendered|prepared).*")
ROUNDS = 4
WINDOWS = 4


@pytest.fixture(scope="module")
def shell() -> Iterator[Shell]:
    """Give the shell once it has kept home by itself."""
    started = launcher_on_niri.started()
    started.launcher_until(lambda shown: shown.get("kept"))
    yield started
    started.stop()


def presented(shell: Shell) -> list[Json]:
    """Give each key presented since the launcher opened."""
    return cast("list[Json]", json.loads(shell.as_account(shell.ipc("keysPresented"))))


def timed(shell: Shell, what: str, press: Callable[[], None], say: Callable[[str], None]) -> None:
    """Press, wait for the key's frame, and say its span with the frames drawn meanwhile."""
    count = len(presented(shell))
    mark = shell.mark()
    press()
    deadline = time.monotonic() + launcher_on_niri.WAITING.total_seconds()
    while len(presented(shell)) <= count and time.monotonic() < deadline:
        time.sleep(launcher_on_niri.POLL)
    # Lab only: the frames after the key's are left to be said.
    time.sleep(0.3)
    key = presented(shell)[-1]
    with shell.log.open("rb") as log:
        log.seek(mark)
        said = log.read().decode(errors="replace").splitlines()
    frames = [found.group(0) for found in map(FRAME.search, said) if found]
    framed = json.loads(shell.as_account(shell.ipc("framed")))
    heard = framed["heard"][-1]
    say(
        f"lab-frame {what}: {round(key['at'] - key['from'])} ms presented, handled"
        f" {round(framed['done'][-1] - heard)}, begun {round(framed['begun'][-1] - heard)}"
    )
    for frame in frames[:4]:
        say(f"lab-frame   {frame}")
    for line in said:
        if 'launcher: "' in line or "step parts" in line:
            say(f"lab-frame   {line.strip()}")


def test_frames_say_where_keys_go(shell: Shell, capsys: pytest.CaptureFixture[str]) -> None:
    """Step into places, type at home and move among rows, saying each key's frames."""
    windows = [shell.opened(f"Lab {nth}", app="foot") for nth in range(WINDOWS)]

    def say(words: str) -> None:
        with capsys.disabled():
            print(words, flush=True)

    places = os.environ.get("LAB_STEPS", "Desktop,Machine").split(",")
    for _ in range(ROUNDS):
        shell.as_account(shell.ipc("toggle"))
        shell.launcher_ready()
        shell.normal()
        for name in places:
            shell.down_to(name)
            time.sleep(0.3)
            timed(shell, f"into {name}", lambda: shell.send_key("ret"), say)
            for _ in range(3):
                timed(shell, f"down in {name}", lambda: shell.send_chars("j"), say)
            shell.normal()
            time.sleep(0.3)
            shell.send_chars("-")
            shell.launcher_until(lambda shown: len(shown["held"]) == 0)
            time.sleep(0.5)
            shell.normal()
        shell.on_find()
        for letter in "fir":
            timed(shell, f"typed {letter}", lambda letter=letter: shell.send_chars(letter), say)
        shell.close_launcher()
        shell.launcher_until(lambda shown: not shown["open"])
        time.sleep(1)
    for window in windows:
        shell.close(window)


def test_drawers_say_where_frames_go(shell: Shell, capsys: pytest.CaptureFixture[str]) -> None:
    """Rest on rows in a place, saying each drawer's parts beside the frames drawn for it."""

    def say(words: str) -> None:
        with capsys.disabled():
            print(words, flush=True)

    def rest(name: str) -> None:
        shown = shell.showing()
        at, was = shown["current"], shown["drawer"]["row"]
        count = shown["drawer"]["timed"]["count"]
        if at + 1 >= len(shown["rows"]):
            say(f"lab-drawer {name}: one row")
            return
        mark = shell.mark()
        shell.send_chars("j")

        def drawn(shown: Json) -> bool:
            drawer = shown["drawer"]
            if shown["current"] != at + 1 or drawer["row"] in {"", was}:
                return False
            return not drawer["wanted"] or drawer["timed"]["count"] > count

        shell.launcher_until(drawn)
        # Lab only: the frames after the drawer's are left to be said.
        time.sleep(0.3)
        drawer = shell.showing()["drawer"]
        if not drawer["wanted"]:
            say(f"lab-drawer {name} {drawer['row']}: none")
            return
        timed = drawer["timed"]
        say(
            f"lab-drawer {name} {timed['key']}: took {timed['took']}, making"
            f" {timed['making']}, waiting {timed['waiting']}, drawing {timed['drawing']}"
        )
        with shell.log.open("rb") as log:
            log.seek(mark)
            said = log.read().decode(errors="replace").splitlines()
        for line in said:
            found = FRAME.search(line)
            if found:
                say(f"lab-drawer   {found.group(0)}")

    places = os.environ.get("LAB_DRAWERS", "Programs,Passwords,Machine").split(",")
    for _ in range(ROUNDS):
        shell.as_account(shell.ipc("toggle"))
        shell.launcher_ready()
        for name in places:
            shell.normal()
            shell.down_to(name)
            shell.step_in("ret")
            for _ in range(3):
                rest(name)
            shell.normal()
            time.sleep(0.3)
            shell.send_chars("-")
            shell.launcher_until(lambda shown: len(shown["held"]) == 0)
            time.sleep(0.5)
        shell.close_launcher()
        shell.launcher_until(lambda shown: not shown["open"])
        time.sleep(1)

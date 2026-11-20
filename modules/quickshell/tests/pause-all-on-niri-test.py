"""Pause All Agents asks the safe word's daemon to stop every agent, from the launcher's top.

modules/quickshell/checks/agents.nix runs this through pytest in tests/nested-niri.sh, with the
shell launcher_on_niri.py starts once a run and pineapple's provider (modules/pineapple/
SafeWord.qml) among its own. A stand-in here answers on the daemon's socket as `pineapple serve`
does (modules/pineapple/src/serve.rs): hello, a watch of nothing, and each `pause` asked as a
case says. Found by typing, its first Return arms it and asks nothing; its second asks the daemon
once, `{"ask": "pause", "scope": "all"}`, and the foot says what stopped, or why nothing did.
"""

from __future__ import annotations

import json
import os
import shlex
import socket
import threading
import time
from pathlib import Path
from typing import TYPE_CHECKING

import launcher_on_niri
import pytest

if TYPE_CHECKING:
    from collections.abc import Iterator

    from launcher_on_niri import Json, Shell

NAME = "Pause All Agents"
# What the stand-in answers a pause with, as serve says what it stopped.
STOPPED = "stopping 3 sessions on this machine"
# How long a press has to reach the stand-in, in seconds.
ASKED_WITHIN = 10.0
# How long after the foot speaks a second ask is waited for, in seconds.
ASKED_AGAIN = 1.0
# Past the key timing's wait, 300 ms in modules/keys/language.toml, in seconds.
BOUNCE = 0.5


class StandIn:
    """Answer on pineapple's socket as serve does, keeping each pause asked."""

    def __init__(self, path: Path) -> None:
        """Listen on `path`, answering each pause with `answer` until a case says otherwise."""
        path.parent.mkdir(mode=0o700, exist_ok=True)
        path.unlink(missing_ok=True)
        self.server = socket.socket(socket.AF_UNIX)
        self.server.bind(str(path))
        self.server.listen()
        self.asked: list[Json] = []
        # Every line it was told, so a case that hears no pause says what it heard instead.
        self.heard: list[Json] = []
        self.answer: Json = {"said": STOPPED}
        threading.Thread(target=self.serve, daemon=True).start()

    def serve(self) -> None:
        """Take each connection the shell makes, on a thread of its own."""
        while True:
            taken, _ = self.server.accept()
            threading.Thread(target=self.talk, args=(taken,), daemon=True).start()

    def talk(self, taken: socket.socket) -> None:
        """Answer one connection's lines until the shell lets it go."""
        with taken, taken.makefile("rw") as lines:
            for line in lines:
                said = json.loads(line)
                answer = self.answered(said)
                # A connection that goes with the ask unanswered, as serve's would if it
                # ended mid-stop.
                if answer.get("drop"):
                    return
                lines.write(json.dumps(answer) + "\n")
                lines.flush()

    def answered(self, said: Json) -> Json:
        """Give what serve would answer one line."""
        self.heard.append(said)
        if said.get("ask") == "hello":
            return {"id": said.get("id"), "state": "answering"}
        if said.get("ask") == "watch":
            return {"id": said.get("id"), "records": []}
        if said.get("ask") == "pause":
            self.asked.append({key: value for key, value in said.items() if key != "id"})
            return {"id": said.get("id"), **self.answer}
        return {"id": said.get("id")}


@pytest.fixture(scope="module")
def stand_in() -> StandIn:
    """Give the stand-in, listening before the shell starts."""
    return StandIn(Path(os.environ["XDG_RUNTIME_DIR"]) / "daemons" / "pineapple.sock")


@pytest.fixture(scope="module")
def shell(stand_in: StandIn) -> Iterator[Shell]:
    """Give the shell, started once a run after the stand-in."""
    assert stand_in.asked == [], stand_in.asked
    with launcher_on_niri.running() as started:
        yield started


def pressed_twice(shell: Shell, stand_in: StandIn) -> None:
    """Find Pause All Agents, arm it with Return, asking nothing, then Return again."""
    shell.answer("pause all")
    shell.launcher_until(lambda shown: NAME in shown["rows"])
    shell.down_to(NAME)
    shell.send_key("ret")
    shell.launcher_until(
        lambda shown: shown["armed"] == "again pauses every agent" and shown["armedRow"] != ""
    )
    assert stand_in.asked == [], f"a first Return asked {stand_in.asked}"
    # A second Return within the key timing's wait is the first bounced, which does nothing
    # (Arming.js's pressed), so the second is a press of its own.
    time.sleep(BOUNCE)
    shell.send_key("ret")
    deadline = time.monotonic() + ASKED_WITHIN
    while not stand_in.asked:
        assert time.monotonic() < deadline, (
            f"a second Return asked nothing; the stand-in heard {stand_in.heard}, and the foot"
            f" said {foot(shell.showing())!r}:\n{shell.tail()}"
        )
        time.sleep(0.1)


def foot(shown: Json) -> str:
    """Give what the foot says in place of its keys, as one line."""
    return " ".join(shown["saying"]).replace("\xa0", " ")


def test_pause_all_agents_asks_once_on_a_second_return_and_says_what_stopped(
    shell: Shell, stand_in: StandIn
) -> None:
    """Its first Return arms it; its second asks the daemon once, and the foot says what stopped.

    WCJQSG, 2026-11-19 [1c8863e1], for Joel's demo: every agent stopped from the launcher.
    """
    stand_in.asked.clear()
    stand_in.answer = {"said": STOPPED}
    pressed_twice(shell, stand_in)
    shell.launcher_until(lambda shown: STOPPED in foot(shown))
    # Read again a while after, so a second ask still on its way is counted too.
    time.sleep(ASKED_AGAIN)
    assert stand_in.asked == [{"ask": "pause", "scope": "all"}], stand_in.asked
    shell.close_launcher()


def test_pause_all_agents_says_why_the_daemon_stopped_nothing(
    shell: Shell, stand_in: StandIn
) -> None:
    """A refusal is said in the foot in the vocabulary's words for its code, not the daemon's."""
    stand_in.asked.clear()
    stand_in.answer = {"error": {"code": "not-joel", "message": "asked from elsewhere"}}
    pressed_twice(shell, stand_in)
    shell.launcher_until(
        lambda shown: f"{NAME} didn't take: only your own launcher may act on a stop" in foot(shown)
    )
    assert "asked from elsewhere" not in foot(shell.showing()), shell.showing()
    shell.close_launcher()


def test_pause_all_agents_says_a_serve_that_cant_pause_cant_yet(
    shell: Shell, stand_in: StandIn
) -> None:
    """A code the words don't know, as a serve too old to pause answers, isn't said as written."""
    stand_in.asked.clear()
    stand_in.answer = {"error": {"code": "unknown-ask", "message": "no ask pause"}}
    pressed_twice(shell, stand_in)
    shell.launcher_until(
        lambda shown: f"{NAME} didn't take: this machine can't do that yet" in foot(shown)
    )
    shell.close_launcher()


def test_pause_all_agents_lost_answer_points_at_for_you(shell: Shell, stand_in: StandIn) -> None:
    """An ask whose connection goes unanswered may have stopped them, so it says to look."""
    stand_in.asked.clear()
    stand_in.answer = {"drop": True}
    pressed_twice(shell, stand_in)
    shell.launcher_until(
        lambda shown: "its answer was lost, so For You says whether they stopped" in foot(shown)
    )
    stand_in.answer = {"said": STOPPED}
    shell.close_launcher()


def test_pause_all_agents_walked_from_outside_asks_nothing(shell: Shell, stand_in: StandIn) -> None:
    """A terminal's `launcher do`, sure, finds it nowhere: serve can't tell an agent from Joel."""
    stand_in.asked.clear()
    stand_in.answer = {"said": STOPPED}
    asked = json.dumps({"steps": [NAME], "sure": True})
    answer = json.loads(shell.as_account(shell.ipc("doing", shlex.quote(asked))))
    time.sleep(ASKED_AGAIN)
    assert stand_in.asked == [], f"a walk from outside asked {stand_in.asked}: {answer}"
    assert answer.get("missing") == NAME, answer

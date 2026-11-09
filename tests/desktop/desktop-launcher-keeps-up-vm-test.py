"""The desktop test's launcher keeps up area.

Here an arrow is held down in the launcher at 30 a second, at Joel's two scales, while about as
many sessions and people as his machine knows keep changing: 300 stand-in sessions, a few
changing each beat, and 2,000 people said anew every 0.3 s. Joel felt it (2026-11-08 19:58):
"weird lag can happen, and it just feels hacky and without polish", and one opening on his shell
made 6,933 rows, most for places nobody looked at. In each place the source nobody sees makes no
rows, the list refreshes at most four times a second, each key's own handler is done within
design.md's 4 ms, and the keys reach their frames as soon as with the changes frozen, but for
those 4 ms. Why a source's rows are wanted is proven in Qt's own engine, in tests/pacing.qml.
tests/desktop.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), and its
opening comment says what the test proves. Every test asks for the desktop, started once a run
(desktop_session.py), and leaves it as it found it.
"""

from __future__ import annotations

import json
import math
from datetime import timedelta
from typing import TYPE_CHECKING, Any, cast

import desktop_session
import pytest
from desktop_session import LAUNCHER_OPEN, PI_PRESENCE, retry

if TYPE_CHECKING:
    from collections.abc import Iterator

    from desktop_session import Desktop, Json
    from test_driver.machine import BaseMachine

# Joel's scales.
JOELS_SCALES = ("2", "1.5")
# Where the stand-in keyboard is told what to press, and says when it did (desktop/keyboard.py).
KEYBOARD = "/run/keyboard"
SAID = "/run/keyboard.log"
# An arrow held down, as niri repeats it: 30 a second.
REPEAT = 1 / 30
# How many keys a turn presses, about three seconds' worth, and how many turns, the changes
# running and frozen in turn. The VM draws in software on a builder shared with other checks,
# which slows a key to its frame by whatever else it runs; it slows both kinds of turn alike, so
# what the changes cost the keys is what's left between them.
KEYS = 90
TURNS = 4
# As many sessions as a busy day of Joel's, a few changing each beat as his do (2026-11-08: his
# openings heard Agents change 27 to 75 times in their first three seconds), and as many people
# as his People knows, said anew each round as Slack's answers say them.
SESSIONS = 300
CHANGING = 10
BEAT = 0.3
PEOPLE = 2000
CROWD = "/run/user/1000/crowd"
# What design.md holds the launcher to: a key's own work within 4 ms, and its frame within one
# frame of its press. One frame here is what the same keys take with the changes frozen.
HANDLED = 4.0
FRAME_MORE = 4.0
TAIL = 0.95
# How often the list may be made again while things change: its pace (Launcher.qml's Pacing).
REFRESHES = 4.0
# Each place an arrow is held in, and the source it doesn't show: home names People but says
# how the sessions stand itself, and each place shows only its own.
PLACES = {"home": "agents", "Agents": "people", "People": "agents"}
# How long the last key may take to reach its frame before the test says it never did.
REACHING = timedelta(seconds=30)
# How long freezing or thawing the changes may take before the test says it couldn't.
FREEZING = timedelta(minutes=3)


@pytest.fixture(scope="module")
def desktop(machine: BaseMachine, values: dict[str, Any]) -> Desktop:
    """Give the desktop, started once a run."""
    return desktop_session.started(machine, cast("desktop_session.Values", values))


@pytest.fixture(scope="module")
def keyboard(desktop: Desktop) -> Iterator[None]:
    """Start the stand-in keyboard and its repeat, and wait until the desktop reads it."""
    machine = desktop.machine
    machine.succeed(f"systemd-run --unit=keyboard {desktop.values['keyboard']}")
    machine.wait_for_file(KEYBOARD, timeout=timedelta(seconds=30))
    # Cage, which holds niri here, reads the machine's input devices and hands their keys on, so
    # it's Cage that opens the keyboard; a press before then would be lost.
    node = machine.succeed("cat /run/keyboard.device").strip()
    held = f"find /proc/$(pgrep -o -f bin/cage)/fd -lname {node} | grep -q ."
    machine.wait_until_succeeds(held, timeout=timedelta(seconds=30))
    # A held key, pressed again each repeat, from inside the machine, since the driver's own
    # commands take longer than a repeat each.
    machine.succeed(
        "cat > /tmp/hold.sh <<'EOF'\n"
        f'while read -r key; do echo "$key" > {KEYBOARD}; sleep {REPEAT:.3f}; done < /tmp/keys\n'
        "EOF"
    )
    try:
        yield
    finally:
        machine.succeed("systemctl stop keyboard")


@pytest.fixture(scope="module")
def changing(desktop: Desktop) -> Iterator[None]:
    """Keep the stand-in sessions and people changing, as user units a turn can freeze."""
    machine = desktop.machine
    machine.succeed(
        "cat > /tmp/crowd-load.sh <<'EOF'\n"
        f"i=0; while [ -e {CROWD}-going ]; do i=$((i + 1));"
        f' echo "{PEOPLE} $i" > {CROWD}.new && mv {CROWD}.new {CROWD}; sleep {BEAT}; done\n'
        "EOF"
    )
    since = desktop.mark()
    sessions = f"{desktop.values['stand_in_sessions']} {PI_PRESENCE} {SESSIONS} {CHANGING} {BEAT}"
    desktop.as_account(f"systemd-run --user --collect --quiet --unit=stand-in-sessions {sessions}")
    # As a unit of the account's own, since a loop started from su's session ends with it.
    desktop.as_account(
        f"touch {CROWD}-going && systemd-run --user --collect --quiet --unit=crowd-load"
        " sh /tmp/crowd-load.sh"
    )
    try:
        desktop.reported("crowd ", lambda rest: rest.startswith(f"{PEOPLE} "), since)
        yield
    finally:
        desktop.as_account("systemctl --user stop stand-in-sessions")
        # Quiet again, and known no more, so no later test finds the crowd.
        desktop.as_account(f"rm -f {CROWD}-going")
        desktop.as_account(f"echo '0 0' > {CROWD}.new && mv {CROWD}.new {CROWD}")


def framed(desktop: Desktop) -> Json:
    """Give how many keys the launcher has timed, and when each of the last was done and drawn."""
    return cast("Json", json.loads(desktop.as_account("qs ipc call launcher framed")))


def made(desktop: Desktop) -> Json:
    """Give how many rows each source has made, and how often the open launcher refreshed."""
    return cast("Json", json.loads(desktop.as_account("qs ipc call launcher made")))


def ranked(times: list[float], share: float) -> float:
    """Give the time the share of them falls within."""
    ordered = sorted(times)
    return ordered[max(0, math.ceil(share * len(ordered)) - 1)]


class Turns:
    """Turns of a held arrow in one place, the changes running and frozen in turn."""

    def __init__(self, desktop: Desktop, unseen: str) -> None:
        """Begin with nothing timed, the changes running as the fixture started them."""
        self.desktop = desktop
        self.unseen = unseen
        self.frozen = False
        self.at = 0
        self.step = 1
        self.unseen_made = 0
        self.seen_made: list[int] = []
        self.refreshes = 0
        self.seconds = 0.0
        self.handled: list[float] = []
        self.under: list[float] = []
        self.free: list[float] = []

    def freeze(self, *, frozen: bool) -> None:
        """Freeze the stand-ins where they stand, or let them change again."""
        verb = "freeze" if frozen else "thaw"
        wait = f"SYSTEMD_BUS_TIMEOUT={round(FREEZING.total_seconds())}"
        self.desktop.as_account(f"{wait} systemctl --user {verb} stand-in-sessions crowd-load")
        self.frozen = frozen

    def keys(self, rows: int) -> list[str]:
        """Give a turn's keys, down and up the rows, turning before an end a key couldn't pass."""
        down, up = self.desktop.row_keys()
        pressed = []
        for _ in range(KEYS):
            if not 0 <= self.at + self.step < rows:
                self.step = -self.step
            pressed.append(down if self.step > 0 else up)
            self.at += self.step
        return pressed

    def turn(self) -> None:
        """Hold the arrow through one turn, and keep what it made and how soon it drew."""
        desktop, machine = self.desktop, self.desktop.machine
        shown = desktop.showing()
        rows, self.at = shown["count"], max(0, shown["current"])
        assert rows > 1, f"{shown['path']} shows {rows} rows, too few to move through"
        pressed = self.keys(rows)
        said = int(machine.succeed(f"wc -l < {SAID}").strip())
        timed = framed(desktop)["timed"]
        before = made(desktop)
        machine.succeed(f"printf '%s\\n' {' '.join(pressed)} > /tmp/keys && sh /tmp/hold.sh")
        noted: Json = {}

        def drawn(_: bool) -> bool:
            noted.update(framed(desktop))
            return bool(noted["timed"] >= timed + KEYS)

        try:
            retry(drawn, timeout=REACHING)
        except AssertionError:
            pytest.fail(f"{timed + KEYS - noted['timed']} of {KEYS} keys never reached a frame")
        after = made(desktop)
        assert noted["timed"] == timed + KEYS, (
            f"the launcher timed {noted['timed'] - timed} keys of {KEYS} pressed"
        )
        machine.wait_until_succeeds(
            f"[ $(wc -l < {SAID}) -ge {said + KEYS} ]", timeout=timedelta(seconds=10)
        )
        downs = [
            float(line.split()[1])
            for line in machine.succeed(f"tail -n +{said + 1} {SAID}").splitlines()[:KEYS]
        ]
        times = zip(
            downs,
            noted["heard"][-KEYS:],
            noted["done"][-KEYS:],
            noted["frames"][-KEYS:],
            strict=True,
        )
        for down, heard, done, frame in times:
            self.handled.append(done - heard)
            (self.free if self.frozen else self.under).append(frame - down)
        if self.frozen:
            return
        rows_made = after["rows"]
        self.unseen_made += rows_made.get(self.unseen, 0) - before["rows"].get(self.unseen, 0)
        seen = "people" if self.unseen == "agents" else "agents"
        self.seen_made.append(rows_made.get(seen, 0) - before["rows"].get(seen, 0))
        self.refreshes += after["refreshes"] - before["refreshes"]
        self.seconds += downs[-1] - downs[0] + REPEAT * 1000

    def held(self, where: str, scale: str) -> None:
        """Say how the turns went, and fail a launcher that didn't keep up."""
        rate = self.refreshes / (self.seconds / 1000)
        slowest = max(self.handled)
        under, free = ranked(self.under, TAIL), ranked(self.free, TAIL)
        print(
            f"in {where} at {scale}, while things changed {self.unseen} made "
            f"{self.unseen_made} rows unseen and the shown source {self.seen_made} a turn; "
            f"{self.refreshes} refreshes in {self.seconds / 1000:.1f} s, {rate:.1f} a second; "
            f"handlers half within {ranked(self.handled, 0.5):.1f} ms, 95% within "
            f"{ranked(self.handled, TAIL):.1f} ms, the slowest {slowest:.1f} ms; keys to their "
            f"frames, 95% within {under:.0f} ms changing against {free:.0f} ms frozen, half "
            f"within {ranked(self.under, 0.5):.0f} against {ranked(self.free, 0.5):.0f} ms"
        )
        assert self.unseen_made == 0, (
            f"in {where} at {scale}, {self.unseen}, unseen, made {self.unseen_made} rows"
        )
        assert rate <= REFRESHES, f"in {where} at {scale}, the list refreshed {rate:.1f} a second"
        assert slowest <= HANDLED, f"in {where} at {scale}, a key's handler took {slowest:.1f} ms"
        assert under <= free + FRAME_MORE, (
            f"in {where} at {scale}, 95% of keys reached their frames within {under:.0f} ms "
            f"while things changed, against {free:.0f} ms frozen"
        )


@pytest.mark.usefixtures("keyboard", "changing")
@pytest.mark.parametrize("scale", JOELS_SCALES)
@pytest.mark.parametrize("where", list(PLACES))
def test_a_held_arrow_keeps_up_while_sessions_and_people_change(
    desktop: Desktop, where: str, scale: str
) -> None:
    """An arrow held in a place keeps up while sessions and people change, the unseen unmade."""
    machine = desktop.machine
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))
    desktop.as_account(f"niri msg output winit scale {scale}")
    turns = Turns(desktop, PLACES[where])
    desktop.as_account("qs ipc call launcher toggle")
    try:
        desktop.launcher_ready()
        if where != "home":
            desktop.down_to(where)
            desktop.step_in()
            desktop.launcher_until(lambda shown: len(shown["held"]) == 1)
        if where == "Agents":
            listed = desktop.showing()["count"]
            assert listed >= SESSIONS, f"Agents lists {listed} sessions, not the {SESSIONS}"
        for at in range(TURNS):
            turns.freeze(frozen=at % 2 == 1)
            turns.turn()
    finally:
        if turns.frozen:
            turns.freeze(frozen=False)
        desktop.close_launcher()
        desktop.as_account("niri msg output winit scale 1")
    if where == "Agents":
        assert all(seen > 0 for seen in turns.seen_made), (
            f"Agents made {turns.seen_made} rows a turn, as if no session changed"
        )
    turns.held(where, scale)

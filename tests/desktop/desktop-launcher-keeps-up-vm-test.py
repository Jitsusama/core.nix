"""The desktop test's launcher keeps up area.

Here an arrow is held down in the launcher at 30 a second, at Joel's two scales, while about as
many sessions and people as his machine knows keep changing: 300 stand-in sessions, a few
changing each beat, and 2,000 people said anew every 0.3 s. Joel felt it (2026-11-08 19:58):
"weird lag can happen, and it just feels hacky and without polish", and one opening on his shell
made 6,933 rows, most for places nobody looked at. In each place a source nobody sees makes no
rows and the list refreshes at most four times a second; and the keys' handlers and frames take
hardly longer than with the changes frozen, since this machine draws in software, far slower than
Joel's. Why a source's rows are wanted is proven in Qt's own engine, in tests/pacing.qml.
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
# Design.md holds a key's own work within 4 ms, and its frame within one frame of its press. Here
# the VM drawing 2880 by 1800 in software took 6.8 ms a handler at home and 35 ms in Agents at
# the median, and 0.8 to 1.8 s a key to its frame, changes running or frozen (2026-11-09), so the
# changes are judged by what they add, as desktop-keeps-pace-vm-test.py judges its load: nineteen
# handlers in twenty, and the middle key to its frame, at most twice their time frozen and a
# little more. Four runs' handlers in Agents swung from 21 ms faster changing than frozen to 7 ms
# slower, so a bare 4 ms more would fail on the builder's noise alone.
HANDLED_MORE = 4.0
FRAME_MORE = 50.0
SLOWER_MOST = 2
TAIL = 0.95
# How often the list may be made again while things change: its pace (Launcher.qml's Pacing).
REFRESHES = 4.0
# Each place an arrow is held in, and the sources it doesn't show: home names People but says
# how the sessions stand itself, Agents shows only its own, and Programs neither.
PLACES = {"home": ["agents"], "Agents": ["people"], "Programs": ["agents", "people"]}
# The rows the arrow goes down and up, as many as the launcher shows at once, so it never
# scrolls, and a row coming or going below them never leaves a key that moves nothing.
BAND = 8
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

    def __init__(self, desktop: Desktop, unseen: list[str]) -> None:
        """Begin with nothing timed, the changes running as the fixture started them."""
        self.desktop = desktop
        self.unseen = unseen
        self.frozen = False
        self.at = 0
        self.step = 1
        # Rows the unseen sources made while they changed, and rows the changing sources made
        # while frozen, which only the keys could have asked for, by source.
        self.unseen_made: dict[str, int] = {}
        self.keys_made: dict[str, int] = {}
        self.agents_made: list[int] = []
        self.walked: dict[str, int] = {}
        self.refreshes = 0
        self.keyed = 0
        self.seconds = 0.0
        self.handled: dict[bool, list[float]] = {True: [], False: []}
        self.drawn: dict[bool, list[float]] = {True: [], False: []}

    def freeze(self, *, frozen: bool) -> None:
        """Freeze the stand-ins where they stand, or let them change again."""
        verb = "freeze" if frozen else "thaw"
        wait = f"SYSTEMD_BUS_TIMEOUT={round(FREEZING.total_seconds())}"
        self.desktop.as_account(f"{wait} systemctl --user {verb} stand-in-sessions crowd-load")
        self.frozen = frozen

    def keys(self, rows: int) -> list[str]:
        """Give a turn's keys, down and up the band, turning before an end a key couldn't pass."""
        rows = min(rows, BAND)
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
        rows, self.at = shown["count"], min(max(0, shown["current"]), BAND - 1)
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
            self.handled[self.frozen].append(done - heard)
            self.drawn[self.frozen].append(frame - down)

        # Rows a source made in the turn, those made for the walk of where rows live aside, which
        # lays out what it walks once the list is quiet, never on a key (Launcher.qml's
        # livesMade), and is kept apart to say.
        def walked_of(name: str) -> int:
            return int(after["walked"].get(name, 0) - before["walked"].get(name, 0))

        def rows_of(name: str) -> int:
            made_in = after["rows"].get(name, 0) - before["rows"].get(name, 0)
            return int(made_in) - walked_of(name)

        for name in ("agents", "people"):
            self.walked[name] = self.walked.get(name, 0) + walked_of(name)

        # An arrow only moves, so none of it is a key's own change to refresh at once.
        self.keyed += after["keyed"] - before["keyed"]
        if self.frozen:
            for name in ("agents", "people"):
                self.keys_made[name] = self.keys_made.get(name, 0) + rows_of(name)
            return
        for name in self.unseen:
            self.unseen_made[name] = self.unseen_made.get(name, 0) + rows_of(name)
        self.agents_made.append(rows_of("agents"))
        self.refreshes += after["refreshes"] - before["refreshes"]
        self.seconds += downs[-1] - downs[0] + REPEAT * 1000

    def held(self, where: str, scale: str) -> None:
        """Say how the turns went, and fail a launcher that didn't keep up."""
        at = f"in {where} at {scale}"
        rate = self.refreshes / (self.seconds / 1000)
        handled = {frozen: ranked(times, TAIL) for frozen, times in self.handled.items()}
        middle = {frozen: ranked(times, 0.5) for frozen, times in self.drawn.items()}
        print(
            f"{at}: while things changed the unseen made {self.unseen_made} rows and Agents "
            f"{self.agents_made} a turn, and while frozen the keys made {self.keys_made}, "
            f"and the walk of where rows live {self.walked}; "
            f"{self.refreshes} refreshes in {self.seconds / 1000:.1f} s, {rate:.1f} a second, "
            f"{self.keyed} taken on a key's path; handlers 95% within {handled[False]:.1f} ms "
            f"changing against {handled[True]:.1f} ms frozen, the slowest "
            f"{max(self.handled[False]):.1f} against {max(self.handled[True]):.1f} ms; keys to "
            f"their frames half within {middle[False]:.0f} ms changing against "
            f"{middle[True]:.0f} ms frozen, 95% within {ranked(self.drawn[False], TAIL):.0f} "
            f"against {ranked(self.drawn[True], TAIL):.0f} ms"
        )
        assert not any(self.unseen_made.values()), f"{at}, unseen, {self.unseen_made} rows made"
        assert not any(self.keys_made.values()), (
            f"{at}, arrows that only move made {self.keys_made} rows"
        )
        assert self.keyed == 0, f"{at}, {self.keyed} refreshes were taken on a key's path"
        assert rate <= REFRESHES, f"{at}, the list refreshed {rate:.1f} a second"
        assert handled[False] <= SLOWER_MOST * handled[True] + HANDLED_MORE, (
            f"{at}, 95% of handlers took {handled[False]:.1f} ms while things changed, against "
            f"{handled[True]:.1f} ms frozen"
        )
        assert middle[False] <= SLOWER_MOST * middle[True] + FRAME_MORE, (
            f"{at}, half the keys took {middle[False]:.0f} ms to their frames while things "
            f"changed, against {middle[True]:.0f} ms frozen"
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
        assert all(made > 0 for made in turns.agents_made), (
            f"Agents made {turns.agents_made} rows a turn, as if no session changed"
        )
    turns.held(where, scale)

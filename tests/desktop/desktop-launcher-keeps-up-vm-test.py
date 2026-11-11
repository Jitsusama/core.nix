"""The desktop test's launcher keeps up area.

Here an arrow is held down in the launcher at 30 a second, at Joel's two scales, while about as
many sessions and people as his machine knows keep changing: 300 stand-in sessions, a few
changing each beat, and 2,000 people said anew every 0.3 s. Joel felt it (2026-11-08 19:58):
"weird lag can happen, and it just feels hacky and without polish", and one opening on his shell
made 6,933 rows, most for places nobody looked at. In each place a source nobody sees makes no
rows and the list refreshes at most four times a second; and an arrow composes the list no more
often than it refreshes, and draws no row afresh with the changes frozen. Those are counts, since
this machine draws in software on a shared builder, far slower than Joel's, so the keys' times
are said rather than held. Why a source's rows are wanted is proven in Qt's own engine, in
tests/pacing.qml. First, with the launcher shut, Joel's 113 sessions moving their steps and
turning between working and listening cost the shell no sorting until it opens; which changes
those are is proven in tests/pi-source.qml.
tests/desktop.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), and its
opening comment says what the test proves. Every test asks for the desktop, started once a run
(desktop_session.py), and leaves it as it found it.
"""

from __future__ import annotations

import json
import math
import time
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
# What the stand-ins wrote just before they froze reached the shell up to 2.5 s after, so a
# frozen turn's arrows were blamed for the rows it made: 40 of Agents' at 2 and 220 at 1.5 (a
# whole run and a diagnostic one, 2026-11-09). A frozen turn begins once the shell has sorted
# pi's sessions no more for longer than that, and gives up after so many looks.
UNHEARD = 3.0
LOOKS = 10
# As many sessions as a busy day of Joel's, a few changing each beat as his do (2026-11-08: his
# openings heard Agents change 27 to 75 times in their first three seconds), and as many people
# as his People knows, said anew each round as Slack's answers say them.
SESSIONS = 300
CHANGING = 10
BEAT = 0.3
PEOPLE = 2000
# As many sessions as Joel's on 2026-11-09, one moving its step and turning between working and
# listening each beat, and how many such moves the shut launcher hears before saying it took
# none.
WORKING = 113
STEPS = 20
CROWD = "/run/user/1000/crowd"
# Design.md holds a key's own work within 4 ms, and its frame within one frame of its press. Here
# the VM drawing 2880 by 1800 in software took 6.8 ms a handler at home and 35 ms in Agents at
# the median, and 0.8 to 1.8 s a key to its frame (2026-11-09), and held to twice their frozen
# times it failed on the builder alone: in Programs at 2 the middle key took 2,246 ms to its frame
# changing against 1,032 frozen, with one refresh and no row made in the turn and handlers alike
# (112.6 against 105.1 ms), and at home at 2 a stalled frozen turn's frames took 8.8 s (net and
# #2602, 2026-11-09). So the times are said, nineteen in twenty and the middle, and what the
# changes cost the keys is held by what the launcher did instead.
TAIL = 0.95
# How often the list may be made again while things change: its pace (Launcher.qml's Pacing).
# This and taking no change on a key's path held on main too once #2495 landed, so they guard
# against a return rather than prove this change.
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


def made(desktop: Desktop) -> tuple[Json, float, float]:
    """Give how many rows each source has made and how often the open launcher refreshed.

    With them come the machine's uptime in seconds just before it was asked and just after it
    answered, so a span between two of them holds every refresh counted between them.
    """
    said = desktop.as_account("cat /proc/uptime; qs ipc call launcher made; cat /proc/uptime")
    asked, answer, answered = said.strip().splitlines()
    return (
        cast("Json", json.loads(answer)),
        float(asked.split()[0]),
        float(answered.split()[0]),
    )


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
        # Refreshes while things changed, counted from before a turn's keys until its last key's
        # frame, and the seconds and turns they were counted over. Divided by the keys' span
        # alone, the frames still coming after it counted at home at 2 as 4.5 a second, though no
        # two refreshes began within the pace's quarter second (net, a99645168, 2026-11-09).
        self.refreshes = 0
        self.window = 0.0
        self.turns = 0
        # The keys' own span, said beside it to show how far the frames run past it.
        self.pressing = 0.0
        self.keyed = 0
        self.handled: dict[bool, list[float]] = {True: [], False: []}
        self.drawn: dict[bool, list[float]] = {True: [], False: []}
        # How often the list was composed, refreshed and drew a row afresh, frozen and not.
        self.composed: dict[bool, int] = {True: 0, False: 0}
        self.refreshed: dict[bool, int] = {True: 0, False: 0}
        self.afresh: dict[bool, int] = {True: 0, False: 0}

    def freeze(self, *, frozen: bool) -> None:
        """Freeze the stand-ins where they stand, or let them change again."""
        verb = "freeze" if frozen else "thaw"
        wait = f"SYSTEMD_BUS_TIMEOUT={round(FREEZING.total_seconds())}"
        self.desktop.as_account(f"{wait} systemctl --user {verb} stand-in-sessions crowd-load")
        self.frozen = frozen
        if frozen:
            self.settle()

    def settle(self) -> None:
        """Wait until the shell has heard the last of what the stand-ins wrote before freezing."""
        heard = sorts(self.desktop)["heard"]
        for _ in range(LOOKS):
            time.sleep(UNHEARD)
            now = sorts(self.desktop)["heard"]
            if now == heard:
                return
            heard = now
        pytest.fail(f"the shell went on sorting pi's sessions {LOOKS * UNHEARD} s after a freeze")

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
        before, since, _ = made(desktop)
        machine.succeed(f"printf '%s\\n' {' '.join(pressed)} > /tmp/keys && sh /tmp/hold.sh")
        noted: Json = {}

        def drawn(_: bool) -> bool:
            noted.update(framed(desktop))
            return bool(noted["timed"] >= timed + KEYS)

        try:
            retry(drawn, timeout=REACHING)
        except AssertionError:
            pytest.fail(f"{timed + KEYS - noted['timed']} of {KEYS} keys never reached a frame")
        after, _, until = made(desktop)
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

        # Rows a source made in the turn, those Ctrl+]'s looks for where a row lives made aside
        # (Launcher.qml's livesMade), kept apart to say: no arrow presses Ctrl+], so a look's row
        # is one made with nobody asking.
        def walked_of(name: str) -> int:
            return int(after["walked"].get(name, 0) - before["walked"].get(name, 0))

        def rows_of(name: str) -> int:
            made_in = after["rows"].get(name, 0) - before["rows"].get(name, 0)
            return int(made_in) - walked_of(name)

        for name in ("agents", "people"):
            self.walked[name] = self.walked.get(name, 0) + walked_of(name)

        # An arrow only moves, so none of it is a key's own change to refresh at once, nor a
        # composing of the list but a refresh's, nor a row drawn afresh but for one that came.
        self.keyed += after["keyed"] - before["keyed"]
        self.composed[self.frozen] += after["composed"] - before["composed"]
        self.refreshed[self.frozen] += after["refreshes"] - before["refreshes"]
        self.afresh[self.frozen] += after["drawn"] - before["drawn"]
        if self.frozen:
            for name in ("agents", "people"):
                self.keys_made[name] = self.keys_made.get(name, 0) + rows_of(name)
            return
        for name in self.unseen:
            self.unseen_made[name] = self.unseen_made.get(name, 0) + rows_of(name)
        self.agents_made.append(rows_of("agents"))
        self.refreshes += after["refreshes"] - before["refreshes"]
        self.window += until - since
        self.turns += 1
        self.pressing += (downs[-1] - downs[0]) / 1000 + REPEAT

    def held(self, where: str, scale: str) -> None:
        """Say how the turns went, and fail a launcher that didn't keep up."""
        at = f"in {where} at {scale}"
        rate = self.refreshes / self.window
        handled = {frozen: ranked(times, TAIL) for frozen, times in self.handled.items()}
        middle = {frozen: ranked(times, 0.5) for frozen, times in self.drawn.items()}
        print(
            f"{at}: while things changed the unseen made {self.unseen_made} rows and Agents "
            f"{self.agents_made} a turn, and while frozen the keys made {self.keys_made}, "
            f"and the look for where rows live {self.walked}; "
            f"{self.refreshes} refreshes in {self.window:.1f} s, {rate:.1f} a second, the keys "
            f"pressed over {self.pressing:.1f} s of it, "
            f"{self.keyed} taken on a key's path; handlers 95% within {handled[False]:.1f} ms "
            f"changing against {handled[True]:.1f} ms frozen, the slowest "
            f"{max(self.handled[False]):.1f} against {max(self.handled[True]):.1f} ms; keys to "
            f"their frames half within {middle[False]:.0f} ms changing against "
            f"{middle[True]:.0f} ms frozen, 95% within {ranked(self.drawn[False], TAIL):.0f} "
            f"against {ranked(self.drawn[True], TAIL):.0f} ms; the list composed "
            f"{self.composed[False]} times in {self.refreshed[False]} refreshes changing and "
            f"{self.composed[True]} in {self.refreshed[True]} frozen, drawing "
            f"{self.afresh[False]} and {self.afresh[True]} rows afresh"
        )
        assert not any(self.unseen_made.values()), f"{at}, unseen, {self.unseen_made} rows made"
        assert not any(self.keys_made.values()), (
            f"{at}, arrows that only move made {self.keys_made} rows"
        )
        # Where a row lives is looked for only on Ctrl+]'s press, which no turn makes, so none
        # is: the walk that kept it for every row was made again on most openings and after each
        # change, 18,693 of People's 19,461 rows at home in Joel's morning (2026-11-09).
        assert not any(self.walked.values()), (
            f"{at}, with no Ctrl+] pressed the look for where rows live made {self.walked} rows"
        )
        assert self.keyed == 0, f"{at}, {self.keyed} refreshes were taken on a key's path"
        # The pace lets one refresh at once and one each quarter second after it, so a turn's span
        # holds at most one more than its share.
        most = REFRESHES * self.window + self.turns
        assert self.refreshes <= most, (
            f"{at}, the list refreshed {self.refreshes} times in {self.window:.1f} s over "
            f"{self.turns} turns, past the pace's {most:.0f}"
        )
        # A key that composed the list, or drew rows afresh with nothing changing, did the work a
        # change costs on its own path, which the times a loaded builder stretches can't tell.
        for frozen in (False, True):
            assert self.composed[frozen] <= self.refreshed[frozen], (
                f"{at}, the list was composed {self.composed[frozen]} times in "
                f"{self.refreshed[frozen]} refreshes, {'frozen' if frozen else 'changing'}"
            )
        assert self.afresh[True] == 0, (
            f"{at}, arrows with nothing changing drew {self.afresh[True]} rows afresh"
        )


def sorts(desktop: Desktop) -> Json:
    """Give how many sortings of pi's sessions the shell has made, and how many it took."""
    return cast("Json", json.loads(desktop.as_account("qs ipc call pi sorts")))


def stepped(desktop: Desktop) -> int:
    """Give the furthest step of the stand-in sessions the node holds, as their usage says it."""
    said = json.loads(desktop.as_account("mycelium --realm harbour sessions"))
    usage = "gen_ai.usage.input_tokens"
    return max(
        [
            int(one["presence"].get("usage", {}).get("last", {}).get(usage, 0))
            for one in said["Ok"]["Sessions"]
            if str(one["address"]).startswith("LD")
        ],
        default=0,
    )


def test_a_shut_launcher_takes_no_step_only_it_shows(desktop: Desktop) -> None:
    """With the launcher shut, sessions moving their steps and standing wake the shell not at all.

    Joel's 113 sessions changed 26 times in 30 s, 21 of them only in their step and usage, and
    each cost his closed shell's main thread 20 to 370 ms (2026-11-09); of the rest, a quarter
    of all his changes turned only between working and listening, at about 50 ms each
    (2026-11-10). Skipping their sorting still parsed each change, and none of 56 a minute
    moved what a shut launcher reads (2026-11-11), so the shell asks the node to send a change
    only as that moves. Opened, it takes them. It runs before the arrows, which want them taken.
    """
    machine = desktop.machine
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))
    working = f"{desktop.values['stand_in_sessions']} {PI_PRESENCE} {WORKING} 1 {BEAT}"
    desktop.as_account(f"systemd-run --user --collect --quiet --unit=working-sessions {working}")
    try:
        # Every session is heard of before counting, since a new one is taken as it comes: they're
        # all written before the first step, and the node tells of them within a beat or two.
        retry(lambda _: stepped(desktop) >= STEPS, REACHING)
        before = sorts(desktop)
        at = stepped(desktop)
        retry(lambda _: stepped(desktop) >= at + STEPS, REACHING)
        after = sorts(desktop)
        assert after["heard"] == before["heard"], (
            f"the shut launcher heard {after['heard'] - before['heard']} changes while the node"
            f" moved {stepped(desktop) - at} steps, which moved only steps and standing"
        )
        desktop.as_account("qs ipc call launcher toggle")
        try:
            desktop.launcher_ready()
            retry(lambda _: sorts(desktop)["taken"] > after["taken"], REACHING)
        finally:
            desktop.close_launcher()
    finally:
        desktop.as_account("systemctl --user stop working-sessions")


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

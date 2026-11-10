"""The desktop test's keeps pace area.

Here the launcher keeps pace with the keys while the machine is busy. Every core is loaded in the
account's jobs.slice and in build.slice, where a workstation's agents and builds run, and each key
is timed from the moment a stand-in keyboard pressed it to the first frame the launcher drew for
it, both on the machine's one clock. Keys come in turns with the load running and frozen, and
those under load are held against the others, so the builder's own load slows both alike. That
text a program types at once is searched once, and a Return among it acts on what it found, is
proven on a nested niri, in tests/launcher-on-niri-test.py.
tests/desktop.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), and its
opening comment says what the test proves. Every test asks for the desktop, started once a run
(desktop_session.py), and leaves it as it found it.
"""

from __future__ import annotations

import json
import math
import re
import shlex
import time
from datetime import timedelta
from functools import partial
from typing import TYPE_CHECKING, Any, NamedTuple, cast

import desktop_session
import pytest
from desktop_session import (
    chrome_download_gone,
    chrome_downloaded,
    chrome_gone,
    chrome_visited,
    neovim_in_kitty,
    neovims_quit,
    retry,
)

if TYPE_CHECKING:
    from collections.abc import Iterator

    from desktop_session import Desktop, Json
    from test_driver.machine import BaseMachine

# Where the stand-in keyboard is told what to press, and says when it did (desktop/keyboard.py).
KEYBOARD = "/run/keyboard"
SAID = "/run/keyboard.log"
# Arrows up and down through Programs, gliding the highlight and scrolling the list, then words
# typed at home and rubbed out, changing the rows under the highlight: 200 arrows, then 50
# letters and their 50 rubbings out.
ARROWS = 200
WORDS = ["power", "sound", "files", "night", "music", "clock", "paint", "light", "watch", "guide"]
# Keys under load are judged against the launcher's own keys with the load frozen, pressed in
# turns of PHASE keys in the same machine. The VM draws in software on a builder shared with
# other checks, whose own load moved the middle key from 70 ms to 2553 ms over three runs of
# the same launcher; it slows both turns alike, so what the machine's own load costs the
# launcher is what's left between them. Design.md's budget is 50 ms from a key to its frame:
# under load the middle key may take twice its time with the load frozen, and that budget more.
# What's judged is read in each turn and judged at the middle turn's: the VM's drawing can stop
# for seconds while the launcher hears every key at once, then draw the keys it held in one
# frame (7 keys in a turn with the load frozen took 1.5 to 3.8 s on 2026-11-02), and a stall
# falling in either kind of turn by chance is one turn's in seven. A launcher the load slows is
# slow in every turn under it. The laptop's own times, the tail of them too, come from its load
# proof, which reads the same list of frames.
#
# The middle key is said as written, from the keyboard to its frame, and judged less two parts
# the VM pays and a laptop doesn't, said beside it: the frame's software swap, from its render's
# end to the screen, and the wait from the key's handler being done to its frame beginning,
# which is the compositor's frame callback. Since a key waits for a frame begun after its
# handler (#2860), one handled as a frame is drawn pays the next frame's whole swap, which under
# load took a median 30 ms a frame against 12 frozen (2026-11-10), so as written the middle key
# read 106 to 125 ms against 28 to 31 on one builder and 58 against 24 on another. A laptop's
# compositor makes a frame callback wait too, so the laptop's journal of Joel's own keys, which
# times the whole of each, stays the final judge.
PHASE = 20
# The frozen turns don't cancel the builder out at the tail: under load all four of the VM's
# processors want to run, and the builder, which can't see the guest's weights, holds the one
# drawing the launcher behind its own work, as it doesn't hold a processor that's idle. On 5
# November main's launcher failed and passed at the same commit by what else the builder ran,
# and these builders give the guest no steal time to tell it by. A desktop the load starves
# doesn't show in the tail either: with the desktop's slices weighed 1 its slowest key in twenty
# took 124 ms against 442 ms frozen. So beside the middle key, what the machine's own weights
# decide is judged: how long the compositor and the shell were ready to run and waited for a
# processor, which the guest's kernel counts in each one's cpu.pressure. Waking an idle
# processor waits on the builder too, so they wait with the load frozen as well, 20 to 38 ms a
# key on 5 November. Under load a key may cost them twice that, and WAITED ms more: weighed as
# a workstation weighs them they waited 50 and 60 ms a key against 35 and 38, and weighed 1,
# 78 against 20.
WAITED = 10
# A builder with nothing else to run wakes the guest's idle processors at once, so the frozen
# turns wait as little as 3.9 ms a key, and twice that leaves the load less than a processor's
# share: on 9 November #2724, which touched nothing the launcher runs, failed three times at
# 21.8 to 26.1 ms against 3.9 to 6.9 frozen, where main passed at 22.8 to 41.8 against 7.8 to
# 20.8. The frozen wait is taken as at least WAITED_FREE_LEAST ms, which still fails the
# desktop weighed 1, 78 ms against 20.
WAITED_FREE_LEAST = 10
SLOWER_MOST = 2
MIDDLE_MORE = 50
TAIL = 0.95
# Cage, which holds the compositor here, and the shell, by the units they run in.
COMPOSITOR = "cage-tty1.service"
SHELL = "quickshell.service"
# How often a key is pressed: a steady typist's pace, slower than the frames a key mostly takes,
# so each key mostly reaches its own. Asking the launcher after each key whether it's drawn
# would pace them instead, and under this load each asking takes over a second.
PACE = timedelta(milliseconds=400)
# How many keys' frames the launcher keeps, so the most a run of keys may be before it's read.
KEPT = 300
# How long the last key may take to reach its frame before the test says it never did.
REACHING = timedelta(seconds=30)
# How long freezing or thawing the load may take before the test says it couldn't.
FREEZING = timedelta(minutes=3)
# Each place a program gives, stepped into as Joel does, in rounds with the load frozen and
# running by turns, after a first round that's only read, since a place's first step makes what
# it keeps for the rest. A step is timed by the launcher from the key it heard to its frame, and
# said in its journal as `"\u2192 Name" took N ms to its frame` (Trials.js), which is what the
# machine's own load proof reads of Joel's. Under load a place's middle step may take twice its
# middle with the load frozen and STEPPED_MORE ms more, as a key may. Each place is moved in
# once after its step, and the drawer made for the row moved onto, off the key's path as the
# selection rests, is timed by the launcher from the work begun for it to its frame
# (Launcher.qml's drawerMade) and held to the same.
ROUNDS = 6
STEPPED_MORE = 50
STEP_SAID = re.compile(r'"\u2192 ([^"]+)" took (\d+) ms to its frame, (\d+) ms searching')
# The programs whose pages are stepped into: what's typed to find each, and its row's key.
PAGES = [
    ("chrome", "program:google-chrome"),
    ("reader", "program:reader"),
    ("inkscape", "program:org.inkscape.Inkscape"),
    ("neovim", "program:nvim"),
]
# Neovim's page lists the files each Neovim holds, so one holds two.
SOUNDINGS = "soundings"
# The shell started again as a switch starts it, by turns with the load frozen and running, and
# opened each time as Joel opens it after one: People listing as many as Joel's lists, 486 on
# 2026-11-10's first opening, and the shell's own start work (Launcher.qml's, 5 s after it
# starts) done, as it is by the time he opens it. A first opening under load may take twice its
# time with the load frozen and STEPPED_MORE ms more, as a step may.
STARTS = 4
CROWD = "/run/user/1000/crowd"
LISTED = 500
STARTED = timedelta(seconds=8)
# A storm of windows changing, as Joel's crowd of agents' windows makes: STORMING windows, each
# a session's, retitled STORM times a second between them, which niri tells the shell of one
# event at a time. The launcher is opened OPENINGS times through it, every core busy, and each
# opening reaches its frame within OPENED_WITHIN ms, at the tail as at the middle: Joel's took
# 6,184 ms to its frame for 107 ms of its own work on 2026-11-10.
STORMING = 40
STORM = 100
OPENINGS = 20
OPENED_WITHIN = 50
OPENED_TAIL = 0.99


@pytest.fixture(scope="module")
def desktop(machine: BaseMachine, values: dict[str, Any]) -> Desktop:
    """Give the desktop, started once a run."""
    return desktop_session.started(machine, cast("desktop_session.Values", values))


@pytest.fixture
def keyboard(desktop: Desktop) -> Iterator[None]:
    """Start the stand-in keyboard, and wait until the desktop reads it."""
    machine = desktop.machine
    machine.succeed(f"systemd-run --unit=keyboard {desktop.values['keyboard']}")
    machine.wait_for_file(KEYBOARD, timeout=timedelta(seconds=30))
    # A press before the keyboard is open would be lost. Here niri runs
    # nested in Cage, which reads the machine's input devices and hands
    # their keys on to niri, so it's Cage that opens it; its process isn't
    # named cage exactly, so it's found by its command.
    node = machine.succeed("cat /run/keyboard.device").strip()
    held = f"find /proc/$(pgrep -o -f bin/cage)/fd -lname {node} | grep -q ."
    try:
        machine.wait_until_succeeds(held, timeout=timedelta(seconds=30))
    except AssertionError:
        seen = machine.execute(f"udevadm info {node}", check_return=False)[1]
        pytest.fail(f"Cage never opened the stand-in keyboard; udev says:\n{seen}")
    try:
        yield
    finally:
        machine.succeed("systemctl stop keyboard")


@pytest.fixture
def loaded(desktop: Desktop) -> Iterator[None]:
    """Load every core in jobs.slice and in build.slice, as agents and builds do, until done."""
    machine = desktop.machine
    # Builds weigh what the workstation role gives them (modules/nix-daemon).
    weight = machine.succeed("systemctl show build.slice --property=CPUWeight --value").strip()
    assert weight == "10", f"build.slice weighs {weight}, not the workstation's 10"
    stress = f"{desktop.values['stress_ng']}/bin/stress-ng --cpu 0 --quiet"
    desktop.as_account(
        f"systemd-run --user --collect --quiet --slice=jobs.slice --unit=stress-jobs {stress}"
    )
    machine.succeed(
        f"systemd-run --collect --quiet --slice=build.slice --unit=stress-build {stress}"
    )
    try:
        desktop.as_account("systemctl --user is-active --quiet stress-jobs")
        machine.succeed("systemctl is-active --quiet stress-build")
        yield
    finally:
        desktop.as_account("systemctl --user stop stress-jobs")
        machine.succeed("systemctl stop stress-build")


def framed(desktop: Desktop) -> Json:
    """Give how many keys the launcher has timed, and when each of the last reached its frame."""
    return cast("Json", json.loads(desktop.as_account("qs ipc call launcher framed")))


def timed_early(keys: Json, timed: int) -> list[tuple[float, float]]:
    """Give each key timed since `timed` whose frame began before its handler was done."""
    count = min(keys["timed"] - timed, len(keys["frames"]))
    if count <= 0:
        return []
    done, begun = keys["done"][-count:], keys["begun"][-count:]
    return [(at, began) for at, began in zip(done, begun, strict=True) if began < at]


class Timing:
    """Keys pressed on the stand-in keyboard at a steady pace, each timed to its first frame.

    The launcher keeps one frame a key, in the order they came, so a run of keys is paired
    with its frames by count: a launcher falling behind can't pass by pairing a key with the
    late frame of one before it. Keys come in turns of PHASE, with the machine's load running
    and then frozen, and each is kept with the turn it was pressed in.
    """

    def __init__(self, desktop: Desktop) -> None:
        """Begin with no key timed, and the load running, as the fixture started it."""
        self.desktop = desktop
        self.under: list[float] = []
        self.free: list[float] = []
        # Each turn's keys, by the turn they were pressed in, under load and frozen.
        self.under_turns: dict[int, list[float]] = {}
        self.free_turns: dict[int, list[float]] = {}
        # From the press the launcher was told of to its frame, which the machine's load proof
        # reads of Joel's own keys, and from the launcher's handler taking it, the shell's part,
        # both under load.
        self.stamped: list[float] = []
        self.handled: list[float] = []
        # Each key less its frame's swap, and less that and its wait for its frame to begin, under
        # load and frozen, the second what's judged.
        self.unswapped: dict[bool, list[float]] = {True: [], False: []}
        self.judged: dict[bool, list[float]] = {True: [], False: []}
        self.next = time.monotonic()
        self.said = self.timed = self.pressing = self.presses = 0
        self.turns: list[tuple[int, bool]] = []
        self.loading = True
        # How long, in ms, the compositor and the shell waited for a processor in each turn, and
        # the turn being pressed with what they'd waited when it began.
        self.waited: dict[int, float] = {}
        self.open: tuple[int, float] | None = None
        show = "show --property=ControlGroup --value"
        self.groups = [
            desktop.machine.succeed(f"systemctl {show} {COMPOSITOR}").strip(),
            desktop.as_account(f"systemctl --user {show} {SHELL}").strip(),
        ]

    def begin(self) -> None:
        """Begin a run of keys, from where the keyboard's lines and the launcher's count stand."""
        self.said = int(self.desktop.machine.succeed(f"wc -l < {SAID}").strip())
        self.timed = framed(self.desktop)["timed"]
        self.pressing = 0
        self.turns = []

    def load(self, *, on: bool) -> None:
        """Let the machine's load run, or freeze it where it stands, which takes no time back."""
        verb = "thaw" if on else "freeze"
        # A builder stalling the VM held a freeze for 50 s on 5 November, past the 25 s systemctl
        # waits for its answer by default, though the unit froze. FREEZING is longer than that.
        wait = f"SYSTEMD_BUS_TIMEOUT={round(FREEZING.total_seconds())}"
        self.desktop.as_account(f"{wait} systemctl --user {verb} stress-jobs")
        self.desktop.machine.succeed(f"{wait} systemctl {verb} stress-build")
        self.loading = on

    def waiting(self) -> float:
        """Give how long the compositor and the shell have waited for a processor, in ms."""
        files = " ".join(f"/sys/fs/cgroup{group}/cpu.pressure" for group in self.groups)
        # The first line, some, counts the time at least one of a unit's tasks waited.
        lines = self.desktop.machine.succeed(f"grep -h '^some' {files}").splitlines()
        assert len(lines) == len(self.groups), f"cpu.pressure of {self.groups} reads {lines}"
        return sum(int(line.rsplit("total=", 1)[1]) for line in lines) / 1000

    def close_turn(self) -> None:
        """End the turn being pressed, keeping how long it made the desktop wait."""
        if self.open is None:
            return
        turn, began = self.open
        self.open = None
        self.waited[turn] = self.waiting() - began

    def press(self, key: str) -> None:
        """Press a key on the beat, however long telling the keyboard took.

        A beat missed, while the test did something else, starts the beat again rather than
        pressing the keys it missed at once. The load turns on or off before the first key of
        each turn, once the launcher has drawn every key before it, so no key waits behind the
        last turn's, and the turn's wait is counted from there to its last key drawn.
        """
        turn = self.presses // PHASE
        loaded = turn % 2 == 0
        if self.open is None or self.open[0] != turn:
            if self.open is not None:
                self.caught_up()
                self.close_turn()
            if loaded != self.loading:
                self.load(on=loaded)
            self.open = (turn, self.waiting())
        self.turns.append((turn, loaded))
        self.presses += 1
        self.next = max(self.next, time.monotonic())
        self.desktop.machine.succeed(f"echo {key} > {KEYBOARD}")
        self.pressing += 1
        self.next += PACE.total_seconds()
        time.sleep(max(0.0, self.next - time.monotonic()))

    def caught_up(self) -> Json:
        """Wait for the launcher to have timed every key of the run so far, and give its times."""
        noted: Json = {}

        def drawn(_: bool) -> bool:
            noted.update(framed(self.desktop))
            return int(noted["timed"]) >= self.timed + self.pressing

        try:
            retry(drawn, timeout=REACHING)
        except AssertionError:
            missing = self.timed + self.pressing - noted["timed"]
            pytest.fail(f"{missing} of {self.pressing} keys never reached a frame")
        assert noted["timed"] == self.timed + self.pressing, (
            f"the launcher timed {noted['timed'] - self.timed} keys of {self.pressing} pressed"
        )
        return noted

    def keep(self) -> None:
        """Pair the run's keys with their frames, once the launcher has timed every one."""
        machine = self.desktop.machine
        assert self.pressing <= KEPT, f"a run of {self.pressing} keys outruns the {KEPT} kept"
        # The keyboard says a key once it has let it go.
        machine.wait_until_succeeds(
            f"[ $(wc -l < {SAID}) -ge {self.said + self.pressing} ]",
            timeout=timedelta(seconds=10),
        )
        lines = machine.succeed(f"tail -n +{self.said + 1} {SAID}").splitlines()
        noted = self.caught_up()
        self.close_turn()
        assert noted["clock"] == "monotonic", f"the launcher keeps time by {noted['clock']}"
        kept = len(noted["frames"])
        assert len(noted["pressed"]) == len(noted["heard"]) == kept, (
            f"{len(noted['pressed'])} keys pressed and {len(noted['heard'])} heard beside "
            f"{kept} frames"
        )
        times = zip(
            lines,
            self.turns,
            noted["pressed"][-self.pressing :],
            noted["heard"][-self.pressing :],
            noted["frames"][-self.pressing :],
            noted["done"][-self.pressing :],
            noted["begun"][-self.pressing :],
            noted["rendered"][-self.pressing :],
            strict=True,
        )
        for line, (turn, loaded), pressed, heard, frame, done, begun, rendered in times:
            key, down = line.split()
            took = frame - float(down)
            assert took >= 0, f"{key}'s frame came {-took:.0f} ms before it was pressed"
            # The guest's one clock: the kernel stamps a key as the keyboard writes it, the
            # launcher hears it after and draws it after that, give or take the millisecond
            # the compositor's stamp rounds to.
            assert pressed is not None, f"the launcher kept no press for {key}"
            assert float(down) - 1 <= pressed <= heard + 1 <= frame + 1, (
                f"{key} went down at {down}, was stamped {pressed:.0f}, heard at {heard:.0f} "
                f"and drawn at {frame:.0f}"
            )
            # A swap whose start wasn't heard is taken as none, so it's never taken out.
            swap = frame - rendered if rendered is not None and begun <= rendered <= frame else 0
            unswapped = took - swap
            self.unswapped[loaded].append(unswapped)
            self.judged[loaded].append(unswapped - max(0.0, begun - done))
            if not loaded:
                self.free.append(took)
                self.free_turns.setdefault(turn, []).append(took)
                continue
            self.under.append(took)
            self.under_turns.setdefault(turn, []).append(took)
            self.stamped.append(frame - pressed)
            self.handled.append(frame - heard)

    def said_and_held(self) -> None:
        """Say how long keys took to their frames, and fail a launcher the load slows."""
        under, free = sorted(self.under), sorted(self.free)
        stamped, handled = sorted(self.stamped), sorted(self.handled)

        def ranked(times: list[float], share: float) -> float:
            return times[max(0, math.ceil(share * len(times)) - 1)]

        def turned(turns: dict[int, list[float]]) -> list[float]:
            return [ranked(sorted(times), TAIL) for _, times in sorted(turns.items())]

        def per_key(turns: dict[int, list[float]]) -> list[float]:
            return [self.waited[turn] / len(times) for turn, times in sorted(turns.items())]

        middle, tail = ranked(under, 0.5), ranked(under, TAIL)
        middle_free, tail_free = ranked(free, 0.5), ranked(free, TAIL)
        tails, tails_free = turned(self.under_turns), turned(self.free_turns)
        turn_tail, turn_tail_free = ranked(sorted(tails), 0.5), ranked(sorted(tails_free), 0.5)
        waits, waits_free = per_key(self.under_turns), per_key(self.free_turns)
        wait, wait_free = ranked(sorted(waits), 0.5), ranked(sorted(waits_free), 0.5)
        print(
            f"the compositor and the shell waited for a processor, a key, in the middle turn "
            f"{wait:.1f} ms under load against {wait_free:.1f} ms frozen; each turn's under load "
            f"{[round(w, 1) for w in waits]} and frozen {[round(w, 1) for w in waits_free]}"
        )
        print(
            f"{len(under)} keys under load and {len(free)} with it frozen, from the keyboard to "
            f"their frames: half within {middle:.0f} ms against {middle_free:.0f} ms, 95% "
            f"within {tail:.0f} ms against {tail_free:.0f} ms, the slowest {under[-1]:.0f} ms "
            f"against {free[-1]:.0f} ms; under load, from the press the launcher was told of: "
            f"half within {ranked(stamped, 0.5):.0f} ms, 95% within {ranked(stamped, TAIL):.0f} "
            f"ms; from the launcher's handler: half within {ranked(handled, 0.5):.0f} ms, 95% "
            f"within {ranked(handled, TAIL):.0f} ms; nineteen in twenty of the middle turn within "
            f"{turn_tail:.0f} ms against {turn_tail_free:.0f} ms, each turn's under load "
            f"{[round(t) for t in tails]} and frozen {[round(t) for t in tails_free]}"
        )
        unswapped = {loaded: ranked(sorted(times), 0.5) for loaded, times in self.unswapped.items()}
        judged = {loaded: ranked(sorted(times), 0.5) for loaded, times in self.judged.items()}
        print(
            f"the middle key less its frame's swap took {unswapped[True]:.0f} ms under load "
            f"against {unswapped[False]:.0f} ms frozen, and less its wait for its frame to begin "
            f"too, {judged[True]:.0f} ms against {judged[False]:.0f} ms"
        )
        assert judged[True] <= SLOWER_MOST * judged[False] + MIDDLE_MORE, (
            f"half the keys under load took {judged[True]:.0f} ms to their frames less their "
            f"swaps and their waits for them to begin, against {judged[False]:.0f} ms with the "
            f"load frozen; "
            f"as written, {middle:.0f} ms against {middle_free:.0f}"
        )
        assert wait <= SLOWER_MOST * max(wait_free, WAITED_FREE_LEAST) + WAITED, (
            f"under load the compositor and the shell waited {wait:.1f} ms a key for a processor "
            f"in the middle turn, against {wait_free:.1f} ms with the load frozen"
        )


def selects(key: str, shown: Json) -> bool:
    """Say whether the selected row is the thing this key names, as a program's row."""
    return (shown["selected"] or {}).get("is") == key


class Drawn(NamedTuple):
    """A drawer's time to its frame, as the launcher split it (Launcher.qml's drawerMade)."""

    took: int
    making: int
    waiting: int
    drawing: int


def moved(desktop: Desktop) -> Drawn | None:
    """Move onto the place's next row and give its drawer's time, once it's drawn.

    A place of one row isn't moved in, and a row with no drawer gives none.
    """
    shown = desktop.showing()
    at = shown["current"]
    if at + 1 >= len(shown["rows"]):
        print(f"no drawer timed after {shown['selected']}: its place has one row")
        return None
    was = shown["drawer"]["row"]
    counted = shown["drawer"]["timed"]["count"]
    rested: list[Json] = []

    def drawn(shown: Json) -> bool:
        drawer = shown["drawer"]
        if shown["current"] != at + 1 or drawer["row"] in {"", was}:
            return False
        timed = drawer["timed"]
        if drawer["wanted"] and not (timed["count"] > counted and timed["key"] == drawer["row"]):
            return False
        rested.append(drawer)
        return True

    desktop.next_row()
    desktop.launcher_until(drawn)
    drawer = rested[-1]
    if not drawer["wanted"]:
        print(f"no drawer timed for {drawer['row']}: it has none")
        return None
    timed = drawer["timed"]
    return Drawn(timed["took"], timed["making"], timed["waiting"], timed["drawing"])


def stepped_round(desktop: Desktop) -> dict[str, Drawn]:
    """Step into Programs, each program's page, Chrome's Recent and Passwords, once each.

    Each is moved in once after its step, and its drawer's time and making given by place.
    """
    drawers: dict[str, Drawn | None] = {}
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.down_to("Programs")
    desktop.step_in("ret")
    drawers["Programs"] = moved(desktop)
    desktop.close_launcher()
    for typed, key in PAGES:
        desktop.answer(typed)
        desktop.launcher_until(partial(selects, key))
        desktop.into_program()
        drawers[typed] = moved(desktop)
        if typed == "chrome":
            desktop.type_in("recent", selecting="Recent")
            desktop.follow()
            drawers["chrome's Recent"] = moved(desktop)
        desktop.close_launcher()
    desktop.answer("passwords")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Passwords")
    desktop.step_in("ret")
    drawers["Passwords"] = moved(desktop)
    desktop.close_launcher()
    return {place: drawer for place, drawer in drawers.items() if drawer is not None}


def steps_said(desktop: Desktop, since: str) -> list[tuple[str, int, int, str]]:
    """Give each step timed since the mark: its place, its frame, its search and its line."""
    said = desktop.as_account(
        "journalctl --user --unit=quickshell --no-pager --output=cat"
        f" --after-cursor={shlex.quote(since)} | grep -a -F ' ms to its frame'; true"
    )
    return [
        (found[1], int(found[2]), int(found[3]), found.string.strip())
        for found in map(STEP_SAID.search, said.splitlines())
        if found
    ]


def slowed_drawers(drawn: dict[bool, dict[str, list[Drawn]]]) -> list[str]:
    """Say each place's drawers by turns, and give those the load slowed past their bound."""
    slowed = []
    for place in sorted(drawn[True]):
        loaded = sorted(one.took for one in drawn[True][place])
        frozen = sorted(one.took for one in drawn[False].get(place, []))
        print(
            f"{place}'s drawer: under load {sorted(drawn[True][place])} ms,"
            f" against {sorted(drawn[False].get(place, []))} with the load frozen"
        )
        if frozen and loaded[(len(loaded) - 1) // 2] > (
            SLOWER_MOST * frozen[(len(frozen) - 1) // 2] + STEPPED_MORE
        ):
            slowed.append(place)
    return slowed


@pytest.fixture
def pages_filled(desktop: Desktop) -> Iterator[None]:
    """Give Chrome its sites, its tab and a download, and a Neovim two files, as Joel's have."""
    folder = f"/home/{desktop.account}/{SOUNDINGS}"
    desktop.as_account(
        f"mkdir -p {folder} && printf 'North pier\\n' > {folder}/tides.txt"
        f" && printf 'Harbour mouth\\n' > {folder}/reefs.txt"
    )
    chrome_visited(desktop)
    downloaded = chrome_downloaded(desktop)
    neovims = [neovim_in_kitty(desktop, folder, "tides.txt", "reefs.txt")]
    try:
        yield
    finally:
        neovims_quit(desktop, neovims)
        chrome_download_gone(desktop, downloaded)
        chrome_gone(desktop)
        desktop.as_account(f"rm -rf {folder}")


@pytest.mark.usefixtures("loaded", "pages_filled")
def test_each_programs_place_is_stepped_into_soon_while_every_core_is_busy(
    desktop: Desktop,
) -> None:
    """Programs, its pages, a page's Recent and Passwords, and their drawers, show soon.

    Every core is busy for half the rounds.

    Joel, 2026-11-10: "loading certain menus takes too long ... this needs to be as fast as
    thought no matter how loaded my machine is." On his laptop that night Programs took 39 to
    111 ms to its frame, Chrome's page 86 and Recent up to 81, nearly all of it composing.
    """
    timing = Timing(desktop)
    under: dict[str, list[int]] = {}
    free: dict[str, list[int]] = {}
    searching: dict[str, list[int]] = {}
    # Each place's slowest step under load as the launcher said it, whose
    # parts say where its time went.
    slowest: dict[str, tuple[int, str]] = {}
    # Keys timed by a frame begun before their handler was done, as a step just after Escape
    # was by Escape's frame: said 33 to 51 ms, its own frame 117 to 158 ms after the key
    # (2026-11-10).
    early: list[tuple[float, float]] = []
    # Each place's drawer, made for the row moved onto, by turns: its time to its frame and
    # how much of it the launcher made it, waited for its frame and drew it.
    drawn: dict[bool, dict[str, list[Drawn]]] = {True: {}, False: {}}
    try:
        stepped_round(desktop)
        for turn in range(ROUNDS):
            loaded = turn % 2 == 1
            if loaded != timing.loading:
                timing.load(on=loaded)
            since = desktop.mark()
            timed = framed(desktop)["timed"]
            for place, drawer in stepped_round(desktop).items():
                drawn[loaded].setdefault(place, []).append(drawer)
            early.extend(timed_early(framed(desktop), timed))
            for place, took, search, line in steps_said(desktop, since):
                (under if loaded else free).setdefault(place, []).append(took)
                if loaded:
                    searching.setdefault(place, []).append(search)
                    slowest[place] = max(slowest.get(place, (0, "")), (took, line))
    finally:
        if not timing.loading:
            timing.load(on=True)
        desktop.close_launcher()

    def middle(times: list[int]) -> int:
        return sorted(times)[(len(times) - 1) // 2]

    slowed = []
    for place in sorted(under):
        print(
            f"stepping into {place}: under load {sorted(under[place])} ms to its frame,"
            f" {sorted(searching[place])} of it searching, against {sorted(free.get(place, []))}"
            f" with the load frozen; its slowest said {slowest[place][1]}"
        )
        if place in free and middle(under[place]) > (
            SLOWER_MOST * middle(free[place]) + STEPPED_MORE
        ):
            slowed.append(place)
    drawers_slowed = slowed_drawers(drawn)
    assert "Programs" in under, f"no step into Programs was timed: {sorted(under)}"
    assert "Programs" in drawn[True], f"no drawer in Programs was timed: {drawn}"
    assert not drawers_slowed, f"the load slowed the drawers of {drawers_slowed}: {drawn}"
    assert not early, f"keys done at and timed by frames begun before: {early}"
    assert not slowed, f"the load slowed stepping into {slowed}: {under} against {free}"


def openings(desktop: Desktop) -> list[Json]:
    """Give the last openings the launcher timed, each from its ask to its first frame."""
    return cast("list[Json]", json.loads(desktop.as_account("qs ipc call launcher openings")))


def opened_timed(desktop: Desktop) -> Json:
    """Open the launcher as its chord does, and give that opening's time to its first frame."""
    timed = len(openings(desktop))
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    each: list[Json] = []

    def drawn(_: bool) -> bool:
        each[:] = openings(desktop)
        return len(each) > timed

    retry(drawn, timeout=REACHING)
    desktop.close_launcher()
    return each[-1]


@pytest.mark.usefixtures("loaded")
def test_the_first_opening_after_the_shell_starts_shows_soon_while_every_core_is_busy(
    desktop: Desktop,
) -> None:
    """The first opening after the shell starts shows soon, as later ones do, every core busy.

    Every switch starts the shell again, several times a day, and Joel named the start as slow
    (Y2YBTS, 2026-11-10): its first opening made every row its places read, 486 of them
    People's, where a later one makes only what changed. The one after it weighs none of the
    words file's verbs again, which every opening once walked, 8 to 11 ms of its 30 to 39.
    """
    timing = Timing(desktop)
    first: dict[bool, list[int]] = {True: [], False: []}
    # Each opening as the launcher timed it, the first after a start and the one after it.
    said: list[tuple[bool, Json, Json]] = []
    desktop.as_account(f"echo '{LISTED} 1 listed' > {CROWD}.new && mv {CROWD}.new {CROWD}")
    try:
        for turn in range(STARTS):
            loaded = turn % 2 == 1
            if loaded != timing.loading:
                timing.load(on=loaded)
            since = desktop.mark()
            desktop.as_account("systemctl --user restart quickshell")
            desktop.reported(
                "crowd ", lambda rest: rest.startswith(f"{LISTED} "), since, REACHING * 4
            )
            time.sleep(STARTED.total_seconds())
            opening = opened_timed(desktop)
            assert opening["first"], f"the opening after a start wasn't its first: {opening}"
            first[loaded].append(opening["took"])
            said.append((loaded, opening, opened_timed(desktop)))
    finally:
        if not timing.loading:
            timing.load(on=True)
        desktop.as_account(f"echo '0 0' > {CROWD}.new && mv {CROWD}.new {CROWD}")
        desktop.launcher_left_closed()

    def middle(times: list[int]) -> int:
        return sorted(times)[(len(times) - 1) // 2]

    for loaded, opening, after in said:
        print(
            f"{'under load' if loaded else 'with the load frozen'}, the first opening after a"
            f" start {opening}, and the one after it {after}"
        )
    weighing = [after for _, _, after in said if after.get("verbs") != 0]
    assert not weighing, f"an opening after the first weighed the verbs again: {weighing}"
    assert middle(first[True]) <= SLOWER_MOST * middle(first[False]) + STEPPED_MORE, (
        f"the load slowed the first opening after a start: {first[True]} ms against "
        f"{first[False]} ms frozen"
    )


@pytest.fixture
def storm(desktop: Desktop) -> Iterator[None]:
    """Retitle STORMING windows STORM times a second, as a crowd of agents' windows does."""
    machine = desktop.machine
    # Through niri, so the windows reach it through xwayland-satellite,
    # with no token to take the keys from the shell's.
    desktop.as_account(
        "niri msg action spawn -- env -u XDG_ACTIVATION_TOKEN"
        f" {desktop.values['window_storm']} {STORMING} {STORM}"
    )
    stormy = 'niri msg --json windows | grep -o \'"title":"storm \' | wc -l'
    listed = f"test $({stormy}) -eq {STORMING}"
    machine.wait_until_succeeds(desktop.in_session(listed), timeout=REACHING)
    try:
        yield
    finally:
        desktop.as_account("pkill -f window-storm")
        gone = '! niri msg --json windows | grep -q \'"title":"storm \''
        machine.wait_until_succeeds(desktop.in_session(gone), timeout=REACHING)


@pytest.mark.usefixtures("loaded", "storm")
def test_each_opening_reaches_its_frame_soon_through_a_storm_of_windows_changing(
    desktop: Desktop,
) -> None:
    """Each opening reaches its frame soon through a storm of windows changing, every core busy.

    Joel's crowd of agents' windows retitle themselves all day, and an opening that waited on
    the shell making every list again for each change took seconds to show (2026-11-10).
    """
    took: list[int] = []
    try:
        for _ in range(OPENINGS):
            opening = opened_timed(desktop)
            print(f"through the storm, an opening {opening}")
            took.append(opening["took"])
    finally:
        desktop.launcher_left_closed()
    tail = sorted(took)[math.ceil(OPENED_TAIL * len(took)) - 1]
    assert tail <= OPENED_WITHIN, (
        f"an opening through the storm took {tail} ms to its frame at the tail: {sorted(took)}"
    )


@pytest.mark.usefixtures("keyboard", "loaded")
def test_the_launcher_draws_each_key_soon_while_every_core_is_busy(desktop: Desktop) -> None:
    """The launcher draws each key soon after it's pressed, while every core is busy."""
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    timing = Timing(desktop)
    try:
        desktop.down_to("Programs")
        desktop.step_in("ret")
        desktop.launcher_until(lambda shown: shown["held"] == ["programs"])
        rows = desktop.showing()["count"]
        assert rows > 1, f"Programs lists {rows} rows, too few to move through"
        timing.begin()
        # Up and down its whole length, by the keys its mode moves by (j
        # and k in normal), turning at each end, where a key would move
        # nothing and so draw nothing.
        at, step = 0, 1
        down, up = desktop.row_keys()
        for _ in range(ARROWS):
            if not 0 <= at + step < rows:
                step = -step
            timing.press(down if step > 0 else up)
            at += step
        timing.keep()
        desktop.close_launcher()
        desktop.as_account("qs ipc call launcher toggle")
        desktop.launcher_ready()
        timing.begin()
        for word in WORDS:
            for letter in word:
                timing.press(letter)
            for _ in word:
                timing.press("backspace")
        timing.keep()
    finally:
        if not timing.loading:
            timing.load(on=True)
        desktop.close_launcher()
    timing.said_and_held()

"""The desktop test's keeps pace area.

Here the launcher keeps pace with the keys while the machine is busy. Every core is loaded in the
account's jobs.slice and in build.slice, where a workstation's agents and builds run, and each key
is timed from the moment a stand-in keyboard pressed it to the first frame the launcher drew for
it, both on the machine's one clock. Keys come in turns with the load running and frozen, and
those under load are held against the others, so the builder's own load slows both alike, and
each test says how much of its time the guest counted stolen in each turn, which a last test
proves it can count, so a run that fails shows whether the builder took the time. That
text a program types at once is searched once, and a Return among it acts on what it found, is
proven on a nested niri, in modules/quickshell/tests/launcher-on-niri-test.py. Long because its
keys, steps and openings are each timed against the same loaded desktop, and a split would boot it
again. tests/desktop.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), and
its opening comment says what the test proves. Every test asks for the desktop, started once a run
(desktop_session.py), and leaves it as it found it.
"""

from __future__ import annotations

import contextlib
import json
import math
import os
import re
import shlex
import time
from datetime import timedelta
from functools import partial
from pathlib import Path
from typing import TYPE_CHECKING, NamedTuple, cast

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
from keeps_pace import PROCESSORS, Ledger, holdable

if TYPE_CHECKING:
    from collections.abc import Iterator, Sequence

    from desktop_session import Desktop, Json

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
# and the tail can't tell that from the shell. A desktop the load starves
# doesn't show in the tail either: with the desktop's slices weighed 1 its slowest key in twenty
# took 124 ms against 442 ms frozen. So beside the middle key, what the machine's own weights
# decide is judged: how long the compositor and the shell were ready to run and waited for a
# processor, which the guest's kernel counts in each one's cpu.pressure. Waking an idle
# processor waits on the builder too, so they wait with the load frozen as well, 20 to 38 ms a
# key on 5 November. The load adds to that wait rather than multiplying it, so under load a key
# may wait WAITED ms more than frozen. Weighed as a workstation weighs them they waited 15 and
# 22 ms more (50 and 60 against 35 and 38), and in the net's 48 runs from 12 to 15 November 20
# to 37.5 ms more, a median 26.8, whether the frozen wait was 17 ms or 31. Weighed 1 they waited
# 58 ms more, 78 against 20. Twice the frozen wait and 10 ms more failed 5 of those 48 runs with
# the weights as they should be, four where the frozen wait came in at 17 to 20 ms; and it failed
# #2724 on 9 November at 21.8 to 26.1 against 3.9 to 6.9, 18 to 19 ms more.
WAITED = 45
SLOWER_MOST = 2
MIDDLE_MORE = 50
# The builder's processors the held subtest runs the VM's four on, so under load each runs about
# half the time, the least part of the guest's busy time it must count stolen for that, and how
# long it counts, in seconds. Held so on 19 November the guest counted 45 to 50% stolen, where
# unheld it counted under 1 to 8%, by how busy the builder was. Held to one, the frozen guest
# starved as much as the loaded one, its idle processors' halts and QEMU's own threads taking
# the builder's one, so that's no busy builder.
HELD_TO = 2
HELD_STOLEN = 0.25
GAUGING = 10
# A halted processor has nothing stolen, so little stolen held is a count blind to the builder
# only while the four are busy: at least this part of every tick the guest counted, by its own
# clock, so no read's round trip weighs in. Held so on 19 November they counted 4204 to 4318 in
# a little over 10 s, of about 4300 the four could.
HELD_BUSY = 0.75
# The ticks /proc/stat counts a processor each second, and the fewest the four may count while
# held, a quarter of what they would in GAUGING seconds, so a window the guest stopped counting
# in fails as that rather than passing on a few ticks. A guest blind to steal, held to two of
# four, would count about half, so it's still the bar on what was stolen that fails it. A slow
# read only lengthens the window, and time the builder takes in it the guest counts as stolen.
USER_HZ = 100
HELD_TICKS = PROCESSORS * GAUGING * USER_HZ // 4
# The passes holding the VM's threads may take, past which a hold still missing one fails as that.
HOLDING = 5
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
# (Launcher.qml's drawerMade), and the launcher's own making of it is held to the same. Its
# wait for its frame and its drawing are said beside it, not judged: under load they took 68
# to 148 ms and 23 to 83 of a drawer's 100 to 218, its making 2 to 19, and Qt's render loop
# put that frame time in the swap onto niri, 11 to 183 ms, where polish and render took 0 to
# 12. Judged whole, the same commit passed in one run and failed in the next (2026-11-11). The
# render thread's swap reaches QML only queued, behind whatever the GUI thread does, so it
# can't be taken out as a key's is.
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
# event at a time. The launcher is opened OPENINGS times through it, every core busy: nineteen
# in twenty openings are ready to draw within OPENED_WITHIN ms, and none past OPENED_AT_MOST,
# so one taking seconds fails though the rest are quick. Joel's took 6,184 ms to its frame for
# 107 ms of its own work on 2026-11-10. Ready to draw is its own work, the shell settling after
# it and the wait for the next frame to begin; the drawing itself is llvmpipe's here, not the
# laptop's GPU, so it's held to its time with the load frozen, over FROZEN openings, and a whole
# frame within OPENED_WITHIN is judged from Joel's own journal. The tail is TAIL's, the 19th of
# 20, as everywhere else here: the 99th of 20 was the slowest alone, and one opening a loaded
# builder held up failed it with the next at 10 ms (88 ms against 10, 2026-11-17). The slowest
# of 70 runs on the net by 2026-11-18 was 91 ms, so OPENED_AT_MOST leaves a lone slow one room
# and still fails one taking seconds. Under every core busy, the storm's windows may take more
# than REACHING to be listed, as they did once at 31.8 s on 2026-11-17, so they're given
# STORM_PATIENCE to come and to go; that isn't judged.
STORMING = 40
STORM = 100
STORM_PATIENCE = timedelta(minutes=2)
OPENINGS = 20
OPENED_WITHIN = 50
OPENED_AT_MOST = 500
FROZEN = 8


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
    # The share the builder splits between the VM's processors is read as at most their count.
    processors = int(machine.succeed("nproc --all").strip())
    assert processors == PROCESSORS, f"the VM has {processors} processors, not {PROCESSORS}"
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
        # The keyboard's lines before this run of keys, and each key's turn with the load's.
        self.logged = self.timed = self.pressing = self.presses = 0
        self.key_turns: list[tuple[int, bool]] = []
        self.loading = True
        # How long, in ms, the compositor and the shell waited for a processor in each turn, and
        # the turn being pressed with what they'd waited when it began.
        self.waited: dict[int, float] = {}
        self.open: tuple[int, float] | None = None
        # The guest's ticks the builder stole, and its busy ones with them, by turn.
        self.ledger = Ledger(desktop.machine.succeed("head -1 /proc/stat"))
        show = "show --property=ControlGroup --value"
        self.groups = [
            desktop.machine.succeed(f"systemctl {show} {COMPOSITOR}").strip(),
            desktop.as_account(f"systemctl --user {show} {SHELL}").strip(),
        ]

    def begin(self) -> None:
        """Begin a run of keys, from where the keyboard's lines and the launcher's count stand."""
        self.logged = int(self.desktop.machine.succeed(f"wc -l < {SAID}").strip())
        self.timed = framed(self.desktop)["timed"]
        self.pressing = 0
        self.key_turns = []
        # What the shell did before the keys is timed by no bar, so its ticks are passed over.
        self.count(kept=False)

    def load(self, *, on: bool) -> None:
        """Let the machine's load run, or freeze it where it stands, which takes no time back.

        The ticks before it turns go to the turn that's ending.
        """
        self.count(kept=True)
        self.ledger.turning()
        self.thaw_or_freeze(on=on)
        # The ticks while the load turned are neither turn's, so they're passed over, and only
        # then is the turn done: one that fails here leaves the ledger between turns.
        self.count(kept=False)
        self.ledger.turned()

    def thaw_or_freeze(self, *, on: bool) -> None:
        """Let the load run or freeze it, counting nothing, as main's load did.

        A count that just failed would likely fail again, so a thaw that mustn't wait on one
        comes here.
        """
        verb = "thaw" if on else "freeze"
        # A builder stalling the VM held a freeze for 50 s on 5 November, past the 25 s systemctl
        # waits for its answer by default, though the unit froze. FREEZING is longer than that.
        wait = f"SYSTEMD_BUS_TIMEOUT={round(FREEZING.total_seconds())}"
        self.desktop.as_account(f"{wait} systemctl --user {verb} stress-jobs", timeout=2 * FREEZING)
        self.desktop.machine.succeed(f"{wait} systemctl {verb} stress-build")
        self.loading = on

    def count(self, *, kept: bool) -> None:
        """Count the ticks since the last count, kept with the load as it stands."""
        stat = self.desktop.machine.succeed("head -1 /proc/stat")
        self.ledger.count(stat, turn=self.loading if kept else None)

    def finish(self) -> None:
        """Say what the guest counted stolen, then let the load run again if it's frozen.

        Said before any of a test's bars is judged, so a run that fails says it too, and before
        the thaw, which a stalled builder can fail. A last count that fails still says the turns
        already counted, without the last, and a load left between turns is let run too.
        """
        try:
            self.count(kept=True)
        finally:
            print(self.ledger.said())
            # Let run as main always tried, the last count failed or not: nothing reads the
            # ledger now, so the thaw counts nothing first, which could fail as the last did.
            if not self.loading or self.ledger.between:
                self.thaw_or_freeze(on=True)

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
        self.key_turns.append((turn, loaded))
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
            f"[ $(wc -l < {SAID}) -ge {self.logged + self.pressing} ]",
            timeout=timedelta(seconds=10),
        )
        lines = machine.succeed(f"tail -n +{self.logged + 1} {SAID}").splitlines()
        noted = self.caught_up()
        self.close_turn()
        # The turn ends once its keys are caught up, so its ticks go to it before what follows.
        # Counted after its wait is read, so the wait spans what main's did.
        self.count(kept=True)
        assert noted["clock"] == "monotonic", f"the launcher keeps time by {noted['clock']}"
        kept = len(noted["frames"])
        assert len(noted["pressed"]) == len(noted["heard"]) == kept, (
            f"{len(noted['pressed'])} keys pressed and {len(noted['heard'])} heard beside "
            f"{kept} frames"
        )
        times = zip(
            lines,
            self.key_turns,
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
        assert wait <= wait_free + WAITED, (
            f"under load the compositor and the shell waited {wait:.1f} ms a key for a processor "
            f"in the middle turn, against {wait_free:.1f} ms with the load frozen"
        )


def ranked(times: Sequence[float], share: float) -> float:
    """Give the time the share of `times`, sorted, falls within."""
    return times[max(0, math.ceil(share * len(times)) - 1)]


def selects(key: str, shown: Json) -> bool:
    """Say whether the selected row is the thing this key names, as a program's row."""
    return (shown["selected"] or {}).get("is") == key


class Drawn(NamedTuple):
    """A drawer's time to its frame, as the launcher split it (Launcher.qml's drawerMade)."""

    took: int
    making: int
    waiting: int
    drawing: int

    def judged(self) -> int:
        """Give the part of the drawer's time that's the launcher's own: its making."""
        return self.making


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


def makings_said(desktop: Desktop, since: str) -> list[str]:
    """Give each making of a drawer or acts said by its parts since the mark (Program.js).

    Programs' drawer made in 20 to 40 ms under load here, which is said, not judged, with
    nothing saying which part (2026-11-14).
    """
    said = desktop.as_account(
        "journalctl --user --unit=quickshell --no-pager --output=cat"
        f" --after-cursor={shlex.quote(since)} | grep -a -F 'applications: '"
        " | grep -a -F ' made in '; true"
    )
    return [line.strip() for line in said.splitlines() if line.strip()]


def slowed_drawers(drawn: dict[bool, dict[str, list[Drawn]]]) -> list[str]:
    """Say each place's drawers by turns, and give those the load slowed past their bound.

    Each is judged by its making alone (Drawn.judged), its wait and drawing said beside it.
    """
    slowed = []
    for place in sorted(drawn[True]):
        loaded = sorted(one.judged() for one in drawn[True][place])
        frozen = sorted(one.judged() for one in drawn[False].get(place, []))
        print(
            f"{place}'s drawer: under load {sorted(drawn[True][place])} ms, made in {loaded},"
            f" against {sorted(drawn[False].get(place, []))}, made in {frozen},"
            " with the load frozen"
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
    made: dict[bool, list[str]] = {True: [], False: []}
    try:
        stepped_round(desktop)
        # The round warming the drawers up is timed by no bar, so its ticks are passed over.
        timing.count(kept=False)
        for turn in range(ROUNDS):
            loaded = turn % 2 == 1
            if loaded != timing.loading:
                timing.load(on=loaded)
            since = desktop.mark()
            timed = framed(desktop)["timed"]
            for place, drawer in stepped_round(desktop).items():
                drawn[loaded].setdefault(place, []).append(drawer)
            early.extend(timed_early(framed(desktop), timed))
            made[loaded].extend(makings_said(desktop, since))
            for place, took, search, line in steps_said(desktop, since):
                (under if loaded else free).setdefault(place, []).append(took)
                if loaded:
                    searching.setdefault(place, []).append(search)
                    slowest[place] = max(slowest.get(place, (0, "")), (took, line))
    finally:
        timing.finish()
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
    print("under load, made by their parts:", *made[True], sep="\n  ")
    assert "Programs" in under, f"no step into Programs was timed: {sorted(under)}"
    assert "Programs" in drawn[True], f"no drawer in Programs was timed: {drawn}"
    assert not drawers_slowed, f"the load slowed the drawers of {drawers_slowed}: {drawn}"
    assert not early, f"keys done at and timed by frames begun before: {early}"
    assert not slowed, f"the load slowed stepping into {slowed}: {under} against {free}"


def openings(desktop: Desktop) -> list[Json]:
    """Give the last openings the launcher timed, each from its ask to its first frame."""
    return cast("list[Json]", json.loads(desktop.as_account("qs ipc call launcher openings")))


def latest(timed: list[Json]) -> int:
    """Give the number of the last opening timed, as only the last 20 are kept."""
    return cast("int", timed[-1]["nth"]) if timed else 0


def opened_timed(desktop: Desktop) -> Json:
    """Open the launcher as its chord does, and give that opening's times once it's usable.

    Its first frame isn't what Joel sees, so it's kept open until it says when it was usable,
    the shell's thread answering on time (Joel, 2026-11-10: "It takes 7 seconds for me to see
    the launcher window"), and until niri says it presented the opening's frame, which is what
    he sees (Joel, 2026-11-10: "time from trigger to visible on the screen").
    """
    timed = latest(openings(desktop))
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    each: list[Json] = []

    def drawn(_: bool) -> bool:
        each[:] = openings(desktop)
        if latest(each) <= timed:
            return False
        return "usable" in each[-1] and "presented" in each[-1]

    retry(drawn, timeout=REACHING)
    nth = each[-1]["nth"]

    def filled(_: bool) -> bool:
        each[:] = [one for one in openings(desktop) if one["nth"] == nth]
        return bool(each) and ("filled" in each[-1] or not each[-1].get("kept"))

    # A kept home's fill says its rows once home is found again after its frame, which can
    # come after it's usable, so the storm's check holds home as found to the frame's. One
    # never said fails here, saying so, rather than as the frame's wait running out.
    try:
        retry(filled, timeout=REACHING)
    except Exception as error:
        message = f"a kept opening's fill was never said: {each}: {error}"
        raise AssertionError(message) from error
    desktop.close_launcher()
    opening = each[-1]
    # Software painting under a deliberate load makes the machine's own figure no measure of
    # Joel's 50 ms, which his shell's journal gives; here niri must have presented the
    # frame, and no sooner than the launcher drew it.
    assert opening["presented"] >= opening["took"], f"niri never presented the opening: {opening}"
    return opening


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
    try:
        desktop.as_account(f"echo '{LISTED} 1 listed' > {CROWD}.new && mv {CROWD}.new {CROWD}")
        # Setting it up is timed by no bar, so its ticks are passed over, as the others' are.
        timing.count(kept=False)
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
        timing.finish()
        desktop.as_account(f"echo '0 0' > {CROWD}.new && mv {CROWD}.new {CROWD}")
        desktop.launcher_left_closed()

    def middle(times: list[int]) -> int:
        return sorted(times)[(len(times) - 1) // 2]

    for loaded, opening, after in said:
        print(
            f"{'under load' if loaded else 'with the load frozen'}, the first opening after a"
            f" start {opening}, and the one after it {after}"
        )
    unusable = [
        each for _, opening, after in said for each in (opening, after) if each["usable"] < 0
    ]
    assert not unusable, f"an opening never answered on time for 100 ms: {unusable}"
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
    # Stopped however the wait for its windows ends, so a storm that's slow
    # to list never retitles through the rest of the module.
    try:
        stormy = 'niri msg --json windows | grep -o \'"title":"storm \' | wc -l'
        listed = f"test $({stormy}) -eq {STORMING}"
        machine.wait_until_succeeds(desktop.in_session(listed), timeout=STORM_PATIENCE)
        yield
    finally:
        # Bracketed, so the pattern never matches the shell running it. Its windows are given as
        # long to go as to come, under the same load.
        desktop.as_account("pkill -f 'window-stor[m]'")
        gone = '! niri msg --json windows | grep -q \'"title":"storm \''
        machine.wait_until_succeeds(desktop.in_session(gone), timeout=STORM_PATIENCE)


def opened_from_kept(desktop: Desktop) -> Json:
    """Open the launcher once the last close has kept home, and give that opening as timed."""
    retry(lambda _: bool(desktop.showing()["kept"]), timeout=REACHING)
    opening = opened_timed(desktop)
    assert opening["kept"], f"an opening wasn't drawn from what was kept: {opening}"
    # Home found again by its slice, as no key or click comes, is what the rows filled in hold;
    # found any other way, or not at all, they'd be the kept rows compared with themselves.
    assert opening.get("filledFoundBy") == "slice", f"home wasn't found again: {opening}"
    return opening


def ready(opening: Json) -> int:
    """Give how long an opening took until its frame began to be drawn: worked, settled, waited."""
    after = opening["after"]
    return int(opening["worked"] + after["settling"] + after["waiting"])


@pytest.mark.usefixtures("loaded", "storm")
def test_openings_are_soon_ready_to_draw_through_a_storm_of_windows_changing(
    desktop: Desktop,
) -> None:
    """Openings are soon ready to draw through a storm of windows changing, every core busy.

    Nineteen in twenty are ready within OPENED_WITHIN ms, and none takes past OPENED_AT_MOST.

    Joel's crowd of agents' windows retitle themselves all day, and an opening that waited on
    the shell making every list again for each change took seconds to show (2026-11-10). So a
    closed launcher keeps home, and an opening draws that before the rest of the shell wakes to
    fill it in, where its rows stand. Its drawing is held to its drawing with the load frozen.
    """
    timing = Timing(desktop)
    each: dict[bool, list[Json]] = {True: [], False: []}
    try:
        # The first opening after the shell starts makes every row, with nothing kept yet.
        opened_timed(desktop)
        # That opening is timed by no bar, so its ticks are passed over.
        timing.count(kept=False)
        for loaded, times in ((True, OPENINGS), (False, FROZEN)):
            if loaded != timing.loading:
                timing.load(on=loaded)
            for _ in range(times):
                opening = opened_from_kept(desktop)
                print(
                    f"through the storm, {'under load' if loaded else 'with the load frozen'},"
                    f" an opening ready to draw in {ready(opening)} ms: {opening}"
                )
                each[loaded].append(opening)
    finally:
        timing.finish()
        desktop.launcher_left_closed()

    def middle(times: list[int]) -> int:
        return sorted(times)[(len(times) - 1) // 2]

    def standing(rows: Json) -> tuple[object, object]:
        return rows["keys"], rows["at"]

    moved = [
        one for one in each[True] + each[False] if standing(one["shown"]) != standing(one["filled"])
    ]
    assert not moved, f"filled in, an opening's rows or cursor moved: {moved}"
    readied = sorted(ready(one) for one in each[True])
    tail = ranked(readied, TAIL)
    assert tail <= OPENED_WITHIN, (
        f"an opening through the storm took {tail} ms to be ready to draw at the tail: {readied}"
    )
    assert readied[-1] <= OPENED_AT_MOST, (
        f"an opening through the storm took {readied[-1]} ms to be ready to draw: {readied}"
    )
    drawing = {loaded: [one["after"]["drawing"] for one in each[loaded]] for loaded in each}
    assert middle(drawing[True]) <= SLOWER_MOST * middle(drawing[False]) + STEPPED_MORE, (
        f"the load slowed drawing an opening through the storm: {drawing[True]} ms against "
        f"{drawing[False]} ms frozen"
    )


@pytest.mark.usefixtures("keyboard", "loaded")
def test_the_launcher_draws_each_key_soon_while_every_core_is_busy(desktop: Desktop) -> None:
    """The launcher draws each key soon after it's pressed, while every core is busy."""
    timing = Timing(desktop)
    try:
        # Opened inside, so a launcher too slow to open under load says what was stolen too.
        desktop.as_account("qs ipc call launcher toggle")
        desktop.launcher_ready()
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
        timing.finish()
        desktop.close_launcher()
    timing.said_and_held()


def vm_threads(started: int) -> dict[int, bool]:
    """Give every thread of the process that started the VM and of those it started.

    They're QEMU's and the virtiofsd's beside it, which run the VM's work too. Each says whether
    the kernel lets it be held to processors, so one it refuses is passed over rather than
    failing the hold or leaving the threads after it unheld, and the held subtest names it.
    """
    children: dict[int, list[int]] = {}
    for process in Path("/proc").iterdir():
        if not process.name.isdigit():
            continue
        # A process that ended as this read is no part of the VM, which runs on; the kernel
        # says so either way.
        with contextlib.suppress(FileNotFoundError, ProcessLookupError):
            # The parent follows the name, which may hold spaces, in its parentheses.
            parent = int((process / "stat").read_text().rsplit(")", 1)[1].split()[1])
            children.setdefault(parent, []).append(int(process.name))
    pids = [started]
    threads: dict[int, bool] = {}
    while pids:
        pid = pids.pop()
        pids.extend(children.get(pid, []))
        with contextlib.suppress(FileNotFoundError, ProcessLookupError):
            for thread in Path(f"/proc/{pid}/task").iterdir():
                # One gone as this read is none to hold.
                with contextlib.suppress(FileNotFoundError, ProcessLookupError):
                    threads[int(thread.name)] = holdable((thread / "stat").read_text())
    return threads


def holdable_threads(started: int) -> list[int]:
    """Give the VM's threads the kernel lets be held to processors."""
    return [thread for thread, held in vm_threads(started).items() if held]


def hold_vm(started: int, processors: Sequence[int]) -> None:
    """Run each of the VM's threads on these of the builder's processors alone.

    A thread started as they're held takes its starter's processors, so each is held again
    until a pass finds none left to hold. One still moving after HOLDING passes fails as that,
    since a VM left partly held would slow whatever runs on it with no line saying why.
    """
    for _ in range(HOLDING):
        moved = [hold_thread(thread, processors) for thread in holdable_threads(started)]
        if not any(moved):
            return
    unsettled = f"the VM's threads still moved after {HOLDING} passes holding them to {processors}"
    raise AssertionError(unsettled)


def hold_thread(thread: int, processors: Sequence[int]) -> bool:
    """Run one of the VM's threads on these of the builder's processors, saying if it moved."""
    try:
        if os.sched_getaffinity(thread) == set(processors):
            return False
        os.sched_setaffinity(thread, processors)
    except ProcessLookupError:
        # QEMU's workers come and go, and one gone is none to hold.
        return False
    except OSError as error:
        # holdable_threads leaves out the threads whose flags say the kernel won't hold them, so
        # a refusal here is one no flag foretold, and is named.
        name = "gone"
        with contextlib.suppress(OSError):
            name = Path(f"/proc/{thread}/comm").read_text().strip()
        refused = f"the builder wouldn't hold the VM's thread {thread} ({name}) to {processors}"
        raise AssertionError(refused) from error
    return True


@pytest.mark.usefixtures("loaded")
def test_the_guest_counts_what_a_builder_holding_it_back_takes(desktop: Desktop) -> None:
    """The guest counts as stolen the time the builder runs its processors on fewer of its own.

    Held to HELD_TO of the builder's processors, the VM's four run about half the time each under
    load, when every one is busy, as a busy builder holds them. A guest whose four were busy
    and that counted little stolen here has a count blind to the builder, which no other
    subtest's bars could tell from a quiet builder, and every share would read 1.
    """
    started = desktop.machine.pid
    assert started, "the VM has no process to hold to the builder's processors"
    # The VM started with the test's own, so they're what it's given back.
    builders = sorted(os.sched_getaffinity(0))
    assert len(builders) >= HELD_TO, f"the builder gives this test processors {builders} only"
    timing = Timing(desktop)
    try:
        hold_vm(started, builders[:HELD_TO])
        # The ticks before the hold took aren't held ones, so they're passed over.
        timing.count(kept=False)
        began = time.monotonic()
        # Said with the builder's processors, so little stolen tells a hold that never took,
        # or one that changed nothing, from a count blind to one.
        held: dict[str, int] = {}
        threads = vm_threads(started)
        for thread in (thread for thread, holds in threads.items() if holds):
            with contextlib.suppress(ProcessLookupError):
                on = ",".join(map(str, sorted(os.sched_getaffinity(thread))))
                held[on] = held.get(on, 0) + 1
        # Those the kernel refuses to hold are named, so a hold that missed them says which.
        refused: list[str] = []
        for thread in (thread for thread, holds in threads.items() if not holds):
            with contextlib.suppress(FileNotFoundError, ProcessLookupError):
                refused.append(Path(f"/proc/{thread}/comm").read_text().strip())
        print(
            f"the VM's threads, by the builder's processors each may run on: {held},"
            f" of the builder's {builders}, and those the kernel lets no one hold: {refused}"
        )
        # A hold that didn't take fails as that, so the bar on what was stolen judges the count.
        holding = ",".join(map(str, builders[:HELD_TO]))
        assert list(held) == [holding], f"the VM's threads weren't held to {holding}: {held}"
        time.sleep(GAUGING)
        timing.count(kept=True)
        gauged = time.monotonic() - began
        stolen, busy = timing.ledger.turns[True]
        every = timing.ledger.ticked[True]
        # Said before the VM is let go, which may fail, so the figures it counted are kept.
        print(
            f"held, the guest counted {stolen} of {busy} busy ticks stolen, of {every} ticks"
            f" in {gauged:.1f} s"
        )
    finally:
        hold_vm(started, builders)
    assert every >= HELD_TICKS, (
        f"held, the guest counted {every} ticks in {gauged:.1f} s, under the {HELD_TICKS} its"
        f" processors count in a quarter of {GAUGING} s"
    )
    assert busy >= HELD_BUSY * every, (
        f"held, the guest's processors were busy {busy} of {every} ticks, under {HELD_BUSY:.0%}"
    )
    assert stolen / busy >= HELD_STOLEN, (
        f"held to {HELD_TO} of the builder's processors, the guest counted {stolen} of {busy}"
        " busy ticks stolen"
    )

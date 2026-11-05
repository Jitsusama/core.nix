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
import time
from datetime import timedelta
from typing import TYPE_CHECKING, Any, cast

import desktop_session
import pytest
from desktop_session import retry

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
            strict=True,
        )
        for line, (turn, loaded), pressed, heard, frame in times:
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
        assert middle <= SLOWER_MOST * middle_free + MIDDLE_MORE, (
            f"half the keys under load took {middle:.0f} ms to their frames, against "
            f"{middle_free:.0f} ms with the load frozen"
        )
        assert wait <= SLOWER_MOST * wait_free + WAITED, (
            f"under load the compositor and the shell waited {wait:.1f} ms a key for a processor "
            f"in the middle turn, against {wait_free:.1f} ms with the load frozen"
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

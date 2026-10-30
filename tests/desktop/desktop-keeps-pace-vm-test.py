"""The desktop test's keeps pace area.

Here the launcher keeps pace with the keys while the machine is busy. Every core is loaded in the
account's jobs.slice and in build.slice, where a workstation's agents and builds run, and each key
is timed from the moment a stand-in keyboard pressed it to the first frame the launcher drew for
it, both on the machine's one clock. tests/desktop.nix runs this through pytest inside the driver
(tests/lib/vm_pytest.py), and its opening comment says what the test proves. Every test asks for
the desktop, started once a run (desktop_session.py), and leaves it as it found it.
"""

from __future__ import annotations

import json
import math
import time
from datetime import timedelta
from typing import TYPE_CHECKING, Any, cast

import desktop_session
import pytest
from desktop_session import LAUNCHER_OPEN, retry

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
# Design.md's budget is 50 ms from a key to its frame, and 100 ms for all but one key in a
# hundred. The VM draws in software on a builder shared with other checks, so this holds it to
# ten times that: a launcher that falls behind its work by whole seconds fails, and the
# laptop's own times come from its load proof, which reads the same list of frames.
MIDDLE_MOST = 500
P99_MOST = 1000
# How often a key is pressed: a steady typist's pace, slower than the frames a key mostly takes,
# so each key mostly reaches its own. Asking the launcher after each key whether it's drawn
# would pace them instead, and under this load each asking takes over a second.
PACE = timedelta(milliseconds=400)
# How many keys' frames the launcher keeps, so the most a run of keys may be before it's read.
KEPT = 300
# How long the last key may take to reach its frame before the test says it never did.
REACHING = timedelta(seconds=30)


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
    late frame of one before it.
    """

    def __init__(self, desktop: Desktop) -> None:
        """Begin with no key timed."""
        self.desktop = desktop
        self.took: list[float] = []
        self.next = time.monotonic()
        self.said = self.timed = self.pressing = 0

    def begin(self) -> None:
        """Begin a run of keys, from where the keyboard's lines and the launcher's count stand."""
        self.said = int(self.desktop.machine.succeed(f"wc -l < {SAID}").strip())
        self.timed = framed(self.desktop)["timed"]
        self.pressing = 0

    def press(self, key: str) -> None:
        """Press a key on the beat, however long telling the keyboard took.

        A beat missed, while the test did something else, starts the beat again rather than
        pressing the keys it missed at once.
        """
        self.next = max(self.next, time.monotonic())
        self.desktop.machine.succeed(f"echo {key} > {KEYBOARD}")
        self.pressing += 1
        self.next += PACE.total_seconds()
        time.sleep(max(0.0, self.next - time.monotonic()))

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
        frames = noted["frames"][-self.pressing :]
        for line, frame in zip(lines, frames, strict=True):
            key, down = line.split()
            took = frame - float(down)
            assert took >= 0, f"{key}'s frame came {-took:.0f} ms before it was pressed"
            self.took.append(took)

    def said_and_held(self) -> None:
        """Say how long keys took to their frames, and hold them to ten times the budget."""
        took = sorted(self.took)

        def ranked(share: float) -> float:
            return took[max(0, math.ceil(share * len(took)) - 1)]

        middle, p95, p99 = ranked(0.5), ranked(0.95), ranked(0.99)
        print(
            f"{len(took)} keys under load, from the keyboard to their frames: half within "
            f"{middle:.0f} ms, 95% within {p95:.0f} ms, 99% within {p99:.0f} ms, "
            f"the slowest {took[-1]:.0f} ms"
        )
        assert middle < MIDDLE_MOST, f"half the keys took over {middle:.0f} ms to their frames"
        assert p99 < P99_MOST, f"one key in a hundred took over {p99:.0f} ms to its frame"


@pytest.mark.usefixtures("keyboard", "loaded")
def test_the_launcher_draws_each_key_soon_while_every_core_is_busy(desktop: Desktop) -> None:
    """The launcher draws each key soon after it's pressed, while every core is busy."""
    machine = desktop.machine
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
        # Up and down its whole length, turning at each end, where a key
        # would move nothing and so draw nothing.
        at, step = 0, 1
        for _ in range(ARROWS):
            if not 0 <= at + step < rows:
                step = -step
            timing.press("down" if step > 0 else "up")
            at += step
        timing.keep()
        machine.send_key("esc")
        desktop.launcher_until(lambda shown: not shown["open"])
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
        if desktop.showing()["open"]:
            machine.send_key("esc")
        machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))
    timing.said_and_held()

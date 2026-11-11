"""The desktop test's draws-on-drm area: a lab, which judges nothing.

niri drives the VM's screen itself here, through the kernel's vkms, whose vblanks come from a
timer at the refresh as a panel's do, rather than nested in Cage as every other area runs it. So
what niri's frame timing costs the launcher's keys shows, as it can't nested: modules/niri's
render-deadline.patch renders each frame just before the vblank it's for, where niri rendered it
as soon as the one before had passed. Each round steps through Programs at a held key's pace and
types at home at a typist's, with the deadline on and off by turns (debug's
disable-render-deadline), with the machine quiet and every core busy, and prints what the
launcher's journal says of each opening's keys and frames. Its sides are compared by a person,
and Joel's own journal after a switch gives the figure that counts (docs/testing.md, Timing the
Launcher). tests/desktop.nix runs this through pytest inside the driver.
"""

from __future__ import annotations

import json
import re
import shlex
import statistics
from datetime import timedelta
from typing import TYPE_CHECKING, Any, NamedTuple, cast

import desktop_session
import pytest
from desktop_session import retry

if TYPE_CHECKING:
    from collections.abc import Iterator

    from desktop_session import Desktop
    from test_driver.machine import BaseMachine

# Where the stand-in keyboard is told what to press (desktop/keyboard.py).
KEYBOARD = "/run/keyboard"
# The account's niri config, and the same with the deadline off beside it, where its includes
# are found. The off side names a workspace too, which niri msg lists once niri has loaded it,
# since niri says nothing when a config loads.
CONFIG = "~/.config/niri/config.kdl"
OFF = "~/.config/niri/deadline-off.kdl"
OFF_SAYS = "deadline-off"
OFF_ADDS = f'debug {{\n    disable-render-deadline\n}}\nworkspace "{OFF_SAYS}"\n'
# Each side's rounds, alternating which goes first so a drift in the builder falls on both.
ROUNDS = 3
# A held key's repeats, about thirty a second, and a steady typist's letters.
HELD = 0.033
TYPED = 0.12
STEPS = 30
WORDS = ["power", "sound", "files", "night", "music"]
# How long the launcher may take to time every key once the last is pressed.
REACHING = timedelta(seconds=60)
PRESENTED = re.compile(
    r"(\d+) keys presented a median (\d+) ms from the key, the slowest (\d+) ms, (\d+) within"
    r" 50 ms(?:: a median (\d+) ms before the shell heard it, (\d+) in the shell and (\d+) from"
    r" its frame's start)?"
)
LATE = re.compile(r"(\d+) of (\d+) frames came late while it moved, the longest (\d+) ms")


class Opening(NamedTuple):
    """What the launcher said of one opening's keys and frames."""

    keys: int
    median: int
    slowest: int
    within: int
    framed: int | None
    late: int
    drawn: int


@pytest.fixture(scope="module")
def desktop(machine: BaseMachine, values: dict[str, Any]) -> Desktop:
    """Give the desktop, started once a run."""
    return desktop_session.started(machine, cast("desktop_session.Values", values))


@pytest.fixture(scope="module")
def keyboard(desktop: Desktop) -> Iterator[None]:
    """Start the stand-in keyboard, and wait until niri, which reads it here, has it open."""
    machine = desktop.machine
    machine.succeed(f"systemd-run --unit=keyboard {desktop.values['keyboard']}")
    machine.wait_for_file(KEYBOARD, timeout=timedelta(seconds=30))
    node = machine.succeed("cat /run/keyboard.device").strip()
    machine.wait_until_succeeds(
        f"find /proc/$(pgrep -o -x niri)/fd -lname {node} | grep -q .",
        timeout=timedelta(seconds=30),
    )
    try:
        yield
    finally:
        machine.succeed("systemctl stop keyboard")


def deadline(desktop: Desktop, *, on: bool) -> None:
    """Have niri render at its deadline, or as soon as it can, and wait until it's loaded."""
    if not on:
        desktop.as_account(f"{{ cat {CONFIG}; printf '%s' {shlex.quote(OFF_ADDS)}; }} > {OFF}")
    desktop.as_account(f"niri msg action load-config-file --path {OFF if not on else CONFIG}")

    def loaded(_: bool) -> bool:
        workspaces = json.loads(desktop.as_account("niri msg -j workspaces"))
        names = [each.get("name") for each in workspaces]
        return (OFF_SAYS in names) != on

    retry(loaded, timeout=timedelta(seconds=30))


def load(desktop: Desktop, *, on: bool) -> None:
    """Load every core in the account's jobs.slice, as agents do, or stop it."""
    if on:
        stress = f"{desktop.values['stress_ng']}/bin/stress-ng --cpu 0 --quiet"
        desktop.as_account(
            f"systemd-run --user --collect --quiet --slice=jobs.slice --unit=lab-load {stress}"
        )
        desktop.as_account("systemctl --user is-active --quiet lab-load")
    else:
        # The quiet rounds come first, before any load has started to stop.
        desktop.as_account("systemctl --user stop lab-load || true")


def timed(desktop: Desktop) -> int:
    """Give how many keys the launcher has timed to their frames."""
    return int(json.loads(desktop.as_account("qs ipc call launcher framed"))["timed"])


def pressed(desktop: Desktop, keys: list[str], pace: float) -> None:
    """Press the keys at the pace from inside the machine, and wait until each reached a frame.

    A loop in the machine keeps the pace, where the driver's own call for each key would add
    its time to every gap.
    """
    before = timed(desktop)
    names = " ".join(keys)
    # The sleep is the pace itself, the gap between one key and the next.
    desktop.machine.succeed(f"for k in {names}; do echo $k > {KEYBOARD}; sleep {pace}; done")

    def reached(_: bool) -> bool:
        return timed(desktop) >= before + len(keys)

    retry(reached, timeout=REACHING)


def said(desktop: Desktop, since: str) -> Opening:
    """Wait for what the launcher says of an opening as it closes, and give it."""
    # The keys line is the journal's own, not one of the shell's reports, so it's read there.
    read = (
        "journalctl --user --unit=quickshell --no-pager --output=cat"
        f" --after-cursor={shlex.quote(since)}"
    )
    lines = ""

    def closed(_: bool) -> bool:
        nonlocal lines
        lines = desktop.as_account(read)
        # The launcher says its keys as it closes, presented then landed, and its late
        # frames in the same turn, only when some were.
        return " keys landed " in lines

    retry(closed, timeout=REACHING)
    lines = desktop.as_account(read)
    shown = PRESENTED.search(lines)
    assert shown is not None, f"the launcher said no keys presented:\n{lines}"
    late = LATE.search(lines)
    keys, median, slowest, within = (int(shown[n]) for n in range(1, 5))
    return Opening(
        keys=keys,
        median=median,
        slowest=slowest,
        within=within,
        framed=int(shown[7]) if shown[7] else None,
        late=int(late[1]) if late else 0,
        drawn=int(late[2]) if late else 0,
    )


def held(desktop: Desktop) -> Opening:
    """Step down Programs and back up at a held key's pace, and give what the launcher said."""
    since = desktop.mark()
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.down_to("Programs")
    desktop.step_in("ret")
    desktop.launcher_until(lambda shown: shown["held"] == ["programs"])
    steps = min(STEPS, desktop.showing()["count"] - 1)
    down, up = desktop.row_keys()
    pressed(desktop, [down] * steps + [up] * steps, HELD)
    desktop.close_launcher()
    return said(desktop, since)


def typed(desktop: Desktop) -> Opening:
    """Type words at home and rub them out at a typist's pace, and give what it said."""
    since = desktop.mark()
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    keys = [key for word in WORDS for key in [*word, *["backspace"] * len(word)]]
    pressed(desktop, keys, TYPED)
    desktop.close_launcher()
    return said(desktop, since)


def summed(openings: list[Opening]) -> str:
    """Say a side's openings in one line: the medians' median, and every late frame."""
    medians = statistics.median(each.median for each in openings)
    framed = [each.framed for each in openings if each.framed is not None]
    start = statistics.median(framed) if framed else "none"
    within = sum(each.within for each in openings)
    keys = sum(each.keys for each in openings)
    late = sum(each.late for each in openings)
    drawn = sum(each.drawn for each in openings)
    slowest = max(each.slowest for each in openings)
    return (
        f"presented a median {medians} ms from the key, {start} from the frame's start,"
        f" {within} of {keys} within 50 ms, the slowest {slowest}; {late} of {drawn} frames late"
    )


@pytest.mark.usefixtures("keyboard")
def test_niri_renders_by_its_deadline_and_at_once_by_turns(desktop: Desktop) -> None:
    """Say what the launcher's keys cost with niri's render deadline on and off, by turns."""
    seen: dict[tuple[str, bool, bool], list[Opening]] = {}
    try:
        for turn in range(ROUNDS):
            for busy in (False, True):
                load(desktop, on=busy)
                try:
                    for on in (True, False) if turn % 2 == 0 else (False, True):
                        deadline(desktop, on=on)
                        for pace, keys in (("held", held), ("typed", typed)):
                            opening = keys(desktop)
                            print(f"lab: round {turn} busy={busy} deadline={on} {pace}: {opening}")
                            seen.setdefault((pace, busy, on), []).append(opening)
                finally:
                    if busy:
                        load(desktop, on=False)
    finally:
        deadline(desktop, on=True)
    for (pace, busy, on), openings in sorted(seen.items()):
        where = "busy" if busy else "quiet"
        side = "on" if on else "off"
        print(f"lab: {pace}, {where}, deadline {side}: {summed(openings)}")

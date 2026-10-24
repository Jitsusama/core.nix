"""Dictation's chord held and tapped on a stand-in keyboard, heard by an account.

tests/dictation-chord.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py),
and its opening comment says what the test proves. Each test asks for the account listening
to dictation-chord as a fixture, and reads only what was heard since it began, so any test
runs alone with `-k`.
"""

from __future__ import annotations

import datetime as dt
from typing import TYPE_CHECKING

import pytest

if TYPE_CHECKING:
    from test_driver.machine import BaseMachine

HEARD = "/home/alice/heard"
# How long the listener may take to connect.
CONNECTING = dt.timedelta(seconds=30)
# How long a keyboard's going may take to be heard.
HEARING = dt.timedelta(seconds=10)
# How long a second line, which mustn't come, is given to come.
AFTERWARDS = dt.timedelta(seconds=1)
# How much sooner than its keys were held a chord may say it ended, in ms, as the stand-in
# keyboard's sleeps and the daemon's clock disagree a little.
EARLY = 50
# The longest a chord held 700 ms may say it was held, in ms, on a loaded runner.
LATE = 1500
# The longest a tap may say it was held, in ms.
TAP = 400


class Listener:
    """The account outside the input group, listening to dictation-chord."""

    def __init__(self, machine: BaseMachine) -> None:
        """Listen on that machine."""
        self.machine = machine

    def lines(self) -> list[str]:
        """Give every line heard so far."""
        return self.machine.succeed(f"cat {HEARD}").splitlines()

    def press(self, *steps: str) -> list[str]:
        """Press the steps on a new stand-in keyboard and give the lines heard since."""
        before = len(self.lines())
        self.machine.succeed(f"chord-keys {' '.join(steps)}")
        return self.lines()[before:]


def held(lines: list[str]) -> int:
    """Give the milliseconds the one line heard says the chord was held."""
    assert len(lines) == 1, lines
    word, ms = lines[0].split()
    assert word == "up", lines
    return int(ms)


@pytest.fixture(scope="module")
def listener(machine: BaseMachine) -> Listener:
    """Boot the machine until dictation-chord listens, and connect the account to it."""
    machine.wait_for_unit("dictation-chord.service")
    machine.wait_for_file("/run/dictation-chord/socket")
    # systemd-run finds socat itself, as a transient unit's own PATH may not.
    machine.succeed(
        "systemd-run --uid=alice --unit=listener"
        f" socat -u UNIX-CONNECT:/run/dictation-chord/socket CREATE:{HEARD}"
    )
    machine.wait_for_file(HEARD, timeout=CONNECTING)
    machine.succeed("systemctl is-active listener.service")
    return Listener(machine)


def test_the_account_cant_read_the_keyboard_itself(listener: Listener) -> None:
    """The account can't read the keyboard itself."""
    machine = listener.machine
    machine.succeed("systemd-run --unit=keyboard chord-keys 5000")
    machine.wait_until_succeeds("grep -l 'stand-in keyboard' /sys/class/input/event*/device/name")
    machine.fail("su alice -c 'cat /dev/input/event* > /dev/null'")


def test_a_held_chord_says_how_long_it_was_held(listener: Listener) -> None:
    """A held chord says how long it was held."""
    holding = 700
    ms = held(listener.press("leftmeta+", "d+", str(holding), "d-", "leftmeta-"))
    assert holding - EARLY <= ms < LATE


def test_letting_go_of_super_first_ends_it_too(listener: Listener) -> None:
    """Letting go of Super first ends it too."""
    holding = 500
    ms = held(listener.press("leftmeta+", "d+", str(holding), "leftmeta-", "d-"))
    assert ms >= holding - EARLY


def test_a_tap_says_it_was_short(listener: Listener) -> None:
    """A tap says it was short."""
    assert held(listener.press("leftmeta+", "d+", "d-", "leftmeta-")) < TAP


def test_d_alone_another_key_or_super_after_d_says_nothing(listener: Listener) -> None:
    """D alone, another key, or Super after D says nothing."""
    assert listener.press("d+", "300", "d-") == []
    assert listener.press("leftmeta+", "a+", "300", "a-", "leftmeta-") == []
    assert listener.press("d+", "leftmeta+", "300", "leftmeta-", "d-") == []


def test_a_keyboard_unplugged_mid_chord_ends_it_once(listener: Listener) -> None:
    """A keyboard unplugged mid-chord ends it once, and the next is heard afresh."""
    machine = listener.machine
    # The stand-in keyboard goes when chord-keys exits, half a second after
    # its last step, with both keys still down, as a dock's keyboard does
    # when the cable comes out during dictation. What's heard then comes as
    # the keyboard is taken down, after chord-keys has returned, so the line
    # is waited for, then a moment more for a second one that mustn't come.
    # The kernel lets a vanishing keyboard's keys go before its readers see
    # it gone, so the chord most often ends on those key-ups, and otherwise
    # on the daemon's own letting go when the read fails; either way, once.
    holding = 600
    before = len(listener.lines())
    machine.succeed(f"chord-keys leftmeta+ d+ {holding}")
    machine.wait_until_succeeds(f"test $(wc -l < {HEARD}) -gt {before}", timeout=HEARING)
    machine.sleep(AFTERWARDS)
    assert held(listener.lines()[before:]) >= holding - EARLY
    assert held(listener.press("leftmeta+", "d+", "d-", "leftmeta-")) < TAP

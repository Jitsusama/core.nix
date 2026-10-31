"""Root's a-call-is-on reads an account's PipeWire, and says a call only while one records.

tests/calls.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), and its
opening comment says what the tests prove. Each test asks for Joel's sound running as a
fixture, and one that starts a call ends it, so any test runs alone with `-k`.
"""

from __future__ import annotations

import shlex
from typing import TYPE_CHECKING

import pytest

if TYPE_CHECKING:
    from collections.abc import Iterator

    from test_driver.machine import BaseMachine

# What a-call-is-on exits with while a call is on, and while none is.
ON = 0
OFF = 1
# How many seconds the call program's stream has to start running in.
RECORDS_WITHIN = 30
# The call program's stream, running, as jq finds it in pw-dump's graph.
RECORDING = (
    '.[] | select(.info.props["application.process.binary"] == "zoom" and .info.state == "running")'
)
# Each node's id, state, class and name, to read why no stream ran.
NODES = (
    '.[] | select(.type == "PipeWire:Interface:Node")'
    ' | [.id, .info.state, .info.props["media.class"], .info.props["node.name"]]'
)


class Sound:
    """Joel's sound on the machine, and a call program recording from it."""

    def __init__(self, machine: BaseMachine) -> None:
        """Act on Joel's sound on that machine."""
        self.machine = machine

    def as_joel(self, command: str) -> str:
        """Run a command in Joel's account, with his PipeWire's runtime folder, giving output."""
        environment = "export XDG_RUNTIME_DIR=/run/user/1000; cd ~; "
        return self.machine.succeed("su - joel -c " + shlex.quote(environment + command))

    def asked(self) -> int:
        """Give what a-call-is-on exits with, asked as root, as an update asks."""
        status, _ = self.machine.execute("a-call-is-on")
        return status


@pytest.fixture(scope="module")
def sound(machine: BaseMachine) -> Sound:
    """Boot the machine with Joel's sound running and a call program ready to record."""
    machine.wait_for_unit("multi-user.target")
    machine.succeed("loginctl enable-linger joel")
    machine.wait_for_unit("user@1000.service")
    joel = Sound(machine)
    joel.as_joel("systemctl --user start pipewire.service pipewire-pulse.service")
    machine.wait_for_unit("wireplumber.service", "joel")
    # PipeWire's PulseAudio server names a stream for the program that opened it.
    joel.as_joel('cp "$(readlink -f "$(command -v pacat)")" zoom')
    return joel


@pytest.fixture
def call(sound: Sound) -> Iterator[Sound]:
    """Have the call program recording, until the test is done."""
    # Collected, so a call that failed leaves nothing to stop the next one starting.
    sound.as_joel("systemd-run --user --collect --unit=call ./zoom --record /dev/null")
    try:
        sound.machine.wait_until_succeeds(
            "su - joel -c 'XDG_RUNTIME_DIR=/run/user/1000 pw-dump --no-colors'"
            f" | jq -e {shlex.quote(RECORDING)}",
            timeout=RECORDS_WITHIN,
        )
    except Exception:
        print(sound.as_joel(f"pw-dump --no-colors | jq -c {shlex.quote(NODES)}"))
        print(sound.as_joel("wpctl status; pw-link -l; pw-metadata"))
        print(
            sound.as_joel("journalctl --user -u wireplumber -u pipewire-pulse -u call --no-pager")
        )
        raise
    yield sound
    # A test may have ended the call itself.
    sound.as_joel("systemctl --user stop call || true")


def test_nothing_recording_is_no_call(sound: Sound) -> None:
    """With Joel's sound running and nothing recording, no call is on."""
    assert sound.asked() == OFF


def test_a_call_program_recording_is_a_call_read_as_root(call: Sound) -> None:
    """A call program recording in Joel's PipeWire is a call, to root as well."""
    assert call.asked() == ON


def test_a_call_ended_is_no_call(call: Sound) -> None:
    """Once the call program stops recording, no call is on."""
    call.as_joel("systemctl --user stop call")
    assert call.asked() == OFF

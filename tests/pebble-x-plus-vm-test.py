"""Tones measured through a Pebble X Plus's tuning in a virtual machine, and on headphones.

tests/pebble-x-plus.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py),
and its opening comment says what the test proves. Each test asks for the state it starts from
as a fixture: Joel's session with the tones made and the tuning running, and the tones as the
speakers hear them with the tuning stopped, which the others are measured against. So any test
runs alone with `-k`. The tests need no value from Nix: the stand-in speakers and headphones
are named in the machine pebble-x-plus.nix builds.
"""

from __future__ import annotations

import datetime as dt
import math
from typing import TYPE_CHECKING

import pytest
from vm_helpers import as_account

if TYPE_CHECKING:
    from test_driver.machine import BaseMachine

# The RMS amplitude a tone has to pass to count as heard at all; each is played at a tenth of
# full scale.
HEARD = 0.01
# How many dB a tone may change by and still count as heard as it was played.
UNCHANGED = 1
# The most a tone in the tuning's deepest cut, of 6 dB, may change by, in dB, and count as cut.
CUT = -4
# The tones played, in Hz: one below every cut of the tuning, and one in its deepest.
KEPT_TONE = 1000
CUT_TONE = 10200
TONES = (KEPT_TONE, CUT_TONE)
# The stand-in speakers, as pebble-x-plus.nix makes them at boot, to make again as if plugged
# back in. Lingering, they outlive the pw-cli that makes them.
PEBBLE = (
    "{ factory.name = support.null-audio-sink node.name = pebble media.class = Audio/Sink"
    " audio.position = [ FL FR ] monitor.channel-volumes = true object.linger = true"
    ' alsa.components = "USB041e:329a" }'
)
# How long PipeWire may take to let the speakers go, and to link them to the tuning again.
LETTING_GO = dt.timedelta(seconds=10)
LINKING = dt.timedelta(seconds=30)


def in_session(command: str) -> str:
    """Give a shell command that runs as joel in his session, for the driver to wait on."""
    return f"su - joel -c 'XDG_RUNTIME_DIR=/run/user/1000 {command}'"


def decibels(heard: float, played: float) -> float:
    """Give how many dB a tone changed by, from as it was played to as it was heard."""
    return 20 * math.log10(heard / played)


class Session:
    """Joel's session, its PipeWire with the stand-in speakers, headphones and tuning."""

    def __init__(self, machine: BaseMachine) -> None:
        """Act on Joel's session on that machine."""
        self.machine = machine

    def run(self, command: str) -> str:
        """Run a command as joel in his session, and give its output."""
        return as_account(self.machine, command)

    def level(self, frequency: int, sink: str) -> float:
        """Play a tone to the default sink, and give the level its middle second reached sink at.

        The middle second is away from the tone's edges. The recorder is linked to the sink's
        monitor by hand: left to WirePlumber, a recording of a sink with a smart filter is given
        the filter's input instead, which is the sound before the filter.
        """
        self.run(
            "pw-record -P '{ node.name = heard node.autoconnect = false }' heard.wav"
            " & recording=$!;"
            " until pw-link -i | grep -q '^heard:input_FR'; do sleep 0.1; done;"
            f" pw-link {sink}:monitor_FL heard:input_FL; pw-link {sink}:monitor_FR heard:input_FR;"
            f" pw-play tone-{frequency}.wav;"
            # pw-record exits with 1 when interrupted, so its file is the
            # evidence rather than its status.
            " kill -INT $recording; wait $recording; test -s heard.wav",
        )
        return float(
            self.run(
                "sox heard.wav -n silence 1 0.01 0.1% trim 1 1 stat 2>&1"
                " | awk '/RMS +amplitude/ {print $3}'"
            )
        )

    def levels(self, sink: str) -> dict[int, float]:
        """Give each tone's level as the sink heard it, by its frequency."""
        return {frequency: self.level(frequency, sink) for frequency in TONES}

    def tuning_ready(self) -> None:
        """Wait until the tuning's unit is up and PipeWire has its node."""
        self.machine.wait_for_unit("pebble-x-plus-tuning.service", "joel")
        tuned = in_session("pw-cli ls Node | grep -q pebble_x_plus_tuning")
        self.machine.wait_until_succeeds(tuned)


@pytest.fixture(scope="module")
def session(machine: BaseMachine) -> Session:
    """Give Joel's session, its PipeWire running, the tones made and the tuning ready."""
    machine.wait_for_unit("multi-user.target")
    machine.succeed("loginctl enable-linger joel")
    machine.wait_for_unit("user@1000.service")
    joel = Session(machine)
    joel.run("systemctl --user start pipewire.service")
    machine.wait_for_unit("wireplumber.service", "joel")
    for frequency in TONES:
        joel.run(f"sox -n -r 48000 -c 2 tone-{frequency}.wav synth 3 sine {frequency} vol 0.1")
    joel.tuning_ready()
    return joel


@pytest.fixture(scope="module")
def played(session: Session) -> dict[int, float]:
    """Give each tone as the speakers hear it with the tuning stopped, then start it again."""
    session.run("systemctl --user stop pebble-x-plus-tuning")
    session.run("pactl set-default-sink pebble")
    heard = session.levels("pebble")
    session.run("systemctl --user start pebble-x-plus-tuning")
    session.tuning_ready()
    return heard


def test_the_tuning_starts_with_pipewire(session: Session) -> None:
    """The tuning starts with PipeWire."""
    session.tuning_ready()


def test_without_the_tuning_the_speakers_hear_what_was_played(played: dict[int, float]) -> None:
    """Without the tuning, the speakers hear what was played."""
    assert all(value > HEARD for value in played.values())


def test_the_speakers_hear_the_tuning(session: Session, played: dict[int, float]) -> None:
    """The speakers hear the tuning."""
    session.run("pactl set-default-sink pebble")
    heard = session.levels("pebble")
    assert abs(decibels(heard[KEPT_TONE], played[KEPT_TONE])) < UNCHANGED
    assert decibels(heard[CUT_TONE], played[CUT_TONE]) < CUT


def test_plugged_back_in_the_speakers_hear_the_tuning_again(
    session: Session, played: dict[int, float]
) -> None:
    """Pulled out and plugged back in, the speakers hear the tuning again."""
    session.run("pactl set-default-sink pebble")
    # Nobody picks the speakers again: PipeWire's default goes back to them as it would at a
    # replug, and the tuning follows them back by their USB id.
    session.run(
        "pw-cli destroy $(pw-cli ls Node | awk '/^\\tid / {id = $2}"
        ' /node.name = "pebble"$/ {sub(",", "", id); print id}\')',
    )
    session.machine.wait_until_fails(
        in_session('pw-cli ls Node | grep -q "node.name = \\"pebble\\""'), timeout=LETTING_GO
    )
    session.run(f"pw-cli create-node adapter '{PEBBLE}'")
    session.machine.wait_until_succeeds(
        in_session("pw-link -l | grep -A1 ^pebble:playback_FL | grep -q pebble_x_plus_tuning"),
        timeout=LINKING,
    )
    heard = session.levels("pebble")
    assert all(value > HEARD for value in heard.values())
    assert abs(decibels(heard[KEPT_TONE], played[KEPT_TONE])) < UNCHANGED
    assert decibels(heard[CUT_TONE], played[CUT_TONE]) < CUT


def test_the_headphones_hear_what_was_played(session: Session, played: dict[int, float]) -> None:
    """The headphones hear what was played."""
    session.run("pactl set-default-sink headphones")
    heard = session.levels("headphones")
    assert all(
        abs(decibels(heard[frequency], played[frequency])) < UNCHANGED for frequency in TONES
    )

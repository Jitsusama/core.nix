"""The XPS 14's speakers hear the speaker tuning, and its headphones hear what was played.

tests/speakers.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), and its
opening comment says what the tests prove. Two silent sinks stand in for the speakers and the
headphone jack under the laptop's own names, given as the `values` fixture. Each test asks for
the state it starts from as a fixture, Joel's sound running with the tuning on and the tones
written, and sets its own default sink, so any test runs alone with `-k`; what was played is
measured once, with the tuning stopped, and the tuning is started again straight after.
"""

from __future__ import annotations

import math
import shlex
from typing import TYPE_CHECKING, Any

import pytest

if TYPE_CHECKING:
    from test_driver.machine import BaseMachine

# The level at or below which the speakers heard nothing of a tone played at a tenth of full
# scale.
HEARD = 0.01
# How many decibels further the tuning must cut 631 Hz than 1356 Hz, of the about 10 it does.
CUT_FURTHER = 6
# The limiter's ceiling in speakers.conf, which keeps the small drivers from being pushed
# however loud the tuning plays, and how far a sample may pass it.
CEILING = 0.891
OVERSHOOT = 1.02
# How far the headphones may hear a tone from how it was played, in decibels.
UNCHANGED_DB = 1
# The tuning's deepest cut and one of its peaks, each played at a tenth of full scale.
MEASURED = (631, 1356)
# The tuning's loudest frequency, its bass peak, and one of its peaks above, each played at
# full scale so the limiter has to hold them.
LOUDEST = (100, 1356)


class Sound:
    """Joel's sound on the machine, playing tones and recording what a sink heard."""

    def __init__(self, machine: BaseMachine, values: dict[str, Any]) -> None:
        """Act on Joel's sound on that machine, whose sinks the values name."""
        self.machine = machine
        self.speakers: str = values["speakers"]
        self.headphones: str = values["headphones"]

    def as_joel(self, command: str) -> str:
        """Run a command in Joel's account, with his PipeWire's runtime folder, giving output."""
        environment = "export XDG_RUNTIME_DIR=/run/user/1000; cd ~; "
        return self.machine.succeed("su - joel -c " + shlex.quote(environment + command))

    def play_to(self, sink: str) -> None:
        """Make the sink the one tones play to."""
        self.as_joel(f"pactl set-default-sink {sink}")

    def level(
        self, frequency: int, sink: str, measure: str = "RMS +amplitude", loud: str = ""
    ) -> float:
        """Play a tone to the default sink, and give the level of what reached the given one.

        The level is that of the tone's middle second, away from its edges: sox finds where
        the tone starts in the recording, so nothing waits for the graph to settle before it
        plays or after it ends. The recorder is linked to the sink's monitor by hand: left to
        WirePlumber, a recording of a sink with a smart filter is given the filter's input
        instead, which is the sound before the filter has touched it.
        """
        self.as_joel(
            "pw-record -P '{ node.name = heard node.autoconnect = false }' heard.wav &"
            " recording=$!;"
            " until pw-link -i | grep -q '^heard:input_FR'; do sleep 0.1; done;"
            f" pw-link {sink}:monitor_FL heard:input_FL; pw-link {sink}:monitor_FR heard:input_FR;"
            f" pw-play tone-{frequency}{loud}.wav;"
            # pw-record exits with 1 when interrupted, so its file is the
            # evidence rather than its status.
            " kill -INT $recording; wait $recording; test -s heard.wav"
        )
        heard = self.as_joel(
            "sox heard.wav -n silence 1 0.01 0.1% trim 1 1 stat 2>&1"
            f" | awk '/{measure}/ {{print $3}}'"
        )
        return float(heard)

    def tuning_ready(self) -> None:
        """Wait until the tuning's service runs and its filter is in PipeWire's graph."""
        self.machine.wait_for_unit("speaker-tuning.service", "joel")
        self.machine.wait_until_succeeds(
            "su - joel -c 'XDG_RUNTIME_DIR=/run/user/1000 pw-cli ls Node' | grep -q speaker_tuning"
        )


def decibels(heard: float, played: float) -> float:
    """Give how many decibels louder what was heard is than what was played."""
    return 20 * math.log10(heard / played)


@pytest.fixture(scope="module")
def sound(machine: BaseMachine, values: dict[str, Any]) -> Sound:
    """Boot the machine with Joel's sound running, and write the tones the tests play."""
    machine.wait_for_unit("multi-user.target")
    machine.succeed("loginctl enable-linger joel")
    machine.wait_for_unit("user@1000.service")
    joel = Sound(machine, values)
    joel.as_joel("systemctl --user start pipewire.service")
    machine.wait_for_unit("wireplumber.service", "joel")
    for frequency in MEASURED:
        joel.as_joel(f"sox -n -r 48000 -c 2 tone-{frequency}.wav synth 3 sine {frequency} vol 0.1")
    for frequency in LOUDEST:
        joel.as_joel(
            f"sox -n -r 48000 -c 2 tone-{frequency}-loud.wav synth 3 sine {frequency} vol 0.99"
        )
    return joel


@pytest.fixture
def tuned(sound: Sound) -> Sound:
    """Have the tuning running, its filter in the graph."""
    sound.as_joel("systemctl --user start speaker-tuning")
    sound.tuning_ready()
    return sound


@pytest.fixture(scope="module")
def played(sound: Sound) -> dict[int, float]:
    """Give the level each measured tone reaches the speakers at with the tuning stopped.

    The tuning is started again once they're measured, as the tests after expect.
    """
    sound.tuning_ready()
    sound.as_joel("systemctl --user stop speaker-tuning")
    sound.play_to(sound.speakers)
    levels = {frequency: sound.level(frequency, sound.speakers) for frequency in MEASURED}
    sound.machine.log(f"played: {levels}")
    sound.as_joel("systemctl --user start speaker-tuning")
    sound.tuning_ready()
    return levels


def test_the_tuning_starts_with_pipewire(sound: Sound) -> None:
    """The tuning starts with PipeWire."""
    sound.tuning_ready()


def test_without_the_tuning_the_speakers_hear_what_was_played(played: dict[int, float]) -> None:
    """Without the tuning, the speakers hear what was played."""
    assert all(value > HEARD for value in played.values()), played


def test_the_speakers_hear_the_tuning(tuned: Sound, played: dict[int, float]) -> None:
    """The speakers hear the tuning."""
    tuned.play_to(tuned.speakers)
    cut = decibels(tuned.level(631, tuned.speakers), played[631])
    lift = decibels(tuned.level(1356, tuned.speakers), played[1356])
    tuned.machine.log(f"631 Hz changed by {cut:.1f} dB, 1356 Hz by {lift:.1f} dB")
    assert lift - cut > CUT_FURTHER, (cut, lift)


def test_the_speakers_never_hear_the_tuning_past_its_ceiling(tuned: Sound) -> None:
    """The speakers never hear the tuning past its ceiling."""
    tuned.play_to(tuned.speakers)
    for frequency in LOUDEST:
        peak = tuned.level(frequency, tuned.speakers, "Maximum amplitude", "-loud")
        tuned.machine.log(f"{frequency} Hz at full scale reached the speakers at {peak:.3f}")
        assert peak <= CEILING * OVERSHOOT, (frequency, peak)


def test_the_headphones_hear_what_was_played(tuned: Sound, played: dict[int, float]) -> None:
    """The headphones hear what was played."""
    tuned.play_to(tuned.headphones)
    for frequency in MEASURED:
        change = decibels(tuned.level(frequency, tuned.headphones), played[frequency])
        tuned.machine.log(f"{frequency} Hz changed by {change:.1f} dB on the headphones")
        assert abs(change) < UNCHANGED_DB, (frequency, change)

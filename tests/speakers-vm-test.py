"""The XPS 14's speakers hear the speaker tuning, and its headphones hear what was played.

tests/speakers.nix runs this, and its opening comment says what the test proves. Two silent
sinks stand in for the speakers and the headphone jack under the laptop's own names, which
the script reads from the JSON file tests/speakers.nix swaps in for the marker below.
"""

import json
import math
import shlex
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from driver_names import machine, subtest

# The sinks' names, which tests/speakers.nix gives the machine's PipeWire and this script alike.
values = json.loads(Path("@values@").read_text())
speakers: str = values["speakers"]
headphones: str = values["headphones"]

# The level at or below which the speakers heard nothing of a tone played at a tenth of full
# scale.
HEARD = 0.01
# How many decibels further the tuning must cut 631 Hz than 1356 Hz, of the about 10 it does.
CUT_FURTHER = 6
# The limiter's ceiling in speakers.conf, which keeps the small drivers from being pushed
# however loud the tuning plays, and how far a sample may pass it.
CEILING = 0.891
OVERSHOOT = 1.02
# The tuning's loudest frequency, its bass peak, and one of its peaks above, each played at
# full scale so the limiter has to hold them.
LOUDEST = (100, 1356)


def as_joel(command: str) -> str:
    """Run a command in Joel's account, with his PipeWire's runtime folder, and give its output."""
    environment = "export XDG_RUNTIME_DIR=/run/user/1000; cd ~; "
    return machine.succeed("su - joel -c " + shlex.quote(environment + command))


def level(frequency: int, sink: str, measure: str = "RMS +amplitude", loud: str = "") -> float:
    """Play a tone to the default sink, and give the level of what reached the given one.

    The level is that of the tone's middle second, away from its edges: sox finds where the
    tone starts in the recording, so nothing waits for the graph to settle before it plays or
    after it ends. The recorder is linked to the sink's monitor by hand: left to WirePlumber, a
    recording of a sink with a smart filter is given the filter's input instead, which is the
    sound before the filter has touched it.
    """
    as_joel(
        "pw-record -P '{ node.name = heard node.autoconnect = false }' heard.wav & recording=$!;"
        " until pw-link -i | grep -q '^heard:input_FR'; do sleep 0.1; done;"
        f" pw-link {sink}:monitor_FL heard:input_FL; pw-link {sink}:monitor_FR heard:input_FR;"
        f" pw-play tone-{frequency}{loud}.wav;"
        # pw-record exits with 1 when interrupted, so its file is the
        # evidence rather than its status.
        " kill -INT $recording; wait $recording; test -s heard.wav"
    )
    rms = as_joel(
        f"sox heard.wav -n silence 1 0.01 0.1% trim 1 1 stat 2>&1 | awk '/{measure}/ {{print $3}}'"
    )
    return float(rms)


def decibels(heard: float, played: float) -> float:
    """Give how many decibels louder what was heard is than what was played."""
    return 20 * math.log10(heard / played)


def tuning_ready() -> None:
    """Wait until the tuning's service runs and its filter is in PipeWire's graph."""
    machine.wait_for_unit("speaker-tuning.service", "joel")
    machine.wait_until_succeeds(
        "su - joel -c 'XDG_RUNTIME_DIR=/run/user/1000 pw-cli ls Node' | grep -q speaker_tuning"
    )


machine.wait_for_unit("multi-user.target")
machine.succeed("loginctl enable-linger joel")
machine.wait_for_unit("user@1000.service")
as_joel("systemctl --user start pipewire.service")
machine.wait_for_unit("wireplumber.service", "joel")
for frequency in (631, 1356):
    as_joel(f"sox -n -r 48000 -c 2 tone-{frequency}.wav synth 3 sine {frequency} vol 0.1")
for frequency in LOUDEST:
    as_joel(f"sox -n -r 48000 -c 2 tone-{frequency}-loud.wav synth 3 sine {frequency} vol 0.99")

with subtest("the tuning starts with PipeWire"):
    tuning_ready()

with subtest("without the tuning, the speakers hear what was played"):
    as_joel("systemctl --user stop speaker-tuning")
    as_joel(f"pactl set-default-sink {speakers}")
    played = {frequency: level(frequency, speakers) for frequency in (631, 1356)}
    machine.log(f"played: {played}")
    assert all(value > HEARD for value in played.values()), played

with subtest("the speakers hear the tuning"):
    as_joel("systemctl --user start speaker-tuning")
    tuning_ready()
    as_joel(f"pactl set-default-sink {speakers}")
    cut = decibels(level(631, speakers), played[631])
    lift = decibels(level(1356, speakers), played[1356])
    machine.log(f"631 Hz changed by {cut:.1f} dB, 1356 Hz by {lift:.1f} dB")
    assert lift - cut > CUT_FURTHER, (cut, lift)

with subtest("the speakers never hear the tuning past its ceiling"):
    for frequency in LOUDEST:
        peak = level(frequency, speakers, "Maximum amplitude", "-loud")
        machine.log(f"{frequency} Hz at full scale reached the speakers at {peak:.3f}")
        assert peak <= CEILING * OVERSHOOT, (frequency, peak)

with subtest("the headphones hear what was played"):
    as_joel(f"pactl set-default-sink {headphones}")
    for frequency in (631, 1356):
        change = decibels(level(frequency, headphones), played[frequency])
        machine.log(f"{frequency} Hz changed by {change:.1f} dB on the headphones")
        assert abs(change) < 1, (frequency, change)

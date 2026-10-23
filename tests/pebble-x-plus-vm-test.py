"""Tones measured through a Pebble X Plus's tuning in a virtual machine, and on headphones.

tests/pebble-x-plus.nix runs this, and its opening comment says what the test proves.
run-subtests cuts the script into subtests at each top-level `with subtest(`, so every
subtest stays at the top level under its own name, and what sits outside them, the boot, the
tones and the helpers, runs whichever subtests are chosen. The script needs no value from
Nix: the stand-in speakers and headphones are named in the machine pebble-x-plus.nix builds.
"""

import math
import shlex
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from driver_names import machine, subtest

# The RMS amplitude a tone has to pass to count as heard at all; each is played at a tenth of
# full scale.
HEARD = 0.01
# How many dB a tone may change by and still count as heard as it was played.
UNCHANGED = 1
# The most a tone in the tuning's deepest cut, of 6 dB, may change by, in dB, and count as cut.
CUT = -4


def as_joel(command: str) -> str:
    """Run a command as Joel, in his home, and give what it said."""
    environment = "export XDG_RUNTIME_DIR=/run/user/1000; cd ~; "
    return machine.succeed("su - joel -c " + shlex.quote(environment + command))


def level(frequency: int, sink: str) -> float:
    """Play a tone to the default sink, and give the level its middle second reached sink at.

    The middle second is away from the tone's edges. The recorder is linked to the sink's
    monitor by hand: left to WirePlumber, a recording of a sink with a smart filter is given the
    filter's input instead, which is the sound before the filter.
    """
    as_joel(
        "pw-record -P '{ node.name = heard node.autoconnect = false }' heard.wav"
        " & recording=$!;"
        " until pw-link -i | grep -q '^heard:input_FR'; do sleep 0.1; done;"
        f" pw-link {sink}:monitor_FL heard:input_FL; pw-link {sink}:monitor_FR heard:input_FR;"
        f" pw-play tone-{frequency}.wav;"
        # pw-record exits with 1 when interrupted, so its file is the
        # evidence rather than its status.
        " kill -INT $recording; wait $recording; test -s heard.wav"
    )
    rms = as_joel(
        "sox heard.wav -n silence 1 0.01 0.1% trim 1 1 stat 2>&1"
        " | awk '/RMS +amplitude/ {print $3}'"
    )
    return float(rms)


def decibels(heard: float, played: float) -> float:
    """Give how many dB a tone changed by, from as it was played to as it was heard."""
    return 20 * math.log10(heard / played)


def tuning_ready() -> None:
    """Wait until the tuning's unit is up and PipeWire has its node."""
    machine.wait_for_unit("pebble-x-plus-tuning.service", "joel")
    machine.wait_until_succeeds(
        "su - joel -c 'XDG_RUNTIME_DIR=/run/user/1000 pw-cli ls Node'"
        " | grep -q pebble_x_plus_tuning"
    )


# The tones played, in Hz: one below every cut of the tuning, and one in its deepest.
tones = (1000, 10200)
machine.wait_for_unit("multi-user.target")
machine.succeed("loginctl enable-linger joel")
machine.wait_for_unit("user@1000.service")
as_joel("systemctl --user start pipewire.service")
machine.wait_for_unit("wireplumber.service", "joel")
for frequency in tones:
    as_joel(f"sox -n -r 48000 -c 2 tone-{frequency}.wav synth 3 sine {frequency} vol 0.1")

with subtest("the tuning starts with PipeWire"):
    tuning_ready()

with subtest("without the tuning, the speakers hear what was played"):
    as_joel("systemctl --user stop pebble-x-plus-tuning")
    as_joel("pactl set-default-sink pebble")
    played = {frequency: level(frequency, "pebble") for frequency in tones}
    machine.log(f"played: {played}")
    assert all(value > HEARD for value in played.values()), played

with subtest("the speakers hear the tuning"):
    as_joel("systemctl --user start pebble-x-plus-tuning")
    tuning_ready()
    as_joel("pactl set-default-sink pebble")
    kept = decibels(level(1000, "pebble"), played[1000])
    cut = decibels(level(10200, "pebble"), played[10200])
    machine.log(f"1 kHz changed by {kept:.1f} dB, 10.2 kHz by {cut:.1f} dB")
    assert abs(kept) < UNCHANGED, kept
    assert cut < CUT, cut

with subtest("the headphones hear what was played"):
    as_joel("pactl set-default-sink headphones")
    for frequency in tones:
        change = decibels(level(frequency, "headphones"), played[frequency])
        machine.log(f"{frequency} Hz changed by {change:.1f} dB on the headphones")
        assert abs(change) < UNCHANGED, (frequency, change)

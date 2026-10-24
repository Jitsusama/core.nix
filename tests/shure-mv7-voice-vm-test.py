"""The MV7's voice filter levels a voice and leaves another microphone as played.

tests/shure-mv7-voice.nix runs this, and its opening comment says what each tone stands for and
the level it should come through at. Players presenting themselves as sources stand in for the
microphones, one with the MV7's USB id. The script needs no value from Nix.
"""

import shlex
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from driver_names import machine, subtest

MV7 = '{ node.name = mv7 media.class = Audio/Source alsa.components = "USB14ed:1012" }'
HEADSET = "{ node.name = headset media.class = Audio/Source }"

# A loud voice peaks under the filter's -1 dBFS limit, give or take a tenth
# of a decibel in the measuring, but no lower than -12, which would mean
# the filter crushed it rather than held it.
LIMIT = -0.9
CRUSHED = -12


def as_joel(command: str) -> str:
    """Run a command as joel, in his session's runtime directory, and give its output."""
    environment = "export XDG_RUNTIME_DIR=/run/user/1000; cd ~; "
    return machine.succeed("su - joel -c " + shlex.quote(environment + command))


def level(tone: str, source: str, *, filtered: bool) -> tuple[float, float]:
    """Play a tone as the given microphone and give the peak and RMS a program records of it.

    The recording asks for that microphone by name, as a program would, and its middle second
    is measured, in dBFS. With the filter running, it waits until WirePlumber has linked the
    stand-in to the filter.
    """
    name = "mv7" if "USB14ed" in source else "headset"
    ready = (
        "pw-link -l | grep -q '<- " + name + ":'"
        if filtered
        else "pw-cli ls Node | grep -q 'node.name = \"" + name + "\"'"
    )
    # Gives up after 20 s, so a stand-in that never appears fails the
    # test rather than hanging it.
    linked = (
        " n=0; until " + ready + "; do n=$((n + 1)); [ $n -lt 200 ] || exit 1; sleep 0.1; done;"
    )
    as_joel(
        f"pw-play -P '{source}' {tone}.wav & player=$!;"
        + linked
        + f" pw-record --format f32 --channels 1 --target {name} heard.wav & recording=$!;"
        " sleep 3; kill -INT $recording; wait $recording; kill $player; wait $player;"
        # pw-record and an interrupted pw-play exit non-zero, so the file
        # is the evidence rather than their status.
        " test -s heard.wav"
    )
    stats = as_joel(
        "sox heard.wav -n silence 1 0.01 0.01% trim 0.5 1 stats 2>&1"
        " | awk '/Pk lev dB/ {print $4} /RMS lev dB/ {print $4}'"
    ).split()
    return float(stats[0]), float(stats[1])


def filter_ready() -> None:
    """Wait until the filter's service runs and PipeWire has its node."""
    machine.wait_for_unit("shure-mv7-voice.service", "joel")
    machine.wait_until_succeeds(
        "su - joel -c 'XDG_RUNTIME_DIR=/run/user/1000 pw-cli ls Node' | grep -q shure_mv7_voice"
    )


machine.wait_for_unit("multi-user.target")
machine.succeed("loginctl enable-linger joel")
machine.wait_for_unit("user@1000.service")
as_joel("systemctl --user start pipewire.service")
machine.wait_for_unit("wireplumber.service", "joel")
# 0.01261 is -41 dBFS RMS for a sine, 0.5 a peak of -6 dBFS.
as_joel("sox -n -r 48000 -c 1 quiet.wav synth 6 sine 1000 vol 0.01261")
as_joel("sox -n -r 48000 -c 1 loud.wav synth 6 sine 1000 vol 0.5")
# -70 dBFS RMS, and -46 at 7 kHz and at 1 kHz.
as_joel("sox -n -r 48000 -c 1 room.wav synth 6 sine 1000 vol 0.000447")
as_joel("sox -n -r 48000 -c 1 hiss.wav synth 6 sine 7000 vol 0.00709")
as_joel("sox -n -r 48000 -c 1 tone.wav synth 6 sine 1000 vol 0.00709")

with subtest("the filter starts with PipeWire"):
    filter_ready()

with subtest("without the filter, the mic is heard as played"):
    as_joel("systemctl --user stop shure-mv7-voice")
    quiet = level("quiet", MV7, filtered=False)
    machine.log(f"quiet tone: peak {quiet[0]} dBFS, RMS {quiet[1]} dBFS")
    assert abs(quiet[1] + 41) < 1, quiet

with subtest("the filter brings a quiet voice up 12 dB"):
    as_joel("systemctl --user start shure-mv7-voice")
    filter_ready()
    peak, rms = level("quiet", MV7, filtered=True)
    machine.log(f"quiet tone through the filter: peak {peak} dBFS, RMS {rms} dBFS")
    assert abs(rms - quiet[1] - 12) < 1, (rms, quiet)

with subtest("the filter holds a loud voice under -1 dBFS"):
    peak, rms = level("loud", MV7, filtered=True)
    machine.log(f"loud tone through the filter: peak {peak} dBFS, RMS {rms} dBFS")
    assert CRUSHED < peak <= LIMIT, peak

with subtest("the filter turns the room down between words"):
    peak, rms = level("room", MV7, filtered=True)
    machine.log(f"room tone through the filter: RMS {rms} dBFS")
    assert rms < -70 + 12 - 5, rms

with subtest("the filter turns s sounds down and leaves the voice"):
    _, s_sound = level("hiss", MV7, filtered=True)
    _, voice = level("tone", MV7, filtered=True)
    machine.log(f"7 kHz through the filter: {s_sound} dBFS, 1 kHz: {voice} dBFS")
    assert abs(voice - (-46 + 12)) < 1, voice
    assert s_sound < voice - 6, (s_sound, voice)

with subtest("another microphone is heard as played"):
    peak, rms = level("quiet", HEADSET, filtered=False)
    machine.log(f"headset: peak {peak} dBFS, RMS {rms} dBFS")
    assert abs(rms + 41) < 1, rms

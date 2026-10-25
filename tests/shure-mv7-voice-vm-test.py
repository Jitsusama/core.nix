"""The MV7's voice filter levels a voice and leaves another microphone as played.

tests/shure-mv7-voice.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py),
and its opening comment says what each tone stands for and the level it should come through
at. Players presenting themselves as sources stand in for the microphones, one with the MV7's
USB id. Each test asks for the state it starts from as a fixture: Joel's session with the tones
made and the filter running, and a quiet voice measured with the filter stopped. So any test
runs alone with `-k`. The tests need no value from Nix.
"""

from __future__ import annotations

import shlex
from typing import TYPE_CHECKING

import pytest

if TYPE_CHECKING:
    from test_driver.machine import BaseMachine

MV7 = '{ node.name = mv7 media.class = Audio/Source alsa.components = "USB14ed:1012" }'
HEADSET = "{ node.name = headset media.class = Audio/Source }"

# A loud voice peaks under the filter's -1 dBFS limit, give or take a tenth
# of a decibel in the measuring, but no lower than -12, which would mean
# the filter crushed it rather than held it.
LIMIT = -0.9
CRUSHED = -12
# The quiet tone's RMS as played, and the 1 kHz tone's beside the s sound's.
QUIET = -41
TONE = -46
# The room tone's RMS as played.
ROOM = -70
# What the filter brings a quiet voice up by, and how far a measure may stray.
BOOST = 12
STRAY = 1
# How much further down than the voice the filter turns an s sound, at least.
DE_ESSED = 6
# How far under a voice brought up the room must be turned down.
GATED = 5


class Session:
    """Joel's session, his PipeWire with the MV7's voice filter, and the tones to play."""

    def __init__(self, machine: BaseMachine) -> None:
        """Act on Joel's session on that machine."""
        self.machine = machine

    def run(self, command: str) -> str:
        """Run a command as joel, in his session's runtime directory, and give its output."""
        environment = "export XDG_RUNTIME_DIR=/run/user/1000; cd ~; "
        return self.machine.succeed("su - joel -c " + shlex.quote(environment + command))

    def level(self, tone: str, source: str, *, filtered: bool) -> tuple[float, float]:
        """Play a tone as the given microphone and give the peak and RMS a program records.

        The recording asks for that microphone by name, as a program would, and its middle
        second is measured, in dBFS. With the filter running, it waits until WirePlumber has
        linked the stand-in to the filter.
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
        self.run(
            f"pw-play -P '{source}' {tone}.wav & player=$!;"
            + linked
            + f" pw-record --format f32 --channels 1 --target {name} heard.wav & recording=$!;"
            " sleep 3; kill -INT $recording; wait $recording; kill $player; wait $player;"
            # pw-record and an interrupted pw-play exit non-zero, so the file
            # is the evidence rather than their status.
            " test -s heard.wav"
        )
        stats = self.run(
            "sox heard.wav -n silence 1 0.01 0.01% trim 0.5 1 stats 2>&1"
            " | awk '/Pk lev dB/ {print $4} /RMS lev dB/ {print $4}'"
        ).split()
        return float(stats[0]), float(stats[1])

    def filter_ready(self) -> None:
        """Wait until the filter's service runs and PipeWire has its node."""
        self.machine.wait_for_unit("shure-mv7-voice.service", "joel")
        self.machine.wait_until_succeeds(
            "su - joel -c 'XDG_RUNTIME_DIR=/run/user/1000 pw-cli ls Node' | grep -q shure_mv7_voice"
        )


@pytest.fixture(scope="module")
def session(machine: BaseMachine) -> Session:
    """Give Joel's session, its PipeWire running, the tones made and the filter ready."""
    machine.wait_for_unit("multi-user.target")
    machine.succeed("loginctl enable-linger joel")
    machine.wait_for_unit("user@1000.service")
    joel = Session(machine)
    joel.run("systemctl --user start pipewire.service")
    machine.wait_for_unit("wireplumber.service", "joel")
    # 0.01261 is -41 dBFS RMS for a sine, 0.5 a peak of -6 dBFS.
    joel.run("sox -n -r 48000 -c 1 quiet.wav synth 6 sine 1000 vol 0.01261")
    joel.run("sox -n -r 48000 -c 1 loud.wav synth 6 sine 1000 vol 0.5")
    # -70 dBFS RMS, and -46 at 7 kHz and at 1 kHz.
    joel.run("sox -n -r 48000 -c 1 room.wav synth 6 sine 1000 vol 0.000447")
    joel.run("sox -n -r 48000 -c 1 hiss.wav synth 6 sine 7000 vol 0.00709")
    joel.run("sox -n -r 48000 -c 1 tone.wav synth 6 sine 1000 vol 0.00709")
    joel.filter_ready()
    return joel


@pytest.fixture(scope="module")
def unfiltered(session: Session) -> float:
    """Give the quiet tone's RMS as the MV7, the filter stopped, and start the filter again."""
    session.run("systemctl --user stop shure-mv7-voice")
    _, rms = session.level("quiet", MV7, filtered=False)
    session.run("systemctl --user start shure-mv7-voice")
    session.filter_ready()
    return rms


def test_the_filter_starts_with_pipewire(session: Session) -> None:
    """The filter starts with PipeWire."""
    session.filter_ready()


def test_without_the_filter_the_mic_is_heard_as_played(unfiltered: float) -> None:
    """Without the filter, the mic is heard as played."""
    assert abs(unfiltered - QUIET) < STRAY


def test_the_filter_brings_a_quiet_voice_up_12_db(session: Session, unfiltered: float) -> None:
    """The filter brings a quiet voice up 12 dB."""
    _, rms = session.level("quiet", MV7, filtered=True)
    assert abs(rms - unfiltered - BOOST) < STRAY


def test_the_filter_holds_a_loud_voice_under_1_dbfs(session: Session) -> None:
    """The filter holds a loud voice under -1 dBFS."""
    peak, _ = session.level("loud", MV7, filtered=True)
    assert CRUSHED < peak <= LIMIT


def test_the_filter_turns_the_room_down_between_words(session: Session) -> None:
    """The filter turns the room down between words."""
    _, rms = session.level("room", MV7, filtered=True)
    assert rms < ROOM + BOOST - GATED


def test_the_filter_turns_s_sounds_down_and_leaves_the_voice(session: Session) -> None:
    """The filter turns s sounds down and leaves the voice."""
    _, s_sound = session.level("hiss", MV7, filtered=True)
    _, voice = session.level("tone", MV7, filtered=True)
    assert abs(voice - (TONE + BOOST)) < STRAY
    assert s_sound < voice - DE_ESSED


def test_another_microphone_is_heard_as_played(session: Session) -> None:
    """Another microphone is heard as played."""
    _, rms = session.level("quiet", HEADSET, filtered=False)
    assert abs(rms - QUIET) < STRAY

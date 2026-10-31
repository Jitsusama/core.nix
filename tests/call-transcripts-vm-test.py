"""A call in Joel's PipeWire is heard by call-transcripts and written down, both sides in order.

tests/call-transcripts.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py),
and its opening comment says what the tests prove. The call is held once for the module, since
Whisper on the VM's CPU takes a while to hear it, and each test reads what that call left.
"""

from __future__ import annotations

import json
import shlex
from datetime import timedelta
from typing import TYPE_CHECKING, Any

import pytest

if TYPE_CHECKING:
    from test_driver.machine import BaseMachine

# Where Joel's PipeWire and the daemon's socket are.
RUNTIME = "/run/user/1000"
SOCKET = f"{RUNTIME}/daemons/calls.sock"
# The meeting on, as the google program writes it.
MEETING = "Review Planning"
# How long the daemon has to answer, and to start and finish hearing the call.
ANSWERS_WITHIN = timedelta(minutes=1)
WRITTEN_WITHIN = timedelta(minutes=10)
# The clips, made as the test builds.
CLIPS = "/etc/call-clips"
# The daemon's ask for every call it holds.
LIST = json.dumps({"id": "list", "ask": "list", "kind": "call"})


class Joel:
    """Joel's account on the machine."""

    def __init__(self, machine: BaseMachine) -> None:
        """Act as Joel on that machine."""
        self.machine = machine

    @staticmethod
    def as_joel(command: str) -> str:
        """Give a command run as Joel, with his runtime folder."""
        return "su - joel -c " + shlex.quote(f"export XDG_RUNTIME_DIR={RUNTIME}; cd ~; {command}")

    def run(self, command: str) -> str:
        """Run a command as Joel, giving its output."""
        return self.machine.succeed(self.as_joel(command))

    def wait_for(self, command: str, timeout: timedelta) -> None:
        """Wait until a command run as Joel succeeds."""
        self.machine.wait_until_succeeds(self.as_joel(command), timeout=timeout)

    @staticmethod
    def listing() -> str:
        """Give the command that prints the daemon's answer to its list."""
        return f"echo {shlex.quote(LIST)} | socat -t 5 - UNIX-CONNECT:{SOCKET}"

    def calls(self) -> list[dict[str, Any]]:
        """Give every call the daemon holds, newest first."""
        records: list[dict[str, Any]] = json.loads(self.run(self.listing()).splitlines()[0])[
            "records"
        ]
        return records

    def explain(self) -> None:
        """Print what PipeWire and the daemon said, for a test that failed."""
        print(self.run("pw-link -l || true"))
        print(self.run("journalctl --user -u call-transcripts --no-pager || true"))


@pytest.fixture(scope="module")
def joel(machine: BaseMachine, values: dict[str, Any]) -> Joel:
    """Boot with Joel's sound, a microphone and speakers, a meeting on and the daemon running."""
    machine.wait_for_unit("multi-user.target")
    machine.succeed("loginctl enable-linger joel")
    machine.wait_for_unit("user@1000.service")
    joel = Joel(machine)
    joel.run("systemctl --user start pipewire.service pipewire-pulse.service")
    machine.wait_for_unit("wireplumber.service", "joel")
    # Speakers that play nowhere, and a microphone that hears what's played into mic-in.
    joel.run(
        "pw-cli create-node adapter '{ factory.name = support.null-audio-sink"
        " node.name = speakers media.class = Audio/Sink audio.position = [ MONO ]"
        " object.linger = true }'"
    )
    joel.run(
        "systemd-run --user --unit=microphone pw-loopback"
        " --capture-props='node.name=mic-in media.class=Audio/Sink audio.position=[MONO]'"
        " --playback-props='node.name=microphone media.class=Audio/Source audio.position=[MONO]'"
    )
    joel.wait_for("pw-cli ls Node | grep -q 'node.name = \"microphone\"'", ANSWERS_WITHIN)
    # PipeWire's PulseAudio server names a stream for the program that opened it.
    joel.run('cp "$(readlink -f "$(command -v pacat)")" zoom')
    now = int(machine.succeed("date +%s%3N"))
    meetings = {"meetings": [{"id": "e1", "title": MEETING, "start": now, "end": now + 3_600_000}]}
    joel.run(f"echo {shlex.quote(json.dumps(meetings))} > {RUNTIME}/google.json")
    joel.run(
        "systemd-run --user --unit=call-transcripts --setenv=DICTATION_DEVICE=CPU"
        " --setenv=DICTATION_CACHE=$HOME/.cache/call-transcripts"
        " --setenv=PATH=/run/current-system/sw/bin"
        f" call-transcripts serve {SOCKET} {values['model']} $HOME/Documents/Calls"
    )
    joel.wait_for(f"test -S {SOCKET}", ANSWERS_WITHIN)
    return joel


@pytest.fixture(scope="module")
def call(joel: Joel) -> dict[str, Any]:
    """Hold one call, Joel and the others speaking, and another program speaking over it."""
    try:
        joel.run(
            "systemd-run --user --collect --unit=call ./zoom --record --device=microphone /dev/null"
        )
        joel.wait_for(f"{joel.listing()} | grep -qF '\"transcribing\":true'", ANSWERS_WITHIN)
        joel.wait_for("pw-link -l | grep -qF call-transcripts-mine", ANSWERS_WITHIN)
        # Both sides from the call's time 0, and another program on the speakers meanwhile,
        # as a notice or music would be; each player ends at its clip's end.
        joel.run(
            f"pw-play --target mic-in {CLIPS}/mine.wav &"
            f" ./zoom --playback --device=speakers --file-format=wav {CLIPS}/theirs.wav &"
            f" pw-play --target speakers {CLIPS}/other.wav &"
            " wait"
        )
        joel.run("systemctl --user stop call")
        joel.wait_for(f"{joel.listing()} | grep -qF '\"transcribing\":false'", WRITTEN_WITHIN)
    except Exception:
        joel.explain()
        raise
    [record] = joel.calls()
    return record


def transcript(joel: Joel, call: dict[str, Any]) -> str:
    """Give the call's transcript as written."""
    written = joel.run(f"cat {shlex.quote(call['transcript'])}")
    print(written)
    return written


def test_a_call_is_kept_under_its_meeting(call: dict[str, Any]) -> None:
    """The call is one record, keyed and titled by the meeting on as it started, now ended."""
    assert call["key"] == "call:e1"
    assert call["title"] == MEETING
    assert call["ended"] is not None
    assert MEETING in call["folder"]


def test_both_sides_are_written_in_the_order_they_spoke(joel: Joel, call: dict[str, Any]) -> None:
    """Joel asks, the others answer, Joel thanks them: three turns, each its speaker's."""
    turns = [line for line in transcript(joel, call).splitlines() if line.startswith("**")]
    asked, answered, thanked = turns
    assert not asked.startswith("**Others**")
    # Whisper on the VM's processor hears a word wrong now and then ("review" as "renew"),
    # so each turn is known by a word it hears every time; dictation's checks hold the words.
    assert "thursday" in asked.lower()
    assert answered.startswith("**Others**")
    assert "morning" in answered.lower()
    assert not thanked.startswith("**Others**")
    assert "thank" in thanked.lower()


def test_another_program_playing_is_in_neither_side(joel: Joel, call: dict[str, Any]) -> None:
    """What another program played over the call isn't the others', nor Joel's."""
    assert "weather" not in transcript(joel, call).lower()


def test_the_recorders_hold_no_call_open(joel: Joel, call: dict[str, Any]) -> None:
    """Once the call program stopped, the call ended, though the daemon's recorders had run."""
    assert call["ended"] is not None
    assert len(joel.calls()) == 1

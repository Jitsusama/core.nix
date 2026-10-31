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
# The meeting on, as the google program writes it, and the one after, which is stopped.
MEETING = "Review Planning"
STANDUP = "Standup"
# How long the daemon has to answer, and to start and finish hearing the call.
ANSWERS_WITHIN = timedelta(minutes=1)
WRITTEN_WITHIN = timedelta(minutes=10)
# How long the summarizer has, on the VM's processor, to load and sum the call up.
SUMMED_WITHIN = timedelta(minutes=15)
# Where the stand-in for the shell's notices keeps each it's given.
TOLD = "/home/joel/told"
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

    def told(self) -> list[tuple[str, str, str]]:
        """Give every notice raised, its program, heading and body, oldest first."""
        lines = self.run(f"cat {TOLD}").splitlines()
        return [(lines[at], lines[at + 1], lines[at + 2]) for at in range(0, len(lines), 3)]

    def choose(self, heading: str, act: str) -> None:
        """Choose an act on the newest notice with that heading, as a click would."""
        self.run(
            "busctl --user call org.freedesktop.Notifications /org/freedesktop/Notifications"
            f" org.freedesktop.Notifications Invoke ss {shlex.quote(heading)} {act}"
        )

    def meeting_on(self, key: str, title: str) -> None:
        """Write the meeting on now, for an hour, as the google program would."""
        now = int(self.machine.succeed("date +%s%3N"))
        meeting = {"id": key, "title": title, "start": now, "end": now + 3_600_000}
        self.run(f"echo {shlex.quote(json.dumps({'meetings': [meeting]}))} > {RUNTIME}/google.json")

    def record(self, key: str) -> dict[str, Any]:
        """Give the daemon's record of one call."""
        return next(record for record in self.calls() if record["key"] == key)

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
    joel.meeting_on("e1", MEETING)
    joel.run(f"systemd-run --user --unit=notices stand-in-notices {TOLD}")
    joel.wait_for("busctl --user status org.freedesktop.Notifications", ANSWERS_WITHIN)
    joel.run(
        "systemd-run --user --unit=call-transcripts --setenv=DICTATION_DEVICE=CPU"
        " --setenv=DICTATION_CACHE=$HOME/.cache/call-transcripts/whisper"
        " --setenv=SUMMARIZER_CACHE=$HOME/.cache/call-transcripts/summarizer"
        f" --setenv=OPENVINO_TOKENIZERS_PATH_GENAI={values['tokenizers']}"
        " --setenv=PATH=/run/current-system/sw/bin"
        f" call-transcripts serve {SOCKET} {values['model']} {values['summarizer']}"
        " $HOME/Documents/Calls"
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


@pytest.fixture(scope="module")
def summed(joel: Joel, call: dict[str, Any]) -> dict[str, Any]:
    """Wait for the call to be summed up, giving its record then."""
    # A summarizer that won't load is said in the daemon's log at once, so it ends the wait.
    refused = 'journalctl --user -u call-transcripts | grep -qF "summarizer didn\'t load"'
    try:
        joel.wait_for(
            f"{joel.listing()} | jq -e '.records[0].summary != null' > /dev/null || {refused}",
            SUMMED_WITHIN,
        )
    except Exception:
        joel.explain()
        raise
    [record] = joel.calls()
    if record["summary"] is None:
        joel.explain()
        pytest.fail("the summarizer didn't load")
    assert record["key"] == call["key"]
    print(record["summary"])
    return record


@pytest.fixture(scope="module")
def stopped(joel: Joel, summed: dict[str, Any]) -> dict[str, Any]:
    """Join the next meeting and stop transcribing it from its notice, the call still on."""
    heading = f"{STANDUP} Is Being Transcribed"
    joel.meeting_on("e2", STANDUP)
    try:
        joel.run(
            "systemd-run --user --collect --unit=call ./zoom --record --device=microphone /dev/null"
        )
        joel.wait_for(f"grep -qxF {shlex.quote(heading)} {TOLD}", ANSWERS_WITHIN)
        joel.wait_for("pw-link -l | grep -qF call-transcripts-mine", ANSWERS_WITHIN)
        joel.choose(heading, "stop")
        joel.wait_for(
            f'{joel.listing()} | jq -e \'.records[] | select(.key == "call:e2")'
            " | .transcribing == false'",
            ANSWERS_WITHIN,
        )
    except Exception:
        joel.explain()
        raise
    assert summed["key"] == "call:e1"
    return joel.record("call:e2")


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


def test_the_call_is_summed_up_beside_its_transcript(joel: Joel, summed: dict[str, Any]) -> None:
    """The summary is written into the call's folder, says what was decided, and is the record's."""
    written = joel.run(f"cat {shlex.quote(summed['folder'])}/summary.md")
    assert written.strip() == summed["summary"].strip()
    assert "thursday" in written.lower()


def test_a_notice_says_the_summary_is_ready(joel: Joel, summed: dict[str, Any]) -> None:
    """The notice comes from Calls, titled by the meeting, and opens with the summary."""
    joel.wait_for(f"grep -qxF 'Summary of {MEETING} Is Ready' {TOLD}", ANSWERS_WITHIN)
    [(app, _, first)] = [n for n in joel.told() if n[1] == f"Summary of {MEETING} Is Ready"]
    assert app == "Calls"
    assert first in summed["summary"]


def test_a_call_starting_offers_the_line_for_its_chat(joel: Joel, call: dict[str, Any]) -> None:
    """As the call started, one notice from Calls said so, with the line to tell the others."""
    [(app, _, line)] = [n for n in joel.told() if n[1] == f"{MEETING} Is Being Transcribed"]
    assert app == "Calls"
    assert "Nothing is uploaded" in line
    assert call["key"] == "call:e1"


def test_stop_transcribing_stops_at_once_with_the_call_still_on(
    joel: Joel, stopped: dict[str, Any]
) -> None:
    """Stopping from the notice lets both recorders go, though the call program still runs."""
    assert stopped["transcribing"] is False
    assert stopped["ended"] is None
    joel.run("systemctl --user is-active call")
    joel.wait_for("! pw-cli ls Node | grep -qF call-transcripts-", ANSWERS_WITHIN)


def test_a_call_stopped_isnt_transcribed_again_once_rejoined(
    joel: Joel, stopped: dict[str, Any]
) -> None:
    """The same meeting joined again after it ended is neither recorded nor announced."""
    key = stopped["key"]
    try:
        joel.run("systemctl --user stop call")
        joel.wait_for(
            f"{joel.listing()} | jq -e '.records[] | select(.key == \"{key}\") | .ended != null'",
            ANSWERS_WITHIN,
        )
        joel.run(
            "systemd-run --user --collect --unit=call ./zoom --record --device=microphone /dev/null"
        )
        joel.wait_for(
            "journalctl --user -u call-transcripts | grep -qF 'a call stopped is rejoined'",
            ANSWERS_WITHIN,
        )
        joel.run("! pw-cli ls Node | grep -qF call-transcripts-")
        assert len([n for n in joel.told() if n[1] == f"{STANDUP} Is Being Transcribed"]) == 1
    except Exception:
        joel.explain()
        raise
    finally:
        joel.run("systemctl --user stop call || true")

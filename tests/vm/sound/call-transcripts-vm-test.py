"""A call in Joel's PipeWire is heard by call-transcripts and written down, both sides in order.

call-transcripts.nix, beside this, runs this through pytest inside the driver
(tests/lib/vm_pytest.py), and its opening comment says what the tests prove. The call is held once
for the module, since Parakeet on the VM's CPU takes a while to hear it, and each test reads what
that call left.
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
# The meeting on, as the google program's daemon holds it, the one that starts while it's summed
# up, and the one after, which is stopped. The stand-in daemon answers every ask with what's in
# its file, as the meetings it holds.
GOOGLE = f"{RUNTIME}/daemons/google.sock"
MEETINGS = f"{RUNTIME}/google-meetings"
MEETING = "Review Planning"
INTERRUPTION = "Interruption"
STANDUP = "Standup"
# How long the daemon has to answer, and to start and finish hearing the call.
ANSWERS_WITHIN = timedelta(minutes=1)
WRITTEN_WITHIN = timedelta(minutes=10)
# Where the session's screen is, said only once the daemon runs, as niri says it.
SCREEN = "stand-in-screen"
# Where the stand-in for the shell's notices keeps each it's given.
TOLD = "/home/joel/told"
# The clips, made as the test builds.
CLIPS = "/etc/call-clips"
# The daemon's ask for every call it holds.
LIST = json.dumps({"id": "list", "ask": "list", "kind": "call"})
# Where the stand-in for the summarizer's model says it started, and its process's id, and
# the pipe that holds its first answer until the test writes to it.
SUMMARIZER_LOG = "/home/joel/summarizer-log"
SUMMARIZER_PID = "/home/joel/summarizer-pid"
GATE = "/home/joel/gate"
# How many times it starts: once to be stopped by the other call, and once to the end.
STARTS = 2
# What a summary says of itself, as summary.rs has it.
HOW_MADE = "These are the lines a language model on this laptop picked"
# Who holds a delay on sleep while a call is summed up, as systemd-inhibit lists it.
HOLDER = "Summing up a call"


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
        """Hold the meeting on now, for an hour, as the google program's daemon would."""
        now = int(self.machine.succeed("date +%s%3N"))
        meeting = {"id": key, "title": title, "start": now, "end": now + 3_600_000}
        record = {"key": f"meeting:{key}", "kind": "meeting", "heard": now, "meeting": meeting}
        answer = json.dumps({"id": 1, "records": [record]})
        self.run(f"echo {shlex.quote(answer)} > {MEETINGS}")
        self.run(
            f"mkdir -p -m 700 {RUNTIME}/daemons;"
            " systemctl --user is-active -q stand-in-google ||"
            " systemd-run --user --unit=stand-in-google"
            f" socat UNIX-LISTEN:{GOOGLE},fork,unlink-early"
            f" SYSTEM:{shlex.quote(f'cat {MEETINGS}')}"
        )
        self.machine.wait_until_succeeds(f"test -S {GOOGLE}", timeout=ANSWERS_WITHIN)

    def record(self, key: str) -> dict[str, Any]:
        """Give the daemon's record of one call."""
        return next(record for record in self.calls() if record["key"] == key)

    def sleep_held(self) -> bool:
        """Say whether the summarizer holds a delay on sleep."""
        return HOLDER in self.run("systemd-inhibit --list --no-pager")

    def starts(self) -> int:
        """Give how many times the stand-in for the summarizer's model has started."""
        return int(self.run(f"grep -cx started {SUMMARIZER_LOG} || true").strip())

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
    joel.run(f"mkfifo {GATE}")
    joel.run(f"systemd-run --user --unit=notices stand-in-notices {TOLD}")
    joel.wait_for("busctl --user status org.freedesktop.Notifications", ANSWERS_WITHIN)
    joel.run(
        "systemd-run --user --unit=call-transcripts --setenv=DICTATION_DEVICE=CPU"
        " --setenv=DICTATION_CACHE=$HOME/.cache/call-transcripts/parakeet"
        f" --setenv=OPENVINO_TOKENIZERS_PATH_GENAI={values['tokenizers']}"
        " --setenv=PATH=/run/current-system/sw/bin"
        f" call-transcripts serve {SOCKET} {values['model']} {values['summarizer']}"
        " $HOME/Documents/Calls"
    )
    joel.wait_for(f"test -S {SOCKET}", ANSWERS_WITHIN)
    joel.run(
        f"systemctl --user set-environment WAYLAND_DISPLAY={SCREEN}"
        " XDG_DATA_DIRS=/run/current-system/sw/share"
    )
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
def interrupted(joel: Joel, call: dict[str, Any]) -> dict[str, bool]:
    """Wait for the call to be summed up, then start another call, which stops it."""
    seen = {}
    try:
        joel.wait_for(f"test -s {SUMMARIZER_LOG}", ANSWERS_WITHIN)
        seen["held while summing"] = joel.sleep_held()
        joel.meeting_on("e3", INTERRUPTION)
        joel.run(
            "systemd-run --user --collect --unit=interruption"
            " ./zoom --record --device=microphone /dev/null"
        )
        joel.wait_for(f"! kill -0 $(cat {SUMMARIZER_PID})", ANSWERS_WITHIN)
        joel.wait_for(
            f"! systemd-inhibit --list --no-pager | grep -qF {shlex.quote(HOLDER)}",
            ANSWERS_WITHIN,
        )
        seen["written once stopped"] = joel.record(call["key"])["summary"] is not None
        seen["started during the call"] = joel.starts() > 1
    except Exception:
        joel.explain()
        raise
    finally:
        joel.run("systemctl --user stop interruption || true")
    return seen


@pytest.fixture(scope="module")
def written(joel: Joel, call: dict[str, Any], interrupted: dict[str, bool]) -> dict[str, Any]:
    """Let the summary started again after the other call go on, giving the record once done."""
    try:
        joel.wait_for(f"test $(grep -cx started {SUMMARIZER_LOG}) -ge {STARTS}", ANSWERS_WITHIN)
        joel.run(f"echo open > {GATE}")
        joel.wait_for(f"grep -qxF 'Summary of {MEETING} Is Ready' {TOLD}", ANSWERS_WITHIN)
    except Exception:
        joel.explain()
        raise
    assert interrupted
    record = joel.record(call["key"])
    assert record["key"] == "call:e1"
    return record


@pytest.fixture(scope="module")
def stopped(joel: Joel, written: dict[str, Any]) -> dict[str, Any]:
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
    assert written["key"] == "call:e1"
    return joel.record("call:e2")


def transcript(joel: Joel, call: dict[str, Any]) -> str:
    """Give the call's transcript as written."""
    written = joel.run(f"cat {shlex.quote(call['transcript'])}")
    print(written)
    return written


@pytest.mark.usefixtures("call")
def test_parakeet_waits_for_a_call_with_no_npu(joel: Joel) -> None:
    """With no NPU to compile for, the service holds no model until a call's first take."""
    said = joel.run("journalctl --user -u call-transcripts -o cat --no-pager")
    assert "Parakeet needs no compile ahead of a call" in said, f"it didn't say so:\n{said}"
    assert "compiled ahead" not in said, f"Parakeet was compiled ahead with no NPU:\n{said}"


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
    # A speech model on the VM's processor can hear a word wrong (Whisper heard "review" as
    # "renew"), so each turn is known by a word it hears every time; dictation's checks hold
    # the words.
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


def test_an_ended_call_lets_the_machine_go_idle(joel: Joel, call: dict[str, Any]) -> None:
    """Once the call ended, call-transcripts holds nothing against the machine going idle."""
    assert call["ended"] is not None
    joel.wait_for(
        "! systemd-inhibit --list --no-legend | grep -qF call-transcripts", ANSWERS_WITHIN
    )


def test_a_summary_holds_off_sleep_while_it_runs(interrupted: dict[str, bool]) -> None:
    """While the model's process runs, a delay on sleep is held; it's let go once stopped."""
    assert interrupted["held while summing"]


def test_a_call_starting_stops_a_summary_which_writes_nothing(
    interrupted: dict[str, bool],
) -> None:
    """Another call starting ends the model's process, nothing is written, none starts again."""
    assert not interrupted["written once stopped"]
    assert not interrupted["started during the call"]


def test_the_summary_is_the_calls_own_words(joel: Joel, written: dict[str, Any]) -> None:
    """Started again once the other call ended, the summary quotes Joel's line, word for word."""
    summary = joel.run(f"cat {shlex.quote(written['folder'])}/summary.md")
    print(summary)
    assert summary == f"{written['summary']}\n"
    assert summary.startswith(f"# {MEETING}\n\n{HOW_MADE}")
    [asked] = [
        line
        for line in transcript(joel, written).splitlines()
        if "thursday" in line.lower() and not line.startswith("**Others**")
    ]
    said = asked.split(": ", 1)[1]
    stamp = asked.split(", ", 1)[1].split(": ", 1)[0]
    # The stand-in copies the line whole, and the quote keeps the line's words from its first to
    # its last, so only the closing mark is left behind.
    assert f'to the others: "{said.rstrip("?.!")}" [{stamp}]' in summary


def test_the_model_is_let_go_once_the_summary_is_written(
    joel: Joel, written: dict[str, Any]
) -> None:
    """Nothing of the summarizer stays: neither its process nor its delay on sleep."""
    assert written["summary"] is not None
    joel.wait_for(f"! kill -0 $(cat {SUMMARIZER_PID})", ANSWERS_WITHIN)
    assert not joel.sleep_held()
    assert joel.starts() == STARTS


def test_the_notice_opens_the_summary_in_the_sessions_reader(
    joel: Joel, written: dict[str, Any]
) -> None:
    """Read It shows the summary on the session's screen, said after the daemon started."""
    [(app, _, _)] = [n for n in joel.told() if n[1] == f"Summary of {MEETING} Is Ready"]
    assert app == "Calls"
    joel.choose(f"Summary of {MEETING} Is Ready", "read")
    try:
        joel.wait_for("test -s read", ANSWERS_WITHIN)
    except Exception:
        joel.explain()
        raise
    assert joel.run("cat read").strip() == f"{SCREEN} {written['folder']}/summary.md"


def test_a_call_starting_offers_the_line_for_its_chat(joel: Joel, call: dict[str, Any]) -> None:
    """As the call started, one notice from Calls said so, with the line to tell the others."""
    [(app, _, line)] = [n for n in joel.told() if n[1] == f"{MEETING} Is Being Transcribed"]
    assert app == "Calls"
    assert "Nothing is uploaded" in line
    assert call["key"] == "call:e1"


def test_a_call_ending_withdraws_its_notice(joel: Joel, call: dict[str, Any]) -> None:
    """Once the call ended, its notice saying it's being transcribed was withdrawn."""
    heading = f"{MEETING} Is Being Transcribed"
    joel.wait_for(f"grep -qxF {shlex.quote(heading)} {TOLD}-withdrawn", ANSWERS_WITHIN)
    assert call["ended"] is not None, "the call is held as ended"


def test_stop_transcribing_stops_at_once_with_the_call_still_on(
    joel: Joel, stopped: dict[str, Any]
) -> None:
    """Stopping from the notice lets both recorders go, though the call program still runs."""
    assert stopped["transcribing"] is False
    assert stopped["ended"] is None
    joel.run("systemctl --user is-active call")
    joel.wait_for("! pw-cli ls Node | grep -qF call-transcripts-", ANSWERS_WITHIN)


def test_a_call_untranscribed_still_keeps_the_machine_from_going_idle(
    joel: Joel, stopped: dict[str, Any]
) -> None:
    """With transcribing stopped and the call still on, idle sleep is still held off."""
    assert stopped["ended"] is None
    held = joel.run("systemd-inhibit --list --no-legend | grep -F call-transcripts")
    assert "idle" in held
    assert "block" in held


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

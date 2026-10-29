"""The desktop test's launcher goes places area.

Here the launcher goes places: pi's sessions, quests and runs, the volume, kitty, the windows it
finds and moves, a meeting's call, and the shortcuts. tests/desktop.nix runs this through pytest
inside the driver (tests/lib/vm_pytest.py), and its opening comment says what the test proves.
Every test asks for the desktop, started once a run (desktop_session.py), and waits at its start
for what it needs, so any test runs alone.
"""

from __future__ import annotations

import datetime as dt
import json
import re
import shlex
import time
from typing import TYPE_CHECKING, Any, cast

import desktop_session
import pytest
from desktop_session import (
    LAUNCHER_OPEN,
    PI_PRESENCE,
    PI_QUEST,
    PI_WINDOW,
    STARTING,
    drawn_at,
    retry,
    settles_on,
)

if TYPE_CHECKING:
    from collections.abc import Callable, Iterator

    from desktop_session import Desktop, Json
    from test_driver.machine import BaseMachine


@pytest.fixture(scope="module")
def desktop(machine: BaseMachine, values: dict[str, Any]) -> Desktop:
    """Give the desktop, started once a run."""
    return desktop_session.started(machine, cast("desktop_session.Values", values))


def test_the_launcher_shows_the_volumes_chords_and_escape_closes_it(desktop: Desktop) -> None:
    """The launcher shows the volume's chords, and Escape closes it."""
    machine = desktop.machine
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("volume")
    desktop.photograph("launcher-volume")
    machine.send_key("esc")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=dt.timedelta(seconds=10))


# One walk, as its subtest was: each step starts where the last left it.
def test_the_launcher_finds_a_pi_session_by_its_quest_and_goes_to_its_window(  # noqa: PLR0915
    desktop: Desktop,
) -> None:
    """The launcher finds a pi session by its quest, and goes to its window."""
    machine = desktop.machine
    desktop.pi_session()
    # What it spent today, as spend-watch writes it, replaced whole.
    spent = {
        "v": 2,
        "sessions": {"01a1face-test": "$12.50"},
        "helpers": {"01a1face-test": [{"on": "councils", "spent": "$4.00"}]},
    }
    desktop.as_account(
        f"mkdir -p $XDG_RUNTIME_DIR/spend-watch && echo '{json.dumps(spent)}'"
        " > $XDG_RUNTIME_DIR/spend-watch/.c"
        " && mv $XDG_RUNTIME_DIR/spend-watch/.c $XDG_RUNTIME_DIR/spend-watch/collection.json"
    )
    # A VM test of its waiting its turn in the machine's queue.
    lined = {
        "v": 1,
        "heartbeat": float(machine.succeed("date +%s")),
        "machine": {},
        "jobs": [
            {
                "id": "t",
                "kind": "vm-test",
                "state": "waiting",
                "owner": {"session": "01a1face-test"},
                "what": "Test the shell",
                "since": machine.succeed("date -u +%FT%TZ").strip(),
                "place": 2,
            },
        ],
    }
    desktop.as_account(
        f"echo '{json.dumps(lined)}' > $XDG_RUNTIME_DIR/jobs/.q"
        " && mv $XDG_RUNTIME_DIR/jobs/.q $XDG_RUNTIME_DIR/jobs/queue.json"
    )
    # From the other window, so going to the session's means a move.
    desktop.as_account(
        f"niri msg action focus-window --id {desktop.window_titled('Elsewhere')['id']}"
    )
    # The launcher's foot wears the agents' mark while a session runs. How
    # many work it says only while the foot has room, which it hasn't at
    # the test machine's width with the keys' room kept, so the mark alone
    # (modules/theme/glyphs.nix's agent) is what's asked of it here.
    since = desktop.mark()
    desktop.as_account("qs ipc call launcher toggle")
    desktop.reported("stands ", lambda rest: "\U000f06a9" in rest, since)
    desktop.launcher_ready()
    # Its collection by what it's called, then the session named
    # whole, as its README is, not as its window's title.
    desktop.type_in("agents")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Agents")
    machine.send_key("ret")
    desktop.launcher_until(
        lambda shown: (
            shown["held"] == ["agents"]
            and (shown["selected"] or {}).get("name") == "A quest of its own & whole"
            and "$12.50 today" in shown["selected"]["side"]
            and "2nd in line" in shown["selected"]["side"]
        )
    )
    desktop.photograph("launcher-agents")
    # Stepped into, what's known of it, each a row of its own.
    desktop.step_in()
    desktop.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("name") == "Switch"
            and shown["selected"]["verb"] == "switch"
        )
    )
    # Each named first, as every part is, then what it says.
    for name, side in [
        ("Quest", r"QEST-20261022-TESTQA"),
        ("Address", r"TESTQA"),
        ("Started", r"\d\d:\d\d"),
        ("Spent Today", r"\$12\.50"),
        ("Councils", r"\$4\.00"),
        ("Test the shell", r"2nd in line"),
        ("Working On", r"Tidy the tests"),
        ("Hears", r"#desk"),
    ]:
        machine.send_key("down")
        # The wait ends before the loop moves on, so it reads this pass's names.
        desktop.launcher_until(
            lambda shown: (
                (shown["selected"] or {}).get("name") == name  # noqa: B023
                and re.fullmatch(side, shown["selected"]["side"])  # noqa: B023
            )
        )
    desktop.photograph("launcher-agent-facts")
    machine.send_key("left")
    desktop.launcher_until(lambda shown: shown["held"] == ["agents"])
    desktop.as_account(
        "rm $XDG_RUNTIME_DIR/spend-watch/collection.json $XDG_RUNTIME_DIR/jobs/queue.json"
    )
    # Found by its address too.
    desktop.type_in("testqa", selecting="A quest of its own & whole")
    machine.send_key("ret")
    machine.wait_until_succeeds(
        desktop.in_session("niri msg --json focused-window | grep -q TESTQA"),
        timeout=dt.timedelta(seconds=10),
    )
    # Its title is read once, outside what's drawn, so nothing drawn
    # reads it again as it changes.
    log = desktop.shell_said("Binding loop")
    assert "Binding loop" not in log, "a session's title is read inside a binding"

    # It comes to want Joel, its presence replaced by rename as pi's
    # is: the margin hangs pi's mark, and lets it go once answered.
    # From another window, so it's said as a notice too, whose
    # Return goes to the session's window.
    desktop.as_account(
        f"niri msg action focus-window --id {desktop.window_titled('Elsewhere')['id']}"
    )

    # The margin may hold other marks, a notice missed earlier among
    # them, so it's pi's own that's waited on.
    # Wants begun now, as the shell only says those it saw begin.
    def begun() -> str:
        """Give the machine's time now, as pi writes when a want began."""
        return machine.succeed("date -u +%Y-%m-%dT%H:%M:%SZ").strip()

    question = {"kind": "question", "since": begun(), "said": "Which <way> & why?"}
    since = desktop.mark()
    # Said as written: the summary is plain, so its & isn't escaped.
    with desktop.told("shows card A quest of its own & whole asks you"):
        desktop.presence_says({"addresses": ["TESTQA"], "busy": False, "wants": question})
    # Its card goes by itself after ten seconds, which a busy host spends
    # on what follows, so Super+N takes the stack at once: held, it waits.
    with desktop.told("keys notices"):
        machine.send_key("meta_l-n")
    # The card says it, so pi's mark waits for the card to go: a card or
    # a mark, never both.
    told_since = desktop.as_account(
        "journalctl --user --unit=quickshell --no-pager --output=cat"
        f" --after-cursor={shlex.quote(since)}"
    )
    assert not re.search("holds margin [^\\n]*\\bagent\\b", told_since), (
        "pi's mark showed beside its card"
    )
    # The same want, its file replaced again, says nothing new.
    desktop.presence_says({"addresses": ["TESTQA"], "busy": False, "wants": question})
    time.sleep(3)
    cards = (
        "journalctl --user --unit=quickshell --no-pager --output=cat"
        " | grep -c 'report\\[shows card A quest of its own & whole asks you'"
    )
    assert desktop.as_account(cards).strip() == "1", "the same want was said twice"
    since = desktop.mark()
    machine.send_key("ret")
    machine.wait_until_succeeds(
        desktop.in_session("niri msg --json focused-window | grep -q TESTQA"),
        timeout=dt.timedelta(seconds=10),
    )
    machine.wait_until_fails(
        desktop.in_session("niri msg layers | grep -q notifications"),
        timeout=dt.timedelta(seconds=10),
    )
    # Its card gone, the want still stands, so pi's mark shows.
    desktop.reported("holds margin ", lambda rest: "agent" in rest.split(), since)
    desktop.photograph("margin-pi-wants")
    # Another, unanswered, from the other window: its notice says what
    # it wants, and goes when the want does, missed by nobody. Notices
    # missed earlier are let go first, so the margin's notices mark is
    # this one's alone.
    held = desktop.as_account(
        "journalctl --user --unit=quickshell --no-pager --output=cat"
        " | grep -o 'report\\[holds margin [^]]*' | tail -n 1"
    )
    since = desktop.mark()
    desktop.as_account("qs ipc call notices seen")
    if "waiting" in held.split()[2:]:
        desktop.reported("holds margin ", lambda rest: "waiting" not in rest.split(), since)
    desktop.as_account(
        f"niri msg action focus-window --id {desktop.window_titled('Elsewhere')['id']}"
    )
    with desktop.told("shows card A quest of its own & whole wants your yes"):
        desktop.presence_says(
            {
                "addresses": ["TESTQA"],
                "busy": False,
                "wants": {"kind": "approval", "since": begun(), "said": "Push it?"},
            }
        )
    # Ended while its notice is up, the notice goes with it, well
    # before its own ten seconds would.
    desktop.presence_says({"addresses": ["TESTQA"], "busy": False})
    machine.wait_until_fails(
        desktop.in_session("niri msg layers | grep -q notifications"),
        timeout=dt.timedelta(seconds=5),
    )
    # A third, left to time out while its want stands: it isn't
    # missed, since pi's mark says the want; ended, the mark goes.
    with desktop.told("shows card A quest of its own & whole is stuck without you"):
        desktop.presence_says(
            {
                "addresses": ["TESTQA"],
                "busy": False,
                "wants": {"kind": "blocked", "since": begun(), "said": "Out of ideas."},
            }
        )
    machine.wait_until_fails(
        desktop.in_session("niri msg layers | grep -q notifications"),
        timeout=dt.timedelta(seconds=30),
    )
    time.sleep(2)

    def margin_holds() -> list[str]:
        """Give what the margin last said it holds."""
        return desktop.as_account(
            "journalctl --user --unit=quickshell --no-pager --output=cat"
            " | grep -o 'report\\[holds margin [^]]*' | tail -n 1"
        ).split()[2:]

    assert "waiting" not in margin_holds(), "pi's notice counted as missed"
    since = desktop.mark()
    desktop.presence_says({"addresses": ["TESTQA"], "busy": False})
    desktop.reported("holds margin ", lambda rest: "agent" not in rest.split(), since)
    assert "waiting" not in margin_holds(), "pi's notice counted as missed"


HARNESS = "~/.local/state/pi/agentic-harness.pi"
INBOX = f"{HARNESS}/steer/TESTQA"


def an_orchestrator(desktop: Desktop) -> None:
    """Make pi's session an orchestrator, Lighthouse, as the harness lists one, its inbox empty."""
    desktop.pi_session()
    lighthouse_listed(desktop)


def lighthouse_listed(desktop: Desktop) -> None:
    """List Lighthouse as an orchestrator, as the harness would, its inbox empty."""
    listed = {
        "orchestrators": [
            {"quest": "QEST-20261022-TESTQA", "address": "TESTQA", "short": "Lighthouse",
             "title": "Keep the lighthouse lit"},
        ]
    }  # fmt: skip
    desktop.as_account(
        f"mkdir -p {HARNESS}/quest-workflow && echo '{json.dumps(listed)}'"
        f" > {HARNESS}/quest-workflow/.o && mv {HARNESS}/quest-workflow/.o"
        f" {HARNESS}/quest-workflow/orchestrators.json && rm -rf {INBOX}"
    )


def into_lighthouse(desktop: Desktop) -> None:
    """Open Orchestrators, and step into Lighthouse."""
    desktop.as_account("qs ipc call launcher collection orchestrators")
    desktop.launcher_ready()
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Lighthouse")
    desktop.step_in("ret")
    desktop.launcher_until(lambda shown: "Talk" in shown["rows"])


def test_an_orchestrator_is_found_by_name_and_talked_to_through_its_inbox(
    desktop: Desktop,
) -> None:
    """An orchestrator is found by its name, and talked to through its session's inbox."""
    machine = desktop.machine
    closed = desktop.in_session(LAUNCHER_OPEN)
    an_orchestrator(desktop)
    # Listed busy, as its presence says.
    desktop.as_account("qs ipc call launcher collection orchestrators")
    desktop.launcher_ready()
    desktop.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("name") == "Lighthouse"
            and str(shown["selected"]["side"]).startswith("busy")
        )
    )
    machine.send_key("esc")
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
    # Found from the root by its name, the right arrow lands inside it, as
    # Return does, never short of it (Joel, 2026-10-26: Return never goes
    # further in than the right arrow), so the foot says the two as one.
    # The left arrow climbs back out the way it came, Lighthouse selected
    # among the orchestrators, then onto what was typed.
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("lighthouse", selecting="Lighthouse")
    told = desktop.showing()["selected"]["told"]
    assert told == ["\u21b5 \u2192 more"], told
    machine.send_key("right")
    desktop.launcher_until(
        lambda shown: (
            shown["path"] == ["Orchestrators", "Lighthouse"]
            and {"Talk", "Pause", "Stop"} <= set(shown["rows"])
        )
    )
    machine.send_key("left")
    desktop.launcher_until(
        lambda shown: (
            shown["path"] == ["Orchestrators"]
            and (shown["selected"] or {}).get("name") == "Lighthouse"
        )
    )
    machine.send_key("left")
    desktop.launcher_until(lambda shown: shown["held"] == [] and shown["typed"] == "lighthouse")
    machine.send_key("esc")
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
    # Return lands inside it the same way, its collection's crumb before
    # its own.
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("lighthouse", selecting="Lighthouse")
    machine.send_key("ret")
    desktop.launcher_until(
        lambda shown: (
            shown["path"] == ["Orchestrators", "Lighthouse"]
            and {"Talk", "Pause", "Stop"} <= set(shown["rows"])
        )
    )
    # Talk asks for words, which go to its session's inbox as Joel's,
    # opening a talk its session follows.
    desktop.type_in("talk", selecting="Talk")
    desktop.step_in("ret")
    machine.send_chars("keep the lamp lit")
    machine.send_key("ret")
    machine.wait_until_succeeds(
        desktop.in_session(f'cat {INBOX}/*.json | grep -q \'"text":"keep the lamp lit"\''),
        timeout=dt.timedelta(seconds=10),
    )
    said = json.loads(desktop.as_account(f"cat {INBOX}/*.json"))
    assert said["from"] == "joel", said
    assert str(said.get("id", "")).startswith("talk-"), said
    machine.send_key("esc")
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))


def test_an_orchestrators_earlier_talks_say_how_each_went(desktop: Desktop) -> None:
    """An orchestrator's earlier talks say how each went, and read its reply."""
    machine = desktop.machine
    closed = desktop.in_session(LAUNCHER_OPEN)
    an_orchestrator(desktop)
    talk = [
        {"at": "2026-10-26T04:00:00Z", "stage": "delivered", "session": "s",
         "text": "keep watch tonight"},
        {"at": "2026-10-26T04:00:05Z", "stage": "replied", "text": "Watching. All quiet."},
    ]  # fmt: skip
    lines = "\n".join(json.dumps(event) for event in talk)
    desktop.as_account(
        f"mkdir -p {INBOX}/replies && printf '%s\\n' '{lines}' > {INBOX}/replies/t1.jsonl"
    )
    # Listed after its actions, saying how it went.
    into_lighthouse(desktop)
    desktop.launcher_until(lambda shown: "keep watch tonight" in shown["rows"])
    desktop.type_in("keep watch", selecting="keep watch tonight")
    assert desktop.showing()["selected"]["side"] == "replied", desktop.showing()["selected"]
    # Read, it says his words, how it went, and the reply below.
    desktop.step_in()
    desktop.launcher_until(
        lambda shown: (
            shown["rows"][:2] == ["You Said", "How It Went"]
            and "Watching" in str((shown["selected"] or {}).get("reads", ""))
        )
    )
    machine.send_key("esc")
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))


def test_a_question_asked_is_talked_over_with_an_orchestrator(desktop: Desktop) -> None:
    """A question asked is talked over with an orchestrator, from Ask's answer."""
    machine = desktop.machine
    closed = desktop.in_session(LAUNCHER_OPEN)
    an_orchestrator(desktop)
    desktop.answer("what is a fjord?")
    machine.send_key("ret")
    desktop.launcher_until(lambda shown: str(shown["footer"]).startswith("answered in"))
    # A step in leads with Return's own Copy, then talking it over with
    # Lighthouse comes before asking in a session of its own.
    desktop.step_in()
    desktop.launcher_until(
        lambda shown: shown["rows"][:3] == ["Copy", "Talk to Lighthouse", "Ask in a Session"]
    )
    machine.send_key("down")
    desktop.launcher_until(
        lambda shown: (shown["selected"] or {}).get("name") == "Talk to Lighthouse"
    )
    machine.send_key("ret")
    # The question goes to its inbox with what was answered, and the
    # launcher lands inside Lighthouse, where its talks say how each went.
    machine.wait_until_succeeds(
        desktop.in_session(f"cat {INBOX}/*.json | grep -q 'The launcher answered: A fjord'"),
        timeout=dt.timedelta(seconds=10),
    )
    said = json.loads(desktop.as_account(f"cat {INBOX}/*.json"))
    assert said["text"].startswith("what is a fjord?\n\nThe launcher answered: "), said
    assert said["from"] == "joel", said
    desktop.launcher_until(
        lambda shown: (
            shown["path"] == ["Agents", "Orchestrators", "Lighthouse"] and "Talk" in shown["rows"]
        )
    )
    machine.send_key("esc")
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))


def test_an_orchestrator_is_paused_with_no_words(desktop: Desktop) -> None:
    """An orchestrator is paused with no words, an act in its inbox."""
    machine = desktop.machine
    closed = desktop.in_session(LAUNCHER_OPEN)
    an_orchestrator(desktop)
    into_lighthouse(desktop)
    desktop.type_in("pause", selecting="Pause")
    machine.send_key("ret")
    machine.wait_until_succeeds(
        desktop.in_session(f'cat {INBOX}/*.json | grep -q \'"act":"pause"\''),
        timeout=dt.timedelta(seconds=10),
    )
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))


def test_the_launcher_finds_pis_quests_going_to_the_session_on_one_and_opening_pi_on_another(
    desktop: Desktop,
) -> None:
    """The launcher finds pi's quests, going to the session on one and opening pi on another."""
    machine = desktop.machine
    desktop.pi_session()
    # Their READMEs as pi keeps them, front matter first: the session's
    # quest, and another in a tree of its own with no session on it.
    another = "~/.local/share/pi/agentic-harness.pi/quest-workflow/quests/QEST-20261022-HALLWY"
    desktop.as_account(
        "printf '%s\\n' '---' 'id: QEST-20261022-TESTQA' 'kind: quest' 'status: active'"
        " 'priority: driving' 'rank: 1' '---' '# A quest of its own & whole'"
        f" > {PI_QUEST}/README.md"
        f" && mkdir -p {another} ~/hallway && printf '%s\\n' '---' 'id: QEST-20261022-HALLWY'"
        " 'kind: quest' 'status: active' 'priority: queued' 'rank: 1' 'trees:'"
        ' "  - path: $HOME/hallway"'
        f" '---' '# Paint the hallway' > {another}/README.md"
    )
    desktop.presence_says(
        {"addresses": ["TESTQA"], "busy": False, "owns": ["QEST-20261022-TESTQA"]}
    )
    desktop.as_account(
        f"niri msg action focus-window --id {desktop.window_titled('Elsewhere')['id']}"
    )
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    # Agents' one word for them finds them.
    desktop.type_in("quests")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Quests")
    machine.send_key("ret")
    # The one driving first, naming the session that has it loaded.
    desktop.launcher_until(
        lambda shown: (
            shown["held"] == ["quests"]
            and (shown["selected"] or {}).get("name") == "A quest of its own & whole"
            and shown["selected"]["side"] == "driving \u00b7 TESTQA"
        )
    )
    machine.send_key("down")
    desktop.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("name") == "Paint the hallway"
            and shown["selected"]["side"] == "queued"
        )
    )
    desktop.photograph("launcher-quests")
    machine.send_key("up")
    desktop.launcher_until(
        lambda shown: (shown["selected"] or {}).get("name") == "A quest of its own & whole"
    )
    machine.send_key("ret")
    machine.wait_until_succeeds(
        desktop.in_session("niri msg --json focused-window | grep -q TESTQA"),
        timeout=dt.timedelta(seconds=10),
    )
    # One with no session, found by a word of its title, opens pi on
    # it in its tree.
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("hallway", selecting="Paint the hallway")
    machine.send_key("ret")
    machine.wait_until_succeeds(
        desktop.in_session(
            'grep -qx "$HOME/hallway QEST-20261022-HALLWY" $XDG_RUNTIME_DIR/pi-opened'
        ),
        timeout=dt.timedelta(seconds=30),
    )
    desktop.as_account(f"rm -rf {another} ~/hallway $XDG_RUNTIME_DIR/pi-opened")


def test_agents_resumes_a_closed_orchestrator_where_it_left_off_from_one_row(
    desktop: Desktop,
) -> None:
    """Agents resumes a closed orchestrator where it left off, from one row and a choice."""
    machine = desktop.machine
    opened = "$XDG_RUNTIME_DIR/pi-opened"
    # Lighthouse orchestrates a quest, and no session runs it; its quest's
    # last session ran in ~/lighthouse, as pi's file for it says.
    lighthouse_listed(desktop)
    index = {"quests": [{"id": "QEST-20261022-TESTQA", "sessions": ["01a1-lighthouse"]}]}
    record = {"type": "session", "id": "01a1-lighthouse", "cwd": "/home/jitsusama/lighthouse"}
    sessions = "~/.pi/agent/sessions/--home-jitsusama-lighthouse--"
    kept = f"{sessions}/2026-10-28T20-00-00-000Z_01a1-lighthouse.jsonl"
    desktop.as_account(
        f"rm -f {PI_PRESENCE}/testqa.json {opened} && mkdir -p {sessions} ~/lighthouse"
        f" && echo '{json.dumps(record)}' > {kept}"
        f" && echo '{json.dumps(index)}' > {HARNESS}/quest-workflow/quests.json"
    )
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.down_to("Agents")
    machine.send_key("ret")
    desktop.launcher_until(
        lambda shown: shown["held"] == ["agents"] and "Start a Session" in shown["rows"]
    )
    # The sessions come first, then starting one, then Agents' places.
    desktop.down_to("Start a Session")
    desktop.photograph("launcher-agents-start")
    machine.send_key("ret")
    desktop.launcher_until(lambda shown: shown["rows"][:2] == ["On a Quest", "As an Orchestrator"])
    desktop.photograph("launcher-agents-start-which")
    desktop.down_to("As an Orchestrator")
    machine.send_key("ret")
    desktop.launcher_until(lambda shown: shown["rows"] == ["Lighthouse"])
    assert desktop.showing()["selected"]["told"][:1] == ["\u21b5 resume"], desktop.showing()
    desktop.photograph("launcher-agents-start-orchestrator")
    # Return reopens its last session where it ran, as the harness
    # restores one, through a login shell so pi can reach its model.
    machine.send_key("ret")
    machine.wait_until_succeeds(
        desktop.in_session(f'grep -qx "$HOME/lighthouse " {opened}'),
        timeout=dt.timedelta(seconds=30),
    )
    said = desktop.as_account(f"cat {opened}").split("\n")
    assert said[1:3] == ["--session", "01a1-lighthouse"], said
    # Resuming one takes Joel to it, so the launcher closes.
    desktop.launcher_until(lambda shown: not shown["open"])
    # Its own row under Orchestrators says the same, Resume its Return.
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("lighthouse", selecting="Lighthouse")
    assert desktop.showing()["selected"]["told"][:1] == ["\u21b5 resume"], desktop.showing()
    machine.send_key("esc")
    desktop.launcher_until(lambda shown: not shown["open"])
    desktop.as_account(
        f"rm -rf {opened} {sessions} ~/lighthouse {HARNESS}/quest-workflow/orchestrators.json"
        f" {HARNESS}/quest-workflow/quests.json"
    )


def test_the_launcher_finds_pis_runs_going_to_the_session_that_started_one_and_reading_an_answer(
    desktop: Desktop,
) -> None:
    """The launcher finds pi's runs, going to the session that started one and reading an answer."""
    machine = desktop.machine
    desktop.pi_session()
    # A fleet as the harness keeps one, started by the session's process:
    # one member still at work, its process the other window's, and one
    # done with its answer.
    leader = desktop.pi_record("TESTQA")["owner"]["pid"]
    crew = "~/.local/state/pi/agentic-harness.pi/subagent-workflow/runs/paint-crew/reviewers"

    def keeps(path: str, said: Json) -> None:
        """Write what the harness keeps of the fleet, at a path inside it."""
        desktop.as_account(
            f"mkdir -p {crew}/{path.split('/', maxsplit=1)[0]}"
            f" && echo '{json.dumps(said)}' > {crew}/{path}"
        )

    asked = {"cwd": "/src/house", "args": ["cli.js", "--model", "anthropic/opus"]}
    for member, child, stands in [
        ("brush", desktop.window_titled("Elsewhere")["pid"], "running"),
        ("roller", 1, "complete"),
    ]:
        keeps(f"{member}/request.json", asked)
        keeps(f"{member}/lease.json", {"parentPid": leader, "childPid": child})
        keeps(
            f"{member}/progress.json",
            {"state": stands, "activity": "painting the trim" if stands == "running" else ""},
        )
    stamp = machine.succeed("date -u +%FT%T.000Z").strip()
    keeps(
        "roller/result.json",
        {
            "state": "complete",
            "startedAt": stamp,
            "completedAt": stamp,
            "usage": {"cost": {"total": 2.5}},
            "finalAssistantText": "Two coats.",
        },
    )
    desktop.presence_says({"addresses": ["TESTQA"], "busy": False})
    desktop.as_account(
        f"niri msg action focus-window --id {desktop.window_titled('Elsewhere')['id']}"
    )
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    # Agents' one word for them finds them.
    desktop.type_in("subagents")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Subagents")
    machine.send_key("ret")
    desktop.launcher_until(
        lambda shown: (
            shown["held"] == ["subagents"]
            and (shown["selected"] or {}).get("name") == "paint-crew"
            and re.fullmatch(
                r"fleet \u00b7 1 of 2 done \u00b7 (under a minute|\d+ min) \u00b7 TESTQA",
                shown["selected"]["side"],
            )
        )
    )
    desktop.photograph("launcher-runs")
    # Stepped into, its members after the way to its session.
    desktop.step_in()
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Switch")
    machine.send_key("down")
    desktop.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("name") == "brush"
            and shown["selected"]["side"].startswith("painting the trim \u00b7 opus")
        )
    )
    machine.send_key("down")
    desktop.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("name") == "roller"
            and shown["selected"]["side"] == "opus \u00b7 $2.50 \u00b7 under a minute"
        )
    )
    desktop.photograph("launcher-run-members")
    # Its answer, read in a terminal named for the run and the member.
    machine.send_key("ret")
    reading = "paint-crew \u00b7 roller"
    machine.wait_until_succeeds(
        desktop.in_session(f"niri msg --json windows | grep -q '{reading}'"),
        timeout=STARTING,
    )
    # Still open a moment on, so the pager holds the answer.
    time.sleep(1)
    desktop.as_account(f"niri msg action close-window --id {desktop.window_titled(reading)['id']}")
    # Return on the run itself goes to its session.
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("subagents")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Subagents")
    machine.send_key("ret")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "paint-crew")
    machine.send_key("ret")
    machine.wait_until_succeeds(
        desktop.in_session("niri msg --json focused-window | grep -q TESTQA"),
        timeout=dt.timedelta(seconds=10),
    )
    desktop.as_account(
        "rm -rf ~/.local/state/pi/agentic-harness.pi/subagent-workflow/runs/paint-crew"
    )
    for title in [PI_WINDOW, "Elsewhere"]:
        desktop.as_account(
            f"niri msg action close-window --id {desktop.window_titled(title)['id']}"
        )
    desktop.as_account(f"rm -rf {PI_QUEST} {PI_PRESENCE}/testqa.json")
    machine.wait_until_fails(
        desktop.in_session("niri msg --json windows | grep -q 'TESTQA\\|\"Elsewhere\"'"),
        timeout=dt.timedelta(seconds=30),
    )


def test_the_launcher_sets_the_volume_to_a_number_and_says_how_loud_it_is(desktop: Desktop) -> None:
    """The launcher sets the volume to a number, and says how loud it is."""
    machine = desktop.machine
    # Muted first, since a level asked for is one to be heard.
    desktop.as_account("wpctl set-mute @DEFAULT_AUDIO_SINK@ 1")
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("volume 40")
    desktop.launcher_until(
        lambda shown: (shown["selected"] or {}).get("name", "").endswith(" to 40%")
    )
    machine.send_key("ret")
    # Exactly that, and no longer muted.
    machine.wait_until_succeeds(
        desktop.in_session("wpctl get-volume @DEFAULT_AUDIO_SINK@ | grep -qx 'Volume: 0.40'"),
        timeout=dt.timedelta(seconds=10),
    )
    # It changed something, so the launcher stays until it's closed.
    machine.send_key("esc")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=dt.timedelta(seconds=10))
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("volume")
    desktop.launcher_until(
        lambda shown: (shown["selected"] or {}).get("name", "").startswith("40%, through ")
    )
    desktop.photograph("launcher-volume-told")
    # A step in, each device's level drawn as a bar.
    desktop.step_in()
    desktop.photograph("launcher-volume-parts")
    machine.send_key("esc")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=dt.timedelta(seconds=10))


def test_the_launcher_finds_and_opens_kitty_and_learns_that_kitty_was_meant(
    desktop: Desktop,
) -> None:
    """The launcher finds and opens kitty, and learns that kitty was meant."""
    machine = desktop.machine
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("kitty", selecting="kitty")
    desktop.photograph("launcher")
    machine.send_key("ret")
    # The first window opened can take as long as the first GTK one.
    machine.wait_until_succeeds(
        desktop.in_session("niri msg windows | grep -i kitty"), timeout=STARTING
    )
    # What was picked for what was typed is kept for next time.
    picked = '\'"kitty":{"program:kitty"\''
    machine.wait_until_succeeds(
        desktop.in_session(f"grep -rqsF {picked} ~/.local/state/quickshell"),
        timeout=dt.timedelta(seconds=10),
    )
    # And what the opening took: five letters, a Return, kitty picked for them.
    kept = desktop.as_account(
        "cat $(find ~/.local/state/quickshell -name launcher-trials.json) 2>/dev/null || true"
    )
    trial = json.loads(kept or "[]")[-1]
    assert trial["picked"] == "program:kitty", trial
    assert trial["typed"] == "kitty", trial
    assert trial["keys"]["letters"] == len("kitty"), trial
    assert trial["keys"]["returns"] == 1, trial
    assert trial["ms"] > 0, trial
    assert not trial["missed"], trial
    # Each key timed from its search to its frame, which comes after; keys
    # typed faster than frames share one, timed by the last.
    timed = trial["timed"]
    assert 1 <= timed["count"] <= len("kitty"), timed
    assert timed["drawnMost"] >= timed["searchedMost"] >= 0, timed


# The border is all that marks the focused window, and niri leaves
# borders off unless its config turns them on.


def test_the_focused_window_is_framed_in_the_accent(desktop: Desktop) -> None:
    """The focused window is framed in the accent."""
    # The kitty the launcher opened, or one of its own when run alone.
    bordered = desktop.focused_kitty()
    desktop.as_account(f"niri msg action move-window-to-floating --id {bordered['id']}")

    def beside() -> str:
        """Give the colour just left of the focused window, where its border is."""
        image = desktop.settled_screen("focused")
        left, top, _, height = drawn_at(
            json.loads(desktop.as_account("niri msg --json focused-window"))
        )
        # An RGB picture's pixels are each a red, a green and a blue.
        pixel = cast("tuple[int, int, int]", image.getpixel((left - 1, top + height // 2)))
        return "#{:02x}{:02x}{:02x}".format(*pixel)

    framed = settles_on(desktop.values["accent"].lower(), beside)
    desktop.as_account(f"niri msg action move-window-to-tiling --id {bordered['id']}")
    assert framed == desktop.values["accent"].lower()


# niri opens a window in a column of its own, and the shell folds it
# in just below the focused one.


def test_a_terminal_opens_below_the_focused_window(desktop: Desktop) -> None:
    """A terminal opens below the focused window."""
    machine = desktop.machine
    anchor = desktop.focused_kitty()
    desktop.as_account("qs ipc call grid below kitty")
    kitties = 'niri msg --json windows | grep -o \'"app_id":"kitty"\' | wc -l'
    machine.wait_until_succeeds(desktop.in_session(f"{kitties} | grep -qx 2"), timeout=STARTING)

    def placed(_: bool) -> bool:
        """Say whether the new terminal sits just below the one focused before."""
        windows = json.loads(desktop.as_account("niri msg --json windows"))
        at = {w["id"]: w["layout"]["pos_in_scrolling_layout"] for w in windows}
        new = next(w["id"] for w in windows if w["id"] != anchor["id"])
        here, there = at[anchor["id"]], at[new]
        return bool(there[0] == here[0] and there[1] == here[1] + 1)

    retry(placed, timeout=dt.timedelta(seconds=15))


# Joel types the moment he presses Super+Space. Whatever letters come
# before the launcher has the keys go to the window beneath, and the
# rest to the launcher's field, but not one may be lost or reordered
# between the two. A terminal beneath keeps every raw key it gets, so
# a letter the launcher missed is found there.


def test_the_launcher_loses_no_letter_typed_straight_after_its_chord(desktop: Desktop) -> None:
    """The launcher loses no letter typed straight after its chord."""
    machine = desktop.machine
    desktop.as_account(
        "niri msg action spawn -- kitty --title type-ahead "
        "sh -c 'stty -icanon -echo; cat > /tmp/type-ahead'"
    )
    machine.wait_until_succeeds(
        desktop.in_session("niri msg focused-window | grep -q type-ahead"),
        timeout=STARTING,
    )
    for gap in (dt.timedelta(milliseconds=ms) for ms in (10, 50, 100)):
        beneath = len(desktop.as_account("cat /tmp/type-ahead"))
        machine.send_key("meta_l-spc")
        machine.send_chars("slack", delay=gap)
        desktop.launcher_ready()

        def none_lost(_: bool, beneath: int = beneath, gap: dt.timedelta = gap) -> bool:
            """Say whether every letter typed went beneath or to the launcher, in order."""
            went = desktop.as_account("cat /tmp/type-ahead")[beneath:]
            held = desktop.showing()["typed"]
            print(f"TYPEAHEAD {gap.total_seconds()}: '{went}' went beneath, '{held}' held")
            return bool(went + held == "slack")

        retry(none_lost, timeout=dt.timedelta(seconds=10))
        machine.send_key("esc")
        machine.wait_until_fails(
            desktop.in_session(LAUNCHER_OPEN), timeout=dt.timedelta(seconds=10)
        )
        machine.wait_until_succeeds(
            desktop.in_session("niri msg focused-window | grep -q type-ahead"),
            timeout=dt.timedelta(seconds=10),
        )
    desktop.as_account("pkill -f 'cat > /tmp/type-ahead'")
    machine.wait_until_fails(
        desktop.in_session("niri msg windows | grep -q type-ahead"),
        timeout=dt.timedelta(seconds=30),
    )


# The launcher finds a window by its title, and Return goes to it.


def test_the_launcher_switches_to_a_window_by_its_title(desktop: Desktop) -> None:
    """The launcher switches to a window by its title."""
    machine = desktop.machine
    desktop.as_account("niri msg action spawn -- kitty --title 'Notes for Tuesday'")
    machine.wait_until_succeeds(
        desktop.in_session("niri msg windows | grep -q 'Notes for Tuesday'"),
        timeout=STARTING,
    )
    desktop.as_account("niri msg action focus-window-previous")
    machine.wait_until_fails(
        desktop.in_session("niri msg focused-window | grep -q 'Notes for Tuesday'"),
        timeout=dt.timedelta(seconds=10),
    )
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("tuesday", selecting="Notes for Tuesday")
    desktop.photograph("launcher-window")
    machine.send_key("ret")
    machine.wait_until_succeeds(
        desktop.in_session("niri msg focused-window | grep -q 'Notes for Tuesday'"),
        timeout=dt.timedelta(seconds=10),
    )


# Super+Tab opens the launcher on every other window, the one used last
# first, so Return goes back to it, and a few letters find any other,
# among two dozen as easily as among two, with nothing held down. The
# windows are one kitty's, so the machine holds them all.


def test_super_tab_goes_back_to_the_window_before_and_finds_any_by_typing(desktop: Desktop) -> None:
    """Super+Tab goes back to the window before, and finds any by typing."""
    machine = desktop.machine
    desktop.notes_window()
    # The last one's title is far wider than the launcher, as a
    # browser tab's can be.
    scratch = [
        *[f"Scratch {n:02}" for n in range(1, 25)],
        (
            "Scratch 25 and a title that goes on well past the edge of the launcher"
            " and the screen, as the titles of web pages often do"
        ),
    ]
    for title in scratch:
        desktop.as_account(
            "niri msg action spawn -- kitty --single-instance"
            f" --instance-group scratch --title '{title}'"
        )
        machine.wait_until_succeeds(
            desktop.in_session(f"niri msg windows | grep -q '{title}'"),
            timeout=STARTING,
        )
    notes = desktop.window_titled("Notes for Tuesday")
    desktop.as_account(f"niri msg action focus-window --id {notes['id']}")
    machine.wait_until_succeeds(
        desktop.in_session("niri msg focused-window | grep -q 'Notes for Tuesday'"),
        timeout=dt.timedelta(seconds=10),
    )
    closed = desktop.in_session(LAUNCHER_OPEN)

    def tab_to(typed: str, title: str) -> None:
        """Open the windows with Super+Tab, type, and wait until the window titled is chosen."""
        machine.send_key("meta_l-tab")
        desktop.launcher_ready()
        desktop.launcher_until(lambda shown: shown["held"] == ["windows"])
        if typed:
            desktop.type_in(typed, selecting=title)
        desktop.launcher_until(
            lambda shown: shown["selected"] and shown["selected"]["name"] == title
        )

    def went(title: str) -> None:
        """Press Return, and wait until the launcher closes on the window titled."""
        machine.send_key("ret")
        machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
        machine.wait_until_succeeds(
            desktop.in_session(f"niri msg focused-window | grep -q '{title}'"),
            timeout=dt.timedelta(seconds=10),
        )

    # The window used last is chosen as it opens, and this one isn't
    # there to choose.
    tab_to("", scratch[-1])
    # Shift+Return opens one below the window Joel is in, a step in as
    # well as at the top, and the foot says so wherever it works.
    told = (desktop.showing()["selected"] or {}).get("told", [])
    assert "\u21e7\u21b5 below" in told, told
    desktop.photograph("launcher-windows")
    went("Scratch 25")
    tab_to("", "Notes for Tuesday")
    went("Notes for Tuesday")
    # A few letters find one among them all, and nothing looser.
    tab_to("scr 07", "Scratch 07")
    went("Scratch 07")
    # A program's name finds its windows, the one used last first.
    tab_to("kitty", "Notes for Tuesday")
    machine.send_key("esc")
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
    # Inside kitty, each of its windows holds what its own row among the
    # windows does (Joel, 2026-10-26: one thing, one set of insides,
    # everywhere). The one Joel is in, Scratch 07, isn't among them.
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("kitty", selecting="kitty")
    desktop.step_in("right")
    desktop.type_in("scratch 08", selecting="Scratch 08")
    told = (desktop.showing()["selected"] or {}).get("told", [])
    assert "\u2192 more" in told, told
    desktop.step_in("right")
    desktop.launcher_until(lambda shown: {"Size", "Float"} <= set(shown["rows"]))
    machine.send_key("esc")
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
    for title in scratch:
        desktop.as_account(
            f"niri msg action close-window --id {desktop.window_titled(title)['id']}"
        )
    machine.wait_until_fails(
        desktop.in_session("niri msg windows | grep -q Scratch"), timeout=dt.timedelta(seconds=30)
    )
    desktop.as_account(f"niri msg action focus-window --id {notes['id']}")


def test_super_tab_finds_a_pi_window_by_its_work_and_puts_one_waiting_next(
    desktop: Desktop,
) -> None:
    """Super+Tab finds a pi window by its work, and puts one waiting on Joel next."""
    machine = desktop.machine
    desktop.pi_session()
    pi = desktop.window_titled(PI_WINDOW)
    between = desktop.opened("Between the two")
    # Used in this order, so the pi window is the oldest of the three
    # Super+Tab lists, and Elsewhere is where Joel is. niri counts a window
    # used only once it has kept focus a while (its recent windows'
    # debounce, 750 ms), so each waits for its time of use to move.
    for window in [pi, between, desktop.notes_window(), desktop.window_titled("Elsewhere")]:
        before = desktop.window_titled(window["title"]).get("focus_timestamp")
        desktop.as_account(f"niri msg action focus-window --id {window['id']}")

        def used(_: bool, title: str = window["title"], was: object = before) -> bool:
            """Say whether niri has moved the window's time of use."""
            return bool(desktop.window_titled(title).get("focus_timestamp") != was)

        retry(used, timeout=dt.timedelta(seconds=10))
    closed = desktop.in_session(LAUNCHER_OPEN)

    def tabbed() -> None:
        """Open the windows with Super+Tab, and wait until they're listed."""
        machine.send_key("meta_l-tab")
        desktop.launcher_ready()
        desktop.launcher_until(lambda shown: shown["held"] == ["windows"])

    def shut() -> None:
        machine.send_key("esc")
        machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))

    # Its session's words, "Tidy the tests", find it, and so does its
    # quest's whole title, though its window says neither.
    for typed in ["tidy the", "whole"]:
        tabbed()
        desktop.type_in(typed)
        desktop.launcher_until(
            lambda shown: (shown["selected"] or {}).get("is") == f"window:{pi['id']}"
        )
        shut()
    # Waiting on Joel, it comes straight after the window used last, and
    # says so beside its name.
    question = {
        "kind": "question",
        "since": machine.succeed("date -u +%Y-%m-%dT%H:%M:%SZ").strip(),
        "said": "Which way?",
    }
    desktop.presence_says({"addresses": ["TESTQA"], "busy": False, "wants": question})
    tabbed()
    desktop.launcher_until(lambda shown: shown["selected"]["name"] == "Notes for Tuesday")
    machine.send_key("down")
    desktop.launcher_until(
        lambda shown: (
            shown["selected"]["is"] == f"window:{pi['id']}"
            and any(
                thing["is"] == f"window:{pi['id']}" and "waiting on you" in thing["reads"]
                for thing in shown["things"]
            )
        )
    )
    shut()
    desktop.presence_says({"addresses": ["TESTQA"], "busy": False})
    desktop.close(between)


# The right arrow steps into what can be done to a window, and a verb
# that needs a target asks for it in the same field.


def test_the_launcher_moves_a_window_and_copies_its_title(desktop: Desktop) -> None:
    """The launcher moves a window and copies its title."""
    machine = desktop.machine
    notes = desktop.notes_window()

    def into_notes(verb: str) -> None:
        """Find the notes' window in the launcher, step into it, and type a verb."""
        desktop.as_account("qs ipc call launcher toggle")
        desktop.launcher_ready()
        desktop.type_in("tuesday", selecting="Notes for Tuesday")
        desktop.step_in()
        desktop.type_in(verb)

    into_notes("copy")
    desktop.photograph("launcher-verbs")
    machine.send_key("ret")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=dt.timedelta(seconds=10))
    assert desktop.as_account("wl-paste --no-newline") == "Notes for Tuesday"

    into_notes("move")
    desktop.step_in("ret")
    desktop.photograph("launcher-target")
    machine.send_key("ret")

    def moved(_: bool) -> bool:
        """Say whether the notes' window is on another workspace now."""
        windows = json.loads(desktop.as_account("niri msg --json windows"))
        now = next(w for w in windows if w["id"] == notes["id"])
        return bool(now["workspace_id"] != notes["workspace_id"])

    retry(moved, timeout=dt.timedelta(seconds=10))


def test_the_window_joel_is_in_holds_what_can_be_done_to_it(desktop: Desktop) -> None:
    """The window Joel is in holds what can be done to it, as This window."""
    machine = desktop.machine
    notes = desktop.notes_window()
    desktop.as_account(f"niri msg action focus-window --id {notes['id']}")
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    # Named This window, the window's title beside it.
    desktop.type_in("this window", selecting="This Window")
    selected = desktop.showing()["selected"]
    assert selected["side"] == "Notes for Tuesday", selected
    # Return steps in, to what can be done to it, all but going to it,
    # since it's the one he's in. It steps in as the right arrow does, so
    # the foot says the two as one, by the one word for stepping in.
    assert selected["told"] == ["\u21b5 \u2192 more"], selected
    desktop.step_in("ret")
    desktop.launcher_until(lambda shown: "Close" in shown["rows"])
    rows = desktop.showing()["rows"]
    assert "Switch" not in rows, rows
    assert "Bring Here" not in rows, rows
    # Full width makes its column the screen's width, as niri says.
    before = desktop.window_titled("Notes for Tuesday")
    desktop.type_in("full", selecting="Full Width")
    machine.send_key("ret")
    desktop.launcher_until(lambda shown: not shown["open"])

    def widened(_: bool) -> bool:
        """Say whether the notes' window has grown wider."""
        now = desktop.window_titled("Notes for Tuesday")
        return bool(now["layout"]["window_size"][0] > before["layout"]["window_size"][0])

    retry(widened, timeout=dt.timedelta(seconds=10))
    # Size gives it a share of the screen, half of it narrower than all.
    wide = desktop.window_titled("Notes for Tuesday")
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("this window", selecting="This Window")
    desktop.step_in("ret")
    desktop.type_in("size", selecting="Size")
    # Its sizes are a step in by the right arrow too, and the foot says so.
    told = (desktop.showing()["selected"] or {}).get("told", [])
    assert "\u21b5 \u2192 more" in told, told
    desktop.step_in("ret")
    desktop.type_in("half", selecting="Half the Screen")
    machine.send_key("ret")
    desktop.launcher_until(lambda shown: not shown["open"])

    def halved(_: bool) -> bool:
        """Say whether the notes' window is narrower than it was at full width."""
        now = desktop.window_titled("Notes for Tuesday")
        return bool(now["layout"]["window_size"][0] < wide["layout"]["window_size"][0])

    retry(halved, timeout=dt.timedelta(seconds=10))


# Stepping back out of a collection finds the launcher as it was left,
# on the row that was selected, not back at the top.


def test_the_launcher_remembers_where_it_was_when_stepping_back(desktop: Desktop) -> None:
    """The launcher remembers where it was when stepping back."""
    machine = desktop.machine
    desktop.notes_window()
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()

    def selected() -> str:
        """Give the name of the launcher's selected row, or nothing."""
        return cast("str", (desktop.showing()["selected"] or {}).get("name", ""))

    # Down past the first row to Programs, a collection the VM always has
    # some of (Agents, above it, is empty here).
    while True:
        before = selected()
        machine.send_key("down")
        # The wait ends before the loop moves on, so it reads this pass's name.
        desktop.launcher_until(
            lambda shown: (shown["selected"] or {}).get("name", "") != before  # noqa: B023
        )
        if desktop.showing()["selected"]["is"] == "collection:applications":
            break
    chosen = selected()
    desktop.step_in()
    # Open programs come first, so kitty, whose notes are open, leads the
    # programs closed, however they're named.
    desktop.launcher_until(lambda shown: shown["rows"][:1] == ["kitty"])
    machine.send_key("down")
    machine.send_key("left")
    desktop.launcher_until(
        lambda shown: shown["held"] == [] and (shown["selected"] or {}).get("name") == chosen
    )
    machine.send_key("esc")
    desktop.launcher_until(lambda shown: not shown["open"])


def at_rotation(shown: Json) -> bool:
    """Say whether the launcher holds a screen, inside Desktop's Screens, its Rotation selected."""
    return bool(
        shown["open"]
        and shown["path"][:2] == ["Desktop", "Screens"]
        and len(shown["path"]) == len(["Desktop", "Screens", "a screen"])
        and (shown["selected"] or {}).get("name") == "Rotation"
    )


def test_a_detail_inside_a_thing_is_found_from_the_root_and_return_goes_there(
    desktop: Desktop,
) -> None:
    """A detail inside a thing is found from the root, and Return goes there."""
    machine = desktop.machine
    closed = desktop.in_session(LAUNCHER_OPEN)
    where = "Desktop \u203a Screens \u203a "

    def spot(shown: Json) -> int:
        """Give where the screen's Rotation is listed, saying where it lives, or -1."""
        for at, thing in enumerate(shown["things"]):
            if thing["reads"].startswith("Rotation ") and where in thing["reads"]:
                return at
        return -1

    def find_rotation() -> None:
        """Open the launcher on the screen's Rotation, found from the root."""
        desktop.answer("rotation")
        desktop.launcher_until(lambda shown: spot(shown) >= 0)
        for _ in range(spot(desktop.showing())):
            machine.send_key("down")
        desktop.launcher_until(
            lambda shown: (
                (shown["selected"] or {}).get("name") == "Rotation"
                and str((shown["selected"] or {}).get("side", "")).startswith(where)
            )
        )

    # A screen's rotation is a step inside the screen, inside Screens,
    # inside Desktop; from the root it's found by its name, saying so,
    # below the focused screen's answer. The right arrow goes there as
    # Return does, the foot saying the two as one (Joel, 2026-10-26: where
    # Return goes somewhere in the launcher, the right arrow does the same).
    find_rotation()
    told = desktop.showing()["selected"]["told"]
    assert "\u21b5 \u2192 go" in told, told
    machine.send_key("right")
    desktop.launcher_until(at_rotation)
    machine.send_key("esc")
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
    # Return goes there: the crumbs say each step, and the spot is selected.
    find_rotation()
    machine.send_key("ret")
    desktop.launcher_until(at_rotation)
    screen = desktop.showing()["path"][-1]
    # The left arrow goes back up, onto the screen it was inside.
    machine.send_key("left")
    desktop.launcher_until(
        lambda shown: (
            shown["path"] == ["Desktop", "Screens"]
            and (shown["selected"] or {}).get("name") == screen
        )
    )
    # And on out the way it came, each step with the row it went through
    # selected, to the search it was found by (Joel, 2026-10-26: every way
    # in climbs back out the same way).
    machine.send_key("left")
    desktop.launcher_until(
        lambda shown: (
            shown["path"] == ["Desktop"] and (shown["selected"] or {}).get("name") == "Screens"
        )
    )
    machine.send_key("left")
    desktop.launcher_until(
        lambda shown: (
            shown["held"] == []
            and shown["typed"] == "rotation"
            and (shown["selected"] or {}).get("name") == "Rotation"
            and str((shown["selected"] or {}).get("side", "")).startswith(where)
        )
    )
    machine.send_key("esc")
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
    # An answer's part is found the same way: the stand-in battery's
    # Health, inside Battery, inside Power, inside Machine.
    inside = "Machine \u203a Power \u203a Battery"

    def health(shown: Json) -> int:
        """Give where the battery's Health is listed, saying where it lives, or -1."""
        for at, thing in enumerate(shown["things"]):
            if thing["reads"].startswith("Health ") and inside in thing["reads"]:
                return at
        return -1

    desktop.answer("health")
    desktop.launcher_until(lambda shown: health(shown) >= 0)
    for _ in range(health(desktop.showing())):
        machine.send_key("down")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("side") == inside)
    machine.send_key("ret")
    desktop.launcher_until(
        lambda shown: (
            shown["path"] == ["Machine", "Power", "Battery"]
            and (shown["selected"] or {}).get("name") == "Health"
        )
    )
    machine.send_key("esc")
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))


@pytest.fixture
def google(desktop: Desktop) -> Iterator[None]:
    """Start the stand-in Google afresh for one test, and stop it after.

    Its meetings are timed from when it starts, so one left running would come to start during
    a later test, wanting Joel in Now and said as a notice.
    """
    desktop.as_account("systemctl --user restart google.service")
    yield
    desktop.as_account("systemctl --user stop google.service")


@pytest.mark.usefixtures("google")
def test_a_meeting_about_to_start_is_found_by_its_name_and_return_joins_it(
    desktop: Desktop,
) -> None:
    """A meeting about to start is found by its name, says when it is, and Return joins it."""
    machine = desktop.machine
    closed = desktop.in_session(LAUNCHER_OPEN)
    opened = "/run/user/1000/google-opened"
    machine.succeed(f"rm -f {opened}")
    # The stand-in's Design Review starts five minutes from now, on Meet.
    desktop.answer("design review")
    desktop.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("name") == "Design Review"
            and re.fullmatch(r"in [1-5] min", str(shown["selected"].get("side", "")))
        )
    )
    # Return joins its call: the Meet link opens in a window of its own,
    # and the launcher closes, since joining takes Joel out of it.
    machine.send_key("ret")
    machine.wait_until_succeeds(
        f"grep -qx https://meet.google.com/pqr-stuv-wxy {opened}",
        timeout=dt.timedelta(seconds=30),
    )
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))


@pytest.mark.usefixtures("google")
def test_todays_meeting_is_said_in_now(desktop: Desktop) -> None:
    """Now lists today's next meeting as a row of its own, joining one about to start."""
    machine = desktop.machine
    closed = desktop.in_session(LAUNCHER_OPEN)
    opened = "/run/user/1000/google-opened"
    machine.succeed(f"rm -f {opened}")
    # The next meeting is the stand-in's Design Review, five minutes on.
    desktop.answer("now")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("is") == "collection:on")
    desktop.step_in("ret")
    desktop.launcher_until(lambda shown: shown["held"] == ["now"] and shown["selected"])
    # The meeting is Now's last row, after the time, the date and the
    # weather, with no Today to step into.
    assert "Today" not in desktop.showing()["rows"], desktop.showing()["rows"]
    desktop.launcher_until(lambda shown: shown["rows"][-1:] == ["Design Review"])
    desktop.down_to("Design Review")
    today = desktop.showing()
    assert today["selected"]["verb"] == "join", today
    # Return joins it, as the meeting's own row does everywhere.
    machine.send_key("ret")
    machine.wait_until_succeeds(
        f"grep -qx https://meet.google.com/pqr-stuv-wxy {opened}",
        timeout=dt.timedelta(seconds=30),
    )
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))


@pytest.mark.usefixtures("google")
def test_a_meeting_further_off_is_shown_by_return_and_joined_from_its_acts(
    desktop: Desktop,
) -> None:
    """Return shows a meeting further off rather than joining it, guests too; its acts join it."""
    machine = desktop.machine
    closed = desktop.in_session(LAUNCHER_OPEN)
    opened = "/run/user/1000/google-opened"
    machine.succeed(f"rm -f {opened}")
    # The stand-in's Standup starts half an hour from now, on Meet.
    desktop.answer("standup")
    desktop.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("name") == "Standup"
            and re.fullmatch(r"in (29|30) min", str(shown["selected"].get("side", "")))
        )
    )
    # Return shows it, a step in named for it, its day and times first,
    # and the launcher stays.
    desktop.step_in("ret")
    desktop.launcher_until(
        lambda shown: (
            shown["open"]
            and shown["path"] == ["Standup"]
            and (shown["selected"] or {}).get("side") == "when"
        )
    )
    machine.fail(f"test -s {opened}")
    # Where it is and who holds it come before who else is asked, each by
    # name with how they answered.
    seen = desktop.showing()["rows"]
    named = ["Room 4", "https://meet.google.com/abc-defg-hij", "Ada Lovelace", "Grace Hopper"]
    assert [row for row in seen if row in named] == named, seen
    assert seen[-2] == "What we did yesterday, what we do today.", seen
    desktop.down_to("Grace Hopper")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("side") == "not going")
    # Back out, its acts lead with seeing it, Return's own, and then
    # joining, which goes into its call.
    machine.send_key("left")
    desktop.launcher_until(
        lambda shown: shown["path"] == [] and (shown["selected"] or {}).get("name") == "Standup"
    )
    desktop.step_in()
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "See It")
    desktop.down_to("Join the Meeting")
    machine.send_key("ret")
    machine.wait_until_succeeds(
        f"grep -qx https://meet.google.com/abc-defg-hij {opened}",
        timeout=dt.timedelta(seconds=30),
    )
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))


@pytest.mark.usefixtures("google")
def test_an_invitation_is_answered_from_its_acts_and_the_launcher_stays(
    desktop: Desktop,
) -> None:
    """An invitation is answered from a meeting's acts, its foot says the answer, and it stays."""
    machine = desktop.machine
    closed = desktop.in_session(LAUNCHER_OPEN)
    asked = "/run/user/1000/google-asked"
    machine.succeed(f"rm -f {asked}")
    # The stand-in's Standup, answered yes, which it hears answered again.
    desktop.answer("standup")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Standup")
    desktop.step_in()
    # Its acts: seeing it, Return's own, and joining first, then the three
    # answers.
    desktop.down_to("Answer Maybe")
    desktop.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("name") == "Answer Maybe" and shown["footer"] == "going"
        )
    )
    machine.send_key("ret")
    line = json.dumps({"answer": {"id": "standup", "with": "maybe"}}, separators=(",", ":"))
    machine.wait_until_succeeds(
        f"grep -qxF {shlex.quote(line)} {asked}", timeout=dt.timedelta(seconds=30)
    )
    # The launcher stays where it was, its foot saying the answer Google took.
    desktop.launcher_until(
        lambda shown: (
            shown["open"]
            and (shown["selected"] or {}).get("name") == "Answer Maybe"
            and shown["footer"] == "maybe"
        )
    )
    machine.send_key("esc")
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))


def now_rows(desktop: Desktop) -> Json:
    """Open the launcher on Now, and give what it shows once it holds Now's rows."""
    desktop.answer("now")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("is") == "collection:on")
    desktop.step_in("ret")
    desktop.launcher_until(lambda shown: shown["held"] == ["now"] and shown["selected"])
    return desktop.showing()


@pytest.mark.usefixtures("google")
def test_a_meetings_call_open_in_meet_is_in_now_named_for_it(desktop: Desktop) -> None:
    """A Meet call in a window of its own is in Now named for its meeting, and goes from it."""
    machine = desktop.machine
    closed = desktop.in_session(LAUNCHER_OPEN)
    # Standup's call, in the window Chrome would open its link in as an
    # app of its own, which kitty stands in for by taking its app id.
    call = "chrome-meet.google.com__abc-defg-hij-Default"
    desktop.as_account(f"niri msg action spawn -- kitty --class {call} --title Meet")
    machine.wait_until_succeeds(
        desktop.in_session(f'niri msg --json windows | grep -qF \'"app_id":"{call}"\''),
        timeout=STARTING,
    )
    # Another window takes the keys, so Return going to the call is seen.
    other = desktop.opened("Beside the call")
    now_rows(desktop)
    desktop.down_to("In a call")
    desktop.launcher_until(
        lambda shown: shown["selected"]["side"] == "Standup" and shown["selected"]["verb"] == "show"
    )
    machine.send_key("ret")
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
    machine.wait_until_succeeds(
        desktop.in_session(f'niri msg --json focused-window | grep -qF \'"app_id":"{call}"\''),
        timeout=dt.timedelta(seconds=10),
    )
    # Its window closed, the call is over, and Now no longer lists it.
    desktop.as_account("niri msg action close-window")
    desktop.close(other)
    machine.wait_until_fails(
        desktop.in_session(f'niri msg --json windows | grep -qF \'"app_id":"{call}"\''),
        timeout=dt.timedelta(seconds=10),
    )
    assert "In a call" not in now_rows(desktop)["rows"], desktop.showing()
    desktop.as_account("qs ipc call launcher toggle")
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))


@pytest.mark.usefixtures("google")
def test_now_lists_what_waits_then_a_call_a_song_and_a_mode_and_the_time_last(
    desktop: Desktop,
) -> None:
    """Now lists what waits on Joel, then a call, a song and a mode, and the time last."""
    queue = "$HOME/.local/state/mission/queue.json"
    kept(
        desktop,
        queue,
        {"orchestrator": "TESTQA", "items": [{"id": "look", "title": "Look", "when": "now"}]},
    )
    # Standup's call, in the window Chrome would open it in, as above.
    call = "chrome-meet.google.com__abc-defg-hij-Default"
    desktop.as_account(f"niri msg action spawn -- kitty --class {call} --title Meet")
    desktop.as_account("systemctl --user start caffeinate.service")
    desktop.as_account(
        "systemd-run --user --unit=player mpv --ao=null --no-video --loop"
        f" {shlex.quote(desktop.values['track'])}"
    )
    # Each by what it is, in the order Now lists its groups, the time and
    # the date, Now's own, last.
    groups = [
        "on:waitingOnYou",
        "on:call",
        "on:playing",
        "session:awake",
        "on:time",
        "on:date",
    ]

    def listed() -> list[str]:
        """Open Now, and give which of those it lists, in its order."""
        shown = now_rows(desktop)
        launcher_closed(desktop)
        return [thing["is"] for thing in shown["things"] if thing["is"] in groups]

    try:
        # Opened afresh each time, since rows that come while Now is
        # open go at its end until it's made again (Steady.js).
        retry(lambda _: listed() == groups, timeout=dt.timedelta(seconds=60))
    finally:
        desktop.as_account(f"systemctl --user stop player caffeinate.service; rm -f {queue}")
        desktop.close(desktop.window_titled("Meet"))


@pytest.mark.usefixtures("google")
def test_a_meeting_coming_wants_joel_and_is_said_at_its_reminder_and_as_it_starts(
    desktop: Desktop,
) -> None:
    """A meeting within the hour is in Now, and is said at its reminder and as it starts.

    Twenty minutes off it's in Now alone, counting down, unsaid; a minute off it's said, and
    again as it starts, the first notice taken back, and Return joins it. Its notice goes by
    itself, read, once its call is open.
    """
    machine = desktop.machine
    closed = desktop.in_session(LAUNCHER_OPEN)
    opened = "/run/user/1000/google-opened"
    machine.succeed(f"rm -f {opened}")
    call = "chrome-meet.google.com__pqr-stuv-wxy-Default"
    notices = "cat ~/.local/state/quickshell/by-shell/*/notices.json"
    try:
        # The stand-in's Design Review twenty minutes on: Now counts it
        # down, and the margin leaves its mark out until ten minutes off.
        desktop.as_account("systemctl --user set-environment GOOGLE_STAND_IN_SOON=1200000")
        desktop.as_account("systemctl --user restart google.service")
        now_rows(desktop)
        desktop.launcher_until(lambda shown: "Design Review" in shown["rows"])
        desktop.down_to("Design Review")
        desktop.launcher_until(
            lambda shown: re.fullmatch(r"in (19|20) min", shown["selected"]["side"]) is not None
        )
        margin = desktop.as_account(
            "journalctl --user --unit=quickshell --no-pager --output=cat"
            " | grep '^holds margin' | tail -n 1"
        )
        assert "meeting" not in margin.split(), margin
        machine.fail(desktop.in_session("niri msg layers | grep -q notifications"))
        desktop.as_account("qs ipc call launcher toggle")
        machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
        # A minute and a half on: said at its reminder, a minute before,
        # once the shell's minute passes it, and again as it starts.
        desktop.as_account("systemctl --user set-environment GOOGLE_STAND_IN_SOON=90000")
        with desktop.told("arrives Design Review", timeout=dt.timedelta(seconds=120)):
            desktop.as_account("systemctl --user restart google.service")
        with desktop.told("arrives Design Review", timeout=dt.timedelta(seconds=120)):
            pass
        # The reminder's notice was taken back, read, as the start's came.
        machine.wait_until_succeeds(
            desktop.in_session(
                f'{notices} | grep -q \'"summary":"Design Review in 1 min"[^}}]*"read":true\''
            ),
            timeout=dt.timedelta(seconds=10),
        )
        now_rows(desktop)
        desktop.down_to("Design Review")
        desktop.launcher_until(
            lambda shown: shown["selected"]["side"] == "now" and shown["selected"]["verb"] == "join"
        )
        machine.send_key("ret")
        machine.wait_until_succeeds(
            f"grep -qx https://meet.google.com/pqr-stuv-wxy {opened}",
            timeout=dt.timedelta(seconds=30),
        )
        machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
        # The call opens in a window of its own, as Chrome would open the
        # link, which kitty stands in for by taking its app id; the meeting
        # no longer wants Joel, so its notice goes, read, untouched.
        desktop.as_account(f"niri msg action spawn -- kitty --class {call} --title Meet")
        machine.wait_until_fails(
            desktop.in_session("niri msg layers | grep -q notifications"),
            timeout=dt.timedelta(seconds=30),
        )
        machine.wait_until_succeeds(
            desktop.in_session(
                f'{notices} | grep -q \'"summary":"Design Review now"[^}}]*"read":true\''
            ),
            timeout=dt.timedelta(seconds=10),
        )
        desktop.close(desktop.window_titled("Meet"))
    finally:
        desktop.as_account("systemctl --user unset-environment GOOGLE_STAND_IN_SOON")


@pytest.mark.usefixtures("google")
def test_a_signed_out_account_wants_joel_to_sign_in_again(desktop: Desktop) -> None:
    """Google signed out wants Joel in Now, unsaid as a notice, and Return signs in in pi."""
    machine = desktop.machine
    closed = desktop.in_session(LAUNCHER_OPEN)
    opened = "$XDG_RUNTIME_DIR/pi-opened"
    desktop.as_account(f"rm -f {opened}")
    desktop.as_account("systemctl --user set-environment GOOGLE_STAND_IN_ACCOUNT=signedOut")
    try:
        desktop.as_account("systemctl --user restart google.service")
        now_rows(desktop)
        desktop.launcher_until(lambda shown: "Google" in shown["rows"])
        assert "Design Review" not in desktop.showing()["rows"], desktop.showing()
        desktop.down_to("Google")
        desktop.launcher_until(
            lambda shown: (
                shown["selected"]["side"] == "signed out" and shown["selected"]["verb"] == "sign in"
            )
        )
        machine.send_key("ret")
        machine.wait_until_succeeds(
            desktop.in_session(f"grep -qx /google-auth {opened}"),
            timeout=dt.timedelta(seconds=30),
        )
        desktop.as_account(f'grep -qx "$HOME " {opened}')
        machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
        machine.fail(desktop.in_session("niri msg layers | grep -q notifications"))
        desktop.as_account(f"rm -f {opened}")
    finally:
        desktop.as_account("systemctl --user unset-environment GOOGLE_STAND_IN_ACCOUNT")


@pytest.mark.usefixtures("google")
def test_the_mail_that_wants_joel_is_counted_in_now_and_return_lists_it_as_mail_does(
    desktop: Desktop,
) -> None:
    """The mail that wants Joel is counted in Now, and Return lists it as Mail does.

    Each piece is the same row as in Programs \u203a Mail, with its details beside it, and Show
    the Mail after them goes there.
    """
    machine = desktop.machine
    closed = desktop.in_session(LAUNCHER_OPEN)
    lunch = {
        "title": "Lunch on Friday",
        "facts": [
            {"label": "From", "value": "Ada Lovelace <ada@example.com>"},
            {"label": "To", "value": "Joel Gerber"},
        ],
        "body": "Shall we try the place on the corner?\n\nAda",
    }

    def lunch_in_full(shown: dict[str, Any]) -> bool:
        """Whether Lunch on Friday is selected, its row and details as Mail gives them."""
        selected = shown["selected"] or {}
        drawer = selected.get("drawer") or {}
        return (
            selected.get("is") == "mail:lunch"
            and re.fullmatch(r"Ada Lovelace \u00b7 \d\d:\d\d", selected["side"]) is not None
            and drawer.get("title") == lunch["title"]
            and drawer.get("facts", [])[:2] == lunch["facts"]
            and drawer.get("facts", [{}] * 3)[2].get("label") == "Date"
            and drawer.get("body") == lunch["body"]
        )

    now_rows(desktop)
    desktop.launcher_until(lambda shown: "Mail" in shown["rows"])
    desktop.down_to("Mail")
    desktop.launcher_until(
        lambda shown: (
            shown["selected"]["side"] == "2 to read" and shown["selected"]["verb"] == "show"
        )
    )
    machine.send_key("ret")
    desktop.launcher_until(
        lambda shown: (
            shown["held"][-1:] == ["Mail"]
            and shown["rows"] == ["Lunch on Friday", "Quarterly Numbers", "Show the Mail"]
        )
    )
    desktop.launcher_until(lunch_in_full)
    desktop.as_account("qs ipc call launcher toggle")
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
    # Mail's own place lists the very same row.
    desktop.answer("lunch on friday")
    desktop.launcher_until(lunch_in_full)
    desktop.step_in()
    desktop.launcher_until(
        lambda shown: shown["rows"] == ["Open in Mail", "Reply", "Archive", "Mark As Read"]
    )
    desktop.as_account("qs ipc call launcher toggle")
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
    # Show the Mail, after the mail, goes to Mail in Programs.
    now_rows(desktop)
    desktop.down_to("Mail")
    machine.send_key("ret")
    desktop.launcher_until(lambda shown: "Show the Mail" in shown["rows"])
    desktop.down_to("Show the Mail")
    machine.send_key("ret")
    desktop.launcher_until(
        lambda shown: (
            shown["held"][-1:] == ["Mail"]
            and "Lunch on Friday" in shown["rows"]
            and "Show the Mail" not in shown["rows"]
        )
    )
    desktop.as_account("qs ipc call launcher toggle")
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))


@pytest.mark.usefixtures("google")
def test_a_reply_to_mail_is_written_in_mail_to_who_sent_it(desktop: Desktop) -> None:
    """Reply writes a new message in Mail to who sent the mail, its subject marked a reply."""
    machine = desktop.machine
    closed = desktop.in_session(LAUNCHER_OPEN)
    opened = "/run/user/1000/google-opened"
    machine.succeed(f"rm -f {opened}")
    desktop.answer("lunch on friday")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("is") == "mail:lunch")
    desktop.step_in()
    desktop.down_to("Reply")
    machine.send_key("ret")
    link = (
        "https://mail.google.com/mail/u/0/?view=cm&fs=1"
        "&to=ada%40example.com&su=Re%3A%20Lunch%20on%20Friday"
    )
    machine.wait_until_succeeds(
        f"grep -qxF {shlex.quote(link)} {opened}", timeout=dt.timedelta(seconds=30)
    )
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))


@pytest.mark.usefixtures("google")
def test_a_meeting_joel_holds_is_cancelled_once_he_is_sure(desktop: Desktop) -> None:
    """A meeting Joel holds is cancelled from its acts, a step saying who's told asking first."""
    machine = desktop.machine
    closed = desktop.in_session(LAUNCHER_OPEN)
    asked = "/run/user/1000/google-asked"
    machine.succeed(f"rm -f {asked}")
    # The stand-in's Planning, two hours on, is Joel's own, with Grace asked.
    desktop.answer("planning")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Planning")
    desktop.step_in()
    desktop.down_to("Cancel the Meeting")
    # Return asks first, saying Google tells the one guest, and cancels nothing yet.
    desktop.step_in("ret")
    desktop.launcher_until(
        lambda shown: (shown["selected"] or {}).get("name") == "Cancel It Now, Telling 1 Guest"
    )
    machine.fail(f"test -s {asked}")
    machine.send_key("ret")
    line = json.dumps({"cancel": {"id": "planning"}}, separators=(",", ":"))
    machine.wait_until_succeeds(
        f"grep -qxF {shlex.quote(line)} {asked}", timeout=dt.timedelta(seconds=30)
    )
    # The launcher stays, and Google lets the meeting go, so it's found no more.
    assert desktop.showing()["open"]
    machine.wait_until_fails(
        'grep -qF \'"id": "planning"\' /run/user/1000/google.json',
        timeout=dt.timedelta(seconds=10),
    )
    desktop.as_account("qs ipc call launcher toggle")
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
    desktop.answer("planning")
    assert not any(
        thing["is"] == "calendar:meeting:planning" for thing in desktop.showing()["things"]
    ), desktop.showing()
    desktop.as_account("qs ipc call launcher toggle")
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))


@pytest.mark.usefixtures("google")
def test_a_meeting_google_lets_joel_change_is_moved_to_a_typed_time_once_he_is_sure(
    desktop: Desktop,
) -> None:
    """A meeting Google lets Joel change is moved to a time he types, a step naming who's told."""
    machine = desktop.machine
    closed = desktop.in_session(LAUNCHER_OPEN)
    asked = "/run/user/1000/google-asked"
    machine.succeed(f"rm -f {asked}")
    # The stand-in's Standup is Ada's, on her calendar, which Joel may change.
    desktop.answer("standup")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Standup")
    desktop.step_in()
    desktop.down_to("Move the Meeting")
    desktop.step_in("ret")
    desktop.launcher_until(lambda shown: "15 Minutes Later" in shown["rows"])
    # A time typed there is offered first, named by its day and times.
    day = machine.succeed("date -d tomorrow '+%A %-d %B'").strip()
    when = f"{day}, 10:00 to 10:30"
    desktop.type_in("tomorrow 10:00", selecting=when)
    # Return asks first, naming who Google tells, and moves nothing yet.
    desktop.step_in("ret")
    desktop.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("name")
            == f"Move It to {when}, Telling Ada Lovelace and Grace Hopper"
        )
    )
    machine.fail(f"test -s {asked}")
    machine.send_key("ret")
    moved = '{"move":{"id":"standup","start":'
    machine.wait_until_succeeds(
        f"grep -qF {shlex.quote(moved)} {asked}", timeout=dt.timedelta(seconds=30)
    )
    # The launcher stays, as it does once a meeting is cancelled.
    assert desktop.showing()["open"]
    desktop.as_account("qs ipc call launcher toggle")
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))


@pytest.mark.usefixtures("google")
def test_mail_that_wants_joel_is_found_by_its_subject_and_return_opens_it(
    desktop: Desktop,
) -> None:
    """Mail that wants Joel is found by its subject, says when it came, and Return opens it."""
    machine = desktop.machine
    closed = desktop.in_session(LAUNCHER_OPEN)
    opened = "/run/user/1000/google-opened"
    machine.succeed(f"rm -f {opened}")
    # The stand-in's Lunch on Friday came two minutes before it started.
    desktop.answer("lunch on friday")
    desktop.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("name") == "Lunch on Friday"
            and re.fullmatch(
                r"Ada Lovelace \u00b7 (\d\d:\d\d|yesterday)",
                str(shown["selected"].get("side", "")),
            )
        )
    )
    # Return opens it in Mail, in a window of its own, and the launcher
    # closes, since reading it takes Joel out of it.
    machine.send_key("ret")
    machine.wait_until_succeeds(
        f"grep -qxF 'https://mail.google.com/mail/u/0/#inbox/lunch' {opened}",
        timeout=dt.timedelta(seconds=30),
    )
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))


@pytest.mark.usefixtures("google")
def test_mail_opened_while_mail_is_open_leaves_one_mail_window(desktop: Desktop) -> None:
    """Mail opened while Mail is open shows in a window beside it, and the old one closes."""
    machine = desktop.machine
    opened = "/run/user/1000/google-opened"
    machine.succeed(f"rm -f {opened}")
    # Mail's own window, and the one Chrome opens a piece of mail in, each
    # named from its link, which kitty stands in for by taking its app id.
    inbox = "chrome-mail.google.com__-Default"
    message = "chrome-mail.google.com__mail_u_0_-Default"
    listed = 'niri msg --json windows | grep -qF \'"app_id":"{}"\''
    focused = 'niri msg --json focused-window | grep -qF \'"app_id":"{}"\''
    desktop.as_account(f"niri msg action spawn -- kitty --class {inbox} --title Gmail")
    machine.wait_until_succeeds(desktop.in_session(listed.format(inbox)), timeout=STARTING)
    # Another window takes the keys, so Mail's coming forward is seen.
    other = desktop.opened("Beside Mail")
    try:
        desktop.answer("lunch on friday")
        desktop.launcher_until(
            lambda shown: (shown["selected"] or {}).get("name") == "Lunch on Friday"
        )
        machine.send_key("ret")
        machine.wait_until_succeeds(
            f"grep -qxF 'https://mail.google.com/mail/u/0/#inbox/lunch' {opened}",
            timeout=dt.timedelta(seconds=30),
        )
        machine.wait_until_succeeds(
            desktop.in_session(focused.format(inbox)), timeout=dt.timedelta(seconds=10)
        )
        # Its window opens, as Chrome's would, and Mail's old one goes,
        # leaving one Mail window, at the mail.
        desktop.as_account(f"niri msg action spawn -- kitty --class {message} --title Lunch")
        machine.wait_until_fails(
            desktop.in_session(listed.format(inbox)), timeout=dt.timedelta(seconds=10)
        )
        machine.succeed(desktop.in_session(listed.format(message)))
    finally:
        desktop.close(other)
        for window in desktop.windows():
            if window["app_id"] in {inbox, message}:
                desktop.close(window)


def tidy_from_its_acts(desktop: Desktop, subject: str, act: str, became: str) -> None:
    """Find mail by its subject, do one of its acts, and see it said, the launcher kept."""
    machine = desktop.machine
    desktop.answer(subject.lower())
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == subject)
    desktop.step_in()
    # Its acts: opening it, replying, archiving it and marking it read.
    for _ in range(4):
        if (desktop.showing()["selected"] or {}).get("name") == act:
            break
        before = (desktop.showing()["selected"] or {}).get("name")
        machine.send_key("down")
        desktop.launcher_until(
            lambda shown: (shown["selected"] or {}).get("name") != before  # noqa: B023
        )
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == act)
    machine.send_key("ret")
    # The launcher stays where it was, its foot saying what became of it
    # once the program no longer holds it as wanting Joel.
    desktop.launcher_until(
        lambda shown: (
            shown["open"]
            and (shown["selected"] or {}).get("name") == act
            and shown["footer"] == became
        )
    )


@pytest.mark.usefixtures("google")
def test_mail_is_marked_read_or_archived_from_its_acts_and_the_launcher_stays(
    desktop: Desktop,
) -> None:
    """Mail is marked read or archived from its acts, its foot says so, and the launcher stays."""
    machine = desktop.machine
    closed = desktop.in_session(LAUNCHER_OPEN)
    asked = "/run/user/1000/google-asked"
    machine.succeed(f"rm -f {asked}")
    for subject, act, became, line in [
        ("Lunch on Friday", "Mark As Read", "read", {"read": {"id": "lunch"}}),
        ("Quarterly Numbers", "Archive", "archived", {"archive": {"id": "numbers"}}),
    ]:
        tidy_from_its_acts(desktop, subject, act, became)
        said = json.dumps(line, separators=(",", ":"))
        machine.succeed(f"grep -qxF {shlex.quote(said)} {asked}")
        machine.send_key("esc")
        machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))


def test_machine_holds_power_which_holds_the_battery_and_what_ends_a_session(
    desktop: Desktop,
) -> None:
    """Machine holds Power, which holds the battery, its profile and what ends a session."""
    machine = desktop.machine
    closed = desktop.in_session(LAUNCHER_OPEN)
    power = [
        "Battery", "Power Profile", "Keep Awake", "Sleep", "Lock", "Log Out", "Reboot",
        "Power Off",
    ]  # fmt: skip
    desktop.answer("machine")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Machine")
    desktop.step_in("ret")
    # Power comes first, saying what it holds, and Memory is a row of its
    # own, before Heat, as the processor is, Disk after it and Lag last,
    # since what it points at is the processor, the memory and the disk.
    # The queue is Agents', so Machine keeps only the machine's own.
    desktop.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("name") == "Power"
            and "Queue" not in shown["rows"]
            and any(
                str(thing["reads"]).startswith("Power \u00b7 Battery, Power Profile")
                for thing in shown["things"]
            )
            and "Memory" in shown["rows"]
            and shown["rows"][shown["rows"].index("Memory") + 1 :][:3] == ["Heat", "Disk", "Lag"]
        )
    )
    desktop.step_in("ret")
    desktop.launcher_until(
        lambda shown: shown["path"] == ["Machine", "Power"] and shown["rows"][:8] == power
    )
    machine.send_key("esc")
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
    # Found by its name from the top, it opens where it lives, and the
    # left arrow climbs out through Machine.
    desktop.answer("power")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Power")
    desktop.step_in("ret")
    desktop.launcher_until(
        lambda shown: shown["path"] == ["Machine", "Power"] and shown["rows"][:8] == power
    )
    machine.send_key("left")
    desktop.launcher_until(
        lambda shown: (
            shown["path"] == ["Machine"] and (shown["selected"] or {}).get("name") == "Power"
        )
    )
    # A name going on from the word and a space is searched for where it's
    # typed, so "power off" never steps into Power under the keys.
    machine.send_key("esc")
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
    desktop.answer("power off")
    desktop.launcher_until(
        lambda shown: shown["path"] == [] and (shown["selected"] or {}).get("name") == "Power Off"
    )
    machine.send_key("esc")
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))


def test_the_launchers_catalogue_opens_places_does_actions_and_offers_what_ends_something(
    desktop: Desktop,
) -> None:
    """The launcher's catalogue opens places, does actions and offers what ends something."""
    machine = desktop.machine
    closed = desktop.in_session(LAUNCHER_OPEN)
    held = "systemd-inhibit --list --no-pager | grep Caffeinate"
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))

    def listed() -> list[Json]:
        return cast("list[Json]", json.loads(desktop.as_account("qs ipc call launcher list")))

    def ran(entry: Json) -> Json:
        said = desktop.as_account(f"qs ipc call launcher run {shlex.quote(str(entry['id']))}")
        return cast("Json", json.loads(said))

    def one(entries: list[Json], words: str, within: str) -> Json:
        found = [e for e in entries if e["words"] == words and e["in"] == within]
        assert len(found) == 1, f"{words} in {within}: {found}"
        return found[0]

    entries = listed()
    ids = [entry["id"] for entry in entries]
    assert len(ids) == len(set(ids)), "an id is listed twice"
    # Programs lists each program, but not each one's own Quit.
    kitty = one(entries, "kitty", "Programs")
    assert kitty["kind"] == "action"
    assert not [e for e in entries if e["in"].startswith("Programs \u203a ")]
    # Windows lists each window but not each one's own Size, and a
    # keyboard's keys, which only say what they do, aren't listed.
    assert not [e for e in entries if e["in"].startswith("Desktop \u203a Windows \u203a ")]
    assert not [e for e in entries if "keyboard:key:" in e["id"]]
    # A collection's own parts are listed, as a step into it shows them:
    # Agents' Orchestrators, and inside it each orchestrator.
    lighthouse_listed(desktop)
    machine.wait_until_succeeds(
        desktop.in_session("qs ipc call launcher list | grep -q Lighthouse"),
        timeout=dt.timedelta(seconds=10),
    )
    lighthouse = one(listed(), "Lighthouse", "Agents \u203a Orchestrators")
    assert ran(lighthouse)["done"]
    desktop.launcher_until(
        lambda shown: shown["open"] and shown["path"] == ["Agents", "Orchestrators", "Lighthouse"]
    )
    machine.send_key("esc")
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
    # A place opens there.
    assert ran(one(entries, "Power", "Machine")) == {
        "done": True,
        "said": "opened Machine \u203a Power",
    }
    desktop.launcher_until(lambda shown: shown["open"] and shown["path"] == ["Machine", "Power"])
    machine.send_key("esc")
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
    # One that ends something is shown ready, never done.
    reboot = one(entries, "Reboot", "Machine \u203a Power")
    assert reboot["disruptive"]
    assert ran(reboot)["offered"]
    desktop.launcher_until(
        lambda shown: (
            shown["path"] == ["Machine", "Power"]
            and (shown["selected"] or {}).get("name") == "Reboot"
        )
    )
    machine.send_key("esc")
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
    # A harmless one is done at once, and once it ends something, offered.
    awake = one(entries, "Keep Awake", "Machine \u203a Power")
    assert not awake["disruptive"]
    assert ran(awake) == {"done": True, "said": "Machine \u203a Power \u203a Keep Awake"}
    machine.wait_until_succeeds(f"{held} | grep -qw idle", timeout=dt.timedelta(seconds=10))
    sleep = one(listed(), "Let It Sleep", "Machine \u203a Power")
    assert sleep["id"] == awake["id"], "its id changed with its state"
    assert ran(sleep)["offered"]
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Let It Sleep")
    machine.send_key("ret")
    machine.wait_until_fails(held, timeout=dt.timedelta(seconds=10))
    if machine.execute(closed)[0] == 0:
        machine.send_key("esc")
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
    # One not there says so.
    assert "error" in ran({"id": "nothing:here"})


def test_escape_closes_only_the_launcher_and_never_reaches_the_window_behind(
    desktop: Desktop,
) -> None:
    """Escape closes only the launcher, and the window behind it never hears the key."""
    # Joel, 2026-10-26: "Escape closes only the launcher; the key never
    # reaches the program behind it." wev says every key its window hears,
    # held as it gets the keyboard or pressed and let go after. The launcher
    # goes on Escape's press, so the keyboard may come back to the window
    # with the key still down: it's then told the key is held, and told of
    # its release, which no program acts on. Only a press would reach it.
    machine = desktop.machine
    values = desktop.values
    log = "/run/user/1000/wev-escape.log"
    desktop.as_account(
        "systemd-run --user --collect --quiet --unit=wev-escape -- sh -c "
        f"'{values['coreutils']}/bin/stdbuf -oL {values['wev']}/bin/wev > {log} 2>&1'"
    )
    retry(lambda _: any(w["title"] == "wev" for w in desktop.windows()), timeout=STARTING)
    wev = next(w for w in desktop.windows() if w["title"] == "wev")
    desktop.as_account(f"niri msg action focus-window --id {wev['id']}")

    def entered() -> int:
        """Give how many times wev's window has had the keyboard."""
        return int(machine.succeed(f"grep -c 'wl_keyboard] enter' {log} || true").strip())

    retry(lambda _: entered() >= 1, timeout=STARTING)
    before = entered()
    try:
        # Opened as Joel opens it, by its chord, so its keys are his too.
        machine.send_key("meta_l-spc")
        desktop.launcher_ready()
        machine.send_key("esc")
        machine.wait_until_fails(
            desktop.in_session(LAUNCHER_OPEN), timeout=dt.timedelta(seconds=10)
        )
        # The keyboard is wev's again, and a key pressed after reaches it,
        # so all it was given as the launcher went is in its log.
        retry(lambda _: entered() > before, timeout=dt.timedelta(seconds=10))
        machine.send_key("a")
        retry(
            lambda _: "sym: a " in machine.succeed(f"cat {log}"),
            timeout=dt.timedelta(seconds=10),
        )
        heard = machine.succeed(f"cat {log}")
        assert not re.search(r"state: 1 \(pressed\)\s*\n\s*sym: Escape", heard), heard
    finally:
        desktop.as_account("systemctl --user stop wev-escape")


def test_every_row_has_a_return_act(desktop: Desktop) -> None:
    """Every row the launcher shows does something on Return."""
    # Joel, 2026-10-26: a row that holds more steps in, a row that only
    # shows a value copies it, and a row that can do neither doesn't
    # belong in the launcher. The launcher walks every row from home down.
    said = cast("Json", json.loads(desktop.as_account("qs ipc call launcher actless")))
    actless = cast("list[Json]", said["rows"])
    print(f"walked {said['walked']} rows")
    # Home, its places, what each holds and deeper, which in the VM is a
    # few hundred rows: a walk that stopped at home would find nothing wrong.
    fewest = 100
    assert said["walked"] > fewest, said["walked"]
    assert actless == [], "\n".join(f"{row['in']} \u203a {row['words']}" for row in actless)


def test_the_launchers_home_lists_the_categories_to_browse_each_saying_what_it_holds(
    desktop: Desktop,
) -> None:
    """The launcher's home lists the categories to browse, each saying what it holds."""
    machine = desktop.machine
    # Desktop holds This window while one is focused, so one is, whatever
    # ran before.
    desktop.focused_kitty()
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    # The same rows every time: Now first, then each place in the order
    # the approved proposal gives them, Disk inside Machine (Joel, 2026-10-26).
    home = ["Now", "Agents", "Programs", "Desktop", "Machine", "Network", "Keyboard"]
    desktop.launcher_until(lambda shown: shown["rows"] == home)

    def selected_is() -> str:
        """Give what the launcher's selected row is, or nothing."""
        return cast("str", (desktop.showing()["selected"] or {}).get("is", ""))

    def foot() -> tuple[int, int]:
        """Give where the foot's keys start, and how long how things stand reads."""
        shown = desktop.showing()
        return shown["keysAt"], len(shown["glance"])

    # Down through every row to the last. The foot holds still on the
    # way, whatever each row's keys say: the keys in their places, and
    # how things stand beside them as long (a minute turning keeps its
    # length).
    feet = {selected_is(): foot()}
    for _row in home[1:]:
        before = selected_is()
        machine.send_key("down")
        # The wait ends before the loop moves on, so it reads this pass's row.
        desktop.launcher_until(
            lambda shown: (shown["selected"] or {}).get("is", "") != before  # noqa: B023
        )
        feet[selected_is()] = foot()
    assert selected_is() == "collection:keyboard"
    assert len(set(feet.values())) == 1, f"the foot moved as the arrows went: {feet}"
    # Each says what it holds after its name, in its own order, from the
    # vocabulary; Now says what's on, or that nothing is.
    reads = {thing["is"]: thing["reads"] for thing in desktop.showing()["things"]}
    assert reads["category:desktop"].startswith("Desktop \u00b7 This Window, Windows, Screens, "), (
        f"desktop reads {reads['category:desktop']!r}"
    )
    assert reads["category:computer"].startswith("Machine \u00b7 Power, Bluetooth, "), (
        f"machine reads {reads['category:computer']!r}"
    )
    assert reads["collection:on"].startswith("Now \u00b7 "), f"now reads {reads['collection:on']!r}"
    assert reads["collection:applications"].startswith("Programs \u00b7 "), reads
    # Home holds still while the machine's queue changes, which a busy
    # machine writes all the time: it once made home again and moved it
    # from under the selection. Nothing of the machine's shows at home,
    # so the queue asks nothing again there at all.
    scrolled = desktop.showing()
    since = desktop.mark()
    queued = {"v": 1, "heartbeat": float(machine.succeed("date +%s")), "machine": {}, "jobs": []}
    desktop.as_account(
        f"echo '{json.dumps(queued)}' > $XDG_RUNTIME_DIR/jobs/.q"
        " && mv $XDG_RUNTIME_DIR/jobs/.q $XDG_RUNTIME_DIR/jobs/queue.json"
    )
    still = desktop.showing()
    assert (still["scrolled"], still["selected"]) == (scrolled["scrolled"], scrolled["selected"]), (
        f"home moved as the queue changed: from {scrolled} to {still}"
    )
    desktop.photograph("launcher-home-categories")
    machine.send_key("esc")
    desktop.launcher_until(lambda shown: not shown["open"])
    said = desktop.as_account(
        f"journalctl --user --unit=quickshell --no-pager --output=cat --after-cursor='{since}'"
    )
    assert "asked again by jobs" not in said, f"the queue asked home again: {said}"
    # Tab, with nothing typed, does nothing and says nothing, since the
    # arrows reach every category; Return on one steps into it. The foot
    # names no Tab either.
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    first = desktop.showing()["selected"]
    assert not any(key.startswith("\u21e5") for key in first["told"]), first
    machine.send_key("tab")
    machine.send_key("down")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("is") == "collection:agents")
    # Had Tab moved to the first place, the arrow would be past it.
    assert first["is"] == "collection:on", first
    for _step in range(2):
        machine.send_key("down")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("is") == "category:desktop")
    desktop.step_in("ret")
    desktop.launcher_until(
        lambda shown: (
            shown["held"] == ["desktop"]
            # This window is there only while one is focused.
            and [row for row in shown["rows"] if row != "This Window"][:2] == ["Windows", "Screens"]
        )
    )
    machine.send_key("esc")
    desktop.launcher_until(lambda shown: not shown["open"])


def test_the_launchers_agents_list_the_sessions_from_the_moment_the_shell_starts(
    desktop: Desktop,
) -> None:
    """The launcher's Agents lists the sessions from the moment the shell starts."""
    machine = desktop.machine
    desktop.pi_session()

    def sessions() -> list[str]:
        """Say what Agents lists of the sessions, as the launcher's catalogue has it."""
        listed = json.loads(machine.succeed(desktop.in_session("qs ipc call launcher list")))
        return [
            entry["says"]
            for entry in listed
            if entry["in"] == "Agents" and entry["kind"] == "action" and "says" in entry
        ]

    retry(lambda _: any("TESTQA" in said for said in sessions()), timeout=dt.timedelta(seconds=30))
    # A switch restarts the shell; the first list it gives already holds
    # the session, never an empty Agents while it reads the folder.
    desktop.as_account("systemctl --user restart quickshell")
    machine.wait_until_succeeds(
        desktop.in_session("qs ipc call launcher list"), timeout=dt.timedelta(seconds=60)
    )
    first = sessions()
    assert any("TESTQA" in said for said in first), first


def test_the_launchers_home_counts_the_sessions_whose_names_are_too_long_to_list(
    desktop: Desktop,
) -> None:
    """The launcher's home counts the sessions whose names are too long to list."""
    machine = desktop.machine
    desktop.pi_session()
    # A session whose quest's name is longer than home's row has room for,
    # as Joel's are, in the window used last, so it's listed first.
    long = "\u03c0 - Bootstrap the Lighthouse to Replace Every Other Machine on the Desk - tree"
    desktop.pi_window("TESTQC", long)
    desktop.presence_says({"addresses": ["TESTQC"], "busy": True, "title": long}, "TESTQC")
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.launcher_until(
        lambda shown: any(
            thing["is"] == "collection:agents"
            and re.match(r"Agents \u00b7 \d+ running", thing["reads"])
            for thing in shown["things"]
        )
    )
    reads = {thing["is"]: thing["reads"] for thing in desktop.showing()["things"]}
    assert "No sessions" not in reads["collection:agents"], reads
    machine.send_key("esc")
    desktop.launcher_until(lambda shown: not shown["open"])
    desktop.as_account(f"niri msg action close-window --id {desktop.window_titled(long)['id']}")
    desktop.as_account(f"rm -f {PI_PRESENCE}/testqc.json")


def test_the_launchers_agents_hold_still_as_one_changes_the_selection_on_its_row(
    desktop: Desktop,
) -> None:
    """The launcher's agents hold still as one changes, the selection on its row."""
    machine = desktop.machine
    desktop.pi_session()
    # A second session in a window of its own, with its own presence. Its
    # label is the shape pi gives one now, the quest's short name and no
    # address; its record's process and kitty window join the two.
    other = "\u03c0 - Another - other-tree"
    desktop.pi_window("TESTQB", other)

    def present(address: str, said: Json) -> None:
        """Replace a session's presence with this, by rename, as pi does."""
        desktop.presence_says(said, address)

    present("TESTQB", {"addresses": ["TESTQB"], "busy": True})
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("agents")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Agents")
    machine.send_key("ret")
    desktop.launcher_until(lambda shown: shown["held"] == ["agents"] and len(shown["rows"]) > 1)
    # The window titled the new way is its session, named by its title.
    assert "Another" in desktop.showing()["rows"], desktop.showing()["rows"]
    # The second, which would go first once it wants Joel.
    machine.send_key("down")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == shown["rows"][1])
    shown = desktop.showing()
    order = shown["rows"]
    chosen = shown["selected"]["name"]
    address = shown["selected"]["side"].split(" \u00b7 ")[0]
    stays = "TESTQA" if address == "TESTQB" else "TESTQB"
    # It comes to want Joel, a want older than the shell so no notice
    # takes the screen, and the other stops working.
    present(
        address,
        {
            "addresses": [address],
            "busy": False,
            "wants": {"kind": "question", "since": "2000-01-01T00:00:00Z", "said": "Which?"},
        },
    )
    present(stays, {"addresses": [stays], "busy": False})
    desktop.launcher_until(
        lambda shown: (
            "asks you" in (shown["selected"] or {}).get("side", "")
            and any("waiting" in thing["reads"] for thing in shown["things"])
        )
    )
    # Said in place: the same rows in the same order, the selection
    # still on its row.
    shown = desktop.showing()
    assert shown["rows"] == order, "the agents moved under the selection"
    assert shown["selected"]["name"] == chosen, "the selection left its row"
    machine.send_key("esc")
    desktop.launcher_until(lambda shown: not shown["open"])
    desktop.as_account(f"niri msg action close-window --id {desktop.window_titled(other)['id']}")
    desktop.as_account(f"rm -f {PI_PRESENCE}/testqb.json")
    present("TESTQA", {"addresses": ["TESTQA"], "busy": False})
    machine.wait_until_fails(
        desktop.in_session("niri msg --json windows | grep -q TESTQB"),
        timeout=dt.timedelta(seconds=30),
    )


# A window closed while Super+Tab shows it stays in its place, gone, until
# a key: rows closing up under the cursor was one of the ways the launcher
# moved with nothing pressed.


def test_a_window_closed_while_shown_stays_in_its_place_until_a_key(desktop: Desktop) -> None:
    """A window closed while shown stays in its place until a key."""
    machine = desktop.machine
    # Opened first, so it isn't the focused window, which Super+Tab leaves out.
    leaving = desktop.opened("Leaving soon")
    desktop.notes_window()
    machine.wait_until_succeeds(
        desktop.in_session("niri msg focused-window | grep -q 'Notes for Tuesday'"),
        timeout=dt.timedelta(seconds=10),
    )
    machine.send_key("meta_l-tab")
    desktop.launcher_ready()
    desktop.launcher_until(
        lambda shown: shown["held"] == ["windows"] and "Leaving soon" in shown["rows"]
    )
    before = desktop.showing()
    desktop.close(leaving)

    def gone(shown: Json) -> bool:
        return any(thing["gone"] for thing in shown["things"])

    desktop.launcher_until(gone)
    after = desktop.showing()
    assert after["rows"] == before["rows"], f"the rows moved: {before['rows']} to {after['rows']}"
    assert after["selected"]["name"] == before["selected"]["name"], "the selection moved"
    left = next(thing for thing in after["things"] if thing["gone"])
    assert left["reads"].startswith("Leaving soon"), left
    # A key searches afresh, and the window's row goes.
    desktop.type_in("n")
    machine.send_key("backspace")
    desktop.launcher_until(
        lambda shown: shown["typed"] == "" and "Leaving soon" not in shown["rows"]
    )
    assert not gone(desktop.showing()), desktop.showing()
    machine.send_key("esc")
    desktop.launcher_until(lambda shown: not shown["open"])


# A window's verbs, stepped into from Super+Tab, follow it as it changes
# while its row reads the same: floated, its Float says Tile.


def test_a_windows_verbs_follow_it_while_theyre_shown(desktop: Desktop) -> None:
    """A window's verbs follow it while they're shown."""
    machine = desktop.machine
    notes = desktop.notes_window()
    # Another window in front, so the notes' is among Super+Tab's.
    desktop.as_account("niri msg action spawn -- kitty --title 'In front of the notes'")
    machine.wait_until_succeeds(
        desktop.in_session("niri msg focused-window | grep -q 'In front of the notes'"),
        timeout=STARTING,
    )
    machine.send_key("meta_l-tab")
    desktop.launcher_ready()
    desktop.launcher_until(lambda shown: shown["held"] == ["windows"])
    desktop.type_in("tuesday", selecting="Notes for Tuesday")
    desktop.step_in()
    desktop.launcher_until(lambda shown: "Float" in shown["rows"])
    desktop.as_account(f"niri msg action toggle-window-floating --id {notes['id']}")
    desktop.launcher_until(lambda shown: "Tile" in shown["rows"] and "Float" not in shown["rows"])
    machine.send_key("esc")
    desktop.launcher_until(lambda shown: not shown["open"])
    desktop.as_account(f"niri msg action toggle-window-floating --id {notes['id']}")
    front = desktop.window_titled("In front of the notes")
    desktop.as_account(f"niri msg action close-window --id {front['id']}")
    machine.wait_until_fails(
        desktop.in_session("niri msg windows | grep -q 'In front of'"),
        timeout=dt.timedelta(seconds=30),
    )


# The programs using the processors and the memory are read as the
# launcher opens, so they're among the parts the first time each is
# asked, even stepped into at once, where nothing is asked again.


def test_the_machines_programs_are_there_the_first_time_its_asked(desktop: Desktop) -> None:
    """The machine's programs are there the first time it's asked."""
    machine = desktop.machine
    for figure in ("processor", "memory"):
        desktop.answer(figure)
        desktop.step_in()
        desktop.launcher_until(lambda shown: "Using It" in shown["rows"])
        machine.send_key("esc")
        desktop.launcher_until(lambda shown: not shown["open"])


def test_a_word_typed_whole_lands_on_what_it_says(desktop: Desktop) -> None:
    """A word typed whole lands on what it says."""
    machine = desktop.machine
    # Agents lists pi's sessions, so there has to be one.
    desktop.pi_session()
    closed = desktop.in_session(LAUNCHER_OPEN)

    def selects(key: str) -> Callable[[Json], bool]:
        """Give a test that Return's row is the one with this key."""
        return lambda shown: bool(shown["selected"]) and shown["selected"]["is"] == key

    # Each by its one word, the name it shows. "light" once found
    # nothing.
    lands: list[tuple[str, Callable[[Json], bool]]] = [
        ("do not disturb", selects("session:quiet")),
        ("agents", selects("collection:agents")),
        ("clipboard", selects("collection:clipboard")),
        ("light", lambda shown: "Brighten the screen" in shown["rows"]),
    ]
    for typed, holds in lands:
        desktop.answer(typed)
        desktop.launcher_until(holds)
        machine.send_key("esc")
        machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))


# One walk, as its subtest was: each step starts where the last left it.
def test_super_shift_shows_every_layers_shortcuts_and_runs_the_desktops(  # noqa: PLR0915
    desktop: Desktop,
) -> None:
    """Super+Shift+/ shows every layer's shortcuts, and runs the desktop's."""
    machine = desktop.machine
    windows = len(json.loads(desktop.as_account("niri msg --json windows")))
    # Over no window only the desktop's keys work, so the layers lead with
    # them: the empty workspace niri keeps last on the focused screen.
    shortcut_spaces: list[Json] = json.loads(desktop.as_account("niri msg --json workspaces"))
    shortcut_screen = next(space for space in shortcut_spaces if space["is_focused"])["output"]
    shortcut_empty = max(
        (space for space in shortcut_spaces if space["output"] == shortcut_screen),
        key=lambda space: int(space["idx"]),
    )
    desktop.as_account(f"niri msg action focus-workspace {shortcut_empty['idx']}")
    machine.send_key("meta_l-shift-slash")
    desktop.launcher_ready()
    # Its top is the layers, each with its count and as many of its first
    # groups as fit whole; the machine has no Neovim or pi, so no layer of
    # theirs, nor Chrome or Slack, so none of theirs either. After the
    # terminal's come btop's, which every machine has, the picture viewer's,
    # the markdown reader's, the document viewer's, the player's,
    # 1Password's, GIMP's, Inkscape's and Spotify's.
    layers = [
        "Desktop",
        "Terminal",
        "Processes",
        "Picture Viewer",
        "Markdown Reader",
        "Document Viewer",
        "Player",
        "Passwords",
        "Picture Editor",
        "Drawing Editor",
        "Music",
    ]
    desktop.launcher_until(lambda shown: shown["rows"] == layers)
    desktop_row = desktop.showing()["things"][0]["reads"]
    assert re.fullmatch(r"Desktop \u00b7 \d+ \u00b7 Windows, Columns", desktop_row), desktop_row
    desktop.photograph("launcher-shortcut-layers")
    # The desktop holds its groups, by what a chord acts on.
    desktop.step_in()
    groups = ["Windows", "Columns", "Workspaces", "Screens", "Launcher", "Typing"]
    desktop.launcher_until(lambda shown: shown["rows"][: len(groups)] == groups)
    desktop.photograph("launcher-shortcut-groups")
    # What's typed there finds a group by its name, before every chord in
    # the layer that its words start, flat; a group as big as Workspaces
    # holds groups of its own, then what none of them takes.
    desktop.type_in("work", selecting="Workspaces")
    assert "Go to workspace 3" in desktop.showing()["rows"]
    desktop.step_in()
    desktop.launcher_until(lambda shown: shown["held"] == ["shortcuts", "desktop", "workspaces"])
    inner = ["Focus", "The Column", "The Window", "Above and Below"]
    assert desktop.showing()["rows"][: len(inner)] == inner
    assert "Show every workspace" in desktop.showing()["rows"]
    # The field says where that is as one path, in the rows' own case.
    assert desktop.showing()["path"] == ["Shortcuts", "Desktop", "Workspaces"]
    # Back out: to the desktop with "work" typed, cleared, then the top.
    for _step in range(3):
        machine.send_key("left")
    desktop.launcher_until(lambda shown: shown["held"] == ["shortcuts"] and shown["rows"] == layers)
    # At the top, what's typed finds a chord in any layer, flat.
    desktop.type_in("paste")
    desktop.photograph("launcher-shortcuts")
    for _ in "paste":
        machine.send_key("backspace")
    # The touchscreen's swipes are the desktop's, found by "swipe": one
    # doing what a chord does is that chord's row, saying the swipe in
    # words, and one no chord does is drawn as its fingers and its way.
    swipe = "swipe workspace below"
    desktop.type_in(swipe, selecting="Go to the workspace below")
    below = desktop.showing()
    assert not below["selected"]["chord"].startswith("3 "), below["selected"]
    assert "3 fingers up" in below["footer"], below["footer"]
    assert below["rows"].count("Go to the workspace below") == 1, below["rows"]
    desktop.photograph("launcher-swipes")
    for _ in swipe:
        machine.send_key("backspace")
    swipe = "swipe stop showing"
    desktop.type_in(swipe, selecting="Stop showing every workspace")
    assert desktop.showing()["selected"]["chord"].startswith("4 ")
    for _ in swipe:
        machine.send_key("backspace")
    desktop.type_in("open a terminal")
    machine.send_key("ret")

    def opened(_: bool) -> bool:
        """Say whether a window has opened since the shortcuts did."""
        return len(json.loads(desktop.as_account("niri msg --json windows"))) > windows

    # kitty starts cold, drawn in software, which on a machine running
    # other tests too has taken 10 seconds and more than 19, so it's
    # given a minute.
    retry(opened, timeout=dt.timedelta(minutes=1))

    # Shortcuts is stepped into from its row, and its path says it once.
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("shortcuts", selecting="Shortcuts")
    desktop.step_in()
    desktop.launcher_until(lambda shown: shown["path"] == ["Shortcuts"])
    # Inside it, a group named what's typed, whole, comes first, and is
    # Return's row.
    desktop.type_in("workspaces", selecting="Workspaces")
    assert desktop.showing()["rows"][0] == "Workspaces"
    machine.send_key("esc")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=dt.timedelta(seconds=10))


def test_super_shift_over_a_terminal_opens_on_the_terminals_shortcuts(desktop: Desktop) -> None:
    """Super+Shift+/ over a terminal opens on the terminal's shortcuts."""
    machine = desktop.machine
    desktop.focused_kitty()
    machine.send_key("meta_l-shift-slash")
    desktop.launcher_ready()
    # Over kitty the chord opens on the terminal's layer, its groups
    # already showing, the path saying whose keys they are.
    desktop.launcher_until(lambda shown: shown["held"] == ["shortcuts", "terminal"])
    assert desktop.showing()["path"] == ["Shortcuts", "Terminal"]
    assert desktop.showing()["rows"][:2] == ["Tabs", "Tab Numbers"]
    desktop.photograph("launcher-shortcuts-over-kitty")
    # The right arrow steps into a group, and the left back out to the
    # layers, the terminal's first and selected, saying it's the window's,
    # and the desktop's next.
    desktop.step_in()
    desktop.launcher_until(lambda shown: shown["held"] == ["shortcuts", "terminal", "tabs"])
    machine.send_key("left")
    desktop.launcher_until(lambda shown: shown["held"] == ["shortcuts", "terminal"])
    machine.send_key("left")
    desktop.launcher_until(lambda shown: shown["held"] == ["shortcuts"])
    assert desktop.showing()["rows"][:2] == ["Terminal", "Desktop"]
    assert (desktop.showing()["selected"] or {}).get("name") == "Terminal"
    assert desktop.showing()["footer"] == "the keys of the window you were in"
    # Return goes back in.
    desktop.step_in("ret")
    desktop.launcher_until(lambda shown: shown["held"] == ["shortcuts", "terminal"])
    machine.send_key("esc")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=dt.timedelta(seconds=10))


# The orchestrator keeps what waits on Joel in one file, and each quest
# keeps its own beside it.


def test_now_leads_with_what_waits_on_joel(desktop: Desktop) -> None:
    """Now leads with what waits on Joel, and Return goes to where it's done."""
    machine = desktop.machine
    closed = desktop.in_session(LAUNCHER_OPEN)
    # pi's session is the orchestrator, by the name the harness gives it.
    desktop.pi_session()
    desktop.opened("Waiting room")
    queue = {
        "orchestrator": "TESTQA",
        "items": [
            {
                "id": "try",
                "title": "Try the new home",
                "why": "to see",
                "minutes": 10,
                "window": "Waiting room",
            },
            {
                "id": "pick",
                "title": "Pick a bucket",
                "why": "backups",
                "minutes": 1,
                "window": None,
            },
        ],
    }
    # A quest's own, ready to do now.
    asked = {"items": [{"id": "ask", "title": "Answer the ask", "minutes": 2, "ready": True}]}
    state = "$HOME/.local/state"
    desktop.as_account(
        f"printf %s {shlex.quote(json.dumps(queue))} > {state}/mission/.q"
        f" && mv {state}/mission/.q {state}/mission/queue.json"
    )
    # One ready is said as a notice the moment it's heard, named by what
    # it asks, so it's never a second "Waiting on you" beside Now's.
    with desktop.told("shows card Answer the ask"):
        desktop.as_account(
            f"printf %s {shlex.quote(json.dumps(asked))} > {state}/mission/waiting/.q"
            f" && mv {state}/mission/waiting/.q {state}/mission/waiting/QUEST1.json"
        )
    # Another window in front, so going to the waiting room is seen.
    desktop.opened("Elsewhere")
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    # Home's Now names the first Joel can do now, with its minutes.
    desktop.launcher_until(
        lambda shown: any(
            thing["reads"].startswith("Now \u00b7 Waiting on You: Answer the ask, 2 min")
            for thing in shown["things"]
        )
    )
    # It counts only what Joel can do now: the queue's two say no when.
    desktop.type_in("waiting on you", selecting="Waiting on You")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("side") == "1")
    # Inside, what's for now leads, then what's for later, each in the
    # agents' order, and Return goes to where one is done.
    desktop.step_in()
    desktop.launcher_until(
        lambda shown: shown["rows"] == ["Answer the ask", "Try the new home", "Pick a bucket"]
    )
    machine.send_key("down")
    desktop.launcher_until(
        lambda shown: (shown["selected"] or {}).get("name") == "Try the new home"
    )
    machine.send_key("ret")
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
    retry(
        lambda _: desktop.window_titled("Waiting room")["is_focused"],
        timeout=dt.timedelta(seconds=10),
    )
    # One naming no window is done with the orchestrator, in its window.
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("waiting on you", selecting="Waiting on You")
    desktop.step_in()
    machine.send_key("down")
    machine.send_key("down")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Pick a bucket")
    machine.send_key("ret")
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
    retry(
        lambda _: desktop.window_titled(PI_WINDOW)["is_focused"],
        timeout=dt.timedelta(seconds=10),
    )
    # Put back as it was, for the tests after.
    desktop.as_account(f"rm {state}/mission/queue.json {state}/mission/waiting/QUEST1.json")
    for title in ["Waiting room", "Elsewhere"]:
        window = desktop.window_titled(title)["id"]
        desktop.as_account(f"niri msg action close-window --id {window}")


# Where the agents keep what waits on Joel, and the quests whose READMEs
# say whether each has ended.
MISSION = "$HOME/.local/state/mission"
QUESTS = "$HOME/.local/share/pi/agentic-harness.pi/quest-workflow/quests"


def kept(desktop: Desktop, path: str, read: dict[str, Any]) -> None:
    """Write a file the agents keep, whole, as they do."""
    desktop.as_account(
        f"printf %s {shlex.quote(json.dumps(read))} > {path}.q && mv {path}.q {path}"
    )


def readme(desktop: Desktop, quest: str, status: str) -> None:
    """Write a quest's README, saying its status."""
    desktop.as_account(
        f"mkdir -p {QUESTS}/{quest} && printf '%s\\n' '---' 'id: {quest}'"
        f" 'status: {status}' '---' '# {quest}' > {QUESTS}/{quest}/README.md"
    )


def waiting_opened(desktop: Desktop) -> None:
    """Open the launcher on Waiting on you's row."""
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("waiting on you", selecting="Waiting on You")


def launcher_closed(desktop: Desktop) -> None:
    """Close the launcher with Escape, and wait until it's gone."""
    desktop.machine.send_key("esc")
    desktop.machine.wait_until_fails(
        desktop.in_session(LAUNCHER_OPEN), timeout=dt.timedelta(seconds=10)
    )


def waiting_inside(desktop: Desktop) -> tuple[str, list[str]]:
    """Give Waiting on you's side and the rows inside it, as a fresh look sees them."""
    waiting_opened(desktop)
    side = cast("str", (desktop.showing()["selected"] or {}).get("side"))
    desktop.step_in()
    rows = cast("list[str]", desktop.showing()["rows"])
    launcher_closed(desktop)
    return side, rows


def picked(desktop: Desktop, name: str) -> None:
    """Wait until the row named is the one Return takes."""
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == name)


def test_waiting_on_you_holds_only_what_is_true(desktop: Desktop) -> None:
    """Waiting on you drops an ended quest's and a copy, and keeps Joel's word."""
    machine = desktop.machine
    state = "$HOME/.local/state"
    quests = QUESTS
    readme(desktop, "QEST-20261026-LIVEQA", "active")
    readme(desktop, "QEST-20261026-DONEQA", "concluded")
    # The queue says one thing Joel can do now; a live quest says it again
    # in other case, and the queue covers another of its by name.
    kept(
        desktop,
        f"{state}/mission/queue.json",
        {
            "orchestrator": "TESTQA",
            "items": [
                {
                    "id": "restart",
                    "title": "Restart once",
                    "when": "now",
                    "covers": ["QEST-20261026-LIVEQA:sitting"],
                }
            ],
        },
    )
    kept(
        desktop,
        f"{state}/mission/waiting/LIVEQA.json",
        {
            "quest": "QEST-20261026-LIVEQA",
            "items": [
                {"id": "again", "title": "restart  ONCE", "ready": True},
                {"id": "sitting", "title": "A sitting", "ready": True},
                {"id": "later", "title": "Look at the desk", "ready": False, "when": "tomorrow"},
            ],
        },
    )
    # A concluded quest's file stays where it was written, and says nothing.
    kept(
        desktop,
        f"{state}/mission/waiting/DONEQA.json",
        {
            "quest": "QEST-20261026-DONEQA",
            "items": [{"id": "over", "title": "Something over", "ready": True}],
        },
    )
    # One ready, not said again in four days: nothing goes by its age.
    said = (dt.datetime.now(dt.UTC) - dt.timedelta(days=4)).isoformat()
    kept(
        desktop,
        f"{state}/mission/waiting/OLDQA.json",
        {
            "updated": said,
            "items": [{"id": "old", "title": "Answer the old", "ready": True, "why": "it asked"}],
        },
    )

    # Two for now: the queue's, which the quest's copy and the one covered
    # leave alone, and the old one; then what's for later.
    retry(
        lambda _: (
            waiting_inside(desktop) == ("2", ["Restart once", "Answer the old", "Look at the desk"])
        ),
        timeout=dt.timedelta(seconds=30),
    )
    # The right arrow steps into one: going to its window, where one is
    # open, Done and Not Now, copying it, then what there is to know of it.
    waiting_opened(desktop)
    desktop.step_in()
    machine.send_key("down")
    picked(desktop, "Answer the old")
    desktop.step_in()
    acts = ["Done", "Not Now", "Copy What It Said", "Why"]
    desktop.launcher_until(
        lambda shown: (
            "Done" in shown["rows"]
            and shown["rows"][shown["rows"].index("Done") :][:4] == acts
            and shown["rows"][0] != "Copy What It Said"
        )
    )
    # Done keeps the launcher, and is kept where Joel's word is.
    desktop.down_to("Done")
    machine.send_key("ret")
    retry(
        lambda _: "OLDQA:old" in desktop.as_account(f"cat {state}/mission/settled.json || true"),
        timeout=dt.timedelta(seconds=10),
    )
    assert desktop.showing()["open"]
    launcher_closed(desktop)
    retry(
        lambda _: waiting_inside(desktop) == ("1", ["Restart once", "Look at the desk"]),
        timeout=dt.timedelta(seconds=30),
    )
    # Not Now puts one under Later, so nothing's for now.
    waiting_opened(desktop)
    desktop.step_in()
    picked(desktop, "Restart once")
    desktop.step_in()
    desktop.down_to("Not Now")
    machine.send_key("ret")
    launcher_closed(desktop)
    retry(
        lambda _: waiting_inside(desktop) == ("2 later", ["Restart once", "Look at the desk"]),
        timeout=dt.timedelta(seconds=30),
    )
    # A quest that ends takes its things with it, as its README says so.
    readme(desktop, "QEST-20261026-LIVEQA", "retired")
    retry(
        lambda _: waiting_inside(desktop) == ("1 later", ["Restart once"]),
        timeout=dt.timedelta(seconds=30),
    )
    # Joel's word stays across a restart of the shell.
    desktop.as_account("systemctl --user restart quickshell")
    machine.wait_until_succeeds(
        desktop.in_session("qs ipc call launcher showing"), timeout=dt.timedelta(seconds=60)
    )
    retry(
        lambda _: waiting_inside(desktop) == ("1 later", ["Restart once"]),
        timeout=dt.timedelta(seconds=60),
    )
    desktop.as_account(
        f"rm -r {state}/mission/queue.json {state}/mission/waiting/LIVEQA.json"
        f" {state}/mission/waiting/DONEQA.json {state}/mission/waiting/OLDQA.json"
        f" {state}/mission/settled.json"
        f" {quests}/QEST-20261026-LIVEQA {quests}/QEST-20261026-DONEQA"
    )


def test_waiting_with_nowhere_to_go_steps_in_and_copies_after_its_acts(
    desktop: Desktop,
) -> None:
    """Waiting on You steps in where nothing is open to go to, and copying comes after Done."""
    machine = desktop.machine
    queue = f"{MISSION}/queue.json"
    # An orchestrator with no session, so the item has no window to go to.
    kept(
        desktop,
        queue,
        {
            "orchestrator": "GONEQA",
            "items": [{"id": "form", "title": "Sign the form", "when": "now", "why": "to send"}],
        },
    )
    try:
        retry(
            lambda _: waiting_inside(desktop) == ("1", ["Sign the form"]),
            timeout=dt.timedelta(seconds=30),
        )
        # Return on Waiting on You steps in, as the right arrow does, rather
        # than copying the first.
        waiting_opened(desktop)
        desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("verb") == "more")
        desktop.step_in("ret")
        picked(desktop, "Sign the form")
        # And on the item: its acts, copying last of them, then its facts.
        desktop.step_in("ret")
        desktop.launcher_until(
            lambda shown: shown["rows"][:4] == ["Done", "Not Now", "Copy What It Said", "Why"]
        )
        desktop.down_to("Copy What It Said")
        machine.send_key("ret")
        machine.wait_until_fails(
            desktop.in_session(LAUNCHER_OPEN), timeout=dt.timedelta(seconds=10)
        )
        machine.wait_until_succeeds(
            desktop.in_session('[ "$(wl-paste --no-newline)" = "Sign the form" ]'),
            timeout=dt.timedelta(seconds=10),
        )
    finally:
        desktop.as_account(f"rm -f {queue}")


def test_a_waiting_notice_goes_read_once_its_item_stops_waiting(desktop: Desktop) -> None:
    """A waiting notice goes, read, once its item can't be done now any more."""
    machine = desktop.machine
    notices = desktop.in_session("niri msg layers | grep -q notifications")
    machine.wait_until_fails(notices, timeout=dt.timedelta(seconds=30))
    readme(desktop, "QEST-20261027-GONEQA", "active")
    path = f"{MISSION}/waiting/GONEQA.json"
    item = {"id": "plug", "title": "Plug in the dock", "ready": True, "minutes": 1}
    with desktop.told("shows card Plug in the dock"):
        kept(desktop, path, {"quest": "QEST-20261027-GONEQA", "items": [item]})
    # Its owner says it's for later: the notice goes well before its own
    # time would, and the history keeps it read, so it isn't missed.
    later = {**item, "ready": False, "when": "soon"}
    kept(desktop, path, {"quest": "QEST-20261027-GONEQA", "items": [later]})
    machine.wait_until_fails(notices, timeout=dt.timedelta(seconds=5))
    machine.wait_until_succeeds(
        desktop.in_session(
            "cat ~/.local/state/quickshell/by-shell/*/notices.json"
            ' | grep -q \'"summary":"Plug in the dock"[^}]*"read":true\''
        ),
        timeout=dt.timedelta(seconds=10),
    )
    desktop.as_account(f"rm {path} && rm -r {QUESTS}/QEST-20261027-GONEQA")


def test_a_session_wants_joel_only_while_it_is_true(desktop: Desktop) -> None:
    """A session's want counts only while its quest has something ready, or it's his."""
    desktop.pi_session()
    waiting = "$HOME/.local/state/mission/waiting/TESTQA.json"

    def wants(kind: str, orchestrated: str | None = None) -> None:
        """Say the session wants Joel, its record still naming its process and window."""
        desktop.presence_says(
            {
                "session": "01a1face-test",
                "addresses": ["TESTQA"],
                "busy": False,
                "owns": ["QEST-20261022-TESTQA"],
                "orchestratedBy": orchestrated,
                "wants": {"kind": kind, "since": "2026-10-26T00:00:00Z", "said": "Do it"},
            }
        )

    def marked() -> bool:
        """Say whether the margin last held pi's mark, which says a session wants Joel."""
        held = desktop.as_account(
            "journalctl --user --unit=quickshell --no-pager --output=cat"
            " | grep -o 'report\\[holds margin [^]]*' | tail -n 1"
        )
        return "agent" in held.split()[2:]

    # An action its quest's file holds only for later isn't Joel's yet.
    desktop.as_account(
        f"printf %s {shlex.quote(json.dumps({'items': [{'title': 'Later', 'ready': False}]}))}"
        f" > {waiting}"
    )
    wants("action", "WCJQSG")
    time.sleep(2)
    assert not marked(), "an action held only for later marked Joel"
    # It is once the file says he can do it now.
    desktop.as_account(
        f"printf %s {shlex.quote(json.dumps({'items': [{'title': 'Now', 'ready': True}]}))}"
        f" > {waiting}.q && mv {waiting}.q {waiting}"
    )
    retry(lambda _: marked(), timeout=dt.timedelta(seconds=20))
    # A question an orchestrated session asks is its orchestrator's; a yes
    # it waits for is always his.
    wants("question", "WCJQSG")
    retry(lambda _: not marked(), timeout=dt.timedelta(seconds=20))
    wants("approval", "WCJQSG")
    retry(lambda _: marked(), timeout=dt.timedelta(seconds=20))
    desktop.as_account(f"rm {waiting}")
    desktop.pi_session()
    for title in ["Elsewhere"]:
        window = desktop.window_titled(title)["id"]
        desktop.as_account(f"niri msg action close-window --id {window}")


# Joel, 2026-10-25: "text alignment and shit constantly shifting around".
# A word that moves without changing tells him nothing and costs him a
# look, so after every key each word on the card that reads as it did must
# be drawn where it was: in the field, in the foot, and in each row that
# is still the same thing saying the same.


def laid_out(desktop: Desktop) -> Json:
    """Give where the launcher draws each word, once two reads in a row agree."""
    was = None
    for _ in range(20):
        now = cast("Json", json.loads(desktop.as_account("qs ipc call launcher laidOut")))
        if now == was:
            return now
        was = now
    return cast("Json", was)


def moved(before: Json, after: Json) -> list[str]:
    """Say each word drawn in both that reads the same and isn't where it was."""
    shifts = []

    def compare(where: str, old: list[Json], new: list[Json]) -> None:
        unmatched: dict[str, list[Json]] = {}
        for word in old:
            unmatched.setdefault(word["text"], []).append(word)
        for word in new:
            same = unmatched.get(word["text"])
            if not same:
                continue
            was = same.pop(0)
            if (was["starts"], was["y"]) != (word["starts"], word["y"]):
                shifts.append(
                    f"{where}: {word['text']!r} moved from {was['starts']},{was['y']}"
                    f" to {word['starts']},{word['y']}"
                )

    compare("the field", before["field"], after["field"])
    compare("the foot", before["foot"], after["foot"])
    rows = {row["at"]: row for row in before["rows"]}
    for row in after["rows"]:
        was = rows.get(row["at"])
        if was and (was["is"], was["reads"]) == (row["is"], row["reads"]):
            compare(f"row {row['at']} ({row['reads']})", was["words"], row["words"])
    return shifts


def test_nothing_on_the_launcher_moves_unless_it_changes(desktop: Desktop) -> None:
    """Nothing on the launcher moves unless it changes, as letters, erasures and arrows go."""
    machine = desktop.machine
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    before = laid_out(desktop)
    shifts: list[str] = []
    typed = ""
    keys = [*"sound", "backspace", "backspace", "down", "down", "up", *"nd", "down"]
    for step, key in enumerate(keys):
        was = desktop.showing()["selected"]
        if len(key) == 1:
            typed += key
            machine.send_chars(key)
        else:
            typed = typed[:-1] if key == "backspace" else typed
            machine.send_key(key)
        # An arrow is heard once the selection moves, a letter once it's held.
        arrowed = key in {"down", "up"}

        def heard(
            shown: Json, typed: str = typed, arrowed: bool = arrowed, was: Json = was
        ) -> bool:
            return shown["typed"] == typed and (not arrowed or shown["selected"] != was)

        desktop.launcher_until(heard)
        after = laid_out(desktop)
        desktop.photograph(f"launcher-still-{step:02}-{key}")
        shifts += [f"after {key!r} ({typed!r}) {shift}" for shift in moved(before, after)]
        before = after
    machine.send_key("esc")
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=dt.timedelta(seconds=10))
    assert not shifts, "words moved without changing:\n" + "\n".join(shifts)


# A binding that throws or assigns the wrong type only warns, and
# only once a surface draws it, which can be long after the start.


def test_quickshell_warned_of_nothing_it_drew_all_along(desktop: Desktop) -> None:
    """Quickshell warned of nothing it drew all along."""
    # Quickshell colours its levels and categories even in the journal,
    # so a colour code may stand between the two words.
    log = desktop.as_account(
        "journalctl --user -u quickshell --no-pager -o cat | grep -a -E 'WARN[^ ]* scene' || true"
    )
    assert log.strip() == "", f"Quickshell warned of a scene: {log}"

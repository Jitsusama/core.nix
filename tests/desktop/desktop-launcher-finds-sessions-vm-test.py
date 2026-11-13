"""The desktop test's launcher finds sessions area.

Here the launcher finds pi's sessions and quests, talks to orchestrators, and reopens
sessions where they sat.

tests/desktop.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), and its
opening comment says what the test proves. Every test asks for the desktop, started once a run
(desktop_session.py), and waits at its start for what it needs, so any test runs alone.
"""

from __future__ import annotations

import datetime as dt
import json
import re
import shlex
import tarfile
import time
from typing import TYPE_CHECKING, Any, cast

import desktop_session
import pytest
from desktop_session import (
    ARRANGED,
    HARNESS,
    INBOX,
    LAUNCHER_OPEN,
    PI_PRESENCE,
    PI_WINDOW,
    SHELL_STARTS,
    focus_given,
    kept,
    laid_out,
    lighthouse_listed,
    listed_quest,
    quests_listed,
    started_apart,
    watch_the_focus,
    window_box,
)
from PIL import Image, ImageChops

if TYPE_CHECKING:
    from collections.abc import Iterator

    from desktop_session import Desktop, Json
    from test_driver.machine import BaseMachine

# How late, in seconds, the journal may stamp a line of the shell's on a busy machine: a
# quarter of a second, since a builder running other VM tests beside it once stamped one
# 0.11 s late (9 November), while starts the shell didn't pace would come together.
JOURNAL_LAG = 0.25
# Where a spawn's frames are recorded, and what the recorder says, and the frames taken out.
RECORDING = "/run/user/1000/spawn.mkv"
RECORDED = "/run/user/1000/spawn.log"
FRAMES = "/run/user/1000/spawn-frames"
# How much lighter or darker a recorded pixel may be than the same pixel
# still, from the encoder alone, and the share of a window's pixels past it
# (one in this many) that is a change Joel would see.
ENCODER_NOISE = 40
CHANGED_SHARE = 500
# The crowd's busy sessions, one stress-ng each.
CROWD = 30
# Pi's session's row, its quest's title, which Lighthouse's is while it runs.
LIGHTHOUSE = "A quest of its own & whole"
# A window of the sessions' kitty, asked over its own socket as the shell asks.
IN_SESSIONS = "kitten @ --to unix:$XDG_RUNTIME_DIR/pi-sessions/kitty.sock launch --type=os-window"
# The sessions' kitty's own process, by the socket it listens on.
SESSIONS_KITTY = "pi-sessions/kitty[.]sock"


@pytest.fixture(scope="module")
def desktop(machine: BaseMachine, values: dict[str, Any]) -> Desktop:
    """Give the desktop, started once a run."""
    return desktop_session.started(machine, cast("desktop_session.Values", values))


@pytest.fixture(autouse=True)
def launcher_closed_after(desktop: Desktop) -> Iterator[None]:
    """Close the launcher after each test, so one failing with it open fails alone."""
    yield
    desktop.launcher_left_closed()


# One walk, as its subtest was: each step starts where the last left it.
def test_the_launcher_finds_a_pi_session_by_its_quest_and_goes_to_its_window(
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
    # whole, as its record says it, not as its window's title cuts it.
    # At its right, how it stands and what it spent today, nothing more;
    # its address, folder and start are its drawer's eyebrow.
    desktop.type_in("agents")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Agents")
    machine.send_key("ret")
    desktop.launcher_until(
        lambda shown: (
            shown["held"] == ["agents"]
            and (shown["selected"] or {}).get("name") == "A quest of its own & whole"
            and shown["selected"]["side"] == "2nd in line \u00b7 $12.50"
            and re.fullmatch(
                r"TESTQA \u00b7 my-tree \u00b7 since \d\d:\d\d", shown["drawer"]["eyebrow"]
            )
            is not None
        )
    )
    desktop.shoot("launcher-agents")
    # Gone into by Ctrl+], as Return goes to it, what's known of it, each
    # a row of its own.
    desktop.follow()
    desktop.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("name") == "Go To"
            and shown["selected"]["verb"] == "go to"
        )
    )
    # Its acts in the order every session keeps them, Close last since it's
    # here, then what's known of it, each named first, as every part is, then
    # what it says.
    assert desktop.showing()["rows"][:5] == ["Go To", "Talk", "Pause", "Stop", "Close"]
    desktop.down_to("Close")
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
        desktop.next_row()
        # The wait ends before the loop moves on, so it reads this pass's names.
        desktop.launcher_until(
            lambda shown: (
                (shown["selected"] or {}).get("name") == name  # noqa: B023
                and re.fullmatch(side, shown["selected"]["side"])  # noqa: B023
            )
        )
    # Scrolled down past its first rows, the selection is still drawn on
    # its own row.
    selection = laid_out(desktop)["selection"]
    assert selection["y"] == selection["row"], f"the selection is off its row: {selection}"
    desktop.shoot("launcher-agent-facts")
    desktop.step_out()
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
    # is: For You's tray lights in the margin at once, and goes once
    # answered. From another window, since none comes while he's in its
    # own; there a session stopped on his keys is a notice too, so Super+N
    # reaches it, and a run ended blocked, which For You lists, isn't
    # (WCJQSG's pivot, 2026-11-12).
    desktop.as_account(
        f"niri msg action focus-window --id {desktop.window_titled('Elsewhere')['id']}"
    )

    # Wants begun now, as the shell only says those it saw begin.
    def begun() -> str:
        """Give the machine's time now, as pi writes when a want began."""
        return machine.succeed("date -u +%Y-%m-%dT%H:%M:%SZ").strip()

    # Each kind of want is the tray, and ended, the tray goes; a question
    # and a yes are a card each, taken back as the want ends.
    for at, wants in enumerate(
        (
            {"kind": "question", "since": begun(), "said": "Which <way> & why?"},
            {"kind": "approval", "since": begun(), "said": "Push it?"},
            {"kind": "blocked", "since": begun(), "said": "Out of ideas."},
        )
    ):
        since = desktop.mark()
        desktop.presence_says({"addresses": ["TESTQA"], "busy": False, "wants": wants})
        desktop.reported("holds margin ", lambda rest: "forYou" in rest.split(), since)
        if at == 0:
            desktop.shoot("margin-pi-wants")
        since = desktop.mark()
        desktop.presence_says({"addresses": ["TESTQA"], "busy": False})
        desktop.reported("holds margin ", lambda rest: "forYou" not in rest.split(), since)
    cards = desktop.as_account(
        "journalctl --user --unit=quickshell --no-pager --output=cat"
        " | grep -o 'report\\[shows card A quest of its own[^]]*]' || true"
    ).splitlines()
    assert cards == [
        "report[shows card A quest of its own & whole needs you]",
        "report[shows card A quest of its own & whole needs your yes]",
    ], f"a session's asks weren't each a card, or a blocked run was: {cards}"
    machine.wait_until_succeeds(
        desktop.in_session(
            'jq -e \'[.notices[] | select(.summary | startswith("A quest of its own"))'
            f" | select(.withdrawn | not)] == []' {desktop.NOTICES_KEPT}"
        ),
        timeout=dt.timedelta(seconds=10),
    )


def an_orchestrator(desktop: Desktop) -> None:
    """Make pi's session an orchestrator, Lighthouse, as the harness lists one, its inbox empty."""
    desktop.pi_session()
    lighthouse_listed(desktop)


def into_lighthouse(desktop: Desktop) -> None:
    """Open Agents, find Lighthouse's session there by its short name, and step into it."""
    desktop.as_account("qs ipc call launcher collection agents")
    desktop.launcher_ready()
    desktop.type_in("lighthouse", selecting=LIGHTHOUSE)
    desktop.follow()
    desktop.launcher_until(lambda shown: "Talk" in shown["rows"])


def test_an_orchestrator_is_found_by_name_and_talked_to_through_its_inbox(
    desktop: Desktop,
) -> None:
    """An orchestrator is found by its name, and talked to through its session's inbox."""
    machine = desktop.machine
    an_orchestrator(desktop)
    # A running one is its session, a row of Agents' tree drawn as every
    # session is: what it's doing beside it and its address in its drawer,
    # and its Return goes to its window.
    desktop.as_account("qs ipc call launcher collection agents")
    desktop.launcher_ready()
    desktop.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("name") == LIGHTHOUSE
            and "TESTQA" not in str(shown["selected"]["side"])
            and str(shown["drawer"]["eyebrow"]).startswith("TESTQA \u00b7 ")
            and shown["selected"]["verb"] == "go to"
        )
    )
    desktop.close_launcher()
    # Found from the root by the short name the harness lists it by, Ctrl+]
    # in normal goes straight inside it, though its Return goes to its
    # window, the crumbs saying where it lives; - comes back to the search,
    # it selected (VKCF3Z, 2026-10-29: "\u2190 is where you came from").
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("lighthouse", selecting=LIGHTHOUSE)
    desktop.follow()
    desktop.launcher_until(
        lambda shown: (
            shown["path"] == ["Agents", LIGHTHOUSE]
            and {"Talk", "Pause", "Stop"} <= set(shown["rows"])
        )
    )
    desktop.step_out()
    desktop.launcher_until(
        lambda shown: (
            shown["held"] == []
            and shown["typed"] == "lighthouse"
            and (shown["selected"] or {}).get("name") == LIGHTHOUSE
        )
    )
    desktop.close_launcher()
    # Inside, its acts come in the order every session keeps them.
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("lighthouse", selecting=LIGHTHOUSE)
    desktop.follow()
    desktop.launcher_until(
        lambda shown: (
            shown["path"] == ["Agents", LIGHTHOUSE]
            and shown["rows"][:4] == ["Go To", "Talk", "Pause", "Stop"]
        )
    )
    # Talk asks for words, which go to its session's inbox as Joel's,
    # opening a talk its session follows.
    desktop.type_in("talk", selecting="Talk")
    desktop.step_in("ret")
    # Its words are typed in insert, which the foot names by its mark, INS, in
    # three capitals as every mode's since #1985, and the mark is whole, since
    # it says where his keys go.
    desktop.launcher_until(lambda shown: shown["mode"] == "insert")
    foot = json.loads(desktop.as_account("qs ipc call launcher laidOut"))["foot"]
    mode = [word for word in foot if word["text"] == "INS"]
    assert mode, f"the foot names no mode for Talk's words: {foot}"
    assert not mode[0]["cut"], f"insert's mark is cut: {mode[0]}"
    machine.send_chars("keep the lamp lit")
    machine.send_key("ret")
    machine.wait_until_succeeds(
        desktop.in_session(f'cat {INBOX}/*.json | grep -q \'"text":"keep the lamp lit"\''),
        timeout=dt.timedelta(seconds=10),
    )
    said = json.loads(desktop.as_account(f"cat {INBOX}/*.json"))
    assert said["from"] == "joel", said
    assert str(said.get("id", "")).startswith("talk-"), said
    desktop.close_launcher()


def test_an_orchestrators_earlier_talks_say_how_each_went(desktop: Desktop) -> None:
    """An orchestrator's earlier talks say how each went, and read its reply."""
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
    # The talk is a thing, its reply in its own drawer beside it.
    desktop.launcher_until(
        lambda shown: (
            (shown.get("drawer") or {}).get("out") is True
            and shown["drawer"]["drawn"] == "keep watch tonight"
            and "Watching" in shown["drawer"]["body"]
        )
    )
    # Read, it says his words and how it went, each a part to copy, gone
    # into by Ctrl+].
    desktop.follow()
    desktop.launcher_until(lambda shown: shown["rows"][:2] == ["You Said", "How It Went"])
    desktop.close_launcher()


def test_a_question_asked_is_talked_over_with_an_orchestrator(desktop: Desktop) -> None:
    """A question asked is talked over with an orchestrator, from Ask's answer."""
    machine = desktop.machine
    an_orchestrator(desktop)
    desktop.answer("what is a fjord?")
    machine.send_key("ret")
    desktop.launcher_until(lambda shown: str(shown["footer"]).startswith("answered in"))
    # A step in leads with talking it over with Lighthouse, before asking
    # in a session of its own, and the copy comes after them.
    desktop.follow()
    desktop.launcher_until(
        lambda shown: shown["rows"][:3] == ["Talk to Lighthouse", "Ask in a Session", "Copy"]
    )
    desktop.launcher_until(
        lambda shown: (shown["selected"] or {}).get("name") == "Talk to Lighthouse"
    )
    # Stepping in left the find for normal, where Return talks at once.
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
        lambda shown: shown["path"] == ["Agents", LIGHTHOUSE] and "Talk" in shown["rows"]
    )
    desktop.close_launcher()


def test_an_orchestrator_is_paused_with_no_words(desktop: Desktop) -> None:
    """An orchestrator is paused with no words, an act in its inbox."""
    machine = desktop.machine
    closed = desktop.in_session(LAUNCHER_OPEN)
    an_orchestrator(desktop)
    into_lighthouse(desktop)
    desktop.type_in("pause", selecting="Pause")
    # Return from the find lands on it, since it changes how things stand, and a second does it.
    desktop.land()
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
    # As the harness lists them: the session's quest, and another with no
    # session on it, which a new session opens in its tree for.
    desktop.as_account("mkdir -p ~/hallway")
    quests_listed(
        desktop,
        [
            listed_quest("QEST-20261022-TESTQA", "A quest of its own & whole"),
            listed_quest(
                "QEST-20261022-HALLWY",
                "Paint the hallway",
                priority="queued",
                start="/home/jitsusama/hallway",
            ),
        ],
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
    desktop.next_row()
    desktop.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("name") == "Paint the hallway"
            and shown["selected"]["side"] == "queued"
        )
    )
    desktop.shoot("launcher-quests")
    desktop.previous_row()
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
    quests_listed(desktop, [])
    desktop.as_account("rm -rf ~/hallway $XDG_RUNTIME_DIR/pi-opened")


def test_agents_resumes_a_closed_orchestrator_where_it_left_off_from_one_row(
    desktop: Desktop,
) -> None:
    """Agents resumes a closed orchestrator where it left off, from its row at the tree's end."""
    machine = desktop.machine
    opened = "$XDG_RUNTIME_DIR/pi-opened"
    # Lighthouse orchestrates a quest, and no session runs it; the quest
    # index names the session to resume, which ran in ~/lighthouse, as
    # pi's file for it says, never the index's sessions, which can hold
    # one long dead.
    a_quest_offers_to_orchestrate(desktop)
    lighthouse_listed(desktop)
    lighthouse = listed_quest(
        "QEST-20261022-TESTQA",
        LIGHTHOUSE,
        sessions=["01a0-long-dead"],
        resume={"id": "01a1-lighthouse", "cwd": "/home/jitsusama/lighthouse"},
    )
    record = {"type": "session", "id": "01a1-lighthouse", "cwd": "/home/jitsusama/lighthouse"}
    sessions = "~/.pi/agent/sessions/--home-jitsusama-lighthouse--"
    kept = f"{sessions}/2026-10-28T20-00-00-000Z_01a1-lighthouse.jsonl"
    desktop.as_account(
        f"rm -f {PI_PRESENCE}/testqa.json {opened} && mkdir -p {sessions} ~/lighthouse"
        f" && echo '{json.dumps(record)}' > {kept}"
    )
    quests_listed(desktop, [lighthouse])
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.down_to("Agents")
    machine.send_key("ret")
    desktop.launcher_until(
        lambda shown: shown["held"] == ["agents"] and "Start a Session" in shown["rows"]
    )
    # Starting one lists the quests themselves; one orchestrating already
    # reopens on its own row, so its step in holds Start Here alone.
    desktop.down_to("Start a Session")
    desktop.shoot("launcher-agents-start")
    machine.send_key("ret")
    desktop.launcher_until(lambda shown: LIGHTHOUSE in shown["rows"])
    desktop.down_to(LIGHTHOUSE)
    desktop.follow()
    desktop.launcher_until(lambda shown: shown["rows"][:1] == ["Start Here"])
    assert "Orchestrate" not in desktop.showing()["rows"], desktop.showing()
    desktop.close_launcher()
    # The closed orchestrator stands dimmed at the tree's end, its Return
    # reopening it, which changes how things stand (the key language's
    # reopen), so Return from the find lands on it, saying what a second
    # Return does.
    desktop.as_account("qs ipc call launcher collection agents")
    desktop.launcher_ready()
    desktop.type_in("lighthouse", selecting="Lighthouse")
    selected = desktop.showing()["selected"]
    assert (selected["side"], selected["hue"]) == ("closed", "muted"), selected
    desktop.land()
    assert desktop.showing()["selected"]["told"][:1] == ["\u21b5 reopen"], desktop.showing()
    desktop.shoot("launcher-agents-closed-orchestrator")
    # Return reopens its last session where it ran, as the harness
    # restores one, through a login shell so pi can reach its model.
    machine.send_key("ret")
    machine.wait_until_succeeds(
        desktop.in_session(f'grep -qx "$HOME/lighthouse " {opened}'),
        timeout=SHELL_STARTS,
    )
    said = desktop.as_account(f"cat {opened}").split("\n")
    # Joel is there, so it's told nothing and waits for him.
    assert said[1:] == ["--session", "01a1-lighthouse", ""], said
    # Reopening one takes Joel to it, so the launcher closes.
    desktop.launcher_until(lambda shown: not shown["open"])
    # Found again while its pi still loads, its row reads as it will once it's up, and
    # Return goes to it rather than starting a second.
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("lighthouse", selecting="Lighthouse")
    desktop.launcher_until(
        lambda shown: (
            shown["selected"]["told"][:1] == ["\u21b5 go to"]
            and shown["selected"]["side"].endswith("listening")
        )
    )
    desktop.close_launcher()
    wakes_parked_orchestrator(desktop)
    desktop.as_account(
        f"rm -rf {opened} {sessions} ~/lighthouse {HARNESS}/quest-workflow/orchestrators.json"
        f" {HARNESS}/quest-workflow/quests.json {HARNESS}/quest-workflow/ended.json"
    )


def a_quest_offers_to_orchestrate(desktop: Desktop) -> None:
    """Start a Session lists a quest nothing orchestrates, whose step in offers Orchestrate.

    It holds Start Here, then Orchestrate, never a question asked first.
    """
    desktop.as_account(
        f"rm -f {HARNESS}/quest-workflow/orchestrators.json {PI_PRESENCE}/testqa.json"
    )
    quests_listed(desktop, [listed_quest("QEST-20261022-TESTQA", LIGHTHOUSE)])
    desktop.as_account("qs ipc call launcher collection agents")
    desktop.launcher_ready()
    desktop.down_to("Start a Session")
    desktop.machine.send_key("ret")
    desktop.launcher_until(lambda shown: LIGHTHOUSE in shown["rows"])
    desktop.down_to(LIGHTHOUSE)
    desktop.follow()
    desktop.launcher_until(lambda shown: shown["rows"][:2] == ["Start Here", "Orchestrate"])
    desktop.shoot("launcher-agents-start-quest")
    desktop.close_launcher()


def wakes_parked_orchestrator(desktop: Desktop) -> None:
    """Wake Lighthouse once it's parked, by its holder's file, rather than resume it.

    Parked while it wants Joel, its want still holds For You's mark, and its row still
    wakes it, since there's no process to go to.
    """
    machine = desktop.machine
    # Parked as the harness parks one: its process ended this boot, its
    # record saying so and naming its holder's file that wakes it, which
    # the node writes when asked to wake it, and the marker beside. Its
    # want stays in its record, as the harness keeps it when it parks.
    parked = f"{HARNESS}/park-workflow/parked"
    boot = desktop.as_account("cat /proc/sys/kernel/random/boot_id").strip()
    began = machine.succeed("date -u +%Y-%m-%dT%H:%M:%SZ").strip()
    held = desktop.as_account('echo "$XDG_RUNTIME_DIR/mycelium/harbour.held"').strip()
    record = {
        "session": "s-testqa", "addresses": ["TESTQA"], "state": "parked", "busy": False,
        "title": "\u03c0 - Lighthouse", "owner": {"pid": 1, "bootId": boot, "startTicks": 1},
        "at": "2026-11-06T12:00:00Z", "wake": f"{held}/testqa.wake",
    }  # fmt: skip
    wanting = record | {"wants": {"kind": "question", "since": began, "said": "Which way?"}}
    since = desktop.mark()
    desktop.as_account(
        f"mkdir -p {parked} {held} && echo '{json.dumps(wanting)}' > {PI_PRESENCE}/testqa.json"
        f' && printf \'{{"session":"s-testqa","wake":"%s"}}\' {held}/testqa.wake'
        f" > {parked}/s-testqa.json"
    )
    desktop.reported("holds margin ", lambda rest: "forYou" in rest.split(), since)
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("lighthouse", selecting="Lighthouse")
    # Its row is its parked session's, which Return wakes, where a closed
    # one's Return resumes it, wanting Joel or not.
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("verb") == "wake")
    # Answered, its want ends and the mark goes, though it stays parked.
    since = desktop.mark()
    desktop.as_account(f"echo '{json.dumps(record)}' > {PI_PRESENCE}/testqa.json.new")
    desktop.as_account(f"mv {PI_PRESENCE}/testqa.json.new {PI_PRESENCE}/testqa.json")
    desktop.reported("holds margin ", lambda rest: "forYou" not in rest.split(), since)
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("verb") == "wake")
    desktop.land()
    assert desktop.showing()["selected"]["told"][:1] == ["\u21b5 wake"], desktop.showing()
    since = desktop.mark()
    machine.send_key("ret")
    # The node writes it, saying who steered, as Wake asks it to.
    machine.wait_until_succeeds(desktop.in_session(f"grep -q '\"steered\"' {held}/testqa.wake"))
    # It wakes in its own window, so the launcher stays, its row reading as it will be
    # from the key's own frame, though its record still says it's parked until pi is up.
    desktop.reported("becoming ", lambda rest: rest == "agent:TESTQA", since)
    assert desktop.showing()["open"], desktop.showing()
    desktop.as_account("qs ipc call launcher collection agents")
    desktop.down_to("Lighthouse")
    desktop.launcher_until(lambda shown: shown["selected"]["side"].endswith("listening"))
    desktop.close_launcher()
    desktop.as_account(f"rm -rf {parked} {PI_PRESENCE}/testqa.json {held}/testqa.wake")


def test_agents_reopens_the_sessions_the_last_boot_took_away(desktop: Desktop) -> None:
    """Agents reopens what the last boot's end took away, Return twice, or puts it off.

    One whose folder is gone says so on its row before Return, and Reopen Them All counts
    only the other.
    """
    machine = desktop.machine
    opened = "$XDG_RUNTIME_DIR/pi-opened"
    # The harness kept two sessions live as an earlier boot ended, one in a
    # folder given back since.
    lost = {
        "v": 1,
        "boot": "an-earlier-boot",
        "sessions": [
            {
                "id": "01b2-harbour",
                "address": "HARBOR",
                "title": "Harbour",
                "cwd": "/home/jitsusama/harbour",
                "lostAt": 1,
            },
            {
                "id": "01b3-given",
                "address": "GIVENB",
                "title": "Given Back",
                "cwd": "/home/jitsusama/given-back",
                "lostAt": 1,
            },
        ],
    }
    desktop.as_account(f"rm -rf {opened} ~/given-back && mkdir -p ~/harbour")
    kept(desktop, "~/.local/state/pi/restore.json", lost)
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.down_to("Agents")
    machine.send_key("ret")
    # Its row comes after the sessions running, its count beside it.
    desktop.launcher_until(lambda shown: "Sessions to Reopen" in shown["rows"])
    desktop.down_to("Sessions to Reopen")
    selected = desktop.showing()["selected"]
    assert (selected["side"], selected["note"]) == ("2 sessions", "from the last boot"), selected
    # Return steps in, all of them first, counting what it will reopen;
    # reopening changes how things stand, so Return lands on it and a
    # second reopens them.
    machine.send_key("ret")
    desktop.launcher_until(
        lambda shown: (
            shown["rows"] == ["Reopen Them All", "Harbour", "Given Back", "Not Now"]
            and (shown["selected"] or {}).get("side") == "1 of 2 \u00b7 1 folder gone"
        )
    )
    desktop.shoot("launcher-agents-last-boot")
    desktop.down_to("Given Back")
    assert desktop.showing()["selected"]["side"] == "~/given-back is gone", desktop.showing()
    desktop.shoot("launcher-agents-last-boot-gone")
    desktop.down_to("Reopen Them All")
    desktop.land()
    assert desktop.showing()["selected"]["told"][:1] == ["\u21b5 reopen"], desktop.showing()
    machine.send_key("ret")
    machine.wait_until_succeeds(
        desktop.in_session(f'grep -qx "$HOME/harbour " {opened}'),
        timeout=SHELL_STARTS,
    )
    said = desktop.as_account(f"cat {opened}").split("\n")
    assert said[1:3] == ["--session", "01b2-harbour"], said
    # Each is told to carry on as it opens, so a boot leaves none waiting.
    assert said[3] == "--", said
    assert said[4].startswith("[Automated message, no human input: "), said
    assert said[4].endswith("Carry on from where you left off: your plan's next open item."), said
    desktop.launcher_until(lambda shown: not shown["open"])
    # Put off, it's gone for this boot, as no harness here drops it.
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("to reopen", selecting="Sessions to Reopen")
    machine.send_key("ret")
    desktop.launcher_until(lambda shown: "Not Now" in shown["rows"])
    desktop.down_to("Not Now")
    # Putting off ends what's offered this boot, so Return writes :snooze, and its line
    # asks first.
    desktop.line_answered("snooze")
    # Nothing is left to reopen, so the launcher closes.
    desktop.launcher_until(lambda shown: not shown["open"])
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.down_to("Agents")
    machine.send_key("ret")
    desktop.launcher_until(
        lambda shown: shown["held"] == ["agents"] and "Start a Session" in shown["rows"]
    )
    assert "Sessions to Reopen" not in desktop.showing()["rows"], desktop.showing()
    desktop.close_launcher()
    desktop.as_account(f"rm -rf {opened} ~/harbour")


def test_agents_reopens_a_session_another_machine_carried_here(desktop: Desktop) -> None:
    """Agents offers a session another machine carried here whatever the boot, without Not Now."""
    machine = desktop.machine
    opened = "$XDG_RUNTIME_DIR/pi-opened"
    restore = "~/.local/state/pi/restore.json"
    # This boot's own file, so nothing of the last boot's is offered, but
    # for one session another machine handed here as it went away.
    boot = desktop.as_account("cat /proc/sys/kernel/random/boot_id").strip()
    at = "2026-10-30T12:00:00Z"
    ferry = {
        "id": "01b2-ferry",
        "address": "FERRYQ",
        "title": "Ferry",
        "cwd": "/home/jitsusama/ferry",
        "lostAt": at,
        "carried": {"from": "lighthouse", "at": at},
    }
    desktop.as_account(f"rm -f {opened} && mkdir -p ~/ferry")
    kept(desktop, restore, {"v": 1, "boot": boot, "sessions": [ferry]})
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("to reopen", selecting="Sessions to Reopen")
    selected = desktop.showing()["selected"]
    assert (selected["side"], selected["note"]) == ("1 session", "carried from lighthouse"), (
        selected
    )
    # Inside there's no Not Now, which puts off only the last boot's.
    machine.send_key("ret")
    desktop.launcher_until(lambda shown: shown["rows"] == ["Reopen Them All", "Ferry"])
    desktop.land()
    machine.send_key("ret")
    machine.wait_until_succeeds(
        desktop.in_session(f'grep -qx "$HOME/ferry " {opened}'),
        timeout=SHELL_STARTS,
    )
    said = desktop.as_account(f"cat {opened}").split("\n")
    assert said[1:3] == ["--session", "01b2-ferry"], said
    desktop.launcher_until(lambda shown: not shown["open"])
    # Once it runs here the harness drops it, and the row goes with it.
    kept(desktop, restore, {"v": 1, "boot": boot, "sessions": []})
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.down_to("Agents")
    machine.send_key("ret")
    desktop.launcher_until(
        lambda shown: (
            shown["held"] == ["agents"]
            and "Start a Session" in shown["rows"]
            and "Sessions to Reopen" not in shown["rows"]
        )
    )
    desktop.close_launcher()
    desktop.as_account(f"rm -rf {opened} ~/ferry")


def test_the_shell_reopens_the_sessions_the_harness_hands_it(desktop: Desktop) -> None:
    """The shell takes the sessions the harness hands it, answering at once, and reopens each.

    One handed over as the one before it had just opened was dropped, as the shell settled where
    that one sat, and never opened (2026-11-04, a second spawn 0.7 s after the first).
    """
    opened = "$XDG_RUNTIME_DIR/pi-opened"
    handed = "$XDG_RUNTIME_DIR/handed.json"
    desktop.as_account(f"rm -f {opened} && mkdir -p ~/quay")

    def handing(sessions: dict[str, Any]) -> str:
        """Hand the shell these, as the harness does, and give its answer."""
        kept(desktop, handed, sessions)
        return desktop.as_account(f'qs ipc call sessions reopen "$(cat {handed})"').strip()

    # What isn't a list of sessions, or names none whole, is refused.
    answer = handing({"id": "01b2-quay"})
    assert answer == "refused: not a list of sessions", answer
    answer = handing({"sessions": [{"id": "", "cwd": "/home/jitsusama/quay"}]})
    assert answer == "refused: no session with an id and a folder", answer
    # The move onto pi's host asks the shell to keep a session's place, and hears whether it
    # could; one with no window here keeps none (tests/sessions.qml holds the rest).
    answer = desktop.as_account("qs ipc call sessions keepPlace 01b2-nowhere").strip()
    assert answer == "no window to keep", answer
    # One whole, as `quest restore` hands it over, is taken at once.
    answer = handing({"sessions": [{"id": "01b2-quay", "cwd": "/home/jitsusama/quay"}]})
    assert answer == "taken", answer
    desktop.machine.wait_until_succeeds(
        desktop.in_session(f'grep -qx "$HOME/quay " {opened}'),
        timeout=SHELL_STARTS,
    )
    said = desktop.as_account(f"cat {opened}").split("\n")
    # The harness says for itself what a session it hands over is to do.
    assert said[1:] == ["--session", "01b2-quay", ""], said
    desktop.as_account(f"rm -rf {opened} {handed}")
    # A second handed over the moment the first's window shows, in one shell on the machine so
    # it lands while the shell settles the first, and not a command's round trip later.
    ahead = {"sessions": [{"id": "01b2-ahead", "cwd": "/home/jitsusama/quay"}]}
    behind = {"sessions": [{"id": "01b2-behind", "cwd": "/home/jitsusama/quay"}]}
    shown = "niri msg --json windows | grep -q 'pi --session 01b2-{}'"
    desktop.as_account("touch $XDG_RUNTIME_DIR/pi-stays")
    try:
        answers = desktop.machine.succeed(
            desktop.in_session(
                f"qs ipc call sessions reopen {shlex.quote(json.dumps(ahead))}"
                f" && until {shown.format('ahead')}; do sleep 0.02; done"
                f" && qs ipc call sessions reopen {shlex.quote(json.dumps(behind))}"
            ),
            timeout=300,
        ).split()
        assert answers == ["taken", "taken"], answers
        desktop.machine.wait_until_succeeds(
            desktop.in_session(shown.format("behind")), timeout=dt.timedelta(seconds=60)
        )
    finally:
        desktop.as_account("rm -f $XDG_RUNTIME_DIR/pi-stays; pkill -f 'sleep 60[0]'; true")
        desktop.as_account("rm -rf ~/quay")


def test_a_session_another_starts_leaves_every_key_where_joel_types(desktop: Desktop) -> None:
    """A session the harness hands over while Joel types opens aside, and every key stays his.

    He types into the session that starts it, a window of the kitty that draws every session,
    so the new one opens beside his, in the kitty that has his keys (Joel, 2026-11-09: "when
    new pi sessions via kitty are launched, it still steals focus for a bit, flashes my screen,
    and often swallows part of what i am typing"). A kitty of its own, or a sender elsewhere,
    never had his keys to give away. It opens while the sessions work, each busy in a window of
    that kitty, as on his machine, and the window he types in never changes on his screen
    (Joel, 2026-11-10: "we need to not have my screen flash when these are loaded either").
    """
    machine = desktop.machine
    owner = "$XDG_RUNTIME_DIR/asideqa-owner"
    # What he types isn't echoed and the cursor is hidden, so the window he
    # types in has nothing of its own to change: any frame of it that does
    # is the spawn's doing.
    desktop.as_account(
        f"touch $XDG_RUNTIME_DIR/pi-stays && mkdir -p ~/aside {PI_PRESENCE} && rm -f ~/typed"
        f" && printf '%s\\n' '#!/bin/sh' 'echo \"$$ $KITTY_LISTEN_ON $KITTY_WINDOW_ID\" > {owner}'"
        " 'stty -echo -icanon' 'printf \"\\33[?25l\"'"
        " 'exec cat > ~/typed' > ~/.asideqa-pi && chmod +x ~/.asideqa-pi"
        " && echo Notes > $XDG_RUNTIME_DIR/asideqa-title"
        f" && niri msg action spawn -- {IN_SESSIONS} --os-window-class notes"
        " --title Notes ~/.asideqa-pi"
    )
    machine.wait_until_succeeds(desktop.in_session(f"test -s {owner}"), timeout=SHELL_STARTS)
    desktop.presence_says(
        {"session": "01c7-sender", "quest": "QEST-20261109-ASIDEQ", "addresses": ["ASIDEQA"]},
        address="ASIDEQA",
    )
    notes = next(window for window in desktop.windows() if window["app_id"] == "notes")
    # Thirty sessions at work on a workspace of their own, each a processor's
    # worth in its own window's scope beside the kitty drawing them all, as
    # Joel's 71 were on 2026-11-09. A window niri opens starts wherever niri
    # runs, where stress-ng may not write, so it writes to /tmp; and each
    # holds its window should stress-ng end, so a count that falls short is
    # the work ending, not the windows.
    stress_ng = f"{desktop.values['stress_ng']}/bin/stress-ng"
    stress = f"{stress_ng} --temp-path /tmp --cpu 1 --timeout 900"
    desktop.as_account(
        "niri msg action focus-workspace 3; for i in $(seq 30); do niri msg action spawn --"
        f" {IN_SESSIONS} --os-window-class crowd --hold {stress}; done"
    )
    machine.wait_until_succeeds(
        desktop.in_session("[ $(niri msg --json windows | grep -o '\"crowd\"' | wc -l) -ge 30 ]"),
        timeout=dt.timedelta(seconds=180),
    )
    # A window is drawn before the work in it starts, so the work is waited on too.
    machine.wait_until_succeeds(
        desktop.in_session(f"[ $(pgrep -c -x stress-ng) -ge {CROWD} ]"),
        timeout=dt.timedelta(seconds=60),
    )
    desktop.as_account(f"niri msg action focus-window --id {notes['id']}")
    box = window_box(desktop, notes["id"])
    try:
        # Every focus niri gives from here on, and Joel typing numbered words
        # until told to stop, so a key that went elsewhere leaves a gap. Ten
        # words a wtype, since each brings its own keymap, which Xwayland
        # compiles: a word each, a loaded builder's slow window let it type
        # 331, and Xwayland was still compiling them a minute into the next
        # subtest, starving it (2026-11-03).
        watch_the_focus(desktop)
        typing = (
            "i=0; while [ ! -e ~/typing-done ]; do"
            ' wtype "$(seq -f "w%.0f " $((i+1)) $((i+10)) | tr -d "\\n")"; i=$((i+10));'
            " echo $i > ~/typing-count; done; wtype -k Return"
        )
        desktop.as_account(f"setsid sh -c {shlex.quote(typing)} > /dev/null 2>&1 &")
        record_the_screen(desktop)
        # As a quest's spawn hands it over: the one that asked, no focus asked for.
        aside = {"id": "01c7-aside", "cwd": "/home/jitsusama/aside", "from": "01c7-sender"}
        handed = {"sessions": [{**aside, "focus": False}]}
        answer = desktop.as_account(
            f"qs ipc call sessions reopen {shlex.quote(json.dumps(handed))}"
        ).strip()
        assert answer == "taken", answer
        machine.wait_until_succeeds(
            desktop.in_session("niri msg --json windows | grep -q 'pi --session 01c7-aside'"),
            timeout=dt.timedelta(seconds=300),
        )
        # Typed on past the window showing and the shell placing it.
        since = int(desktop.as_account("cat ~/typing-count").strip() or "0")
        machine.wait_until_succeeds(
            desktop.in_session(f"[ $(cat ~/typing-count) -ge {since + 20} ]"),
            timeout=dt.timedelta(seconds=120),
        )
        desktop.as_account("touch ~/typing-done")
        changed = frames_changed(desktop, box)
        machine.wait_until_succeeds(
            desktop.in_session("[ -s ~/typed ]"), timeout=dt.timedelta(seconds=60)
        )
        typed = desktop.as_account("cat ~/typed").split()
        assert typed == [f"w{n}" for n in range(1, len(typed) + 1)], typed
        windows, workspaces = focus_given(desktop)
        assert set(windows) <= {notes["id"]}, windows
        assert set(workspaces) <= {notes["workspace_id"]}, workspaces
        assert changed == [], f"the window Joel types in changed in frames {changed}"
    finally:
        desktop.as_account(
            "touch ~/typing-done; pkill -f 'event-strea[m]'; rm -f $XDG_RUNTIME_DIR/pi-stays;"
            f" pkill -f 'sleep 60[0]'; niri msg action close-window --id {notes['id']};"
            f" rm -f {PI_PRESENCE}/asideqa.json {owner}; pkill -x stress-ng;"
            " pkill -INT -x wf-recorder; true"
        )
        desktop.as_account(
            "rm -rf ~/aside ~/typed ~/typing-done ~/typing-count ~/focus-events ~/.asideqa-pi"
        )


def test_a_session_another_restores_goes_to_its_workspace_and_leaves_joel_his_focus(
    desktop: Desktop,
) -> None:
    """Sessions handed back with places kept go each to its kept workspace, never focused."""
    boot, sessions, notes = two_kept_beside_notes(desktop, "01c8-near", "01c8-far")
    try:
        watch_the_focus(desktop)
        # As `quest restore` hands them over: no focus asked for.
        handed = shlex.quote(json.dumps({"sessions": sessions}))
        answer = desktop.as_account(f"qs ipc call sessions reopen {handed}").strip()
        assert answer == "taken", answer
        deadline = time.monotonic() + 300
        while not apart_as_kept(back := sat(desktop, "01c8-")) and time.monotonic() < deadline:
            time.sleep(1)
        assert apart_as_kept(back), back
        windows, workspaces = focus_given(desktop)
        assert set(windows) <= {notes["id"]}, windows
        assert set(workspaces) <= {notes["workspace_id"]}, workspaces
    finally:
        desktop.as_account(
            "pkill -f 'event-strea[m]'; rm -f $XDG_RUNTIME_DIR/pi-stays; pkill -f 'sleep 60[0]'"
        )
        kept(desktop, "~/.local/state/pi/restore.json", {"v": 1, "boot": boot, "sessions": []})
        desktop.as_account("rm -rf ~/near ~/far ~/focus-events")


def test_a_session_another_starts_goes_to_its_senders_workspace_from_the_sessions_kitty(
    desktop: Desktop,
) -> None:
    """A session another starts goes to its sender's workspace, the sender in the sessions' kitty.

    On 2026-11-03 nine of ten builder sessions landed on the workspace Joel was on rather than
    their senders'. Each sender ran in the kitty that holds every session, and a window of
    that kitty is known only once kitty has said which of its windows is whose, which the
    new session's own window had just made stale.
    """
    machine = desktop.machine
    owner = "$XDG_RUNTIME_DIR/sendqa-owner"
    desktop.as_account("touch $XDG_RUNTIME_DIR/pi-stays && mkdir -p ~/first ~/spawned")
    stand_in(desktop, "notes", "Notes")
    notes = next(window for window in desktop.windows() if window["app_id"] == "notes")
    try:
        # The sessions' kitty, started by the shell for a session of its own.
        first = {"sessions": [{"id": "01ce-first", "cwd": "/home/jitsusama/first"}]}
        answer = desktop.as_account(
            f"qs ipc call sessions reopen {shlex.quote(json.dumps(first))}"
        ).strip()
        assert answer == "taken", answer
        machine.wait_until_succeeds(
            desktop.in_session("niri msg --json windows | grep -q 'pi --session 01ce-first'"),
            timeout=dt.timedelta(seconds=300),
        )
        # The sender, a second window of that kitty, with pi's record of it.
        desktop.as_account(
            f"printf '%s\\n' '#!/bin/sh' 'echo \"$$ $KITTY_LISTEN_ON $KITTY_WINDOW_ID\" > {owner}'"
            " 'exec sleep infinity' > ~/.sendqa-pi && chmod +x ~/.sendqa-pi"
            " && echo Sender > $XDG_RUNTIME_DIR/sendqa-title"
            f" && niri msg action spawn -- {IN_SESSIONS} --os-window-class kitty --title Sender"
            " ~/.sendqa-pi"
        )
        machine.wait_until_succeeds(desktop.in_session(f"test -s {owner}"), timeout=SHELL_STARTS)
        desktop.presence_says(
            {"session": "01ce-sender", "quest": "QEST-20261103-SENDQA", "addresses": ["SENDQA"]},
            address="SENDQA",
        )
        sender = desktop.window_titled("Sender")
        # On the second of three workspaces with windows, neither focused nor last.
        desktop.as_account(
            f"niri msg action move-window-to-workspace --window-id {sender['id']} --focus false 2"
        )
        stand_in(desktop, "tide", "Tide")
        tide = next(window for window in desktop.windows() if window["app_id"] == "tide")
        desktop.as_account(
            f"niri msg action move-window-to-workspace --window-id {tide['id']} --focus false 3"
            f" && niri msg action focus-window --id {notes['id']}"
        )
        sender = desktop.window_titled("Sender")
        notes = desktop.window_titled("Notes")
        assert sender["workspace_id"] != notes["workspace_id"], (sender, notes)
        watch_the_focus(desktop)
        # As a quest's spawn hands one over: its own quest, the one that asked, no focus.
        spawned = {
            "id": "01ce-spawned",
            "cwd": "/home/jitsusama/spawned",
            "quest": "QEST-20261103-SPAWNQ",
            "from": "01ce-sender",
            "focus": False,
        }
        handed = shlex.quote(json.dumps({"sessions": [spawned]}))
        answer = desktop.as_account(f"qs ipc call sessions reopen {handed}").strip()
        assert answer == "taken", answer
        machine.wait_until_succeeds(
            desktop.in_session("niri msg --json windows | grep -q 'pi --session 01ce-spawned'"),
            timeout=dt.timedelta(seconds=300),
        )

        def where() -> int:
            """Give the workspace the session started sits on."""
            return next(
                int(window["workspace_id"])
                for window in desktop.windows()
                if "pi --session 01ce-spawned" in (window["title"] or "")
            )

        # Opened there, rather than where Joel is and moved after (Joel, 2026-11-07).
        assert first_on(desktop, "pi --session 01ce-spawned") == sender["workspace_id"], (
            f"not opened on its sender's workspace {sender['workspace_id']}"
            f" but {first_on(desktop, 'pi --session 01ce-spawned')},"
            f" Notes' being {notes['workspace_id']}"
        )
        assert where() == sender["workspace_id"], where()
        windows, workspaces = focus_given(desktop)
        assert set(windows) <= {notes["id"]}, windows
        assert set(workspaces) <= {notes["workspace_id"]}, workspaces
    finally:
        desktop.as_account(
            f"pkill -f 'event-strea[m]'; rm -f $XDG_RUNTIME_DIR/pi-stays {PI_PRESENCE}/sendqa.json"
            f"; pkill -f 'sleep 60[0]'; kill $(cut -d' ' -f1 {owner}); rm -f {owner}; true"
        )
        desktop.as_account("rm -rf ~/first ~/spawned ~/.sendqa-pi ~/focus-events")


def test_sessions_handed_over_while_locked_open_behind_the_lock_and_go_to_their_place_after(
    desktop: Desktop,
) -> None:
    """Sessions handed over while the screen is locked open behind it, and are placed after.

    niri dropped a program asked for over its socket while locked, so a session a quest started
    overnight was lost outright (2026-11-03). Behind the lock only the lock is drawn and the
    keys go to it; once it's let go, each goes to its kept workspace, and nothing is focused.
    """
    machine = desktop.machine
    boot, sessions, notes = two_kept_beside_notes(desktop, "01cd-dusk", "01cd-dawn")

    def apart(back: dict[str, tuple[int, int]]) -> bool:
        """Whether both are back, dawn on the workspace next after dusk's, as kept."""
        return len(back) == 2 and back["01cd-dawn"][0] == back["01cd-dusk"][0] + 1  # noqa: PLR2004

    try:
        with desktop.told("keys lock"):
            machine.send_key("meta_l-l")
        desktop.as_account("qs ipc call lock isLocked | grep -q true")
        locked = desktop.settled_screen("locked-before-the-hand-over")
        watch_the_focus(desktop)
        # As a quest's spawn hands them over: no focus asked for.
        handed = shlex.quote(json.dumps({"sessions": sessions}))
        answer = desktop.as_account(f"qs ipc call sessions reopen {handed}").strip()
        assert answer == "taken", answer
        deadline = time.monotonic() + 300
        while len(back := sat(desktop, "01cd-")) < 2 and time.monotonic() < deadline:  # noqa: PLR2004
            time.sleep(1)
        assert len(back) == 2, f"not both opened behind the lock: {back}"  # noqa: PLR2004
        # The sky over the lock's time and field, which nothing else draws on.
        behind = desktop.settled_screen("locked-after-the-hand-over")
        sky = (0, 0, behind.width, behind.height * 2 // 5)
        assert behind.crop(sky).tobytes() == locked.crop(sky).tobytes(), "a window showed"
        with desktop.told("refused lock"):
            machine.send_chars("not the secret\n")
        with desktop.told("released lock"):
            machine.send_chars(f"{desktop.values['secret']}\n")
        deadline = time.monotonic() + 120
        while not apart(back := sat(desktop, "01cd-")) and time.monotonic() < deadline:
            time.sleep(1)
        assert apart(back), f"not on their kept workspaces once unlocked: {back}"
        windows, workspaces = focus_given(desktop)
        assert set(windows) <= {notes["id"], None}, windows
        assert set(workspaces) <= {notes["workspace_id"]}, workspaces
    finally:
        still, _ = machine.execute(desktop.in_session("qs ipc call lock isLocked | grep -q true"))
        if still == 0:
            with desktop.told("released lock"):
                machine.send_chars(f"{desktop.values['secret']}\n")
        desktop.as_account(
            "pkill -f 'event-strea[m]'; rm -f $XDG_RUNTIME_DIR/pi-stays; pkill -f 'sleep 60[0]'"
        )
        kept(desktop, "~/.local/state/pi/restore.json", {"v": 1, "boot": boot, "sessions": []})
        desktop.as_account("rm -rf ~/dusk ~/dawn ~/focus-events")


def two_kept_beside_notes(
    desktop: Desktop, first: str, second: str
) -> tuple[str, list[dict[str, str]], Json]:
    """Keep two sessions from a boot before, on the first and second workspace, Notes focused.

    Each session's folder is its id past the prefix, in the home folder, and each stays open
    (pi-stays). Gives this boot's id, the sessions to hand over, and Notes' window.
    """
    boot = desktop.as_account("cat /proc/sys/kernel/random/boot_id").strip()
    output = next(iter(json.loads(desktop.as_account("niri msg --json outputs"))))
    where = {
        first: {"output": output, "workspace": 1, "column": 1, "tile": 1},
        second: {"output": output, "workspace": 2, "column": 1, "tile": 1},
    }
    sessions = [{"id": one, "cwd": f"/home/jitsusama/{one[5:]}"} for one in where]
    desktop.as_account(f"touch $XDG_RUNTIME_DIR/pi-stays && mkdir -p ~/{first[5:]} ~/{second[5:]}")
    lost = {"v": 1, "boot": "a-boot-before", "sessions": sessions}
    kept(desktop, "~/.local/state/pi/restore.json", lost)
    kept(desktop, ARRANGED, {"v": 1, "boots": {"a-boot-before": where}})
    stand_in(desktop, "notes", "Notes")
    notes = next(window for window in desktop.windows() if window["app_id"] == "notes")
    desktop.as_account(f"niri msg action focus-window --id {notes['id']}")
    return boot, sessions, notes


def apart_as_kept(back: dict[str, tuple[int, int]]) -> bool:
    """Whether near and far sit as kept, far on the workspace next after near's.

    A workspace kept holds only its own sessions, but a window of Joel's may sit
    on it, so near may share Notes' workspace.
    """
    if len(back) != 2:  # noqa: PLR2004
        return False
    return back["01c8-far"][0] == back["01c8-near"][0] + 1


def record_the_screen(desktop: Desktop) -> None:
    """Record every frame of the focused screen from now on, for frames_changed to read."""
    output = (
        'niri msg --json focused-output | grep -o \'"name":"[^"]*"\' | head -1 | cut -d\'"\' -f4'
    )
    # Every frame, damaged or not (-D), as the looks area records a flash,
    # so a still window is a run of equal frames rather than none.
    desktop.as_account(
        f'rm -f {RECORDING}; setsid -f wf-recorder -y -D --output "$({output})"'
        f" --file {RECORDING} </dev/null >{RECORDED} 2>&1"
    )
    began, _ = desktop.machine.execute(
        f"timeout 20 sh -c 'until grep -q \"selected region\" {RECORDED}; do sleep 0.1; done'"
    )
    assert began == 0, f"wf-recorder never began: {desktop.machine.execute(f'cat {RECORDED}')[1]}"


def frames_changed(desktop: Desktop, box: tuple[int, int, int, int]) -> list[int]:
    """Stop recording, and give each frame in which the box differs from the first.

    A frame differs where over a five-hundredth of its pixels are lighter or darker by more
    than the encoder's own noise. The first that does is kept in the gallery, for a person.
    """
    machine = desktop.machine
    machine.succeed("pkill -INT -x wf-recorder")
    machine.wait_until_fails("pgrep -x wf-recorder", timeout=dt.timedelta(seconds=30))
    x, y, width, height = box
    ffmpeg = f"{desktop.values['ffmpeg_headless']}/bin/ffmpeg -v error"
    machine.succeed(
        f"rm -rf {FRAMES} && mkdir -p {FRAMES} && {ffmpeg} -i {RECORDING}"
        f" -vf crop={width}:{height}:{x}:{y},fps=30 {FRAMES}/%05d.png"
        f" && tar -C {FRAMES} -cf {FRAMES}.tar ."
    )
    machine.copy_from_machine(f"{FRAMES}.tar")
    kept = machine.out_dir / "spawn-frames.tar"
    frames = []
    with tarfile.open(kept) as tar:
        for member in sorted(tar.getmembers(), key=lambda member: member.name):
            read = tar.extractfile(member)
            if read is not None:
                frames.append(Image.open(read).convert("L"))
    kept.unlink()
    assert len(frames) > 1, f"the recording held {len(frames)} frames"
    changed: list[int] = []
    for at, frame in enumerate(frames):
        apart = ImageChops.difference(frame, frames[0]).point(
            lambda light: 255 * (light > ENCODER_NOISE)
        )
        if sum(apart.histogram()[255:]) * CHANGED_SHARE > frame.width * frame.height:
            if not changed:
                frame.save(machine.out_dir / "spawn-changed.png")
            changed.append(at)
    return changed


def first_on(desktop: Desktop, titled: str) -> int | None:
    """Give the workspace niri first said a window titled so was on, since watch_the_focus."""
    ids = {window["id"] for window in desktop.windows() if titled in (window["title"] or "")}
    said = [json.loads(line) for line in desktop.as_account("cat ~/focus-events").splitlines()]
    return next(
        (
            one["WindowOpenedOrChanged"]["window"]["workspace_id"]
            for one in said
            if one.get("WindowOpenedOrChanged", {}).get("window", {}).get("id") in ids
        ),
        None,
    )


def sat(desktop: Desktop, prefix: str) -> dict[str, tuple[int, int]]:
    """Give each stand-in session's workspace and column, by its session, as niri lists them."""
    spaces = json.loads(desktop.as_account("niri msg --json workspaces"))
    idx = {one["id"]: one["idx"] for one in spaces}
    return {
        # "pi --session <id>", then what it was told, if anything.
        window["title"].split()[2]: (
            idx[window["workspace_id"]],
            window["layout"]["pos_in_scrolling_layout"][0],
        )
        for window in desktop.windows()
        if window["title"].startswith(f"pi --session {prefix}")
    }


def test_agents_reopens_each_session_where_it_sat(desktop: Desktop) -> None:
    """Where each session sits is kept, and Reopen Them All puts each back there, in order."""
    machine = desktop.machine
    # A session's workspace, column and tile are kept by its own id for
    # this boot, once its windows have been still a moment.
    desktop.pi_session()
    arranged = ARRANGED
    machine.wait_until_succeeds(
        desktop.in_session(f"grep -q 01a1face-test {arranged}"), timeout=dt.timedelta(seconds=30)
    )
    boot = desktop.as_account("cat /proc/sys/kernel/random/boot_id").strip()
    pi = desktop.window_titled(PI_WINDOW)
    spaces = json.loads(desktop.as_account("niri msg --json workspaces"))
    space = next(one for one in spaces if one["id"] == pi["workspace_id"])
    at = pi["layout"]["pos_in_scrolling_layout"]
    held = json.loads(desktop.as_account(f"cat {arranged}"))["boots"][boot]["01a1face-test"]
    # Its screen is kept by what it is as well as its connector, as niri's
    # config names one, since a dock or a boot can move it to another, and
    # whether it leads others, which this one doesn't, so Reopen Them All
    # begins orchestrators first.
    output = json.loads(desktop.as_account("niri msg --json outputs"))[space["output"]]
    screen = " ".join(part for part in (output["make"], output["model"], output["serial"]) if part)
    assert held == {
        "screen": screen,
        "output": space["output"],
        "workspace": space["idx"],
        "column": at[0],
        "tile": at[1],
        "leads": False,
    }, held
    # The last boot's three, listed out of order: two on the first
    # workspace, west then east, and one on the second, kept on this
    # screen when it was on another connector.
    elsewhere = {"screen": screen, "output": "DP-9"}
    where = {
        "01c3-east": {**elsewhere, "workspace": 1, "column": 2, "tile": 1},
        "01c3-north": {**elsewhere, "workspace": 2, "column": 1, "tile": 1},
        "01c3-west": {**elsewhere, "workspace": 1, "column": 1, "tile": 1},
    }
    lost = {
        "v": 1,
        "boot": "a-boot-before",
        "sessions": [
            {"id": one, "title": one[5:].title(), "cwd": f"/home/jitsusama/{one[5:]}"}
            for one in where
        ],
    }
    desktop.as_account("touch $XDG_RUNTIME_DIR/pi-stays && mkdir -p ~/east ~/north ~/west")
    # Its sessions end however it ends, so a failure here leaves the next
    # subtest no sessions of its own to trip on.
    try:
        kept(desktop, "~/.local/state/pi/restore.json", lost)
        kept(desktop, arranged, {"v": 1, "boots": {"a-boot-before": where}})
        desktop.as_account("qs ipc call launcher toggle")
        desktop.launcher_ready()
        desktop.type_in("to reopen", selecting="Sessions to Reopen")
        machine.send_key("ret")
        desktop.launcher_until(lambda shown: shown["rows"][:1] == ["Reopen Them All"])
        desktop.land()
        # kitty focuses a window it made with the token niri gave it, and on a
        # busy machine that can come after the next session's workspace was
        # focused, so north opens on the first. This stands in for that, every
        # time: the first workspace is focused again until north is there.
        pulling = (
            "for _ in $(seq 400); do niri msg --json windows | grep -q 'session 01c3-north'"
            " && break; niri msg action focus-workspace 1; sleep 0.05; done"
        )
        desktop.as_account(f"setsid sh -c {shlex.quote(pulling)} > /dev/null 2>&1 &")
        watch_the_focus(desktop)
        since = desktop.mark()
        machine.send_key("ret")
        opening = "niri msg --json windows | grep -o 'pi --session 01c3-' | wc -l"
        machine.wait_until_succeeds(
            desktop.in_session(f"[ $({opening}) = 3 ]"), timeout=dt.timedelta(seconds=60)
        )
        desktop.launcher_until(lambda shown: not shown["open"])
        # Each began two seconds or more after the one before, so they don't
        # all start at once. The shell paces them by its own clock, and the
        # journal stamps each line as it reads it, late on a busy machine: one
        # read late once made a gap of 1.89 s between two the shell began 2 s
        # apart.
        began = started_apart(desktop, since, "report[resumed session 01c3-")
        assert len(began) == len(where), began
        assert all(gap >= 2 - JOURNAL_LAG for gap in began[1:]), began
        back = sat(desktop, "01c3-")
        # Each workspace's sessions go back together, in order, on a workspace
        # of their own: by who sits with whom, never by the number a workspace
        # had last boot, which the windows already here have taken (Joel,
        # 2026-11-09: one placer puts every window back by identity).
        assert back["01c3-west"][0] == back["01c3-east"][0], back
        assert back["01c3-west"][1] < back["01c3-east"][1], back
        assert back["01c3-north"][0] == back["01c3-west"][0] + 1, back
        # Each opened there, never on the workspace focused and moved after,
        # though the focus was on the first while north opened. A title goes on
        # past its session with what it was told, so it's found by what it
        # begins with.
        for one in where:
            titled = f"pi --session {one}"
            window = next(each for each in desktop.windows() if each["title"].startswith(titled))
            assert first_on(desktop, titled) == window["workspace_id"], one
    finally:
        # Each pattern bracketed, so pkill doesn't match the shell it runs in.
        desktop.as_account(
            "rm -rf $XDG_RUNTIME_DIR/pi-stays ~/east ~/north ~/west ~/focus-events;"
            " pkill -f 'sleep 60[0]'; pkill -f 'focus-workspac[e] 1';"
            " pkill -f 'event-strea[m]'; true"
        )
        # Nothing's left to reopen once this boot is the one kept.
        kept(desktop, "~/.local/state/pi/restore.json", {"v": 1, "boot": boot, "sessions": []})


def test_agents_reopens_each_session_where_it_sat_whatever_joel_does_meanwhile(
    desktop: Desktop,
) -> None:
    """Each session goes back where it sat while Joel moves the focus and the workspace."""
    machine = desktop.machine
    arranged = ARRANGED
    boot = desktop.as_account("cat /proc/sys/kernel/random/boot_id").strip()
    outputs = json.loads(desktop.as_account("niri msg --json outputs"))
    output, described = next(iter(outputs.items()))
    screen = " ".join(
        part for part in (described["make"], described["model"], described["serial"]) if part
    )

    # Two workspaces' sessions: west, then east with one under it, then
    # north and south. niri opens a window beside the column focused, so
    # while they open, Joel goes back to the first column of one workspace
    # and then the other, as a click or a key would.
    def place(workspace: int, column: int, tile: int = 1) -> dict[str, Any]:
        return {
            "screen": screen,
            "output": output,
            "workspace": workspace,
            "column": column,
            "tile": tile,
        }

    where = {
        "01c5-west": place(1, 1),
        "01c5-east": place(1, 2),
        "01c5-under": place(1, 2, 2),
        "01c5-north": place(2, 1),
        "01c5-south": place(2, 2),
    }
    lost = {
        "v": 1,
        "boot": "a-boot-before",
        "sessions": [
            {"id": one, "title": one[5:].title(), "cwd": f"/home/jitsusama/{one[5:]}"}
            for one in where
        ],
    }
    desktop.as_account(
        "touch $XDG_RUNTIME_DIR/pi-stays && mkdir -p ~/west ~/east ~/under ~/north ~/south"
    )
    kept(desktop, "~/.local/state/pi/restore.json", lost)
    kept(desktop, arranged, {"v": 1, "boots": {"a-boot-before": where}})
    opening = "niri msg --json windows | grep -o 'pi --session 01c5-' | wc -l"
    meddling = (
        f"for i in $(seq 600); do [ $({opening}) = 5 ] && break;"
        " niri msg action focus-workspace $((i % 2 + 1)); niri msg action focus-column-first;"
        " sleep 0.05; done"
    )
    try:
        desktop.as_account("qs ipc call launcher toggle")
        desktop.launcher_ready()
        desktop.type_in("to reopen", selecting="Sessions to Reopen")
        machine.send_key("ret")
        desktop.launcher_until(lambda shown: shown["rows"][:1] == ["Reopen Them All"])
        desktop.land()
        desktop.as_account(f"setsid sh -c {shlex.quote(meddling)} > /dev/null 2>&1 &")
        machine.send_key("ret")
        machine.wait_until_succeeds(
            desktop.in_session(f"[ $({opening}) = 5 ]"), timeout=dt.timedelta(seconds=90)
        )
        desktop.launcher_until(lambda shown: not shown["open"])
        deadline = time.monotonic() + 30
        while not sat_as_kept(back := sat(desktop, "01c5-")) and time.monotonic() < deadline:
            time.sleep(1)
        assert sat_as_kept(back), back
    finally:
        desktop.as_account(
            "pkill -f 'focus-column-firs[t]'; rm -f $XDG_RUNTIME_DIR/pi-stays;"
            " pkill -f 'sleep 60[0]'"
        )
        kept(desktop, "~/.local/state/pi/restore.json", {"v": 1, "boot": boot, "sessions": []})
        desktop.as_account("rm -rf ~/west ~/east ~/under ~/north ~/south")


def test_agents_reopens_each_session_where_it_sat_though_joel_opens_a_terminal(
    desktop: Desktop,
) -> None:
    """A terminal Joel opens while sessions reopen isn't taken for one, and each goes back."""
    machine = desktop.machine
    boot = desktop.as_account("cat /proc/sys/kernel/random/boot_id").strip()
    output = next(iter(json.loads(desktop.as_account("niri msg --json outputs"))))
    where = {
        "01c9-first": {"output": output, "workspace": 1, "column": 1, "tile": 1},
        "01c9-second": {"output": output, "workspace": 1, "column": 2, "tile": 1},
        "01c9-third": {"output": output, "workspace": 2, "column": 1, "tile": 1},
    }
    sessions = [{"id": one, "cwd": f"/home/jitsusama/{one[5:]}"} for one in where]
    desktop.as_account("touch $XDG_RUNTIME_DIR/pi-stays && mkdir -p ~/first ~/second ~/third")
    kept(
        desktop,
        "~/.local/state/pi/restore.json",
        {"v": 1, "boot": "a-boot-before", "sessions": sessions},
    )
    kept(desktop, ARRANGED, {"v": 1, "boots": {"a-boot-before": where}})
    # The kitty that holds the sessions, started and then held still, so the
    # first session's window waits on it and Joel's own terminal shows
    # first, as it can when that kitty is slow, as it is starting cold.
    sessions_kitty = f"pkill -{{}} -f '{SESSIONS_KITTY}'"
    # The first zsh in a fresh machine makes its completion dump, which
    # takes minutes on a loaded builder; made here, it doesn't hold each
    # session's title back past the wait below.
    desktop.as_account("zsh -lic true")
    # Untitled, and classed only as kitty's own: a window asked of it with no
    # class would take the sessions kitty's hidden one (kitty's boss.py
    # add_os_window), which the shell never lists.
    kitties = 'niri msg --json windows | grep -o \'"app_id":"kitty"\' | wc -l'
    before = int(desktop.as_account(kitties))
    desktop.as_account(f"niri msg action spawn -- {IN_SESSIONS} --os-window-class kitty sleep 600")
    machine.wait_until_succeeds(
        desktop.in_session(f"[ $({kitties}) -gt {before} ]"), timeout=dt.timedelta(seconds=120)
    )
    try:
        desktop.as_account(sessions_kitty.format("STOP"))
        desktop.as_account("qs ipc call launcher toggle")
        desktop.launcher_ready()
        desktop.type_in("to reopen", selecting="Sessions to Reopen")
        machine.send_key("ret")
        desktop.launcher_until(lambda shown: shown["rows"][:1] == ["Reopen Them All"])
        desktop.land()
        machine.send_key("ret")
        machine.wait_until_succeeds(
            desktop.in_session(
                "journalctl --user --unit=quickshell | grep -q 'resumed session 01c9-first'"
            ),
            timeout=dt.timedelta(seconds=60),
        )
        # As Joel's own terminal key starts one: kitty of its own.
        desktop.as_account("niri msg action spawn -- kitty --title Mine sleep 600")
        machine.wait_until_succeeds(
            desktop.in_session('niri msg --json windows | grep -q \'"title":"Mine"\''),
            timeout=dt.timedelta(seconds=60),
        )
        desktop.as_account(sessions_kitty.format("CONT"))
        deadline = time.monotonic() + 300
        while not three_as_kept(back := sat(desktop, "01c9-")) and time.monotonic() < deadline:
            time.sleep(1)
        assert three_as_kept(back), back
    finally:
        desktop.as_account(
            f"{sessions_kitty.format('CONT')}; rm -f $XDG_RUNTIME_DIR/pi-stays;"
            " pkill -f 'sleep 60[0]'; true"
        )
        # The kitty started here, outside the shell's slice, is ended, so
        # no later session opens in it.
        desktop.as_account(f"{sessions_kitty.format('TERM')}; true")
        kept(desktop, "~/.local/state/pi/restore.json", {"v": 1, "boot": boot, "sessions": []})
        desktop.as_account("rm -rf ~/first ~/second ~/third")


def test_a_session_opens_in_the_agents_slice(desktop: Desktop) -> None:
    """A session's kitty, and the session in it, run in agents.slice, and the kitty stays.

    kitty's instance quit as its last window closed, and a session asked of it meanwhile was
    lost: on a loaded builder, Reopen Them All asked for its first session 0.07 s before the
    instance left by the subtest before it quit, and that session never opened (2026-11-04).
    So the kitty outlives its last session's window, and the next session opens in it.
    """
    machine = desktop.machine
    holder = SESSIONS_KITTY
    sessions = 'niri msg --json windows | grep -o \'"app_id":"pi-s\' | wc -l'
    handed = {"sessions": [{"id": "01cb-sliced", "cwd": "/home/jitsusama/sliced"}]}
    desktop.as_account("touch $XDG_RUNTIME_DIR/pi-stays && mkdir -p ~/sliced ~/next")
    try:
        desktop.as_account(f"qs ipc call sessions reopen {shlex.quote(json.dumps(handed))}")
        machine.wait_until_succeeds(
            desktop.in_session("niri msg --json windows | grep -q 'pi --session 01cb-sliced'"),
            timeout=dt.timedelta(seconds=300),
        )
        cgroups = "pgrep -f '{}' | sed 's|.*|/proc/&/cgroup|' | xargs cat"
        kitty = desktop.as_account(cgroups.format(holder)).split()
        session = desktop.as_account(cgroups.format("session 01cb-slice[d]")).split()
        assert kitty, kitty
        assert all("/agents.slice/" in line for line in kitty), kitty
        assert session, session
        assert all("/agents.slice/" in line for line in session), session
        # Every session's window closed, this one's the last.
        desktop.as_account("rm -f $XDG_RUNTIME_DIR/pi-stays; pkill -f 'sleep 60[0]'; true")
        machine.wait_until_succeeds(
            desktop.in_session(f"[ $({sessions}) = 0 ]"), timeout=dt.timedelta(seconds=60)
        )
        # It quit within a tenth of a second of its last window when it did.
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            status, _ = machine.execute(desktop.in_session(f"pgrep -f '{holder}'"))
            assert status == 0, "the sessions' kitty quit with its last window"
            time.sleep(0.5)
        desktop.as_account("touch $XDG_RUNTIME_DIR/pi-stays")
        handed = {"sessions": [{"id": "01cb-next", "cwd": "/home/jitsusama/next"}]}
        desktop.as_account(f"qs ipc call sessions reopen {shlex.quote(json.dumps(handed))}")
        machine.wait_until_succeeds(
            desktop.in_session("niri msg --json windows | grep -q 'pi --session 01cb-next'"),
            timeout=dt.timedelta(seconds=60),
        )
        # What keeps it is no window niri lists.
        listed = desktop.as_account(sessions).strip()
        assert listed == "1", f"{listed} session windows listed for one session"
    finally:
        desktop.as_account("rm -f $XDG_RUNTIME_DIR/pi-stays; pkill -f 'sleep 60[0]'; true")
        desktop.as_account("rm -rf ~/sliced ~/next")


def three_as_kept(back: dict[str, tuple[int, int]]) -> bool:
    """Whether first and second sit side by side on one workspace, and third on the next."""
    if len(back) != 3:  # noqa: PLR2004
        return False
    first, second, third = back["01c9-first"], back["01c9-second"], back["01c9-third"]
    return first[0] == second[0] and second[1] == first[1] + 1 and third[0] == first[0] + 1


def sat_as_kept(back: dict[str, tuple[int, int]]) -> bool:
    """Whether the five sessions sit as kept: west, then east over under, then north and south."""
    if len(back) != 5:  # noqa: PLR2004
        return False
    west, east, under = back["01c5-west"], back["01c5-east"], back["01c5-under"]
    north, south = back["01c5-north"], back["01c5-south"]
    return (
        west[0] == east[0] == under[0]
        and east[1] == under[1] == west[1] + 1
        and north[0] == south[0] == west[0] + 1
        and south[1] == north[1] + 1
    )


def stand_in(desktop: Desktop, app: str, title: str) -> None:
    """Open a window of another program's, as niri starts one, and wait for it."""
    desktop.as_account(
        f"niri msg action spawn -- kitty --class {app} --title {shlex.quote(title)} sleep 600"
    )
    desktop.machine.wait_until_succeeds(
        desktop.in_session(f'niri msg --json windows | grep -q \'"app_id":"{app}"\''),
        timeout=dt.timedelta(seconds=60),
    )


def where_now(desktop: Desktop, prefix: str, titles: set[str]) -> dict[str, tuple[int, int]]:
    """Give each stand-in session's workspace and column, and each titled window's."""
    spaces = json.loads(desktop.as_account("niri msg --json workspaces"))
    idx = {one["id"]: one["idx"] for one in spaces}
    windows = {
        window["title"]: (
            idx[window["workspace_id"]],
            window["layout"]["pos_in_scrolling_layout"][0],
        )
        for window in desktop.windows()
        if window["title"] in titles
    }
    return {**sat(desktop, prefix), **windows}


def put_back(now: dict[str, tuple[int, int]]) -> bool:
    """Whether the browser sits between its sessions, and the chat on its own after them."""
    if not {"01c4-left", "01c4-right", "01c4-up", "Open Acceluary", "Chat"} <= set(now):
        return False
    left, right, up = now["01c4-left"], now["01c4-right"], now["01c4-up"]
    browser, chat = now["Open Acceluary"], now["Chat"]
    return (
        left[0] == browser[0] == right[0]
        and browser[1] == left[1] + 1
        and right[1] == browser[1] + 1
        and chat == (left[0] + 1, 1)
        and up[0] == left[0] + 2
    )


def kept_with(desktop: Desktop, arranged: str, boot: str, app: str) -> dict[str, Any]:
    """Give the places kept once the agents daemon keeps a window of a program's for a boot."""
    deadline = time.monotonic() + 30
    while True:
        read = json.loads(desktop.as_account(f"cat {arranged}"))
        if any(one["app"] == app for one in read.get("others", {}).get(boot, [])):
            return cast("dict[str, Any]", read)
        assert time.monotonic() < deadline, f"no {app} window kept for {boot}: {read}"
        time.sleep(1)


def test_agents_puts_back_the_windows_that_reopen_themselves(desktop: Desktop) -> None:
    """Reopen Them All puts back the windows that reopen themselves, beside their sessions."""
    machine = desktop.machine
    arranged = ARRANGED
    boot = desktop.as_account("cat /proc/sys/kernel/random/boot_id").strip()
    [(output, said)] = json.loads(desktop.as_account("niri msg --json outputs")).items()
    screen = " ".join(part for part in (said["make"], said["model"], said["serial"]) if part)
    on = {"screen": screen, "output": output, "tile": 1}
    # The last boot's: a browser between two sessions on the first
    # workspace, a chat alone on the second, and a session on the third,
    # whose workspace is the second once reopened, as the chat's isn't.
    where = {
        "01c4-left": {**on, "workspace": 1, "column": 1},
        "01c4-right": {**on, "workspace": 1, "column": 3},
        "01c4-up": {**on, "workspace": 3, "column": 1},
    }
    others = [
        {"app": "stand-in-browser", "title": "Open Acceluary", **on, "workspace": 1, "column": 2},
        {"app": "stand-in-chat", "title": "Chat", **on, "workspace": 2, "column": 1},
    ]
    lost = {
        "v": 1,
        "boot": "a-boot-before",
        "sessions": [
            {"id": one, "title": one[5:].title(), "cwd": f"/home/jitsusama/{one[5:]}"}
            for one in where
        ],
    }
    try:
        desktop.as_account("touch $XDG_RUNTIME_DIR/pi-stays && mkdir -p ~/left ~/right ~/up")
        # The chat is open before Reopen Them All, the browser after it.
        desktop.as_account("niri msg action focus-workspace 1")
        stand_in(desktop, "stand-in-chat", "Chat")
        kept(desktop, "~/.local/state/pi/restore.json", lost)
        kept(
            desktop,
            arranged,
            {"v": 1, "boots": {"a-boot-before": where}, "others": {"a-boot-before": others}},
        )
        desktop.as_account("qs ipc call launcher toggle")
        desktop.launcher_ready()
        desktop.type_in("to reopen", selecting="Sessions to Reopen")
        machine.send_key("ret")
        desktop.launcher_until(lambda shown: shown["rows"][:1] == ["Reopen Them All"])
        desktop.land()
        machine.send_key("ret")
        opening = "niri msg --json windows | grep -o 'pi --session 01c4-' | wc -l"
        machine.wait_until_succeeds(
            desktop.in_session(f"[ $({opening}) = 3 ]"), timeout=dt.timedelta(seconds=60)
        )
        desktop.launcher_until(lambda shown: not shown["open"])
        # The harness names another boot once every session is back, as
        # it did on 2026-11-02; the places kept for this one stay.
        kept(
            desktop,
            "~/.local/state/pi/restore.json",
            {"v": 1, "boot": "another-boot", "sessions": []},
        )
        stand_in(desktop, "stand-in-browser", "Open Acceluary")
        deadline = time.monotonic() + 60
        while not put_back(now := where_now(desktop, "01c4-", {"Open Acceluary", "Chat"})):
            assert time.monotonic() < deadline, f"not put back where they sat: {now}"
            time.sleep(1)
        # The shell's first keep with the browser in this boot's windows
        # comes after the harness named another boot.
        read = kept_with(desktop, arranged, boot, "stand-in-browser")
        assert "a-boot-before" in read["boots"], sorted(read["boots"])
    finally:
        # Its own pattern bracketed, so pkill doesn't match the shell it runs in.
        desktop.as_account("rm -f $XDG_RUNTIME_DIR/pi-stays; pkill -f 'sleep 60[0]'; true")
        kept(desktop, "~/.local/state/pi/restore.json", {"v": 1, "boot": boot, "sessions": []})
        desktop.as_account("rm -rf ~/left ~/right ~/up")


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

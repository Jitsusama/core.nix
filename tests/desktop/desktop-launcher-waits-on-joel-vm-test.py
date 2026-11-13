"""The desktop test's launcher waits on Joel area.

Here the launcher's Now and For You hold what waits on Joel, its agents list the sessions,
and Super+Tab puts a pi window waiting on him next.

tests/desktop.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), and its
opening comment says what the test proves. Every test asks for the desktop, started once a run
(desktop_session.py), and waits at its start for what it needs, so any test runs alone.
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
    HARNESS,
    INBOX,
    LAUNCHER_OPEN,
    MISSION,
    PI_PRESENCE,
    PI_QUEST,
    PI_WINDOW,
    STARTING,
    drawn_held_still,
    kept,
    laid_out,
    retry,
)

if TYPE_CHECKING:
    from collections.abc import Iterator

    from desktop_session import Desktop, Json
    from test_driver.machine import BaseMachine

# The scales Joel uses, where what VKCF3Z judges is shot.
JOELS_SCALES = ("1.5", "2")
# How long a shot waits for a selected row that reads on to settle: twice through, a few
# seconds each, and then the screen's own settling (docs/design.md, A long name reads on).
READ_ON_TWICE = dt.timedelta(seconds=30)


@pytest.fixture(scope="module")
def desktop(machine: BaseMachine, values: dict[str, Any]) -> Desktop:
    """Give the desktop, started once a run."""
    return desktop_session.started(machine, cast("desktop_session.Values", values))


@pytest.fixture(autouse=True)
def launcher_closed_after(desktop: Desktop) -> Iterator[None]:
    """Close the launcher after each test, so one failing with it open fails alone."""
    yield
    desktop.launcher_left_closed()


def test_the_launcher_finds_pis_runs_going_to_the_session_that_started_one_and_reading_an_answer(
    desktop: Desktop,
) -> None:
    """The launcher finds pi's runs, going to the session that started one and reading an answer."""
    machine = desktop.machine
    desktop.pi_session()
    # A fleet as the harness publishes it for the node, started by the session:
    # one member still at work, and one done with its answer, which its result
    # keeps.
    leader = desktop.pi_record("TESTQA")["session"]
    harness = "~/.local/state/pi/agentic-harness.pi"
    answer = "$HOME/.local/state/pi/agentic-harness.pi/subagent-workflow/runs/paint-crew/roller"
    stamp = machine.succeed("date -u +%FT%T.000Z").strip()
    desktop.as_account(
        f"mkdir -p \"{answer}\" && echo '{json.dumps({'finalAssistantText': 'Two coats.'})}'"
        f' > "{answer}/result.json"'
    )
    home = desktop.as_account("echo $HOME").strip()
    member = {"model": "anthropic/opus", "since": stamp}
    fleet = {
        "id": "fleet.paint-crew", "kind": "fleet", "parent": leader, "about": "paint-crew",
        "state": "running", "since": stamp, "at": stamp, "pi.cost.usd": 2.5,
        "members": [
            {**member, "id": "brush", "state": "running"},
            {**member, "id": "roller", "state": "complete", "ended": stamp,
             "pi.cost.usd": 2.5,
             "answer": answer.replace("$HOME", home) + "/result.json"},
        ],
    }  # fmt: skip
    desktop.as_account(
        f"mkdir -p {harness}/mesh-workflow/runs"
        f" && echo '{json.dumps({'v': 1, 'runs': [fleet]})}'"
        f" > {harness}/mesh-workflow/runs/fleet.paint-crew.json"
    )
    desktop.presence_says({"addresses": ["TESTQA"], "busy": False})
    desktop.as_account(
        f"niri msg action focus-window --id {desktop.window_titled('Elsewhere')['id']}"
    )
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    # Agents' one word for them finds them.
    desktop.type_in("runs")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Runs")
    machine.send_key("ret")
    desktop.launcher_until(
        lambda shown: (
            shown["held"] == ["runs"]
            and (shown["selected"] or {}).get("name") == "paint-crew"
            and re.fullmatch(
                r"fleet \u00b7 1 of 2 done \u00b7 (under a minute|\d+ min) \u00b7 TESTQA",
                shown["selected"]["side"],
            )
        )
    )
    # The fleet's own row says more than its share, so what it says reads on while selected, as
    # a long name does (docs/design.md). That a fleet at work draws nothing while it's only
    # looked at is held among its members, where the member at work shows below the way to
    # the fleet's session, whose row isn't cut.
    reads_on_beside(desktop, "paint-crew", "fleet \u00b7")
    # Shot once it has read on twice and rests at its start, which takes longer than the
    # screen's usual settling.
    desktop.shoot("launcher-runs", READ_ON_TWICE)
    # Gone into by Ctrl+], as Return goes to its session, its members after the way there.
    desktop.follow()
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Go To")
    # What a fleet says changes as its files do, not every frame.
    draws_nothing_held_still(desktop, "a fleet", "its member at work")
    desktop.next_row()
    desktop.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("name") == "brush"
            and shown["selected"]["side"].startswith("running \u00b7 opus")
        )
    )
    desktop.next_row()
    desktop.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("name") == "roller"
            and shown["selected"]["side"] == "opus \u00b7 $2.50 \u00b7 under a minute"
        )
    )
    desktop.shoot("launcher-run-members")
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
    desktop.type_in("runs")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Runs")
    machine.send_key("ret")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "paint-crew")
    machine.send_key("ret")
    machine.wait_until_succeeds(
        desktop.in_session("niri msg --json focused-window | grep -q TESTQA"),
        timeout=dt.timedelta(seconds=10),
    )
    # Closed on the fleet's row, whose words read on while seen, the launcher draws nothing.
    draws_nothing_held_still(desktop, "the closed launcher", "a row that reads on")
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

    def tabbed() -> None:
        """Open the windows with Super+Tab, and wait until they're listed."""
        machine.send_key("meta_l-tab")
        desktop.launcher_ready()
        desktop.launcher_until(lambda shown: shown["held"] == ["windows"])

    # Its session's words, "Tidy the tests", find it, and so does its
    # quest's whole title, though its window says neither.
    for typed in ["tidy the", "whole"]:
        tabbed()
        desktop.type_in(typed)
        desktop.launcher_until(
            lambda shown: (shown["selected"] or {}).get("is") == f"window:{pi['id']}"
        )
        desktop.close_launcher()
    # Waiting on Joel, it comes straight after the window used last, folded
    # with the other agents' windows into one Agents row that says how many
    # need him, and Tab finds it inside.
    question = {
        "kind": "question",
        "since": machine.succeed("date -u +%Y-%m-%dT%H:%M:%SZ").strip(),
        "said": "Which way?",
    }
    # The shell hears it from the node, a moment after pi writes it, so the
    # windows are opened once it has: as For You's tray counts it, since a
    # want posts no notice of its own.
    desktop.presence_says({"addresses": ["TESTQA"], "busy": False, "wants": question})
    tray_counts(desktop, 1)
    tabbed()
    desktop.launcher_until(lambda shown: shown["selected"]["name"] == "Notes for Tuesday")
    desktop.next_row()
    desktop.launcher_until(
        lambda shown: (
            shown["selected"]["is"] == "windows:agents"
            and shown["selected"]["count"] == "1 needs you"
        )
    )
    desktop.into_program()
    desktop.launcher_until(
        lambda shown: any(thing["is"] == f"window:{pi['id']}" for thing in shown["things"])
    )
    desktop.close_launcher()
    desktop.presence_says({"addresses": ["TESTQA"], "busy": False})
    desktop.close(between)


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
            entry["words"]
            for entry in listed
            if entry["in"] == "Agents" and entry["kind"] == "action" and "says" in entry
        ]

    # TESTQA's session, named by its quest as its record says it.
    named = "A quest of its own & whole"
    retry(lambda _: named in sessions(), timeout=dt.timedelta(seconds=30))
    # A switch restarts the shell; the first list it gives already holds
    # the session, never an empty Agents while it reads the folder.
    desktop.as_account("systemctl --user restart quickshell")
    machine.wait_until_succeeds(
        desktop.in_session("qs ipc call launcher list"), timeout=dt.timedelta(seconds=60)
    )
    first = sessions()
    assert named in first, first


def test_the_launchers_home_says_how_its_sessions_stand_rather_than_naming_them(
    desktop: Desktop,
) -> None:
    """The launcher's home says how its sessions stand together, never their names.

    "1 working" at its right and nothing after its name, as "1 needs you \u00b7 3 working \u00b7
    $214 today" would, however long a quest's name is (VKCF3Z, 2026-11-07, dae7fb67).
    """
    desktop.pi_session()
    # A session whose quest's name is longer than home's row has room for,
    # as Joel's are, in the window used last.
    long = "\u03c0 - Bootstrap the Lighthouse to Replace Every Other Machine on the Desk - tree"
    desktop.pi_window("TESTQC", long)
    desktop.presence_says({"addresses": ["TESTQC"], "busy": True, "title": long}, "TESTQC")
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.launcher_until(
        lambda shown: any(
            thing["is"] == "collection:agents"
            and re.fullmatch(r"Agents \u00b7 \d+ working(?: \u00b7 \S+ today)?", thing["reads"])
            for thing in shown["things"]
        )
    )
    desktop.shoot("launcher-home-agents")
    desktop.close_launcher()
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
    # The window titled the new way is its session, named by its title, once the agents
    # daemon has heard of it from the node, which is after the window opens.
    desktop.launcher_until(lambda shown: shown["held"] == ["agents"] and "Another" in shown["rows"])
    # The later of the two, which would go first once it wants Joel.
    both = {"Another", "A quest of its own & whole"}
    later = [row for row in desktop.showing()["rows"] if row in both][-1]
    desktop.down_to(later)
    # Its address leads its drawer's eyebrow, since the row says only how
    # it stands and what it spent.
    desktop.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("name") == later
            and shown["drawer"]["eyebrow"].split(" \u00b7 ")[0] in {"TESTQA", "TESTQB"}
        )
    )
    shown = desktop.showing()
    order = shown["rows"]
    chosen = shown["selected"]["name"]
    address = shown["drawer"]["eyebrow"].split(" \u00b7 ")[0]
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
            "needs you" in (shown["selected"] or {}).get("side", "")
            and any("listening" in thing["reads"] for thing in shown["things"])
        )
    )
    # Said in place: the same rows in the same order, the selection
    # still on its row.
    shown = desktop.showing()
    assert shown["rows"] == order, "the agents moved under the selection"
    assert shown["selected"]["name"] == chosen, "the selection left its row"
    desktop.close_launcher()
    desktop.as_account(f"niri msg action close-window --id {desktop.window_titled(other)['id']}")
    desktop.as_account(f"rm -f {PI_PRESENCE}/testqb.json")
    present("TESTQA", {"addresses": ["TESTQA"], "busy": False})
    machine.wait_until_fails(
        desktop.in_session("niri msg --json windows | grep -q TESTQB"),
        timeout=dt.timedelta(seconds=30),
    )


def test_where_an_agents_title_is_cut_holds_still_as_its_side_changes(desktop: Desktop) -> None:
    """Where an agent's long title is cut holds still as what it says at its right shortens."""
    desktop.pi_session()
    long = "\u03c0 - Bootstrap the Lighthouse to Replace Every Other Machine on the Desk - tree"

    def holds_still() -> None:
        """Open Agents on the long session and see its title's cut hold as its side shortens."""
        machine = desktop.machine
        # Agents is opened once the agents daemon serves it, since a row coming while a place
        # is open joins at its foot, below what's drawn, so the order he's reading holds.
        for _ in range(2):
            desktop.as_account("qs ipc call launcher toggle")
            desktop.launcher_ready()
            desktop.type_in("agents")
            desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Agents")
            machine.send_key("ret")
            desktop.launcher_until(
                lambda shown: any(thing["is"] == "agent:TESTQC" for thing in shown["things"])
            )
            if "Bootstrap" in str(laid_out(desktop)["rows"]):
                break
            desktop.close_launcher()

        def its_row(says: str) -> Json | None:
            """Give the long session's row as it's drawn, once its right says this."""
            laid = laid_out(desktop)
            row = next((row for row in laid["rows"] if "Bootstrap" in row["reads"]), None)
            return row if row and says in row["reads"] else None

        def name_of(row: Json) -> Json:
            return max(row["words"], key=lambda word: len(word["text"]))

        retry(lambda _: its_row("needs you") is not None, timeout=dt.timedelta(seconds=20))
        before = name_of(cast("Json", its_row("needs you")))
        assert before["cut"], f"the title wasn't cut, so its cut can't be seen to hold: {before}"
        # It goes back to work: "needs you" becomes the shorter "working", and
        # the title is cut where it was rather than reaching one letter on:
        # its room holds, whether it's cut by an ellipsis or, selected, reads
        # on inside it.
        desktop.presence_says({"addresses": ["TESTQC"], "busy": True}, "TESTQC")
        retry(lambda _: its_row("working") is not None, timeout=dt.timedelta(seconds=20))
        after = name_of(cast("Json", its_row("working")))
        assert after["room"] == before["room"], (before, after)

    desktop.pi_window("TESTQC", long)
    question = {"kind": "question", "since": "2000-01-01T00:00:00Z", "said": "Which?"}
    desktop.presence_says({"addresses": ["TESTQC"], "busy": False, "wants": question}, "TESTQC")
    # Its session goes however this ends, since left, it waits on Joel in For You's cases.
    try:
        holds_still()
    finally:
        if desktop.showing()["open"]:
            desktop.close_launcher()
        desktop.as_account(f"niri msg action close-window --id {desktop.window_titled(long)['id']}")
        desktop.as_account(f"rm -f {PI_PRESENCE}/testqc.json")


# Each quest keeps what only Joel can do in a file of its own, written by
# the session that needs him.


def shoots_now_at_joels_scales(desktop: Desktop, reads: str) -> None:
    """Shoot home at 1 and Joel's scales, its Now row reading as given, and the plate beside it.

    The plate says what waits no more, where the row says it (VKCF3Z, 2026-11-09).
    """
    # The to-do's card away first, which would stand over the row and the plate.
    with desktop.told("gone notifications"):
        desktop.as_account("qs ipc call notices putAway")
    desktop.shoot("launcher-home-now-waiting")
    for scale in JOELS_SCALES:
        desktop.as_account(f"niri msg output winit scale {scale}")
        desktop.launcher_until(
            lambda shown: any(thing["reads"] == reads for thing in shown["things"])
        )
        desktop.shoot(f"launcher-home-now-waiting-at-{scale.replace('.', '-')}")
    desktop.as_account("niri msg output winit scale 1")


def test_now_leads_with_what_waits_on_joel(desktop: Desktop) -> None:
    """Now leads with what waits on Joel, and Return goes to where it's done."""
    machine = desktop.machine
    closed = desktop.in_session(LAUNCHER_OPEN)
    # pi's session is the one that needs Joel, by the address the harness
    # gives it.
    desktop.pi_session()
    desktop.opened("Waiting room")
    # What it needs him for, in its own file: one done in a window it names,
    # and one naming none, done in the session's own; neither ready yet.
    own = {
        "quest": "TESTQA",
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
    asked = {
        "quest": "QUEST1",
        "items": [{"id": "ask", "title": "Answer the ask", "minutes": 2, "ready": True}],
    }
    state = "$HOME/.local/state"
    kept(desktop, f"{state}/mission/waiting/TESTQA.json", own)
    # One ready is said as a notice the moment it's heard, named by what
    # it asks, so it's never a second "For You" beside Now's.
    with desktop.told("shows card Answer the ask"):
        desktop.as_account(
            f"printf %s {shlex.quote(json.dumps(asked))} > {state}/mission/waiting/.q"
            f" && mv {state}/mission/waiting/.q {state}/mission/waiting/QUEST1.json"
        )
    # Another window in front, so going to the waiting room is seen.
    desktop.opened("Elsewhere")
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    # Home's Now names the first Joel can do now at its right, with its minutes, and
    # nothing more, since it's the only one ready (VKCF3Z, bd5cb965).
    desktop.launcher_until(
        lambda shown: any(
            thing["reads"] == "Now \u00b7 Answer the ask \u00b7 2 min" for thing in shown["things"]
        )
    )
    # Shot with its card put away, which only the gallery does.
    if desktop.values["shots"]:
        shoots_now_at_joels_scales(desktop, "Now \u00b7 Answer the ask \u00b7 2 min")
    # It counts only what Joel can do now, by its band: the session's two aren't ready.
    desktop.type_in("for you", selecting="For You")
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("side") == "1 to do")
    # Inside, what's for now, then Later, holding what can't be done yet,
    # the quickest first. Return on one steps into it, and its first act goes where
    # it's done.
    machine.send_key("ret")
    desktop.launcher_until(lambda shown: shown["rows"] == ["Answer the ask", "Later"])
    desktop.next_row()
    picked(desktop, "Later")
    desktop.follow()
    desktop.launcher_until(lambda shown: shown["rows"] == ["Pick a bucket", "Try the new home"])
    desktop.next_row()
    picked(desktop, "Try the new home")
    # Done in a window of its own, going there leads.
    desktop.step_in("ret")
    desktop.launcher_until(lambda shown: shown["rows"][:2] == ["Go To", "Deal With It"])
    machine.send_key("ret")
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
    retry(
        lambda _: desktop.window_titled("Waiting room")["is_focused"],
        timeout=dt.timedelta(seconds=10),
    )
    # One naming no window is done in its session's window.
    waiting_opened(desktop)
    desktop.follow()
    desktop.next_row()
    picked(desktop, "Later")
    desktop.follow()
    picked(desktop, "Pick a bucket")
    # Done in its session, dealing with it there leads.
    desktop.step_in("ret")
    desktop.launcher_until(lambda shown: shown["rows"][:2] == ["Deal With It", "Go To"])
    machine.send_key("ret")
    machine.wait_until_fails(closed, timeout=dt.timedelta(seconds=10))
    retry(
        lambda _: desktop.window_titled(PI_WINDOW)["is_focused"],
        timeout=dt.timedelta(seconds=10),
    )
    # Put back as it was, for the tests after.
    desktop.as_account(
        f"rm {state}/mission/waiting/TESTQA.json {state}/mission/waiting/QUEST1.json"
    )
    for title in ["Waiting room", "Elsewhere"]:
        window = desktop.window_titled(title)["id"]
        desktop.as_account(f"niri msg action close-window --id {window}")


INDEX = f"{HARNESS}/quest-workflow/quests.json"


def tray_counted(desktop: Desktop) -> int:
    """Say how many For You's tray last counted waiting on Joel, none while it was out."""
    sets = desktop.as_account(
        "journalctl --user --unit=quickshell --no-pager --output=cat"
        " | grep -o 'report\\[sets margin .*\\]$' | tail -n 1"
    )
    said = sets.strip().removeprefix("report[sets margin ").removesuffix("]")
    drawn = json.loads(said or "[]")
    return next((int(glyph["state"]) for glyph in drawn if glyph["mark"] == "forYou"), 0)


def tray_counts(desktop: Desktop, count: int) -> None:
    """Wait for For You's tray to count this many, as the margin last set it."""
    retry(lambda _: tray_counted(desktop) == count, timeout=dt.timedelta(seconds=20))


def indexed(desktop: Desktop, *quests: str) -> None:
    """Write the harness's index of quests, which holds only those still open, as these."""
    index = {
        "quests": [
            {"id": quest, "address": quest.rsplit("-", 1)[-1], "title": quest, "status": "active"}
            for quest in quests
        ]
    }
    kept(desktop, INDEX, index)


def waiting_opened(desktop: Desktop) -> None:
    """Open the launcher on For You's row."""
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("for you", selecting="For You")


def waiting_inside(desktop: Desktop) -> tuple[str, list[str]]:
    """Give For You's side and the rows inside it, as a fresh look sees them."""
    waiting_opened(desktop)
    side = cast("str", (desktop.showing()["selected"] or {}).get("side"))
    desktop.follow()
    rows = cast("list[str]", desktop.showing()["rows"])
    desktop.close_launcher()
    return side, rows


def picked(desktop: Desktop, name: str) -> None:
    """Wait until the row named is the one Return takes."""
    desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == name)


def test_waiting_on_you_holds_only_what_is_true(desktop: Desktop) -> None:
    """For You drops what's found done and an ended quest's, and keeps Joel's word."""
    machine = desktop.machine
    state = "$HOME/.local/state"
    # The verdicts here are written by hand, standing in for the to-do
    # daemon, which would otherwise judge them as they are. Its socket goes
    # too: left listening, the tray's first ask of to-do.sock started the
    # daemon again a tenth of a second later, and it judged them anyway.
    # Starting the daemon again brings the socket back, which it requires.
    desktop.as_account("systemctl --user stop to-do.socket to-do")
    # The index holds the live quest, and not the concluded one.
    indexed(desktop, "QEST-20261026-LIVEQA")
    # A live quest's, as mesh_wait writes them: one ready, one the to do
    # daemon finds ready, one it finds done, and one for later.
    kept(
        desktop,
        f"{state}/mission/waiting/LIVEQA.json",
        {
            "v": 2,
            "quest": "QEST-20261026-LIVEQA",
            "items": [
                {
                    "id": "restart",
                    "title": "Restart Once",
                    "minutes": 5,
                    "ready": True,
                    "doneWhen": {"signal": "boot"},
                },
                {
                    "id": "keys",
                    "title": "Prove the Keys",
                    "minutes": 10,
                    "ready": False,
                    "doneWhen": {"check": "true"},
                    "readyWhen": {"signal": "device:dock"},
                },
                {
                    "id": "dock",
                    "title": "Plug In the Dock",
                    "minutes": 1,
                    "ready": True,
                    "doneWhen": {"signal": "device:dock"},
                },
                {"id": "later", "title": "Look at the Desk", "ready": False, "when": "tomorrow"},
            ],
        },
    )
    found = dt.datetime.now(dt.UTC).isoformat()
    kept(
        desktop,
        f"{state}/mission/verdicts.json",
        {
            "v": 1,
            "updated": found,
            "items": {
                "QEST-20261026-LIVEQA:keys": {
                    "state": "ready",
                    "at": found,
                    "how": {"signal": "device:dock"},
                },
                "QEST-20261026-LIVEQA:dock": {
                    "state": "done",
                    "at": found,
                    "how": {"signal": "device:dock"},
                },
            },
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
            "quest": "OLDQA",
            "items": [{"id": "old", "title": "Answer the old", "ready": True, "why": "it asked"}],
        },
    )

    # Three for now, the quickest first and one saying no time last, since
    # they weigh the same and keep the order they came in, then Later.
    everything = ["Restart Once", "Prove the Keys", "Answer the old", "Later"]
    retry(
        lambda _: waiting_inside(desktop) == ("3 to do", everything),
        timeout=dt.timedelta(seconds=30),
    )
    # One with nowhere to go steps in to Mark It Done and Put It Off.
    waiting_opened(desktop)
    desktop.follow()
    desktop.down_to("Answer the old")
    # No check or signal will see it done, and no session of its owner's is live, so its
    # note says nobody's watching it, in place of the old file's why (Waiting.js's watched).
    desktop.launcher_until(
        lambda shown: (shown["selected"] or {}).get("note") == "nobody's watching this"
    )
    desktop.step_in("ret")
    desktop.launcher_until(lambda shown: shown["rows"] == ["Mark It Done", "Put It Off"])
    # Mark It Done ends what waits, so Return writes :delete, and its line asks first; done, it
    # keeps the launcher, and is kept where Joel's word is.
    desktop.line_answered("delete")
    retry(
        lambda _: "OLDQA:old" in desktop.as_account(f"cat {state}/mission/settled.json || true"),
        timeout=dt.timedelta(seconds=10),
    )
    assert desktop.showing()["open"]
    desktop.close_launcher()
    # What Joel said was done is in History, last, while its file tells of it.
    retry(
        lambda _: (
            waiting_inside(desktop)
            == ("2 to do", ["Restart Once", "Prove the Keys", "Later", "History"])
        ),
        timeout=dt.timedelta(seconds=30),
    )
    # Put It Off puts one under Later.
    waiting_opened(desktop)
    desktop.follow()
    picked(desktop, "Restart Once")
    desktop.follow()
    desktop.down_to("Put It Off")
    desktop.line_answered("snooze")
    desktop.close_launcher()
    retry(
        lambda _: waiting_inside(desktop) == ("1 to do", ["Prove the Keys", "Later", "History"]),
        timeout=dt.timedelta(seconds=30),
    )
    # Joel's word stays across a restart of the shell.
    desktop.as_account("systemctl --user restart quickshell")
    machine.wait_until_succeeds(
        desktop.in_session("qs ipc call launcher showing"), timeout=dt.timedelta(seconds=60)
    )
    retry(
        lambda _: waiting_inside(desktop) == ("1 to do", ["Prove the Keys", "Later", "History"]),
        timeout=dt.timedelta(seconds=60),
    )
    # A quest that ends takes its things with it, as it leaves the index;
    # what was done of one that names no quest the index holds stays in
    # History.
    indexed(desktop)
    retry(
        lambda _: waiting_inside(desktop) == ("nothing now", ["History"]),
        timeout=dt.timedelta(seconds=30),
    )
    desktop.as_account(
        f"rm -r {state}/mission/waiting/LIVEQA.json {state}/mission/verdicts.json"
        f" {state}/mission/waiting/DONEQA.json {state}/mission/waiting/OLDQA.json"
        f" {state}/mission/settled.json {INDEX}"
    )
    desktop.as_account("systemctl --user start to-do")


def test_a_to_do_waiting_on_a_file_leaves_for_you_as_it_comes(desktop: Desktop) -> None:
    """The to-do daemon finds one done by its signal, and For You lets it go by itself."""
    asked = f"{MISSION}/waiting/FILEQA.json"
    made = "/run/user/1000/to-do-made/here"
    desktop.as_account("systemctl --user start to-do")
    # Its quest names no quest of the index's, so nothing but the daemon
    # can see it done.
    kept(
        desktop,
        asked,
        {
            "v": 2,
            "quest": "FILEQA",
            "items": [
                {
                    "id": "made",
                    "title": "Make the File",
                    "ready": True,
                    "doneWhen": {"signal": f"file:{made}"},
                }
            ],
        },
    )
    try:
        retry(
            lambda _: "Make the File" in waiting_inside(desktop)[1],
            timeout=dt.timedelta(seconds=30),
        )
        desktop.as_account(f"mkdir -p {made.rsplit('/', 1)[0]} && touch {made}")
        retry(
            lambda _: "Make the File" not in waiting_inside(desktop)[1],
            timeout=dt.timedelta(seconds=30),
        )
        verdicts = json.loads(desktop.as_account(f"cat {MISSION}/verdicts.json"))
        assert verdicts["items"]["FILEQA:made"]["state"] == "done", verdicts
    finally:
        desktop.as_account(f"rm -rf {asked} /run/user/1000/to-do-made")


def test_waiting_with_nowhere_to_go_steps_in_and_its_drawer_says_its_steps(
    desktop: Desktop,
) -> None:
    """For You steps into one with nowhere to go, whose drawer gives its steps and why."""
    machine = desktop.machine
    asked = f"{MISSION}/waiting/GONEQA.json"
    # Its quest by its title, in the eyebrow (VKCF3Z, 2026-11-08), which
    # the index gives only a quest's own id (Waiting.js). The index goes
    # first, so the node has told its title before the item can be
    # stepped into, whose drawer is the one it was entered with.
    kept(
        desktop,
        INDEX,
        {
            "quests": [
                {
                    "id": "QEST-20261027-GONEQA",
                    "address": "GONEQA",
                    "title": "Reef Quest",
                    "status": "active",
                }
            ]
        },
    )
    # A session that's gone, so the item has no window to go to.
    kept(
        desktop,
        asked,
        {
            "quest": "QEST-20261027-GONEQA",
            "items": [
                {
                    "id": "form",
                    "title": "Sign the Form",
                    "when": "now",
                    "why": "It goes out in your name.",
                    "steps": ["Print it", "Sign it"],
                    "at": "2026-11-09T09:30:00Z",
                    "deadline": "2026-11-09T15:00:00Z",
                    "raisedAt": "2026-11-08T09:00:00Z",
                    "where": {"machine": "harbour"},
                    "owner": {"address": "8IDU4E"},
                    "doneWhen": {"check": "test -e /run/user/1000/form/signed"},
                }
            ],
        },
    )
    # Its steps, then why only Joel can; how it'll be known done is its
    # Clears When fact, so the words don't say it again.
    steps = "1. Print it\n2. Sign it\n\nIt goes out in your name."
    # What its owner said of it, each a fact in For You's order (VKCF3Z,
    # 2026-11-08). Its row's side says when it's due, so Due doesn't, and
    # with no window open there's no Where.
    owned = ["Set For", "Waiting Since", "Clears When", "Machine", "Address", "Check"]
    try:
        retry(
            lambda _: waiting_inside(desktop) == ("1 to do", ["Sign the Form"]),
            timeout=dt.timedelta(seconds=30),
        )
        # Return on For You steps in, as l does in normal mode.
        waiting_opened(desktop)
        desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("verb") == "more")
        desktop.step_in("ret")
        picked(desktop, "Sign the Form")
        # Its drawer gives its steps, numbered, then why only Joel can.
        desktop.launcher_until(
            lambda shown: (
                (shown.get("drawer") or {}).get("out") is True
                and shown["drawer"]["body"] == steps
                and [fact for fact in shown["drawer"]["facts"] if fact in owned] == owned
                and "Due" not in shown["drawer"]["facts"]
                # Its quest is the eyebrow's, so no fact says it again.
                and "Quest" not in shown["drawer"]["facts"]
                and shown["drawer"]["eyebrow"] == "from Reef Quest \u00b7 to-do"
            )
        )
        desktop.shoot("launcher-for-you-drawer")
        # Return on it steps in, with nowhere to go: Mark It Done, then Put
        # It Off, then its quest, which the index holds.
        desktop.step_in("ret")
        desktop.launcher_until(
            lambda shown: shown["rows"] == ["Mark It Done", "Put It Off", "Open Its Quest"]
        )
        desktop.close_launcher()
    finally:
        desktop.as_account(f"rm -f {asked} {INDEX}")
        # For You's tray goes a moment after its item does; until then it
        # holds it, and Now would open on it in the next subtest.
        machine.wait_until_succeeds(
            desktop.in_session(
                "journalctl --user --unit=quickshell --no-pager --output=cat"
                " | grep -o 'report\\[holds margin [^]]*' | tail -n 1 | grep -qvw forYou"
            ),
            timeout=dt.timedelta(seconds=10),
        )


def test_waiting_on_you_leads_now_with_nothing_waiting_and_says_so(desktop: Desktop) -> None:
    """For You is in Now with nothing waiting, says so, and Now opens on it."""
    machine = desktop.machine
    # Nothing waits; Keep Awake is on, a mode left on, which Now never
    # opens on, since Return would turn it off.
    # The account's shell is zsh, which refuses a pattern matching nothing,
    # so what waits is found rather than named by one.
    desktop.as_account(
        f"mkdir -p {MISSION}/waiting && find {MISSION}/waiting -name '*.json' -delete"
    )
    desktop.as_account("systemctl --user start keep-awake.service")
    try:
        desktop.as_account("qs ipc call launcher collection on")
        desktop.launcher_ready()
        desktop.launcher_until(
            lambda shown: (
                shown["held"] == ["now"]
                and "Keep Awake" in shown["rows"]
                and (shown["selected"] or {}).get("name") == "For You"
            )
        )
        selected = desktop.showing()["selected"] or {}
        assert (selected["side"], selected["verb"]) == ("nothing now", "more"), selected
        # What's typed narrows Now to the rows it names, and clearing it, by
        # cc in normal mode, brings them all back.
        desktop.type_in("awake")
        desktop.launcher_until(
            lambda shown: shown["rows"][:1] == ["Keep Awake"] and not shown["nothing"]
        )
        desktop.normal()
        machine.send_chars("cc")
        desktop.launcher_until(
            lambda shown: shown["typed"] == "" and shown["rows"][:1] == ["For You"]
        )
        desktop.normal()
        # Clearing keeps the row typing chose, so the selection goes back up.
        shown = desktop.showing()
        for _ in range(shown["rows"].index(shown["selected"]["name"])):
            desktop.previous_row()
        desktop.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "For You")
        # Return steps in, as it does with things waiting, to a list saying
        # nothing does, rather than closing as if the launcher vanished.
        machine.send_key("ret")
        desktop.launcher_until(
            lambda shown: (
                shown["open"]
                and shown["held"] == ["now", "For You"]
                and shown["rows"] == []
                and shown["nothing"] == "Nothing waits on you."
            )
        )
        desktop.close_launcher()
        # Calm, with Keep Awake on, home's Now says nothing at its right,
        # neither For You nor how many are on (VKCF3Z, bd5cb965).
        desktop.as_account("qs ipc call launcher toggle")
        desktop.launcher_ready()
        desktop.launcher_until(
            lambda shown: any(
                thing["is"] == "collection:on" and thing["reads"] == "Now"
                for thing in shown["things"]
            )
        )
        desktop.close_launcher()
    finally:
        desktop.as_account("systemctl --user stop keep-awake.service")


def test_a_session_wanting_joel_waits_on_him_as_the_session(desktop: Desktop) -> None:
    """A session wanting Joel is in For You as its Agents row, and Now has no pi row.

    Super+O, Return, Return goes to its window: Now opens on For You, which counts it,
    Return steps in to the session, selected, and its Return goes to its window.
    """
    machine = desktop.machine
    desktop.pi_session()
    desktop.as_account(
        f"mkdir -p {MISSION}/waiting && find {MISSION}/waiting -name '*.json' -delete"
    )
    desktop.as_account(
        f"niri msg action focus-window --id {desktop.window_titled('Elsewhere')['id']}"
    )
    since = machine.succeed("date -u +%Y-%m-%dT%H:%M:%SZ").strip()
    question = {"kind": "question", "since": since, "said": "Which way?"}
    desktop.presence_says({"addresses": ["TESTQA"], "busy": False, "wants": question})
    try:
        desktop.as_account("qs ipc call launcher collection on")
        desktop.launcher_ready()
        desktop.launcher_until(
            lambda shown: (
                shown["held"] == ["now"]
                and (shown["selected"] or {}).get("name") == "For You"
                and shown["selected"]["side"] == "1 to do"
            )
        )
        assert "pi" not in desktop.showing()["rows"], desktop.showing()
        machine.send_key("ret")
        desktop.launcher_until(
            lambda shown: (
                shown["held"] == ["now", "For You"]
                and shown["rows"][:1] == ["A quest of its own & whole"]
                and (shown["selected"] or {}).get("verb") == "more"
            )
        )
        # Return steps into what it asks, never straight into the session, and
        # dealing with it there leads.
        machine.send_key("ret")
        desktop.launcher_until(
            lambda shown: (
                shown["held"] == ["now", "For You", "A quest of its own & whole"]
                and shown["rows"] == ["Deal With It", "Go To"]
            )
        )
        machine.send_key("ret")
        machine.wait_until_succeeds(
            desktop.in_session("niri msg --json focused-window | grep -q TESTQA"),
            timeout=dt.timedelta(seconds=10),
        )
        desktop.launcher_until(lambda shown: not shown["open"])
        answered_in_place(desktop, since)
    finally:
        desktop.presence_says({"addresses": ["TESTQA"], "busy": True})


def answered_in_place(desktop: Desktop, since: str) -> None:
    """Answer a session's question from For You: in its row, Return on the recommended answer.

    The answer reaches the session's inbox as the harness takes one, by the want's id.
    """
    machine = desktop.machine
    desktop.as_account(f"mkdir -p {INBOX} && find {INBOX} -name '*.json' -delete")
    choices = [{"value": "hold", "label": "Hold it"}, {"value": "push", "label": "Push it"}]
    ask = {"choices": choices, "recommended": "push"}
    question = {"kind": "question", "since": since, "said": "Push it?", "ask": ask}
    desktop.presence_says({"addresses": ["TESTQA"], "busy": False, "wants": question})
    desktop.as_account("qs ipc call launcher collection on")
    desktop.launcher_ready()
    desktop.launcher_until(
        lambda shown: (
            shown["held"] == ["now"]
            and (shown["selected"] or {}).get("name") == "For You"
            and shown["selected"]["side"] == "1 to do"
        )
    )
    machine.send_key("ret")
    desktop.launcher_until(
        lambda shown: (
            shown["held"] == ["now", "For You"]
            and (shown["selected"] or {}).get("side") == "needs your answer \u00b7 2 choices"
        )
    )
    machine.send_key("ret")
    desktop.launcher_until(
        lambda shown: (
            shown["rows"] == ["Push it", "Hold it", "Deal With It"]
            and (shown["selected"] or {}).get("name") == "Push it"
        )
    )
    machine.send_key("ret")
    machine.wait_until_succeeds(
        desktop.in_session(f'cat {INBOX}/*.json | grep -q \'"act":"answer"\''),
        timeout=dt.timedelta(seconds=10),
    )
    said = json.loads(desktop.as_account(f"cat {INBOX}/*.json"))
    want = f"question@{since.removesuffix('Z')}.000Z"
    assert (said["act"], said["want"], said["value"], said["from"]) == (
        "answer",
        want,
        "push",
        "joel",
    ), said
    desktop.as_account("qs ipc call launcher collection off")


def test_a_sessions_action_is_in_needs_you_once_as_its_item(desktop: Desktop) -> None:
    """A session wanting Joel to do one of its items is in For You once, as the item.

    And it's said once, by For You's notice: the session's own notice for the same thing
    put two up in the same second (2026-11-06), and a session's want posts none now.
    """
    desktop.pi_session()
    # From another window, so the session's notices aren't passed over.
    desktop.as_account(
        f"niri msg action focus-window --id {desktop.window_titled('Elsewhere')['id']}"
    )
    asked = f"{MISSION}/waiting/TESTQA.json"
    desktop.as_account(f"mkdir -p {MISSION}/waiting")
    begun = desktop.mark()
    with desktop.told("shows card Sign the Form"):
        kept(
            desktop,
            asked,
            {"quest": "TESTQA", "items": [{"id": "sign", "title": "Sign the Form", "ready": True}]},
        )
    since = desktop.machine.succeed("date -u +%Y-%m-%dT%H:%M:%SZ").strip()
    # The session's presence mirrors the item as an action it wants.
    action = {"kind": "action", "since": since, "said": "Sign the Form"}
    desktop.presence_says({"addresses": ["TESTQA"], "busy": False, "wants": action})
    try:
        desktop.as_account("qs ipc call launcher collection on")
        desktop.launcher_ready()
        desktop.launcher_until(
            lambda shown: (
                (shown["selected"] or {}).get("name") == "For You"
                and shown["selected"]["side"] == "1 to do"
            )
        )
        # The tray counts as For You does, its row and the margin one number.
        retry(lambda _: tray_counted(desktop) == 1, timeout=dt.timedelta(seconds=10))
        desktop.machine.send_key("ret")
        desktop.launcher_until(
            lambda shown: shown["held"] == ["now", "For You"] and shown["rows"] == ["Sign the Form"]
        )
        desktop.close_launcher()
        # By the time For You lists it, the shell has heard the session's want, and
        # only Needs You's card was said.
        cards = desktop.as_account(
            "journalctl --user --unit=quickshell --no-pager --output=cat"
            f" --after-cursor={shlex.quote(begun)} | grep -o 'report\\[shows card [^]]*'"
        ).splitlines()
        assert cards == ["report[shows card Sign the Form"], cards
    finally:
        desktop.as_account(f"rm -f {asked}")
        desktop.presence_says({"addresses": ["TESTQA"], "busy": True})


def test_a_waiting_notice_goes_read_once_its_item_stops_waiting(desktop: Desktop) -> None:
    """A waiting notice goes, read, once its item can't be done now any more."""
    machine = desktop.machine
    notices = desktop.in_session("niri msg layers | grep -q notifications")
    machine.wait_until_fails(notices, timeout=dt.timedelta(seconds=30))
    indexed(desktop, "QEST-20261027-GONEQA")
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
        desktop.notice_kept('.summary == "Plug in the dock" and .read'),
        timeout=dt.timedelta(seconds=10),
    )
    desktop.as_account(f"rm {path} {INDEX}")


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

    def counted() -> int:
        """Say how many For You's tray last counted waiting on Joel."""
        return tray_counted(desktop)

    # An action its quest's file holds only for later isn't Joel's yet.
    later = {"quest": "TESTQA", "items": [{"id": "later", "title": "Later", "ready": False}]}
    desktop.as_account(f"printf %s {shlex.quote(json.dumps(later))} > {waiting}")
    wants("action", "WCJQSG")
    time.sleep(2)
    assert counted() == 0, "an action held only for later marked Joel"
    # It is once the file says he can do it now, and then it's that to-do,
    # counted once, as For You lists it (Holding.js's wanting).
    now = {"quest": "TESTQA", "items": [{"id": "now", "title": "Now", "ready": True}]}
    desktop.as_account(
        f"printf %s {shlex.quote(json.dumps(now))} > {waiting}.q && mv {waiting}.q {waiting}"
    )
    both, the_to_do = 2, 1
    retry(lambda _: counted() == the_to_do, timeout=dt.timedelta(seconds=20))
    # A question an orchestrated session asks is its orchestrator's; a yes
    # it waits for is always his.
    wants("question", "WCJQSG")
    retry(lambda _: counted() == the_to_do, timeout=dt.timedelta(seconds=20))
    wants("approval", "WCJQSG")
    retry(lambda _: counted() == both, timeout=dt.timedelta(seconds=20))
    desktop.as_account(f"rm {waiting}")
    desktop.pi_session()
    for title in ["Elsewhere"]:
        window = desktop.window_titled(title)["id"]
        desktop.as_account(f"niri msg action close-window --id {window}")


def reads_on_beside(desktop: Desktop, name: str, said: str) -> None:
    """Hold that what the selected row says, cut to its share, moves while its name holds."""

    def words() -> list[Json]:
        laid = json.loads(desktop.as_account("qs ipc call launcher laidOut"))
        return next(row["words"] for row in laid["rows"] if row["selected"])

    def at(drawn: list[Json], starting: str) -> float:
        return float(next(word["x"] for word in drawn if word["text"].startswith(starting)))

    first = words()
    named = at(first, name)
    seen = {at(first, said)}

    def moved(_: bool) -> bool:
        drawn = words()
        assert at(drawn, name) == named, f"{name} moved while what it says read on: {drawn}"
        seen.add(at(drawn, said))
        return len(seen) > 1

    retry(moved, timeout=dt.timedelta(seconds=30))


def draws_nothing_held_still(desktop: Desktop, who: str, on: str) -> None:
    """Hold that the open launcher, left still on a row, draws at most the foot's minute."""
    # Counted before it's asserted: pytest's rewriting runs a walrus in an assert again for its
    # report (pytest-dev/pytest#14445), which would count four more seconds.
    drawn = drawn_held_still(desktop)
    if drawn <= 1:
        return
    # A selected row with words cut reads on, so the failure says which it holds.
    laid = json.loads(desktop.as_account("qs ipc call launcher laidOut"))
    selected = [
        (word["text"], "cut" if word["cut"] else "whole")
        for row in laid["rows"]
        if row["selected"]
        for word in row["words"]
    ]
    assert drawn <= 1, (
        f"{who} drew {drawn} frames in three seconds left still on {on}, selecting {selected}"
    )


def test_for_yous_filter_lists_what_is_there_and_steps_into_a_kind(desktop: Desktop) -> None:
    """In For You, To Look At holds a look and :filter look steps into it, shot at 1.5 and 2.

    What a filter keeps and the words it takes are tests/for-you-filter.qml's, which band a
    look stands in tests/for-you.qml's, and the filter's one path through the launcher the
    nested niri's; here To Look At, the menu and the head are shot at Joel's scales, for
    VKCF3Z to judge (2026-11-08, 2026-11-09).
    """
    asked = {
        "quest": "QUEST2",
        "items": [{"id": "ask", "title": "Answer the ask", "minutes": 2, "ready": True}],
    }
    # A look names the quest that pointed at it, which the index holds paused, and a
    # session that's gone. It came just now, since one three days old goes quiet.
    look = {"v": 1, "id": "filtered", "title": "Read the plan", "target": "/etc/hostname",
            "kind": "file", "state": "waiting",
            "at": desktop.machine.succeed("date -u +%Y-%m-%dT%H:%M:%SZ").strip(),
            "quest": "QEST-20261027-GONEQA", "session": "pi-reef"}  # fmt: skip
    desktop.as_account(f"mkdir -p {MISSION}/waiting {HARNESS}/looks")
    try:
        kept(desktop, INDEX, {"quests": [{"id": "QEST-20261027-GONEQA", "address": "GONEQA",
                                          "title": "Reef Quest", "status": "paused"}]})  # fmt: skip
        kept(desktop, f"{HARNESS}/looks/filtered.json", look)
        # A to-do ready now is raised as a notice the moment it's heard, which would stand
        # over the launcher in the shots, so it's left to rest first, ten seconds on.
        with desktop.told("rests Answer the ask", timeout=dt.timedelta(seconds=60)):
            kept(desktop, f"{MISSION}/waiting/QUEST2.json", asked)
        # Closed, the launcher lists no looks, so the tray counts the look as the to-do
        # daemon's stream tells it, beside the to-do.
        tray_counts(desktop, 2)
        for_you_counts_its_bands(desktop)
        for scale in (*JOELS_SCALES, "1"):
            named = at_scale_shooting_now(desktop, scale)
            waiting_opened(desktop)
            desktop.machine.send_key("ret")
            # The look blocks nothing and isn't due, so it's in To Look At, after the to-do,
            # a place's row saying a bare count, its drawer naming the heaviest it holds
            # (VKCF3Z, 2026-11-09).
            desktop.launcher_until(
                lambda shown: (
                    shown["rows"][:2] == ["Answer the ask", "To Look At"]
                    and "Read the plan" not in shown["rows"]
                )
            )
            desktop.normal()
            desktop.down_to("To Look At")
            desktop.launcher_until(
                lambda shown: (
                    (shown["selected"] or {}).get("side") == "1"
                    and not (shown["selected"] or {}).get("note")
                    and shown["drawer"]["title"] == "To Look At"
                    and shown["drawer"]["facts"] == ["Heaviest"]
                    and shown["drawer"]["values"] == ["Read the plan"]
                )
            )
            if scale != "1":
                desktop.shoot(f"launcher-for-you-at-{named}")
            desktop.step_in()
            desktop.launcher_until(
                lambda shown: (
                    shown["path"][-2:] == ["For You", "To Look At"]
                    and shown["rows"] == ["Read the plan"]
                )
            )
            if scale != "1":
                desktop.shoot(f"launcher-for-you-to-look-at-at-{named}")
            desktop.machine.send_chars("-")
            desktop.launcher_until(lambda shown: shown["path"][-1:] == ["For You"])
            desktop.normal()
            whole = desktop.showing()["hints"]
            desktop.machine.send_chars(":filter ")
            desktop.launcher_until(lambda shown: (shown["command"] or {}).get("line") == "filter ")
            if scale != "1":
                desktop.shoot(f"launcher-for-you-filter-menu-at-{named}")
            desktop.machine.send_chars("look\n")
            desktop.launcher_until(
                lambda shown: (
                    shown["path"][-2:] == ["For You", "To Look At"]
                    and shown["rows"] == ["Read the plan"]
                )
            )
            # - is the one way back out of a filter, so its foot keeps - up and : wherever
            # For You's own does, the look's d and y giving way first (VKCF3Z, 2026-11-08): at 1.5
            # those are - up and : commands, which the six keys' 505 against 413 of room
            # had cut; at 2 the test's screen, 636 across, leaves room for neither, there
            # or in For You.
            hints = desktop.showing()["hints"]
            assert {"- up", ": commands"} & set(whole) <= set(hints), (scale, whole, hints)
            if scale == "1.5":
                assert {"- up", ": commands"} <= set(hints), (scale, hints)
            if scale != "1":
                desktop.shoot(f"launcher-for-you-filtered-at-{named}")
            else:
                # Its drawer names its quest by its title from the index, once, and says the
                # quest's standing beside its session (VKCF3Z, 2026-11-08).
                desktop.launcher_until(
                    lambda shown: (
                        looks_drawer(shown)
                        == ("from Reef Quest \u00b7 to look at", "pi-reef \u00b7 paused", False)
                    )
                )
            desktop.machine.send_chars("-")
            desktop.launcher_until(lambda shown: shown["path"][-1:] == ["For You"])
            desktop.close_launcher()
        home_counts_what_is_calm(desktop)
    finally:
        desktop.as_account("niri msg output winit scale 1")
        desktop.as_account(
            f"rm -f {MISSION}/waiting/QUEST2.json {HARNESS}/looks/filtered.json {INDEX}"
        )


def for_you_counts_its_bands(desktop: Desktop) -> None:
    """Hold that For You's own row, holding an ask and a look, counts each band it holds.

    It's a place's row, so nothing stands after its name, and its drawer names the heaviest.
    """
    # VKCF3Z, 3a92c450 and 16efb1dc: its row says the bands as a run, needed most first.
    waiting_opened(desktop)
    desktop.launcher_until(
        lambda shown: (
            (shown["selected"] or {}).get("side") == "1 to do \u00b7 1 to look at"
            and not shown["selected"].get("count")
            and not shown["selected"].get("note")
            and shown["drawer"]["title"] == "For You"
            and shown["drawer"]["facts"] == ["Heaviest"]
            and shown["drawer"]["values"] == ["Answer the ask \u00b7 2 min"]
        )
    )
    desktop.close_launcher()


def at_scale_shooting_now(desktop: Desktop, scale: str) -> str:
    """Put niri at a scale and, at Joel's, shoot Now's rows, For You's counting its bands.

    Gives the scale as a shot's name says it, for VKCF3Z to judge (3a92c450).
    """
    desktop.as_account(f"niri msg output winit scale {scale}")
    named = scale.replace(".", "-")
    if scale == "1":
        return named
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.type_in("now", selecting="Now")
    desktop.machine.send_key("ret")
    desktop.launcher_until(lambda shown: shown["path"][-1:] == ["Now"])
    desktop.normal()
    desktop.down_to("For You")
    desktop.launcher_until(
        lambda shown: (shown["selected"] or {}).get("side") == "1 to do \u00b7 1 to look at"
    )
    desktop.shoot(f"launcher-now-bands-at-{named}")
    desktop.close_launcher()
    return named


def home_counts_what_is_calm(desktop: Desktop) -> None:
    """Hold that home's Now, once the ask is gone, says what To Look At holds by its band."""
    # VKCF3Z, a31a0e1b: with nothing needing Joel now, a count stands in for a first thing.
    desktop.as_account(f"rm -f {MISSION}/waiting/QUEST2.json")
    desktop.as_account("qs ipc call launcher toggle")
    desktop.launcher_ready()
    desktop.launcher_until(
        lambda shown: any(thing["reads"] == "Now \u00b7 1 to look at" for thing in shown["things"])
    )
    desktop.close_launcher()


def looks_drawer(shown: dict[str, Any]) -> tuple[str, str | None, bool] | None:
    """Give a shown look's drawer's eyebrow, its Session, and whether a Quest fact says more."""
    drawer = shown.get("drawer") or {}
    if not drawer.get("shown"):
        return None
    facts = dict(zip(drawer["drawnFacts"], drawer["values"], strict=True))
    return drawer["eyebrow"], facts.get("Session"), "Quest" in facts


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

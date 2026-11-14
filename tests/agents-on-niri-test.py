"""An agent's row in the launcher does, on Return, the act its state makes harmless, on niri.

modules/quickshell/checks/agents.nix runs this through pytest in tests/nested-niri.sh. The sessions
are the realm's own mycelium node's, which reads the presence records laid here as pi lays
them, each owned by this test's own process so the node judges it alive; the shell knows them
from the node alone. A listening session's Return asks what to say to it, and a paused one's
lands on Resume, which a second Return sends it; Ctrl+] still steps into all of either's acts.
A session's Close asks first, saying what of it stays, and y steers it a close from Joel.
A drawer's eyebrow names the repo a record says, rather than the folder it works in. One past
80% of its context says how full first, in the warning's hue, and its drawer its figures. A
drawer's first screen says how a session stands, its quest, what it landed last and its
figures, and what its quest is about, which runs long, comes last. Landed lists what landed,
saying how many landed today; what says what to try waits in For You until it's read, its
Return stepping into it in Landed, then stands in For You's History. Both
sit under an orchestrator at work, so they fold into one row after its own, which Return steps
into, and typing still finds each. Of two sessions the last boot left to reopen, the one whose
folder is gone says so on its row before Return, holds only Forget It, and Reopen Them All
counts only the other. Home's Agents row, and the row "agents" finds, say how they stand
together and what they spent today, as spend-watch sums it. Two orchestrators the harness lists,
closed, stand dimmed at the tree's end, the one last live first, each saying what its
orchestration spent today only where it spent anything, Return reopening it. A session closed
this boot, its owner gone, stands alone in Recent, after Landed and before Start a Session,
saying when it closed and where it ran. A folder the disk guard measured offers to talk to the
session working in it, by its title and how it stands, and what Joel tells it reaches its inbox
with the folder and its size. A look's choice, stepped into from For You, reaches the session
that left it as an answer, and its row says so until the session writes it resolved. Shut, a
session arriving with a landing makes none of Agents' rows,
which show both once it opens, and a change to its rows isn't told the shell at once, but told it
at once as it opens.

Long because every case here shares one shell, its node and its sessions, started once a run,
which cost more than all the cases, so splitting it would start them twice.
"""

from __future__ import annotations

import json
import logging
import os
import re
import shlex
import socket
import subprocess
import time
from datetime import UTC, datetime, timedelta
from pathlib import Path
from typing import TYPE_CHECKING, Any, cast

import launcher_on_niri
import pytest

if TYPE_CHECKING:
    from collections.abc import Callable, Iterator

    from launcher_on_niri import Json, Shell

REALM = "harbour"
STATE = Path(os.environ["XDG_STATE_HOME"])
PRESENCE = STATE / "pi/agentic-harness.pi/mesh-workflow/presence"
STEER = STATE / "pi/agentic-harness.pi/steer"
NODE = Path(os.environ["XDG_RUNTIME_DIR"]) / "mycelium" / f"{REALM}.sock"
# Where the agents daemon serves Agents' rows, as its unit's socket is.
ROWS = Path(os.environ["XDG_RUNTIME_DIR"]) / "daemons" / "agents.sock"
RESTORE = STATE / "pi/restore.json"
# Where the last boot's two sessions to reopen ran: one still there, one given back.
KEPT = Path.home() / "harbour"
GIVEN_BACK = Path.home() / "trees/given-back"


def lost_on_the_last_boot() -> None:
    """Lay what the harness keeps of the last boot: two sessions, one whose folder is gone."""
    KEPT.mkdir(parents=True, exist_ok=True)
    lost = [("01b2-harbour", "HARBOR", "Harbour", KEPT), ("01b3-given", "GIVENB", "Given Back",
             GIVEN_BACK)]  # fmt: skip
    RESTORE.parent.mkdir(parents=True, exist_ok=True)
    sessions = [{"id": key, "address": address, "title": title, "cwd": str(cwd), "lostAt": 1}
                for key, address, title, cwd in lost]  # fmt: skip
    RESTORE.write_text(json.dumps({"v": 1, "boot": "an-earlier-boot", "sessions": sessions}))


# What spend-watch writes for the launcher, here only the day's whole spend.
SPEND = Path(os.environ["XDG_RUNTIME_DIR"]) / "spend-watch" / "collection.json"
WORKFLOW = STATE / "pi/agentic-harness.pi/quest-workflow"
# Two orchestrators closed, by address, short name, quest's title and when each was last live:
# Lamplight the later, though the harness lists it second.
CLOSED = [
    ("BEACON", "Beacon", "Light the beacon", "2026-11-06T22:00:00Z"),
    ("LAMPLT", "Lamplight", "Keep the lamp lit", "2026-11-07T04:00:00Z"),
]


def orchestrators_listed() -> None:
    """Lay what the harness keeps of its orchestrators: the one at work, and two closed."""
    WORKFLOW.mkdir(parents=True, exist_ok=True)
    listed = [{"quest": "QEST-20261107-BOSSES", "address": "BOSSES",
               "title": "Orchestrating Quest"}]  # fmt: skip
    quests = []
    for address, short, title, at in CLOSED:
        quest = f"QEST-20261107-{address}"
        listed.append({"quest": quest, "address": address, "title": title, "short": short})
        resume = {"id": f"01b4-{address.lower()}", "cwd": str(KEPT), "at": at}
        quests.append({"id": quest, "address": address, "title": title, "status": "active",
                       "resume": resume, "start": str(KEPT)})  # fmt: skip
    (WORKFLOW / "orchestrators.json").write_text(json.dumps({"orchestrators": listed}))
    (WORKFLOW / "quests.json").write_text(json.dumps({"quests": quests}))


# What the paused session landed, newest first: a change merged, a plan item ticked alone, which
# Landed leaves out, and a change saying what to try.
STOPPED = "An Agent's Stop Waits for a Second Return"
CLOSING = "A Session Closes From the Launcher"
# Its line for Joel, which names it by what to try, its first sentence, in Landed and To Try
# alike, and says the rest in its drawer (VKCF3Z, c2d01d53).
TRY_FIRST = "Press d on a session."
TRY_REST = "It asks what stays before it closes."
TRY = f"{TRY_FIRST} {TRY_REST}"
ORCHESTRATING = "Orchestrating Quest"
# How many times the drawer case goes down and back up between two sessions.
ARROWS_PAST = 3
# The two seconds a session's drawer must stay on its row to read its answer, and a while longer.
A_STAY = 2.0
PAST_A_STAY = 2.5
# What a session works on, as pi says it: its quest's title, then its summary, which runs to
# about 290 letters on every real session.
WORKS_ON = (
    "Listening Quest: A builder under the owner's plan, which keeps the launcher's agents"
    " quiet, told and easy to read: every session says how it stands, what it does, what it"
    " spent and what it needs from Joel, from the node, with nothing guessed and nothing"
    " said twice, held to the launcher's family and its keys."
)
# They landed minutes before the walk, inside the day a landing waits in To Try for.
LANDED = datetime.now(UTC)
LANDINGS = [
    {"at": (LANDED - timedelta(minutes=1)).isoformat(), "ref": "core.nix#2140", "what": STOPPED},
    {
        "at": (LANDED - timedelta(minutes=2)).isoformat(),
        "ref": "PLAN-20261107-PAUSED#2",
        "what": "Ticked alone",
    },
    {
        "at": (LANDED - timedelta(minutes=3)).isoformat(),
        "ref": "core.nix#2138",
        "what": CLOSING,
        "person": TRY,
    },
]


def landed_today() -> str:
    """Say what Landed counts of the walk's changes landed on today's date where it runs."""
    today = datetime.now().astimezone().date()
    count = sum(
        1
        for one in LANDINGS
        if "core.nix#" in one["ref"]
        and datetime.fromisoformat(one["at"]).astimezone().date() == today
    )
    return f"{count} today" if count else "none today"


# How full a session's context is, and its turns today, as the harness's usage keeps them by
# gen_ai's names, counted on the day the sandbox's clock says.
FULL = {
    "day": datetime.now().astimezone().date().isoformat(),
    "gen_ai.response.model": "claude-opus-5-5",
    "last": {"gen_ai.usage.output_tokens": 4100},
    "today": {
        "gen_ai.usage.input_tokens": 786,
        "gen_ai.usage.output_tokens": 272470,
        "gen_ai.usage.cache_read.input_tokens": 124620036,
        "gen_ai.usage.cache_creation.input_tokens": 1194960,
    },
    "context": {"used": 842000, "limit": 1000000},
    "turns": {"today": 359, "last": {"duration_ms": 38000, "first_token_ms": 900}},
}


def owner() -> Json:
    """Give this process as pi's presence names its owner: its pid, the boot and its start."""
    stat = Path("/proc/self/stat").read_text()
    ticks = int(stat[stat.rindex(")") + 2 :].split()[19])
    return {
        "pid": os.getpid(),
        "bootId": Path("/proc/sys/kernel/random/boot_id").read_text().strip(),
        "startTicks": ticks,
        "startedAt": int(time.time() * 1000),
    }


def gone_owner() -> Json:
    """Give an owner whose process ran and ended this boot, as a session closed since reads."""
    process = subprocess.Popen(["true"])
    process.wait()
    return {**owner(), "pid": process.pid}


def lay(address: str, title: str, said: Json, *, closed: bool = False) -> None:
    """Lay a session's presence record whole, by rename, as pi does; closed, its owner gone."""
    record = {
        "session": f"s-{address.lower()}",
        "addresses": [address],
        "title": f"\u03c0 - {title}",
        "questInfo": {"id": f"QEST-20261107-{address}", "title": title, "short": address,
                      "status": "active"},
        "cwd": str(STATE / address.lower()),
        "owner": gone_owner() if closed else owner(),
        # When it was written, which the node reads every record by: one
        # without it isn't a session to the node.
        "at": datetime.now(UTC).isoformat(),
        **said,
    }  # fmt: skip
    laid = PRESENCE / f"{address.lower()}.json.tmp"
    laid.write_text(json.dumps(record))
    laid.rename(PRESENCE / f"{address.lower()}.json")


def held_by_node() -> list[str]:
    """Give the sessions the node holds alive, by their keys, asked over its daemon interface."""
    with socket.socket(socket.AF_UNIX) as node:
        node.connect(str(NODE))
        node.sendall(b'{"id": 1, "ask": "hello"}\n{"id": 2, "ask": "list", "kind": "session"}\n')
        said = node.makefile()
        while True:
            answer = json.loads(said.readline())
            if answer.get("id") == 2:  # noqa: PLR2004
                return sorted(
                    record["key"]
                    for record in answer.get("records", [])
                    if record.get("state") != "ended"
                )


@pytest.fixture(scope="module")
def shell() -> Iterator[Shell]:
    """Give the shell, started once a run after the realm's node and the two sessions."""
    PRESENCE.mkdir(parents=True, exist_ok=True)
    lay("BOSSES", ORCHESTRATING, {"busy": True, "state": "busy"})
    under = {"orchestratedBy": "BOSSES", "busy": False}
    lay(
        "LISTEN",
        "Listening Quest",
        {
            **under,
            "state": "listening",
            "repo": "core.nix",
            "usage": FULL,
            "worksOn": WORKS_ON,
        },
    )
    lay("PAUSED", "Paused Quest", {**under, "state": "paused", "landings": LANDINGS})
    lay("CLOSED", "Closed Quest", {"state": "idle", "repo": "tidepool"}, closed=True)
    lost_on_the_last_boot()
    orchestrators_listed()
    SPEND.parent.mkdir(parents=True, exist_ok=True)
    SPEND.write_text(json.dumps({"v": 2, "agentsToday": "$214"}))
    logs = Path(os.environ["LOGS"])
    logs.mkdir(parents=True, exist_ok=True)
    # It serves the harness's quests, as the account's node does (modules/mycelium/home.nix),
    # which say how each closed orchestrator reopens, the talks in the steer inboxes, and what
    # the void wrote of each line past it, as one record.
    serves = logs / "serves.json"
    quest = {"file": str(WORKFLOW / "quests.json"), "records": "quests", "key": "id"}
    answers = {"file": str(STATE / "void/answers.jsonl"), "whole": True}
    (STATE / "void").mkdir(parents=True, exist_ok=True)
    serves.write_text(
        json.dumps({"quest": quest, "talk": {"talks": str(STEER)}, "void-answers": answers})
    )
    # The node serves no talks from a folder missing as it starts, as Joel's never is: run
    # alone, Super+S's cases laid the first steer after it, and their answers never came.
    STEER.mkdir(parents=True, exist_ok=True)
    with (logs / "mycelium.log").open("w") as log:
        # The node is the store's own, which the check names.
        node = subprocess.Popen(
            [os.environ["MYCELIUM"], "--realm", REALM, "node", "--serves", str(serves)],
            stdout=log,
            stderr=log,
        )
    deadline = time.monotonic() + 30
    while not NODE.exists():
        assert node.poll() is None, (logs / "mycelium.log").read_text()
        assert time.monotonic() < deadline, "the node never listened"
        time.sleep(0.1)
    wanted = ["session:s-bosses", "session:s-listen", "session:s-paused"]
    while held_by_node() != wanted:
        assert time.monotonic() < deadline, f"the node holds {held_by_node()}, not {wanted}"
        time.sleep(0.1)
    with (logs / "agents.log").open("w") as log:
        # The agents daemon makes Agents' rows from the node, the store's own as the check
        # names it, as the account's unit runs it, and the sessions to reopen from what
        # the harness kept; this sandbox has no to-do daemon, nor any windows daemon.
        rows = subprocess.Popen(
            [os.environ["AGENTS"], "--vocabulary", os.environ["WORDS"], "--node", str(NODE),
             "--to-do", str(ROWS.parent / "to-do.sock"),
             "--windows", str(ROWS.parent / "windows.sock"), "--spend", str(SPEND),
             "--orchestrators", str(WORKFLOW / "orchestrators.json"),
             "--restore", str(RESTORE),
             "--kept", str(STATE / "agents/reopen.json"),
             "--landed", str(STATE / "agents/landed.json"), "--steer", str(STEER),
             "--shells", str(STATE / "quickshell/by-shell"), "--socket", str(ROWS)],
            stdout=log, stderr=log,
        )  # fmt: skip
    while not ROWS.exists():
        assert rows.poll() is None, (logs / "agents.log").read_text()
        assert time.monotonic() < deadline, "the agents daemon never listened"
        time.sleep(0.1)
    started = launcher_on_niri.started()
    yield started
    started.stop()
    rows.terminate()
    rows.wait()
    node.terminate()
    node.wait()


@pytest.fixture(autouse=True)
def launcher_closed_after(shell: Shell) -> Iterator[None]:
    """Close the launcher after each test, so one failing with it open fails alone."""
    yield
    shell.close_launcher()


def opened(shell: Shell) -> None:
    """Open Agents, once it lists the orchestration as the daemon serves it, one row."""
    shell.as_account(shell.ipc("collection", "agents"))
    shell.launcher_ready()
    shell.launcher_until(
        lambda shown: (
            shown["held"] == ["agents"]
            and shown["rows"][:2] == ["Orchestrating Quest", "Closed"]
            and shown["selected"]["side"] == "1 of 3 working"
        )
    )


FOLD = "1 listening, 1 paused"


def into_orchestration(shell: Shell) -> None:
    """Open Agents and step into its orchestration: its own acts, Talk first, then its fold.

    Its acts lead what's inside, as a meeting's do, so Ctrl+] and the command line reach
    them as the tree's orchestrator row did (KP0L6A, 2026-11-12).
    """
    opened(shell)
    shell.step_in("ctrl-bracketright")
    shell.launcher_until(
        lambda shown: (
            shown["rows"][:1] == ["Talk"]
            and FOLD in shown["rows"]
            and "Stop" in shown["rows"][: shown["rows"].index(FOLD)]
        )
    )


def found(shell: Shell, typed: str, name: str) -> Json:
    """Open Agents, find a session's row there by its quest's name, and give the row.

    Typing finds it in its orchestration's fold as well.
    """
    opened(shell)
    shell.type_in(typed, selecting=name)
    return cast("Json", shell.showing()["selected"])


def test_a_listening_sessions_return_asks_what_to_say_and_its_acts_stay_inside(
    shell: Shell,
) -> None:
    """A listening session's Return asks what to say to it; Ctrl+] holds every act, Talk too."""
    row = found(shell, "listening quest", "Listening Quest")
    assert row["verb"] == "talk", row
    assert row["told"][:1] == ["\u21b5 talk"], row
    # Asking changes nothing, so Return from the find asks at once.
    shell.send_key("ret")
    shell.launcher_until(
        lambda shown: shown["asking"] and shown["held"] == ["agents", "Listening Quest"]
    )
    # :q typed alone leaves the line rather than going to it as words, as Joel's vi keys
    # mean it: words given keep it asking for more, saying whom they went to.
    shell.send_chars(":q")
    shell.send_key("ret")
    shell.launcher_until(lambda shown: not shown["asking"])
    assert "Listening Quest" not in shell.showing()["footer"], shell.showing()
    shell.close_launcher()
    found(shell, "listening quest", "Listening Quest")
    shell.step_in("ctrl-bracketright")
    shell.launcher_until(lambda shown: shown["rows"][:2] == ["Talk", "Pause"])
    assert not shell.showing()["asking"], shell.showing()


def test_a_talks_words_come_back_after_the_shell_starts_again(shell: Shell) -> None:
    """What Joel was typing to a session is in its field again once a switch restarts the shell.

    A switch took his words with the shell (Joel, 2026-11-12 16:00: "had it disappear through a
    quickshell switch losing what i was typing"). They're written a quarter second after he
    stops, and a switch waits half a minute past the launcher closing, so the shell ends once
    they're written. What each change keeps, and when sent words go, is tests/drafts.qml's.
    """
    words = "is the harbour light lit"
    found(shell, "listening quest", "Listening Quest")
    shell.send_key("ret")
    shell.launcher_until(lambda shown: shown["asking"])
    shell.send_chars(words)
    shell.launcher_until(lambda shown: shown["typed"] == words)
    caret = shell.showing()["caret"]
    shells = STATE / "quickshell/by-shell"
    deadline = time.monotonic() + 10
    while not any(words in kept.read_text() for kept in shells.glob("*/launcher-drafts.json")):
        assert time.monotonic() < deadline, f"his words were never kept in {shells}"
        time.sleep(0.05)
    shell.restart()
    found(shell, "listening quest", "Listening Quest")
    shell.send_key("ret")
    shell.launcher_until(lambda shown: shown["asking"])
    shown = shell.showing()
    assert (shown["typed"], shown["caret"]) == (words, caret), shown
    # Emptied, the field keeps nothing for the next time it opens.
    shell.send_key("ctrl-u")
    shell.launcher_until(lambda shown: shown["typed"] == "")


def test_a_sessions_drawer_names_the_repo_its_record_says_over_its_folder(shell: Shell) -> None:
    """The drawer's eyebrow says the repo a record names, not the folder it works in."""
    found(shell, "listening quest", "Listening Quest")
    shell.launcher_until(
        lambda shown: (
            (shown["drawer"] or {}).get("eyebrow", "").startswith("LISTEN \u00b7 core.nix \u00b7")
        )
    )


def test_a_session_past_80_percent_full_says_so_first_and_its_drawer_its_figures(
    shell: Shell,
) -> None:
    """Past 80% of its context, a row says how full before how it stands, in the warning's hue."""
    row = found(shell, "listening quest", "Listening Quest")
    # Found by typing, whom it's under leads, before what's its own.
    assert row["side"].endswith(" \u00b7 84% full \u00b7 listening"), row
    assert row["hue"] == "warning", row
    # Its drawer gives its figures by gen_ai's names after what's known of it, how full
    # its context is and its last turn first.
    figures = ["Context", "Last Turn", "Model", "Turns Today", "Tokens", "Cache"]
    shell.launcher_until(lambda shown: ", ".join(figures) in ", ".join(shown["drawer"]["facts"]))
    # Context alone draws a figure beside its value, Meter's stroke of how full.
    shell.launcher_until(lambda shown: drawn_figures(shown) == {"Context": "stroke"})
    # Once spend-watch says how its day went, its spend today carries its hours as a
    # spark, and what rebuilding its context cost follows, with no key pressed.
    day = {"graph": {"shape": "spark", "values": [0, 0.4, 1]}, "rebuilt": "$1.20"}
    spent = {"sessions": {"s-listen": "$4.20"}, "sessionsToday": {"s-listen": day}}
    SPEND.write_text(json.dumps({"v": 2, "agentsToday": "$214", **spent}))
    try:
        shell.launcher_until(
            lambda shown: (
                drawn_figures(shown) == {"Context": "stroke", "Spent Today": "spark"}
                and "Context Rebuilt" in shown["drawer"]["facts"]
            )
        )
    finally:
        SPEND.write_text(json.dumps({"v": 2, "agentsToday": "$214"}))


def shown_first(drawer: Json) -> list[str]:
    """Give the facts a drawer's first screen shows, before any it says are more."""
    said = str(drawer.get("moreSays") or "")
    hidden = int(said.split(maxsplit=1)[0]) if said else 0
    facts = cast("list[str]", drawer["facts"])
    return facts[: len(facts) - hidden]


def test_a_sessions_drawer_shows_what_matters_first_and_what_its_quest_is_about_last(
    shell: Shell,
) -> None:
    """Its first screen holds what it landed, what it spent and its figures.

    How it stands is its row's, so with no step it says no Doing; its quest, nearly always
    active, comes after its figures. What its quest is about runs long, as every real
    session's does, so it comes last and leaves out the title the drawer already says; it
    once came first and hid all the rest.
    """
    found(shell, "listening quest", "Listening Quest")
    wanted = "Context, Last Turn, Model"
    shell.launcher_until(
        lambda shown: (
            shown["drawer"]["shown"]
            and wanted in ", ".join(shown_first(shown["drawer"]))
            and shown["drawer"]["facts"][-1] == "Working On"
        )
    )
    drawer = shell.showing()["drawer"]
    facts = drawer["facts"]
    assert "Doing" not in facts, drawer
    assert facts.index("Quest") > facts.index("Cache"), drawer
    # Its quest is active, which goes unsaid.
    assert drawer["values"][facts.index("Quest")] == "QEST-20261107-LISTEN", drawer
    summary = WORKS_ON.removeprefix("Listening Quest: ")
    assert drawer["values"][-1] == summary, drawer
    # Nor does its row's note, under the same title; found by typing, whom it's under leads.
    note = shell.showing()["selected"]["note"]
    assert note == f"under BOSSES \u00b7 {summary}", note
    # One that landed something says what it landed last on its first screen too.
    found(shell, "paused quest", "Paused Quest")
    shell.launcher_until(
        lambda shown: (
            shown["drawer"]["shown"]
            and shown_first(shown["drawer"])[:1] == ["Last Landing"]
            and shown["drawer"]["values"][0].startswith(f"{STOPPED} \u00b7 core.nix#2140 \u00b7 ")
        )
    )


def drawn_figures(shown: dict[str, Any]) -> dict[str, str]:
    """Each drawer fact drawing a figure, by its name, to the figure's shape."""
    drawer = shown["drawer"]
    return {
        fact: figure
        for fact, figure in zip(drawer["drawnFacts"], drawer["figures"], strict=True)
        if figure
    }


def test_a_paused_sessions_glyph_is_never_drawn_in_a_live_ones_colour(shell: Shell) -> None:
    """A paused row's glyph is muted, never the listening row's colour (#2052)."""
    # Both only wait under the orchestrator, so they're drawn inside its fold.
    into_orchestration(shell)
    shell.down_to("1 listening, 1 paused")
    shell.step_in()
    names = ("Listening Quest", "Paused Quest")
    shell.launcher_until(lambda shown: all(name in shown["rows"] for name in names))
    laid = json.loads(shell.as_account(shell.ipc("laidOut")))
    inks = {
        name: row["words"][0]["ink"]
        for row in laid["rows"]
        for name in names
        if any(word["text"] == name for word in row["words"])
    }
    assert set(inks) == set(names), laid["rows"]
    assert inks["Paused Quest"] != inks["Listening Quest"], f"paused looks live: {inks}"


def test_a_sessions_drawer_lists_its_acts_and_arrows_past_it_make_none_of_its_rows(
    shell: Shell,
) -> None:
    """A session's drawer lists only its acts, and moving between sessions makes no rows.

    Listing them by making the row's verbs made its facts and acts as rows, about nine for each
    arrow resting on a session, 891 in a held arrow's turns (keeps-up, 2026-11-12), and named
    each fact as an act. The sessions are inside their orchestration's fold.
    """
    into_orchestration(shell)
    shell.down_to(FOLD)
    shell.step_in()
    names = ("Listening Quest", "Paused Quest")
    shell.launcher_until(lambda shown: all(name in shown["rows"] for name in names))

    def rested_on(name: str) -> Callable[[Json], bool]:
        return lambda shown: (
            (shown["selected"] or {}).get("name") == name
            and bool((shown["drawer"] or {}).get("shown"))
            and bool((shown["drawer"] or {}).get("acts"))
        )

    shell.down_to(names[0])
    shell.launcher_until(rested_on(names[0]))
    acts = shell.showing()["drawer"]["acts"]
    # Inside its orchestration a session needing nothing of Joel isn't talked to there
    # (VKCF3Z, 3a742b67), so Return's act leads whichever the daemon serves first.
    assert acts[0].startswith("\u21b5 "), acts
    assert not {"Quest", "Address"} & {act.split(" ")[-1] for act in acts}, acts
    before = made_by_agents(shell)
    for _ in range(ARROWS_PAST):
        for key, name in (("down", names[1]), ("up", names[0])):
            shell.send_key(key)
            shell.launcher_until(rested_on(name))
    made = made_by_agents(shell) - before
    assert made == 0, f"arrows between two sessions made {made} of Agents' rows"


def test_a_paused_sessions_return_lands_on_resume_and_a_second_sends_it(shell: Shell) -> None:
    """A paused session's Return lands on Resume, which changes how it stands; a second resumes."""
    row = found(shell, "paused quest", "Paused Quest")
    assert row["verb"] == "resume", row
    # From the find its foot names the act Return leads to, though the first
    # press only lands: no Return hint is the glyph alone (VKCF3Z, 2026-11-07).
    assert row["told"][:1] == ["\u21b5 resume"], row
    assert "\u21b5" not in row["told"], row
    shell.send_key("ret")
    shell.launcher_until(
        lambda shown: (
            shown["mode"] == "normal"
            and (shown["selected"] or {}).get("told", [])[:1] == ["\u21b5 resume"]
            # A Return this soon after landing is taken for the same one
            # doubled, and does nothing.
            and not shown["landedFresh"]
        )
    )
    shell.send_key("ret")
    shell.launcher_until(lambda shown: not shown["open"])
    inbox = STEER / "PAUSED"
    deadline = time.monotonic() + 10
    while not any(inbox.glob("*.json")):
        assert time.monotonic() < deadline, "nothing reached the paused session's inbox"
        time.sleep(0.1)
    told = [json.loads(one.read_text()) for one in inbox.glob("*.json")]
    assert [one["act"] for one in told] == ["resume"], told
    # A line told from anywhere, naming the quest, says who hears it before it's sent, then
    # reaches that session's inbox as a talk does.
    asked = {"ask": "tell", "window": None, "quest": "QEST-20261107-PAUSED"}
    assert rows_answer(asked) == {"to": "PAUSED", "name": "Paused Quest", "why": "quest"}
    sent = rows_answer({**asked, "words": "how goes it?"})
    assert "steered" in sent["sent"], sent
    deadline = time.monotonic() + 10
    while not any("text" in json.loads(one.read_text()) for one in inbox.glob("*.json")):
        assert time.monotonic() < deadline, "the line told never reached the session's inbox"
        time.sleep(0.1)
    talks = [json.loads(one.read_text()) for one in inbox.glob("*.json")]
    assert [one["text"] for one in talks if "text" in one] == ["how goes it?"], talks


def test_a_sessions_stop_is_an_ending_return_writes_and_a_second_steers(shell: Shell) -> None:
    """Return on Stop writes :interrupt rather than stopping; a second steers a stop, unasked."""
    into_orchestration(shell)
    shell.down_to("Stop")
    inbox = STEER / "BOSSES"

    def stops() -> list[Path]:
        return [p for p in inbox.glob("*.json") if json.loads(p.read_text())["act"] == "stop"]

    shell.send_key("ret")
    shell.launcher_until(
        lambda shown: (
            (shown["command"] or {}).get("line") == "interrupt" and not shown["linedFresh"]
        )
    )
    assert stops() == [], "the first Return stopped the session"
    shell.send_key("ret")
    deadline = time.monotonic() + 10
    while not stops():
        assert time.monotonic() < deadline, "nothing reached the orchestrator's inbox"
        assert not shell.showing()["takesAnswer"], shell.showing()
        time.sleep(0.1)
    told = [json.loads(path.read_text()) for path in stops()]
    assert [(one["act"], one["from"]) for one in told] == [("stop", "joel")], told
    # Close's case below reads every word the orchestrator was sent.
    for path in stops():
        path.unlink()


def test_a_sessions_close_asks_saying_what_stays_and_y_steers_it_closed(shell: Shell) -> None:
    """Close, last among a session's acts, asks what it leaves, and y steers it a close."""
    into_orchestration(shell)
    rows = shell.showing()["rows"]
    assert rows[: rows.index(FOLD)][-1:] == ["Close"], rows
    shell.down_to("Close")
    # Return writes the line Close is, and a second runs it, which asks.
    shell.send_key("ret")
    shell.launcher_until(
        lambda shown: (shown["command"] or {}).get("line") == "close" and not shown["linedFresh"]
    )
    shell.send_key("ret")
    shell.launcher_until(lambda shown: shown["takesAnswer"])
    asked = "Close BOSSES? Its folder stays. (y/n)"
    assert shell.showing()["question"] == asked, shell.showing()
    shell.send_chars("y")
    inbox = STEER / "BOSSES"
    deadline = time.monotonic() + 10
    while not any(inbox.glob("*.json")):
        assert time.monotonic() < deadline, "nothing reached the orchestrator's inbox"
        time.sleep(0.1)
    told = [json.loads(one.read_text()) for one in inbox.glob("*.json")]
    assert [(one["act"], one["from"]) for one in told] == [("close", "joel")], told


def test_landed_keeps_each_landing_and_what_to_try_waits_in_for_you_until_read(
    shell: Shell,
) -> None:
    """Landed lists what landed, but a plan item ticked alone, saying how many landed today.

    What says what to try waits in For You's To Try, never counted in Landed too; its Return
    steps into it where it lives, in Landed, and reads it, so it goes to For You's History.
    A session changing while Joel is in Landed makes none of its rows again: they were made
    anew at each change, 1,683 a change on Joel's shell (LR3WMN, 2026-11-12).
    """
    opened(shell)
    shell.launcher_until(lambda shown: "Landed" in shown["rows"])
    shell.down_to("Landed")
    # It counts what landed today, a plan item ticked alone not among them, and what's to
    # read is For You's alone to count.
    assert shell.showing()["selected"]["side"] == landed_today(), shell.showing()
    shell.step_in()
    shell.launcher_until(lambda shown: shown["rows"] == [STOPPED, TRY_FIRST])
    assert shell.showing()["selected"]["side"] not in {"", "core.nix#2140"}, shell.showing()
    # No link to it is known and its session has no window, so normal's Return only reads it,
    # keeping the launcher; the find's lands on it, as on any row that changes something.
    shell.normal()
    shell.send_key("ret")
    shell.close_launcher()
    # Only the one saying what to try waits in For You, the one read or not.
    in_for_you(shell)
    shell.launcher_until(lambda shown: "To Try" in shown["rows"])
    shell.down_to("To Try")
    shell.step_in()
    shell.launcher_until(lambda shown: shown["rows"] == [TRY_FIRST])
    assert shell.showing()["selected"]["side"] not in {"", "core.nix#2138"}, shell.showing()
    # Its side says when it landed; its drawer is titled by what to try and says the rest
    # after its facts, one of which is what changed. Its change is among the nerdy bits, never
    # a fact, with where it landed, which this one doesn't say (VKCF3Z, ee3bac47, c2d01d53).
    shell.launcher_until(
        lambda shown: (
            (shown["drawer"] or {}).get("title") == TRY_FIRST
            and shown["drawer"]["body"] == TRY_REST
            and "Change" in shown["drawer"]["nerd"]
            and CLOSING in shown["drawer"]["values"]
            and "core.nix#2138" not in shown["drawer"]["values"]
        )
    )
    # Return steps into it where it lives, never a copy of it.
    shell.send_key("ret")
    shell.launcher_until(lambda shown: shown["held"] == ["agents", "Landed", TRY_FIRST])
    assert "Copy What to Try" in shell.showing()["rows"], shell.showing()
    # What Joel makes of it goes to whoever landed it, its session here, as his finding, and
    # is kept a line each for the loop that reads how landings were taken. The foot names
    # where it went as Joel reads it, by its quest's title, never an address (VKCF3Z).
    shell.down_to("Say What You Make of It")
    shell.send_key("ret")
    shell.launcher_until(lambda shown: shown["asking"])
    shell.send_chars("too dim")
    shell.send_key("ret")
    shell.launcher_until(lambda shown: "sent to Paused Quest" in shown["footer"])
    kept = json.loads((STATE / "agents/feedback.jsonl").read_text())
    assert (kept["ref"], kept["address"], kept["line"], kept["words"]) == (
        "core.nix#2138",
        "PAUSED",
        TRY,
        "too dim",
    ), kept
    assert kept["sent"], kept
    # Kept in whole seconds, as the loop keeps its days.
    assert isinstance(kept["at"], int), kept
    shell.close_launcher()
    # Read, it's gone from To Try and in For You's History, saying when; Landed keeps both.
    in_for_you(shell)
    shell.launcher_until(lambda shown: "To Try" not in shown["rows"])
    shell.down_to("History")
    shell.step_in()
    shell.launcher_until(lambda shown: TRY_FIRST in shown["rows"])
    shell.down_to(TRY_FIRST)
    assert re.fullmatch(r"read .+", shell.showing()["selected"]["side"]), shell.showing()
    shell.close_launcher()
    opened(shell)
    boss = {"busy": True, "state": "busy"}
    at_top = made_through(shell, lambda: lay("BOSSES", ORCHESTRATING, {**boss, "usage": FULL}))
    shell.down_to("Landed")
    shell.step_in()
    shell.launcher_until(lambda shown: shown["rows"] == [STOPPED, TRY_FIRST])
    # The same row changing back costs no more held in Landed than it did at Agents' top.
    inside = made_through(shell, lambda: lay("BOSSES", ORCHESTRATING, boss))
    # Agents' top is laid out once for each source that hears a change, Sessions and the agents
    # daemon, and the two land in one pass or two as they fall, so a change costs up to twice
    # what one pass did; remaking Landed's landings cost 26 to the top's 9.
    assert inside <= 2 * at_top, f"a change made {inside} rows in Landed, {at_top} at Agents' top"


def made_through(shell: Shell, change: Callable[[], None]) -> int:
    """Give how many rows Agents made as the shell heard what `change` changed, a second on."""
    lines, before = agents_lines(shell), made_by_agents(shell)
    change()
    deadline = time.monotonic() + 10
    while agents_lines(shell) == lines:
        assert time.monotonic() < deadline, "the shell never heard the change"
        time.sleep(0.1)
    # A change can come as a few lines, its row's then its drawer's, so it's taken once a
    # second passes with none.
    lines, quiet = agents_lines(shell), time.monotonic()
    while time.monotonic() - quiet < 1:
        time.sleep(0.1)
        if agents_lines(shell) != lines:
            lines, quiet = agents_lines(shell), time.monotonic()
    return made_by_agents(shell) - before


def in_for_you(shell: Shell) -> None:
    """Open Now and step into For You."""
    shell.as_account(shell.ipc("collection", "on"))
    shell.launcher_ready()
    shell.launcher_until(lambda shown: shown["held"] == ["now"] and "For You" in shown["rows"])
    shell.down_to("For You")
    shell.step_in()
    shell.launcher_until(lambda shown: shown["held"] == ["now", "For You"])


def test_an_orchestrations_waiting_sessions_fold_into_one_row_return_steps_into(
    shell: Shell,
) -> None:
    """Those only waiting fold into one row saying each state it holds; Return steps in to them."""
    into_orchestration(shell)
    shell.down_to("1 listening, 1 paused")
    assert shell.showing()["selected"]["told"][:1] == ["\u21b5 open"], shell.showing()
    shell.step_in()
    shell.launcher_until(lambda shown: shown["rows"][:2] == ["Listening Quest", "Paused Quest"])


def test_agents_top_level_is_the_rows_its_daemon_serves_ready(shell: Shell) -> None:
    """Agents opens on the orchestration rows the agents daemon serves as agents-top.

    The shell draws them as served rather than laying them out (VKCF3Z, 3a742b67).
    """
    opened(shell)
    row = shell.showing()["selected"]
    # The daemon says how its sessions stand together; the tree said only its own state.
    assert row["side"] == "1 of 3 working", row
    assert row["told"][:1] == ["\u21b5 talk"], row

    # Its drawer is its session's, worded as every session's is: the daemon
    # serves that eyebrow as items and its facts as labels and values, which
    # read as a ready row's drew "[object" over blank facts.
    def worded(shown: Json) -> bool:
        drawer = shown["drawer"]
        return (
            drawer["shown"]
            and isinstance(drawer["eyebrow"], str)
            and drawer["eyebrow"] != ""
            and drawer["facts"] != []
            and all(label != "" for label in drawer["facts"])
        )

    shell.launcher_until(worded)
    assert not shell.showing()["drawer"]["eyebrowDrawn"].startswith("[object"), shell.showing()


def test_a_session_whose_folder_is_gone_says_so_before_return_and_is_forgotten(
    shell: Shell,
) -> None:
    """One to reopen whose folder is gone says so on its row and holds only Forget It.

    Reopen Them All counts what it will reopen, the other alone, rather than a notice saying
    after that one wasn't (VKCF3Z, 2026-11-07).
    """
    opened(shell)
    shell.down_to("Sessions to Reopen")
    shell.step_in()
    shell.launcher_until(
        lambda shown: (
            shown["rows"] == ["Reopen Them All", "Harbour", "Given Back", "Not Now"]
            and (shown["selected"] or {}).get("side") == "1 of 2 \u00b7 1 folder gone"
        )
    )
    shell.down_to("Given Back")
    row = shell.showing()["selected"]
    gone = (row["side"], row["alerts"], row["hue"])
    assert gone == ("~/trees/given-back is gone", True, "alert"), row
    # The count of what failed wears the ink the row that failed says it in,
    # and "1 of 2" the side's own.
    words = [w for r in json.loads(shell.as_account(shell.ipc("laidOut")))["rows"]
             for w in r["words"]]  # fmt: skip
    alert = next(w["ink"] for w in words if w["text"] == "~/trees/given-back is gone")
    count = next(w["styled"] for w in words if w["text"] == "1 of 2 \u00b7 1 folder gone")
    assert f'<font color="{alert}">1 folder gone</font>' in count, (alert, count)
    assert count.startswith("1 of 2 "), count
    # It holds no Reopen: Return steps in, to Forget It alone.
    assert row["told"][:1] == ["\u21b5 open"], row
    shell.step_in()
    shell.launcher_until(lambda shown: shown["rows"] == ["Forget It"])
    # Forgetting ends what's offered, so Return writes :forget, which asks.
    shell.send_key("ret")
    shell.launcher_until(
        lambda shown: (shown["command"] or {}).get("line") == "forget" and not shown["linedFresh"]
    )
    shell.send_key("ret")
    shell.launcher_until(lambda shown: shown["takesAnswer"])
    shell.send_chars("y")
    shell.launcher_until(lambda shown: not shown["open"])
    # Forgotten, it's no longer offered, and the harness's record is as it wrote it.
    opened(shell)
    shell.down_to("Sessions to Reopen")
    assert shell.showing()["selected"]["side"] == "1 session", shell.showing()
    shell.step_in()
    shell.launcher_until(lambda shown: shown["rows"] == ["Reopen Them All", "Harbour", "Not Now"])
    kept = [one["id"] for one in json.loads(RESTORE.read_text())["sessions"]]
    assert kept == ["01b2-harbour", "01b3-given"], kept


def test_the_sessions_to_reopen_are_a_thing_to_do_in_for_you_not_a_row_of_nows(
    shell: Shell,
) -> None:
    """Sessions to Reopen waits on Joel in For You, Agents' own row, and Now has none of it."""
    shell.as_account(shell.ipc("collection", "on"))
    shell.launcher_ready()
    shell.launcher_until(lambda shown: shown["held"] == ["now"] and "For You" in shown["rows"])
    assert "Sessions to Reopen" not in shell.showing()["rows"], shell.showing()
    shell.close_launcher()
    in_for_you(shell)
    shell.launcher_until(lambda shown: "Sessions to Reopen" in shown["rows"])
    # The same row as in Agents: Return steps in to Reopen Them All.
    shell.down_to("Sessions to Reopen")
    shell.step_in()
    shell.launcher_until(lambda shown: shown["rows"][:1] == ["Reopen Them All"])


def test_agents_says_how_its_sessions_stand_together_at_home_and_found_by_name(
    shell: Shell,
) -> None:
    """Home's Agents row and the row "agents" finds say "1 working \u00b7 $214 today" at the right.

    At home, as everywhere a place's row stands, nothing is said after its name, how its sessions
    stand together is said at its right, and they're named in its drawer (VKCF3Z, dae7fb67).
    """
    together = "1 working \u00b7 $214 today"
    shell.close_launcher()
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    shell.launcher_until(
        lambda shown: any(
            thing["is"] == "collection:agents" and thing["reads"] == f"Agents \u00b7 {together}"
            for thing in shown["things"]
        )
    )
    # Found by its name it says the same at its right, and Return still opens it.
    shell.type_in("agents", selecting="Agents")
    row = shell.showing()["selected"]
    assert (row["name"], row["side"]) == ("Agents", together), row
    # spend-watch's next sum reaches the open row, with no key pressed.
    SPEND.write_text(json.dumps({"v": 2, "agentsToday": "$215"}))
    shell.launcher_until(lambda shown: shown["selected"]["side"] == "1 working \u00b7 $215 today")
    shell.step_in()
    shell.launcher_until(lambda shown: shown["held"] == ["agents"])


def agents_at_home(shell: Shell) -> str:
    """Give what home's Agents row says, the launcher opened at home."""
    shown = shell.showing()
    reads = [thing["reads"] for thing in shown["things"] if thing["is"] == "collection:agents"]
    return reads[0] if reads else ""


def passed_over_unread(
    shell: Shell, talk: Path, unread: str, answering: Callable[[Json], bool]
) -> None:
    """Pass over a session's drawer, found and gone again, and see the talk left unread."""
    # Typing went past the session's row a letter at a time, its drawer out for as long as
    # the letters took, which a loaded builder stretches past a stay: that's a read, as the
    # shell is right to say (Y2YBTS, 2026-11-12). With nothing selected now no stay runs, so
    # once any read begun while typing has landed, the talk is made unread again.
    time.sleep(PAST_A_STAY)
    talk.write_text(unread)
    # Passed over: found, then gone again at one more letter. The shell's drawer is out only
    # between the Backspace sent and its going seen, so that bounds it: under a stay, a read
    # is wrong. A loaded builder can stretch it past one, where a read is right, so that pass
    # proves nothing and is tried again with the talk unread.
    for _ in range(3):
        sent = time.monotonic()
        shell.send_key("backspace")
        shell.launcher_until(answering)
        shell.send_chars("x")
        shell.launcher_until(lambda shown: not answering(shown))
        out = time.monotonic() - sent
        # Past a stay, the shell would have read it by now if it reads a pass-over.
        time.sleep(PAST_A_STAY)
        read = '"stage":"read"' in talk.read_text()
        if out < A_STAY:
            assert not read, f"a drawer out at most {out:.1f} s was read"
            return
        talk.write_text(unread)
    slow = "no pass over a drawer took under two seconds in three"
    raise AssertionError(slow)


def test_an_answered_talk_is_counted_at_home_until_joel_reads_it_in_its_drawer(
    shell: Shell,
) -> None:
    """A session that answered Joel's talk is counted at home, calmly, until he reads it.

    Its drawer leads with what he asked and what came back; keeping it there two seconds is
    reading it, which the daemon says in the talk's own file, so the count goes, but the drawer
    keeps it there while he reads (Joel, 2026-11-11: "how do i know when a sessions answers a
    talk?"). A drawer only passed over isn't read (VKCF3Z, 2026-11-12, on #3547).
    """
    asked, answer = "is the harbour light lit?", "Yes, since nine, and the fog has lifted."
    replies = STEER / "LISTEN" / "replies"
    replies.mkdir(parents=True, exist_ok=True)
    talk = replies / "talk-1794414593809.jsonl"
    lines = [
        {"stage": "delivered", "at": "2026-11-11T21:00:00.000Z", "session": "s-listen",
         "from": "joel", "text": asked},
        {"stage": "replied", "at": "2026-11-11T21:01:00.000Z", "text": answer},
    ]  # fmt: skip
    unread = "".join(json.dumps(line) + "\n" for line in lines)
    talk.write_text(unread)
    try:
        shell.close_launcher()
        shell.as_account(shell.ipc("toggle"))
        shell.launcher_ready()
        shell.launcher_until(lambda _: " \u00b7 1 answer \u00b7 " in agents_at_home(shell))
        shell.close_launcher()
        # One letter past its name finds no session, so its drawer shows only from the
        # Backspace below, for as long as the test takes to type the letter again.
        opened(shell)
        shell.type_in("listening questx")
        # The answer says when it came after it, as a landing does.
        answered = f"{answer} \u00b7 Wed 11 Nov, 21:01"

        def answering(shown: Json) -> bool:
            drawer = cast("Json", shown["drawer"] or {})
            facts = cast("list[str]", drawer.get("facts", []))
            values = cast("list[str]", drawer.get("values", []))
            return facts[:2] == ["You Asked", "Its Answer"] and values[:2] == [asked, answered]

        assert shell.showing()["selected"] is None, shell.showing()
        passed_over_unread(shell, talk, unread, answering)
        shell.send_key("backspace")
        shell.launcher_until(answering)
        deadline = time.monotonic() + 10
        while '"stage":"read"' not in talk.read_text():
            assert time.monotonic() < deadline, talk.read_text()
            time.sleep(0.1)
        # Read, it's still there to read on.
        assert shell.showing()["drawer"]["values"][:2] == [asked, answered], shell.showing()
        shell.close_launcher()
        shell.as_account(shell.ipc("toggle"))
        shell.launcher_ready()
        shell.launcher_until(lambda _: "answer" not in agents_at_home(shell))
    finally:
        talk.unlink()


def test_a_closed_orchestrator_stands_dimmed_at_the_trees_end_the_last_live_first(
    shell: Shell,
) -> None:
    """The closed are one row after every live one, holding the last live first, dimmed.

    Return on one reopens it (VKCF3Z, 3a742b67: "Closed is one row at the end").
    """
    # Lamplight's orchestration spent today; Beacon's spent nothing.
    family = {"all": "$14.20", "own": "$3.10", "under": "$11.10"}
    SPEND.write_text(
        json.dumps({"v": 2, "agentsToday": "$214", "families": {"QEST-20261107-LAMPLT": family}})
    )
    opened(shell)
    shell.down_to("Closed")
    shell.step_in()
    shell.launcher_until(lambda shown: shown["rows"][:2] == ["Lamplight", "Beacon"])
    shell.down_to("Lamplight")
    shell.launcher_until(lambda shown: shown["selected"]["side"] == "closed \u00b7 $14.20 in all")
    row = shell.showing()["selected"]
    assert (row["hue"], row["note"], row["verb"]) == ("muted", "Keep the lamp lit", "reopen"), row
    # Its note follows its name, and its right is left to how it stands (VKCF3Z, 2026-11-09).
    shell.launcher_until(
        lambda shown: (shown["noted"], shown["stands"]) == ("Keep the lamp lit", "")
    )
    assert row["told"][:1] == ["\u21b5 reopen"], row
    shell.down_to("Beacon")
    assert shell.showing()["selected"]["side"] == "closed", shell.showing()["selected"]
    # Its acts: Reopen, then its talks, of which it has none.
    shell.step_in("ctrl-bracketright")
    shell.launcher_until(lambda shown: shown["rows"] == ["Reopen"])


def test_recent_holds_the_sessions_closed_this_boot_after_landed_and_before_start(
    shell: Shell,
) -> None:
    """Recent holds a session closed this boot, never one to reopen; Return reopens it."""
    opened(shell)
    shell.launcher_until(lambda shown: "Recent" in shown["rows"])
    rows = shell.showing()["rows"]
    assert rows.index("Landed") < rows.index("Recent") < rows.index("Start a Session"), rows
    shell.down_to("Recent")
    # Nothing after Recent's name, how many at its right, and its sessions in its drawer, a
    # place's (VKCF3Z, dafc2199).
    recent = shell.showing()["selected"]
    assert (recent["count"], recent["side"]) == ("", "1"), recent
    shell.launcher_until(
        lambda shown: (
            (shown["drawer"]["eyebrow"], shown["drawer"]["body"]) == ("place", "Closed Quest")
        )
    )
    shell.step_in()
    # Only the one closed: the last boot's are Sessions to Reopen's, and the closed
    # orchestrators stand at the tree's end.
    shell.launcher_until(lambda shown: shown["rows"] == ["Closed Quest"])
    row = shell.showing()["selected"]
    assert re.fullmatch(r"closed \d\d:\d\d", row["side"]), row
    # Its note never says its name again, but where it ran.
    assert row["note"].endswith("\u00b7 tidepool"), row
    assert (row["verb"], row["told"][:1]) == ("reopen", ["\u21b5 reopen"]), row


def drawers_in(shell: Shell, path: str) -> dict[str, Json]:
    """Give each row a place lists by its name, with the drawer it shows, as the shell says."""
    asked = f"qs -p {shell.folder} ipc call -- launcher drawers {shlex.quote(path)}"
    rows = cast("list[Json]", json.loads(shell.as_account(asked)).get("rows", []))
    return {str(row.get("name")): cast("Json", row.get("drawer") or {}) for row in rows}


def test_each_of_agents_places_says_what_it_holds_in_a_drawer(shell: Shell) -> None:
    """Landed, Recent, Start a Session and the rest each have a drawer saying what they hold.

    Every row has a drawer giving a little more than its line (Joel, 2026-11-11 22:00), a
    place's saying what it holds even while it holds nothing, as Windows' does. AI Spend's Start
    the Watch shows only while the watch is down, so tests/spend-served.qml holds its case.
    """
    opened(shell)
    shell.launcher_until(lambda shown: "Start a Session" in shown["rows"])
    places = ("Landed", "Recent", "Start a Session", "Quests", "Queue", "Runs", "AI Spend")
    drawers = drawers_in(shell, "\u203aagents")
    bare = [
        name
        for name in places
        if not (drawers.get(name, {}).get("body") or drawers.get(name, {}).get("facts"))
    ]
    assert bare == [], {name: drawers.get(name) for name in bare}


def guard_holds(status: Path, holder: Json) -> None:
    """Write what the disk guard says, holding this one folder, whole, by rename as it does."""
    filling = {"path": "/", "band": "filling", "free": 5 * 2**30, "total": 30 * 2**30}
    status.parent.mkdir(parents=True, exist_ok=True)
    laid = status.with_suffix(".new")
    laid.write_text(
        json.dumps(
            {
                "version": 1,
                "heartbeat": time.time() + 3600,
                "filesystems": [filling],
                "frozen": [],
                "holders": [holder],
                "holders_at": time.time(),
            }
        )
    )
    laid.rename(status)


def test_a_folder_filling_the_disk_offers_to_talk_to_the_session_working_in_it(
    shell: Shell,
) -> None:
    """A folder the guard measured offers its session, by title and state, told with its size."""
    folder = STATE / "listen"
    folder.mkdir(parents=True, exist_ok=True)
    named = f"~/{folder.relative_to(Path.home())}"
    status = Path(os.environ["XDG_RUNTIME_DIR"]) / "disk-guard" / "status.json"
    guard_holds(
        status,
        {
            "path": str(folder),
            "name": named,
            "bytes": 12_300_000_000,
            "clears": 0,
            "grew": None,
            "owner": {"kind": "worktree", "name": "listen"},
            "clearable": False,
            "why": None,
            "kept": None,
            "says": f"{named} holds 11.5 GiB.",
        },
    )
    inbox = STEER / "LISTEN"
    try:
        shell.answer("disk")
        shell.step_in()
        shell.type_in("filling", selecting="What's Filling It")
        shell.step_in()
        # The folder's row, named for what owns it, holds its parts.
        held = f"machine:disk:holder:{folder}"
        shell.launcher_until(lambda shown: shown["things"][0]["is"] == held)
        shell.step_in()
        shell.normal()
        shell.launcher_until(lambda shown: "Talk to Its Session" in shown["rows"])
        shell.down_to("Talk to Its Session")
        assert shell.showing()["selected"]["side"] == "Listening Quest \u00b7 listening"
        # Return asks what to tell it, by its name, and it's told that with the folder and
        # its size.
        shell.send_key("ret")
        shell.launcher_until(lambda shown: shown["asking"])
        asked = shell.showing()["hint"]
        assert asked == "what to tell Listening Quest?", asked
        shell.send_chars("clear what you built")
        shell.send_key("ret")
        shell.launcher_until(lambda shown: shown["footer"] == "told Listening Quest")
        # The node names each steer's file itself, so what was told is found by its words.
        deadline = time.monotonic() + 10
        while not (told := talks(inbox)):
            assert time.monotonic() < deadline, "nothing reached the session's inbox"
            time.sleep(0.1)
        said = f"clear what you built (The disk guard says {named} holds 12.3 GB.)"
        assert [json.loads(one.read_text())["text"] for one in told] == [said]
    finally:
        status.unlink()
        for one in talks(inbox):
            one.unlink()


def test_a_looks_choice_steers_its_answer_to_the_session_that_left_it(shell: Shell) -> None:
    """A look's choice, a step into its row in For You, reaches its session's inbox as an answer.

    Joel answers it where it is, rather than in a session that's moved on (2026-11-11), and the
    session writes it resolved; once the node takes the answer, it waits no longer.
    """
    looks = STATE / "pi/agentic-harness.pi/looks"
    looks.mkdir(parents=True, exist_ok=True)
    look = looks / "asking.json"
    # It came just now, since one three days old goes quiet (launcher/Looks.js).
    came = datetime.now(UTC).strftime("%Y-%m-%dT%H:%M:%SZ")
    launcher_on_niri.laid_whole(
        look,
        json.dumps(
            {
                "v": 1,
                "id": "asking",
                "title": "Asking Look",
                "target": "/etc/hostname",
                "kind": "file",
                "state": "waiting",
                "at": came,
                "address": "LISTEN",
                "ask": "Say whether it reads right.",
                "choices": [{"value": "reads-right", "label": "Reads Right"}],
            }
        ),
    )
    inbox = STEER / "LISTEN"
    try:
        in_for_you(shell)
        shell.launcher_until(lambda shown: "To Look At" in shown["rows"])
        shell.down_to("To Look At")
        shell.step_in()
        shell.launcher_until(lambda shown: "Asking Look" in shown["rows"])
        shell.down_to("Asking Look")
        # Return opens a look, so its acts are stepped into as any row's are.
        shell.step_in("ctrl-bracketright")
        shell.launcher_until(lambda shown: "Reads Right" in shown["rows"])
        shell.normal()
        shell.down_to("Reads Right")
        # Stepping into the look was the deliberate act, so one Return answers, with no line
        # and no ask (modules/keys/language.toml's answer, VKCF3Z 2026-11-12).
        shell.send_key("ret")
        deadline = time.monotonic() + 10
        while not (told := answers(inbox)):
            shown = shell.showing()
            assert time.monotonic() < deadline, (
                "no answer reached the session's inbox",
                shown["command"],
                shown["question"],
                shown["returned"],
            )
            time.sleep(0.1)
        said = json.loads(told[0].read_text())
        assert (said["act"], said["look"], said["value"], said["from"]) == (
            "answer",
            "asking",
            "reads-right",
            "joel",
        ), said
        shell.close_launcher()
        # The node took it, so it has left what waits for History, saying the answer, though its
        # file waits until the session writes it resolved (VKCF3Z, 2026-11-12).
        in_for_you(shell)
        shell.launcher_until(
            lambda shown: "To Look At" not in shown["rows"], timeout=timedelta(seconds=5)
        )
        shell.down_to("History")
        shell.step_in()
        shell.launcher_until(lambda shown: "Asking Look" in shown["rows"])
        shell.down_to("Asking Look")
        side = shell.showing()["selected"]["side"]
        assert re.fullmatch(r"answered Reads Right .+", side), shell.showing()
        shell.close_launcher()
    finally:
        look.unlink()
        for one in answers(inbox):
            one.unlink()


def answers(inbox: Path) -> list[Path]:
    """Give the steers in an inbox that answer a look."""
    return [
        one for one in inbox.glob("*.json") if json.loads(one.read_text()).get("act") == "answer"
    ]


def talks(inbox: Path) -> list[Path]:
    """Give the steers in an inbox that told the session what the disk guard says."""
    return [
        one
        for one in inbox.glob("*.json")
        if "The disk guard says" in str(json.loads(one.read_text()).get("text", ""))
    ]


def made_by_agents(shell: Shell) -> int:
    """Give how many rows Agents has made since the shell started."""
    return int(json.loads(shell.as_account(shell.ipc("made")))["rows"]["agents"])


def made_lately(shell: Shell) -> str:
    """Say the last rows Agents made, each marked shut or walked as the launcher was then."""
    lately = json.loads(shell.as_account(shell.ipc("made")))["lately"]["agents"]
    return ", ".join(lately) or "none"


def sortings_taken(shell: Shell) -> int:
    """Give how many sortings of pi's sessions the shell has taken, each a change it heard."""
    sorts = f"qs -p {shell.folder} ipc call pi sorts"
    return int(json.loads(shell.as_account(sorts))["taken"])


def test_a_shut_launcher_makes_no_agents_rows_as_a_session_comes_and_lands(shell: Shell) -> None:
    """Shut, a session arriving with a landing makes no row of Agents'; opened, it's there.

    Joel's shut launcher made 312 of Agents' rows in 240 s that nobody saw, and each burst is
    what his openings wait behind (2026-11-10).
    """
    shell.launcher_closed()
    warmed(shell)
    made_settled(shell)
    before = made_by_agents(shell)
    landing = {"at": datetime.now(UTC).isoformat(), "ref": "core.nix#2141", "what": "Arrived"}
    arrived_shut(
        shell,
        lambda: lay(
            "ARRIVE", "Arriving Quest", {"state": "listening", "busy": False, "landings": [landing]}
        ),
    )
    made = made_by_agents(shell) - before
    assert made == 0, (
        f"the shut launcher made {made} of Agents' rows as a session arrived: {made_lately(shell)}"
    )
    shell.as_account(shell.ipc("collection", "agents"))
    shell.launcher_ready()
    shell.launcher_until(lambda shown: "Arriving Quest" in shown["rows"])
    shell.launcher_until(lambda shown: "Landed" in shown["rows"])
    shell.down_to("Landed")
    shell.step_in()
    shell.launcher_until(lambda shown: "Arrived" in shown["rows"])


def arrived_shut(shell: Shell, arrive: Callable[[], None]) -> None:
    """Have a session arrive with the launcher shut, and wait until the shell has heard all of it.

    It's heard twice, once as pi's source sorts it in and once in the line the agents daemon tells
    Agents of it at once, its row new (daemon-serve's serve.rs), in either order. Counted after
    the first alone, a loaded run counted the rows the line made only as it fell first (#4090).
    """
    taken, lines = sortings_taken(shell), agents_lines(shell)
    arrive()
    deadline = time.monotonic() + 10
    while sortings_taken(shell) == taken or agents_lines(shell) == lines:
        assert time.monotonic() < deadline, (
            f"the shell took {sortings_taken(shell) - taken} sortings and heard"
            f" {agents_lines(shell) - lines} of Agents' lines as a session arrived"
        )
        time.sleep(0.1)
    made_settled(shell)


def rows_answer(said: Json) -> Json:
    """Give what the agents daemon answers an ask of its own, asked over its socket."""
    with socket.socket(socket.AF_UNIX) as rows:
        rows.settimeout(10)
        rows.connect(str(ROWS))
        rows.sendall(json.dumps({"id": 1, **said}).encode() + b"\n")
        answer = json.loads(rows.makefile().readline())
    answer.pop("id", None)
    return cast("Json", answer)


def agents_lines(shell: Shell) -> int:
    """Give how many lines the shell has read of the agents daemon."""
    return int(json.loads(shell.as_account(shell.ipc("linesHeard")))["agents"])


def daemon_says(said: str) -> None:
    """Wait until the agents daemon tells a watch of its own, told at once, Agents' summary."""
    with socket.socket(socket.AF_UNIX) as rows:
        rows.settimeout(10)
        rows.connect(str(ROWS))
        rows.sendall(b'{"id": 1, "ask": "watch", "kind": "summary"}\n')
        lines = rows.makefile()
        while True:
            told = json.loads(lines.readline())
            records = told.get("records", told.get("changed", []))
            if any(said in record["summary"]["said"] for record in records):
                return


def test_a_shut_launcher_hears_agents_rows_once_a_pace_and_opening_is_told_at_once(
    shell: Shell,
) -> None:
    """Shut, a change to Agents' rows isn't told the shell at once; opened, it's told it at once.

    A spend side ticking or a session turning to listen moved a row about every three seconds,
    each line waking Joel's shut shell for nothing it showed (RO9YDD, 21 lines in 60 s on
    2026-11-11), so the shut launcher paces its watch at ten seconds.
    """
    shell.launcher_closed()
    before = agents_lines(shell)
    SPEND.write_text(json.dumps({"v": 2, "agentsToday": "$216"}))
    daemon_says("$216 today")
    # The span measured, for a line told at once to reach the shell, not a wait for it.
    time.sleep(1)
    heard = agents_lines(shell) - before
    assert heard == 0, f"the shut shell heard {heard} lines of Agents' rows within a second"
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    shell.launcher_until(
        lambda shown: any(
            thing["is"] == "collection:agents" and thing["reads"].endswith("$216 today")
            for thing in shown["things"]
        ),
        timedelta(seconds=5),
    )


def test_the_first_letter_typed_at_home_makes_a_few_rows_after_one_arrived_while_shut(
    shell: Shell,
) -> None:
    """A session arriving while the launcher's shut moves every row after it, but only in order.

    So the first letter typed at home makes the arriving session's row and a few, the rest keeping
    their wraps: Joel's made 176 to 185 of them, 11 to 17 ms of letters that took 56 to 81 (RCTRF8,
    2026-11-12). That home makes no row unseen while things change is vm-desktop-launcher-keeps-up's
    to prove, by its own count; opening here warms Agents, which that count leaves out.
    """
    shell.launcher_closed()
    arrived_shut(
        shell, lambda: lay("TIDEPL", "Tidepool Quest", {"state": "listening", "busy": False})
    )
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    made_settled(shell)
    opened = made_by_agents(shell)
    shell.type_in("tidepool", selecting="Tidepool Quest")
    made = made_by_agents(shell) - opened
    assert made <= FIRST_LETTER_MAKES, (
        f"typing at home made {made} of Agents' rows, not a few: {made_lately(shell)}"
    )


# The most rows the first letter at home makes after a session arrived while shut: the arriving
# one and those its coming moved within their orchestration, never the list. Measured at 5: the
# arriving session's row with its Talk in Agents' top level and again in what's found. It was 3
# while the top level stayed paced with the launcher open, so the arriving one wasn't in it yet
# (2026-11-13).
FIRST_LETTER_MAKES = 5


# How long Agents makes no row before an opening's work is taken as done: many frames.
SETTLED = 0.5


# What the shell says once it has warmed every source and kept home and each chord's place
# (Launcher.qml's warmNext), and how long a case waits for it.
WARMED_SAID = "places kept as the shell started"
WARMED = 30


def warmed(shell: Shell) -> None:
    """Wait until the shell has warmed its sources, so no count takes in Agents' turn of it.

    Warming wraps every session's row shut, a new one's too, so the first letter finds them
    wrapped. Each opening puts it off until the launcher has been shut a second and a half, so
    after the openings before it, it fell inside a shut test's count as an arrival's rows
    (`agent:ARRIVE shut` made from warmNext, 2026-11-13).
    """
    deadline = time.monotonic() + WARMED
    while WARMED_SAID.encode() not in shell.log.read_bytes()[shell.launched :]:
        assert time.monotonic() < deadline, f"the shell never finished warming:\n{shell.tail()}"
        time.sleep(0.1)


def made_settled(shell: Shell) -> None:
    """Wait until Agents has made no row for a while, the opening's work done."""
    deadline = time.monotonic() + 10
    made, still_since = made_by_agents(shell), time.monotonic()
    while time.monotonic() - still_since < SETTLED:
        assert time.monotonic() < deadline, "Agents never stopped making rows"
        time.sleep(0.1)
        now = made_by_agents(shell)
        if now != made:
            made, still_since = now, time.monotonic()


@pytest.fixture
def shoot() -> Callable[[str], None]:
    """Give what pictures niri's screen as it stands, by name, into the folder SHOTS names.

    A run that asks for pictures sets SHOTS to the build's output, as `--apply` can, and
    SHOTS_SCALE to one of Joel's scales; a gate sets neither, and takes none.
    """

    def shot(name: str) -> None:
        folder = os.environ.get("SHOTS", "")
        if folder == "":
            return
        Path(folder).mkdir(exist_ok=True)
        # grim is the check's own, on its PATH, given a path the test made.
        subprocess.run(["grim", str(Path(folder) / f"{name}.png")], check=True)

    return shot


def own_title(title: str) -> str:
    """Give a pi window's title as Super+S's line shows it, without pi's letter leading it."""
    return title.removeprefix("\u03c0 - ")


@pytest.fixture(scope="module")
def said_over(shell: Shell) -> Iterator[str]:
    """Give the title of a window Super+S's line is said over, with the void standing at the top."""
    if os.environ.get("SHOTS_SCALE"):
        outputs = json.loads(shell.as_account("niri msg --json outputs"))
        for name in outputs:
            shell.as_account(f"niri msg output {name} scale {os.environ['SHOTS_SCALE']}")
    lay("VOID", "Void Quest", {"state": "listening", "standsAtTop": {"name": "The Void"}})
    # Only that Agents has heard of the void, whatever else earlier cases laid beside it.
    shell.as_account(shell.ipc("collection", "agents"))
    shell.launcher_ready()
    shell.launcher_until(lambda shown: "The Void" in shown["rows"])
    shell.close_launcher()
    title = "\u03c0 - Tune the launcher so it fits the way he works"
    window = shell.opened(title, app="kitty")
    # A window opened while a layer had the keys isn't always given them, as Joel's own is.
    shell.as_account(f"niri msg action focus-window --id {window['id']}")
    yield title
    shell.close(window)


def test_super_s_line_wraps_what_is_typed_and_sends_it_with_its_window(
    shell: Shell, said_over: str, shoot: Callable[[str], None]
) -> None:
    """Super+S's line wraps what's typed, keeps it put away, and sends it with its window.

    Talking to the void took over Joel's screen and lost his words to a switch (2026-11-12
    16:00), and its first line read as a badge with no room to type (2026-11-13 04:33). The
    line is one frame a step inside the top of the screen, over his window: what's typed
    wraps as it grows, the void hears it, named quietly beside the window it's said over,
    which goes with the words, and Return keeps it up, saying they went (VKCF3Z, 2026-11-13
    [092d52ca]). What goes with them shows under a hairline before they're sent, a row each,
    and Ctrl+R * and Ctrl+R + attach the selection and the clipboard, as Vim's registers
    (Joel, 2026-11-12: "i often want to talk about what i'm looking at").
    """
    line = shell.ipc("line")
    words = (
        "is this the right fix for the cut title, or should the drawer give way first? "
        "it reads fine at rest but twitches while it reads on"
    )
    since = shell.mark()
    shell.as_account(line)
    shell.launcher_until(
        lambda shown: shown["line"]["keys"] and shown["line"]["hears"] == "to the void"
    )
    # Under what's typed, the window it's said over, by its title, once niri has said it to
    # the shell, without pi's letter, which its mark says; the foot names who hears it, in
    # words rather than a stray name (VKCF3Z, 2026-11-13 [00c78048]).
    shell.launcher_until(
        lambda shown: [row["text"] for row in shown["line"]["attached"]] == [own_title(said_over)]
    )
    about = shell.showing()["line"]["about"]
    assert about == "to the void", about
    # It says how long it took to reach the screen, held to Joel's 50 ms (VKCF3Z, e8b4139e).
    logging.getLogger(__name__).info(
        "the line was up %s ms after it was asked", line_was_up(shell, since)
    )
    shown = shell.showing()
    assert not shown["open"], shown
    assert shown["line"]["mode"] == "insert", shown["line"]
    # A step inside the top, as niri lays an overlay: the window under it keeps the rest.
    assert 0 < shown["line"]["top"] < 100, shown["line"]  # noqa: PLR2004
    shoot("empty")
    shell.send_chars(words)
    # A sentence or two wraps at a whole word, never scrolling off its start.
    shell.launcher_until(
        lambda shown: shown["line"]["typed"] == words and shown["line"]["lines"] > 1
    )
    shoot("typed")
    # Escape gives normal, as the launcher's line does, and never puts it away.
    shell.send_key("esc")
    shell.launcher_until(lambda shown: shown["line"]["mode"] == "normal")
    assert shell.showing()["line"]["up"], shell.showing()["line"]
    # The chord again does, and once more gives his words back.
    shell.as_account(line)
    shell.launcher_until(lambda shown: not shown["line"]["up"])
    shell.as_account(line)
    shell.launcher_until(lambda shown: shown["line"]["keys"] and shown["line"]["typed"] == words)
    # Ctrl+R * attaches what's selected and Ctrl+R + what's copied, each a row under the
    # window's, the line's words as they were.
    attach_both(shell, said_over)
    assert shell.showing()["line"]["typed"] == words, shell.showing()["line"]
    shoot("attached")
    # Return sends them, what's attached with them, and the line stays up saying whom they
    # wait on, attaching only its window again.
    shell.send_key("ret")
    shell.launcher_until(
        lambda shown: (
            shown["line"]["up"]
            and shown["line"]["typed"] == ""
            and shown["line"]["standing"] == "waiting on the void"
        )
    )
    inbox = STEER / "VOID"
    sent = reached(inbox, words)
    assert "\n\nAttached:\n- window: " in sent[0], sent
    assert sent[0].endswith(
        f"{said_over}\n- selection: the cut title\n- clipboard: twitches while it reads on"
    ), sent
    assert [row["kind"] for row in shell.showing()["line"]["attached"]] == ["window"]
    shoot("sent")
    times = re.findall(
        r"talk line: up (\d+) ms", shell.log.read_bytes()[since:].decode(errors="replace")
    )
    logging.getLogger(__name__).info("the line was up after %s ms each time", times)
    shell.as_account(line)
    shell.launcher_until(lambda shown: not shown["line"]["up"])


def line_was_up(shell: Shell, since: int) -> str:
    """Give how many milliseconds the line took to reach the screen, once it says."""
    deadline = time.monotonic() + 10
    while not (
        up := re.search(
            r"talk line: up (\d+) ms", shell.log.read_bytes()[since:].decode(errors="replace")
        )
    ):
        assert time.monotonic() < deadline, shell.tail()
        time.sleep(0.1)
    return up[1]


def reached(inbox: Path, words: str) -> list[str]:
    """Give what reached the inbox beginning with the words, once anything has."""
    deadline = time.monotonic() + 10
    while not (
        sent := [
            text
            for one in inbox.glob("*.json")
            if (text := str(json.loads(one.read_text()).get("text", ""))).startswith(words)
        ]
    ):
        assert time.monotonic() < deadline, "the line never reached the void's inbox"
        time.sleep(0.1)
    return sent


def attach_both(shell: Shell, said_over: str) -> None:
    """Select a title and copy a line, then attach each by its register, under the window."""
    # wl-copy stays behind to serve each, so it's given nothing to hold open.
    shell.as_account("printf %s 'the cut title' | wl-copy --primary > /dev/null 2>&1")
    shell.as_account("printf %s 'twitches while it reads on' | wl-copy > /dev/null 2>&1")
    attach(shell, "*", [("window", said_over), ("selection", "the cut title")])
    attach(
        shell,
        "+",
        [
            ("window", said_over),
            ("selection", "the cut title"),
            ("clipboard", "twitches while it reads on"),
        ],
    )


def attach(shell: Shell, register: str, rows: list[tuple[str, str]]) -> None:
    """Press Ctrl+R and the register until the line's attached rows are the rows given.

    A copy reaches the clipboard's history, and the shell from it, a moment after wl-copy
    returns, and a register attached while it's empty attaches nothing, so it's pressed again.
    """
    deadline = time.monotonic() + 10
    while True:
        shell.send_key("ctrl-r")
        shell.send_chars(register)
        try:
            shell.launcher_until(
                lambda shown: (
                    [(row["kind"], row["text"]) for row in shown["line"]["attached"]] == rows
                ),
                timedelta(seconds=1),
            )
        except AssertionError:
            assert time.monotonic() < deadline, shell.showing()["line"]
        else:
            return


def test_super_s_line_says_the_answer_and_takes_it_back_inside_itself(
    shell: Shell, said_over: str, shoot: Callable[[str], None]
) -> None:
    """Super+S's line says the answer in sent's place, and :take takes it back, inside itself.

    Its answer was a notice outside the line, which Joel found "fucking weird" (2026-11-13
    04:33): the answer takes sent's place under his words, where it went first, while the
    void can undo it the foot offers :take, and the void's undone reads taken back, all in
    the line, kept for its next opening (VKCF3Z, 2026-11-13 [092d52ca]). It follows the line
    the case before sent.
    """

    def steered(test: Callable[[str], bool]) -> str:
        """Give the words of the first steer in the void's inbox the test holds for."""
        deadline = time.monotonic() + 10
        while True:
            for one in sorted((STEER / "VOID").glob("*.json")):
                text = str(json.loads(one.read_text()).get("text", ""))
                if test(text):
                    return text
            assert time.monotonic() < deadline, "the void's inbox never had the words"
            time.sleep(0.1)

    def replied(asked: str, answer: str) -> None:
        """Lay the void's talk of these words as delivered now, then replied to, as pi does.

        pi names a talk by the id the line gave it as it was said, so the line knows it
        for the one it said however late the void answers.
        """
        replies = STEER / "VOID" / "replies"
        replies.mkdir(parents=True, exist_ok=True)
        now = datetime.now(UTC)
        ids = [
            str(steer.get("id", ""))
            for one in (STEER / "VOID").glob("*.json")
            if (steer := json.loads(one.read_text())).get("text") == asked
        ]
        named = ids[0] if ids and ids[0].startswith("talk-") else ""
        talk = named or f"talk-{int(now.timestamp() * 1000)}"
        lines = [
            {"stage": "delivered", "at": now.isoformat(), "session": "s-void", "from": "joel",
             "text": asked},
            {"stage": "replied", "at": now.isoformat(), "text": answer},
        ]  # fmt: skip
        laid = replies / f"{talk}.jsonl.tmp"
        laid.write_text("".join(json.dumps(line) + "\n" for line in lines))
        laid.rename(laid.with_suffix(""))

    line = shell.ipc("line")
    shell.as_account(line)
    shell.launcher_until(lambda shown: shown["line"]["keys"] and shown["line"]["talk"] == "sent")
    sent = steered(lambda text: "\n\nAttached:\n- window: " in text and said_over in text)
    # The answer takes sent's place, where it went first, its undo offered in the foot.
    replied(sent, "\u2192 Delta \u00b7 undo 04:58\nFiled under the cut title's fix.")
    shell.launcher_until(
        lambda shown: shown["line"]["talk"] == "answered" and shown["line"]["takes"]
    )
    assert shell.showing()["line"]["standing"].startswith("\u2192 Delta\n"), shell.showing()
    shoot("answered")
    # :take in normal tells the void to undo it, and its undone reads taken back, in the line.
    shell.send_key("esc")
    shell.launcher_until(lambda shown: shown["line"]["mode"] == "normal")
    shell.send_chars(":take")
    shell.send_key("ret")
    shell.launcher_until(lambda shown: shown["line"]["talk"] == "taking")
    replied(steered(lambda text: text == "undo 04:58"), "undone")
    shell.launcher_until(lambda shown: shown["line"]["standing"] == "taken back")
    assert not shell.showing()["line"]["takes"], shell.showing()["line"]
    shoot("taken")
    # Put away and opened again, the talk is still there, the field empty.
    shell.as_account(line)
    shell.launcher_until(lambda shown: not shown["line"]["up"])
    shell.as_account(line)
    shell.launcher_until(
        lambda shown: (
            shown["line"]["keys"]
            and shown["line"]["typed"] == ""
            and shown["line"]["standing"] == "taken back"
        )
    )
    shell.as_account(line)
    shell.launcher_until(lambda shown: not shown["line"]["up"])


def test_super_s_line_shows_every_answer_to_two_lines_and_whom_each_waits_on(
    shell: Shell, said_over: str, shoot: Callable[[str], None]
) -> None:
    """Two lines in a row each say whom they wait on, then every answer, a newer in place.

    Joel asked twice in a row and the line showed only the first answer, and not the one
    that superseded it (WCJQSG [cf8639ab], 2026-11-13 07:46). The void writes each event to
    its file as it happens (8GYNJ6 [a9490eb0]); the line reads it while it's up.
    """
    del said_over

    def asked(words: str, kept: str) -> None:
        """Say the words on the line, and lay the void's answer that your orchestrator has it."""
        shell.send_chars(words)
        # In the field before Return, since an empty field after it says nothing of a
        # Return that came before the letters landed.
        shell.launcher_until(lambda shown: shown["line"]["typed"] == words)
        shell.send_key("ret")
        shell.launcher_until(lambda shown: shown["line"]["typed"] == "")
        deadline = time.monotonic() + 10
        while not (
            sent := [
                text
                for one in sorted((STEER / "VOID").glob("*.json"))
                if (text := str(json.loads(one.read_text()).get("text", ""))).startswith(words)
            ]
        ):
            held = [one.name for one in sorted((STEER / "VOID").glob("*"))]
            assert time.monotonic() < deadline, (
                f"the void's inbox never had {words!r}: it held {held}, "
                f"the line {shell.showing()['line']}"
            )
            time.sleep(0.1)
        replies = STEER / "VOID" / "replies"
        replies.mkdir(parents=True, exist_ok=True)
        now = datetime.now(UTC)
        lines = [
            {"stage": "delivered", "at": now.isoformat(), "session": "s-void", "from": "joel",
             "text": sent[0]},
            {"stage": "replied", "at": now.isoformat(),
             "text": f"Your orchestrator has it \u00b7 undo {kept}"},
        ]  # fmt: skip
        laid = replies / f"talk-{int(now.timestamp() * 1000)}.jsonl.tmp"
        laid.write_text("".join(json.dumps(line) + "\n" for line in lines))
        laid.rename(laid.with_suffix(""))

    line = shell.ipc("line")
    shell.as_account(line)
    shell.launcher_until(lambda shown: shown["line"]["keys"])
    shell.launcher_until(lambda shown: shown["line"]["mode"] == "insert")
    # A key of another keymap first, as Shell.restart presses, since the line's first letter
    # was read by the last keymap and lost, leaving "ow is it going?". F24 does nothing here.
    subprocess.run(["wtype", "-k", "F24"], check=True)
    asked("how is it going?", "07:42")
    asked("what is being worked on?", "07:43")
    waiting = ["waiting on your orchestrator"] * 2
    shell.launcher_until(
        lambda shown: [talk["waiting"] for talk in shown["line"]["talks"][:2]] == waiting
    )
    shoot("waiting")
    day = datetime.now().astimezone().strftime("%Y-%m-%d")
    events = [
        {"line": f"{day} 07:42", "msg": "m1", "who": "your orchestrator", "event": "routed"},
        {"line": f"{day} 07:43", "msg": "m2", "who": "your orchestrator", "event": "routed"},
        {"line": f"{day} 07:42", "msg": "m3", "who": "your orchestrator",
         "event": "answered", "at": "07:44", "text": "Going well: eleven clean updates."},
        {"line": f"{day} 07:43", "msg": "m4", "who": "your orchestrator",
         "event": "answered", "at": "07:44", "text": "The cat's clips, and the void."},
        {"line": f"{day} 07:42", "msg": "m5", "supersedes": "m3", "who": "your orchestrator",
         "event": "answered", "at": "07:45", "text": "Going well: thirteen clean updates."},
    ]  # fmt: skip
    written = STATE / "void" / "answers.jsonl"
    written.parent.mkdir(parents=True, exist_ok=True)
    with written.open("a") as file:
        file.writelines(json.dumps(event) + "\n" for event in events)
    answered = [["The cat's clips, and the void."], ["Going well: thirteen clean updates."]]
    shell.launcher_until(
        lambda shown: [talk["answers"] for talk in shown["line"]["talks"][:2]] == answered
    )
    talks = shell.showing()["line"]["talks"]
    assert [talk["waiting"] for talk in talks[:2]] == ["", ""], talks
    # Its right says when he said it, the undo's time, however late the answer; who
    # answered stands beside the answer (WCJQSG [e05f6231]).
    assert [talk["by"] for talk in talks[:2]] == ["your orchestrator"] * 2, talks
    assert all(re.fullmatch(r"\d\d:\d\d", talk["side"]) for talk in talks[:2]), talks
    shoot("answered-twice")
    shell.as_account(line)
    shell.launcher_until(lambda shown: not shown["line"]["up"])


def test_return_on_the_void_in_agents_opens_super_s_line_over_the_window_left(
    shell: Shell, said_over: str
) -> None:
    """Return on The Void in Agents opens Super+S's line, over the window the launcher left.

    The void is said to in one place, the line (QCXZEB, 2026-11-13 [ae5dfdb6], d): Talk to
    It, The Void's Return, puts the launcher away and brings the line up, its talk kept and
    the window under it attached, rather than asking in the launcher's own field.
    """
    shell.as_account(shell.ipc("collection", "agents"))
    shell.launcher_ready()
    shell.launcher_until(lambda shown: "The Void" in shown["rows"])
    shell.down_to("The Void")
    shell.send_key("ret")
    shell.launcher_until(
        lambda shown: not shown["open"] and shown["line"]["up"] and shown["line"]["keys"]
    )
    # Over the window the launcher went from, which goes with what's said.
    shell.launcher_until(
        lambda shown: [row["text"] for row in shown["line"]["attached"]] == [own_title(said_over)]
    )
    # Its talks kept: the case before's newest answer.
    standing = shell.showing()["line"]["standing"]
    assert standing == "The cat's clips, and the void.", shell.showing()["line"]
    shell.as_account(shell.ipc("line"))
    shell.launcher_until(lambda shown: not shown["line"]["up"])


def test_super_s_line_shows_every_line_still_open_after_a_restart_an_earlier_days_too(
    shell: Shell, shoot: Callable[[str], None]
) -> None:
    """After the shell restarts, every line still waiting or answered unseen shows first.

    Joel sent several lines, didn't wait for their answers, and after a switch restarted the
    shell the line showed none of them (WCJQSG [a2eb9575], 2026-11-13 18:05: "now going in
    shows me nothing"). The void keeps every talk and answer, so the line is made from what
    it keeps, and what he has seen is kept on disk: the two answered lines the case before
    showed stay after them, one still waiting and an answer from yesterday he hasn't seen
    come first.
    """
    replies = STEER / "VOID" / "replies"
    now = datetime.now(UTC)
    yesterday = now - timedelta(days=1)

    def kept(at: datetime, words: str, undo: str) -> None:
        """Lay a talk the void kept, said at `at` and taken by your orchestrator at `undo`."""
        lines = [
            {"stage": "delivered", "at": at.isoformat(), "session": "s-void", "from": "joel",
             "text": words},
            {"stage": "replied", "at": at.isoformat(),
             "text": f"Your orchestrator has it \u00b7 undo {undo}"},
        ]  # fmt: skip
        laid = replies / f"talk-{int(at.timestamp() * 1000)}.jsonl.tmp"
        laid.write_text("".join(json.dumps(line) + "\n" for line in lines))
        laid.rename(laid.with_suffix(""))

    kept(yesterday, "is the beacon fixed?", "09:05")
    kept(now, "move the windows back", "07:50")
    day = datetime.now().astimezone().strftime("%Y-%m-%d")
    before = yesterday.astimezone().strftime("%Y-%m-%d")
    events = [
        {"line": f"{before} 09:05", "msg": "y1", "who": "Delta", "event": "routed"},
        {"line": f"{before} 09:05", "msg": "y2", "who": "Delta", "event": "answered",
         "at": "09:20", "text": "Fixed, and merged."},
        {"line": f"{day} 07:50", "msg": "w1", "who": "your orchestrator", "event": "routed"},
    ]  # fmt: skip
    with (STATE / "void" / "answers.jsonl").open("a") as file:
        file.writelines(json.dumps(event) + "\n" for event in events)
    shell.restart()
    line = shell.ipc("line")
    shell.as_account(line)
    shell.launcher_until(lambda shown: shown["line"]["up"])
    said = ["move the windows back", "is the beacon fixed?", "what is being worked on?",
            "how is it going?"]  # fmt: skip
    shell.launcher_until(
        lambda shown: [talk["line"] for talk in shown["line"]["talks"]][:4] == said
    )
    talks = shell.showing()["line"]["talks"]
    assert talks[0]["waiting"] == "waiting on your orchestrator", talks
    assert (talks[1]["answers"], talks[1]["by"]) == (["Fixed, and merged."], "Delta"), talks
    assert re.fullmatch(r"\S+ \d\d:\d\d", talks[1]["side"]), talks
    assert talks[2]["answers"] == ["The cat's clips, and the void."], talks
    shoot("restarted")
    shell.as_account(line)
    shell.launcher_until(lambda shown: not shown["line"]["up"])


def test_super_s_line_says_whether_it_is_up_for_a_switch_to_wait_on(shell: Shell) -> None:
    """Super+S's line says in a file whether it's up, as the launcher does, open then closed.

    A switch restarts the shell only while Joel isn't in it (quickshell-takes-new-files), and
    a restart under the line would take his words from under him mid-sentence (2026-11-12
    16:00), so the line's mark is read as the launcher's is.
    """
    mark = Path(os.environ["XDG_RUNTIME_DIR"]) / "quickshell-talk-line"

    def marked(word: str) -> None:
        """Wait until the mark says `word`, as a switch reading it would."""
        deadline = time.monotonic() + 10
        while (mark.read_text() if mark.exists() else "") != f"{word}\n":
            assert time.monotonic() < deadline, f"the line's mark never said {word}"
            time.sleep(0.1)

    line = shell.ipc("line")
    shell.as_account(line)
    shell.launcher_until(lambda shown: shown["line"]["up"])
    marked("open")
    shell.as_account(line)
    shell.launcher_until(lambda shown: not shown["line"]["up"])
    marked("closed")

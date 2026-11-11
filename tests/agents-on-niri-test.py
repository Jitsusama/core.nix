"""An agent's row in the launcher does, on Return, the act its state makes harmless, on niri.

modules/quickshell/checks.nix runs this through pytest inside tests/nested-niri.sh. The sessions
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
with the folder and its size. Shut, a session arriving with a landing makes none of Agents' rows,
which show both once it opens, and a change to its rows isn't told the shell at once, but told it
at once as it opens.

Long because every case here shares one shell, its node and its sessions, started once a run,
which cost more than all the cases, so splitting it would start them twice.
"""

from __future__ import annotations

import json
import os
import re
import socket
import subprocess
import time
from datetime import UTC, datetime, timedelta
from pathlib import Path
from typing import TYPE_CHECKING, Any, cast

import launcher_on_niri
import pytest

if TYPE_CHECKING:
    from collections.abc import Iterator

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
TRY = "press d on a session"
# What a session works on, as pi says it: its quest's title, then its summary, which runs to
# about 290 letters on every real session.
WORKS_ON = (
    "Listening Quest: A builder under the owner's plan, which keeps the launcher's agents"
    " quiet, told and easy to read: every session says how it stands, what it does, what it"
    " spent and what it needs from Joel, from the node, with nothing guessed and nothing"
    " said twice, held to the launcher's family and its keys."
)
LANDINGS = [
    {"at": "2026-11-07T11:31:00Z", "ref": "core.nix#2140", "what": STOPPED},
    {"at": "2026-11-07T11:25:00Z", "ref": "PLAN-20261107-PAUSED#2", "what": "Ticked alone"},
    {"at": "2026-11-07T11:19:00Z", "ref": "core.nix#2138", "what": CLOSING, "person": TRY},
]


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
    process = subprocess.Popen(["true"])  # noqa: S607
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
    lay("BOSSES", "Orchestrating Quest", {"busy": True, "state": "busy"})
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
    with (logs / "mycelium.log").open("w") as log:
        # The node is the store's own, which the check names.
        node = subprocess.Popen(  # noqa: S603
            [os.environ["MYCELIUM"], "--realm", REALM, "node"], stdout=log, stderr=log
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
        # names it, as the account's unit runs it; this sandbox has no to-do daemon, nor
        # any windows daemon.
        rows = subprocess.Popen(  # noqa: S603
            [os.environ["AGENTS"], "--vocabulary", os.environ["WORDS"], "--node", str(NODE),
             "--to-do", str(ROWS.parent / "to-do.sock"),
             "--windows", str(ROWS.parent / "windows.sock"), "--spend", str(SPEND),
             "--orchestrators", str(WORKFLOW / "orchestrators.json"),
             "--restore", str(WORKFLOW / "restore.json"),
             "--kept", str(WORKFLOW / "reopen.json"), "--socket", str(ROWS)],
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
    """Open Agents, once it lists the orchestrator and, after its own rows, its fold."""
    shell.as_account(shell.ipc("collection", "agents"))
    shell.launcher_ready()
    shell.launcher_until(
        lambda shown: (
            shown["held"] == ["agents"]
            and shown["rows"][:2] == ["Orchestrating Quest", "1 listening, 1 paused"]
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
    shell.close_launcher()
    found(shell, "listening quest", "Listening Quest")
    shell.step_in("ctrl-bracketright")
    shell.launcher_until(lambda shown: shown["rows"][:2] == ["Talk", "Pause"])
    assert not shell.showing()["asking"], shell.showing()


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
    opened(shell)
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


def test_a_sessions_stop_is_an_ending_return_writes_and_a_second_steers(shell: Shell) -> None:
    """Return on Stop writes :interrupt rather than stopping; a second steers a stop, unasked."""
    opened(shell)
    shell.step_in("ctrl-bracketright")
    shell.launcher_until(lambda shown: "Stop" in shown["rows"])
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
    opened(shell)
    shell.step_in("ctrl-bracketright")
    shell.launcher_until(lambda shown: shown["rows"][:4] == ["Talk", "Pause", "Stop", "Close"])
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
    """
    opened(shell)
    shell.launcher_until(lambda shown: "Landed" in shown["rows"])
    shell.down_to("Landed")
    # None of them landed today, and what's to read is For You's alone to count.
    assert shell.showing()["selected"]["side"] == "none today", shell.showing()
    shell.step_in()
    shell.launcher_until(lambda shown: shown["rows"] == [STOPPED, CLOSING])
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
    shell.launcher_until(lambda shown: shown["rows"] == [CLOSING])
    assert shell.showing()["selected"]["side"] not in {"", "core.nix#2138"}, shell.showing()
    # Its side says when it landed; its drawer says what to try, a sentence, after its facts.
    shell.launcher_until(lambda shown: (shown["drawer"] or {}).get("body") == TRY)
    assert shell.showing()["drawer"]["values"][-1] == "core.nix#2138", shell.showing()
    # Return steps into it where it lives, never a copy of it.
    shell.send_key("ret")
    shell.launcher_until(lambda shown: shown["held"] == ["agents", "Landed", CLOSING])
    assert "Copy What to Try" in shell.showing()["rows"], shell.showing()
    shell.close_launcher()
    # Read, it's gone from To Try and in For You's History, saying when; Landed keeps both.
    in_for_you(shell)
    shell.launcher_until(lambda shown: "To Try" not in shown["rows"])
    shell.down_to("History")
    shell.step_in()
    shell.launcher_until(lambda shown: CLOSING in shown["rows"])
    shell.down_to(CLOSING)
    assert re.fullmatch(r"read .+", shell.showing()["selected"]["side"]), shell.showing()
    shell.close_launcher()
    opened(shell)
    shell.down_to("Landed")
    shell.step_in()
    shell.launcher_until(lambda shown: shown["rows"] == [STOPPED, CLOSING])


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
    opened(shell)
    shell.down_to("1 listening, 1 paused")
    assert shell.showing()["selected"]["told"][:1] == ["\u21b5 open"], shell.showing()
    shell.step_in()
    shell.launcher_until(lambda shown: shown["rows"][:2] == ["Listening Quest", "Paused Quest"])


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


def test_a_closed_orchestrator_stands_dimmed_at_the_trees_end_the_last_live_first(
    shell: Shell,
) -> None:
    """The closed stand after every live row, the one last live first, dimmed, Return reopening."""
    # Lamplight's orchestration spent today; Beacon's spent nothing.
    family = {"all": "$14.20", "own": "$3.10", "under": "$11.10"}
    SPEND.write_text(
        json.dumps({"v": 2, "agentsToday": "$214", "families": {"QEST-20261107-LAMPLT": family}})
    )
    opened(shell)
    shell.launcher_until(lambda shown: "Beacon" in shown["rows"])
    rows = shell.showing()["rows"]
    tree = ["Orchestrating Quest", "1 listening, 1 paused", "Lamplight", "Beacon"]
    assert rows[:4] == tree, rows
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
    before, taken = made_by_agents(shell), sortings_taken(shell)
    landing = {"at": datetime.now(UTC).isoformat(), "ref": "core.nix#2141", "what": "Arrived"}
    lay("ARRIVE", "Arriving Quest", {"state": "listening", "busy": False, "landings": [landing]})
    # Taken once the shell has sorted it in, which makes whatever its change makes at once.
    deadline = time.monotonic() + 10
    while sortings_taken(shell) == taken:
        assert time.monotonic() < deadline, "the shell never took the arriving session"
        time.sleep(0.1)
    made = made_by_agents(shell) - before
    assert made == 0, f"the shut launcher made {made} of Agents' rows as a session arrived"
    shell.as_account(shell.ipc("collection", "agents"))
    shell.launcher_ready()
    shell.launcher_until(lambda shown: "Arriving Quest" in shown["rows"])
    shell.launcher_until(lambda shown: "Landed" in shown["rows"])
    shell.down_to("Landed")
    shell.step_in()
    shell.launcher_until(lambda shown: "Arrived" in shown["rows"])


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

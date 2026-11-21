"""An agent's row in the launcher does, on Return, the act its state makes harmless, on niri.

modules/quickshell/checks/agents.nix runs this through pytest in tests/nested-niri.sh. The sessions
are the realm's own mycelium node's, which reads the presence records laid here as pi lays
them, each owned by this test's own process so the node judges it alive; the shell knows them
from the node alone. A listening session's Return asks what to say to it, and a paused one's
lands on Resume, which a second Return sends it; Ctrl+] still steps into all of either's acts.
What Joel says to one shows under the field, sent, then how its row says it stands, then its
answer.
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

import contextlib
import json
import logging
import os
import re
import shlex
import socket
import subprocess
import time
from datetime import UTC, datetime, timedelta
from functools import partial
from pathlib import Path
from typing import TYPE_CHECKING, Any, cast

import launcher_on_niri
import pytest

if TYPE_CHECKING:
    from collections.abc import Callable, Iterator, Mapping

    from launcher_on_niri import Json, Shell

REALM = "harbour"
STATE = Path(os.environ["XDG_STATE_HOME"])
PRESENCE = STATE / "pi/agentic-harness.pi/mesh-workflow/presence"
STEER = STATE / "pi/agentic-harness.pi/steer"
# Where the harness marks each session it parks, a JSON file named by its id, which the node
# reads to call one whose owner is gone parked rather than closed.
PARKS = STATE / "pi/agentic-harness.pi/park-workflow/parked"
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
# Long past a round trip to the lab's agents daemon, a few milliseconds, so a drawer watched
# ahead as another is drawn is held by then.
PAST_A_ROUND_TRIP = 0.5
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
# Landed holds hundreds on Joel's shell, so a run of older ones, past To Try's day and today's
# count, each going nowhere but read, as the first two do: rows saying the same keys. They
# run back from noon two days before, so they land on one day whatever the hour: run back
# from the minute the walk began, a walk soon after midnight split them over two days.
EARLIER = 40
EARLIER_FROM = (
    (LANDED - timedelta(days=2)).astimezone().replace(hour=12, minute=0, second=0, microsecond=0)
)
LANDINGS += [
    {
        "at": (EARLIER_FROM - timedelta(minutes=at)).isoformat(),
        "ref": f"core.nix#{1000 + at}",
        "what": f"Earlier change {at}.",
    }
    for at in range(EARLIER)
]
# How many rows' keys a step into Landed may weigh: one of each run alike, not each of them.
WEIGHED_INTO_LANDED = 3


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
# gen_ai's names, counted on the day the sandbox's clock says as the file loads, which a case
# reading its figures lays again once the day has turned (usage_dated_today).
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
    laid_whole(address, record)


def laid_whole(address: str, record: Json) -> None:
    """Write a session's presence record whole, by rename, as pi does."""
    laid = PRESENCE / f"{address.lower()}.json.tmp"
    laid.write_text(json.dumps(record))
    laid.rename(PRESENCE / f"{address.lower()}.json")


def laid_record(address: str) -> Json:
    """Give a session's presence record as it was last laid."""
    return cast("Json", json.loads((PRESENCE / f"{address.lower()}.json").read_text()))


def usage_dated_today(address: str) -> None:
    """Lay a session's usage again on today's date, once the day has turned since it was laid.

    The daemon says a day's figures only on their day (modules/agents/src/figures.rs), and the
    builders run on UTC, so a run begun before midnight would find none in a case after it, as
    one at 00:02Z did (XY9H5C, 1b5eff6c). A session's next turn lays its usage on the new day,
    as this does.
    """
    record = laid_record(address)
    usage = cast("Json", record["usage"])
    today = datetime.now().astimezone().date().isoformat()
    if usage["day"] != today:
        dated = {**usage, "day": today}
        laid_whole(address, {**record, "usage": dated, "at": datetime.now(UTC).isoformat()})


def closed_side(address: str) -> str:
    """Say when a closed session ended as Recent's row does: its time today, else its day too."""
    ended = datetime.fromisoformat(str(laid_record(address)["at"])).astimezone()
    today = ended.date() == datetime.now().astimezone().date()
    return f"closed {ended:%H:%M}" if today else f"closed {ended:%a %H:%M}"


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
            # The mesh groups it joined, as pi names them, each a network (mycelium's
            # networks): its repository's, titled by its repo, and a group of its own.
            "groups": ["#repo/core-nix", "#harbour"],
            "usage": FULL,
            "worksOn": WORKS_ON,
        },
    )
    lay(
        "PAUSED",
        "Paused Quest",
        {**under, "state": "paused", "landings": LANDINGS, "groups": ["#repo/core-nix"]},
    )
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


# How long what a link opens drops every key (Prompt.qml's linkQuiet), and a little more.
LINK_QUIET = 0.8
# What a case types into what a link opened, and how often it tries to type inside the link's
# quiet.
LINK_KEYS = "hi"
LINK_TRIES = 5


def followed(shell: Shell, verb: str, said: str, *, quiet: bool = True) -> Json:
    """Ask the launcher what a mycelium: link asks (talkTo, goTo, network, machine), and answer.

    What a link opens drops every key for a moment, so unless the case is that moment, it's
    waited out before the case presses one.
    """
    did = cast("Json", json.loads(shell.as_account(shell.ipc(verb, shlex.quote(said)))))
    if quiet:
        time.sleep(LINK_QUIET)
    return did


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


def test_the_first_step_into_landed_after_a_start_makes_no_landings_row_anew(
    shell: Shell,
) -> None:
    """The first step into Landed after the shell starts finds its rows kept as it started.

    Landed's rows were made only as a step first went into it, so each first To Try Return
    after a switch, which steps through Landed, made all of Joel's 2,600 landings' rows: 350
    to 376 ms of its work, against 34 to 50 ms for each later one (his journal, 2026-11-15).
    It's the module's first case, so no case before it has stepped into Landed, and it reads
    no landing, which To Try's Return would, for the cases after it.
    """
    launcher_on_niri.warmed(shell)
    shell.reported("landings kept as the shell started", lambda _: True, shell.launched)
    opened(shell)
    shell.launcher_until(lambda shown: "Landed" in shown["rows"])
    shell.down_to("Landed")
    before = made_by_agents(shell)
    shell.step_in()
    shell.launcher_until(lambda shown: shown["rows"][:2] == [STOPPED, TRY_FIRST])
    made = made_by_agents(shell) - before
    assert made < len(LANDINGS), f"the first step into Landed made {made}: {made_lately(shell)}"


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
    # Its acts are its drawer's, as the agents daemon serves them: one held here has a
    # window to go to, its kitty's, so Go To comes first.
    shell.launcher_until(lambda shown: shown["rows"][:3] == ["Go To", "Talk", "Pause"])
    assert not shell.showing()["asking"], shell.showing()


def test_talk_to_opens_a_sessions_talk_empty_and_sends_nothing_until_joel_does(
    shell: Shell,
) -> None:
    """TalkTo and an address opens the launcher on that session's Talk, its field empty.

    A cell on the realm's map asks it (XUBK2E, 2026-11-15 [2b5c927a]): it lands where Return
    on the session's row does, and words after the address are never typed in, so a page can't
    say anything for Joel. An address no session has is said as one the launcher hasn't, and
    opens nothing. Whom each word names is tests/talk-to.qml's.
    """
    inbox = STEER / "LISTEN"
    steered = set(inbox.glob("*.json"))
    for said in ("LISTEN", "listen send this now"):
        did = followed(shell, "talkTo", said)
        assert did["done"], did
        shell.launcher_until(
            lambda shown: (
                shown["open"] and shown["asking"] and shown["held"] == ["agents", "Listening Quest"]
            )
        )
        shown = shell.showing()
        assert (shown["typed"], shown["mode"]) == ("", "insert"), shown
        shell.close_launcher()
    assert set(inbox.glob("*.json")) == steered, sorted(inbox.glob("*.json"))
    did = followed(shell, "talkTo", "GONE42", quiet=False)
    assert did == {"error": "the launcher has nothing called GONE42"}, did
    assert not shell.showing()["open"], shell.showing()


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


def test_a_talk_left_in_normal_opens_typing_after_its_last_letter(shell: Shell) -> None:
    """A talk left with Escape opens again typing where insert would take it back.

    Escape rests normal's caret on the last letter, one left of where it was typed, and that was
    kept, so the next letter landed one short (RCTRF8, 2026-11-17: "hi" then x gave "hxi").
    Stepped back a letter while typing first, so a caret kept nowhere ("hix", the line's end)
    or kept where normal rests ("xhi") each types somewhere else. Where each caret is kept is
    modules/quickshell/tests/query-keys.qml's.
    """
    found(shell, "listening quest", "Listening Quest")
    shell.send_key("ret")
    shell.launcher_until(lambda shown: shown["asking"])
    try:
        shell.send_chars("hi")
        shell.launcher_until(lambda shown: shown["typed"] == "hi")
        shell.send_key("left")
        shell.launcher_until(lambda shown: shown["caret"] == 1)
        shell.send_key("esc")
        shell.launcher_until(lambda shown: shown["mode"] == "normal" and shown["caret"] == 0)
        shell.close_launcher()
        talk_opened(shell)
        shell.send_chars("x")
        shell.launcher_until(lambda shown: shown["typed"] in ("hix", "hxi", "xhi"))
        assert shell.showing()["typed"] == "hxi", shell.showing()
    finally:
        talk_cleared(shell)


def test_a_talk_left_after_i_opens_typing_where_i_took_it(shell: Shell) -> None:
    """A talk left in insert by i, nothing typed, opens again typing before the letter it rests on.

    Escape keeps the caret insert left from, after the letter, so i, back in insert where normal
    rests, must keep its own caret rather than that one ("hxi" rather than "hix"). The shell
    starts again with the talk open, as at a switch, since a shutting launcher leaves insert
    through normal and keeps Escape's caret again.
    """
    found(shell, "listening quest", "Listening Quest")
    shell.send_key("ret")
    shell.launcher_until(lambda shown: shown["asking"])
    try:
        shell.send_chars("hi")
        shell.launcher_until(lambda shown: shown["typed"] == "hi")
        shell.send_key("esc")
        shell.launcher_until(lambda shown: shown["mode"] == "normal" and shown["caret"] == 1)
        shell.send_chars("i")
        shell.launcher_until(lambda shown: shown["mode"] == "insert" and shown["caret"] == 1)
        deadline = time.monotonic() + 10
        while 1 not in drafted_carets("hi"):
            assert time.monotonic() < deadline, f"i's caret was never kept: {drafted_carets('hi')}"
            time.sleep(0.05)
        shell.restart()
        talk_opened(shell)
        shell.send_chars("x")
        shell.launcher_until(lambda shown: shown["typed"] in ("hix", "hxi", "xhi"))
        assert shell.showing()["typed"] == "hxi", shell.showing()
    finally:
        talk_cleared(shell)


def test_a_talk_left_from_its_start_opens_typing_at_its_start(shell: Shell) -> None:
    """A talk whose insert was left at the line's start opens again typing there, not a letter on.

    Normal rests on the first letter whether insert was left before it or after it, so only
    where insert was as Escape came (Prompt.qml's insertLeft, set before the mode moves) tells
    the start from one past it; kept from the caret alone, x lands after the h ("hxi").
    """
    found(shell, "listening quest", "Listening Quest")
    shell.send_key("ret")
    shell.launcher_until(lambda shown: shown["asking"])
    try:
        shell.send_chars("hi")
        shell.launcher_until(lambda shown: shown["typed"] == "hi")
        # A talk's normal edits no words, there being no row to name them, so its line's start
        # is reached in insert, as the line's own caret moves.
        shell.send_key("left")
        shell.send_key("left")
        shell.launcher_until(lambda shown: shown["mode"] == "insert" and shown["caret"] == 0)
        shell.send_key("esc")
        shell.launcher_until(lambda shown: shown["mode"] == "normal" and shown["caret"] == 0)
        shell.close_launcher()
        talk_opened(shell)
        shell.send_chars("x")
        shell.launcher_until(lambda shown: shown["typed"] in ("xhi", "hxi", "hix"))
        assert shell.showing()["typed"] == "xhi", shell.showing()
    finally:
        talk_cleared(shell)


def drafted_carets(words: str) -> list[int]:
    """Each caret the shell's drafts file keeps with `words`, wherever in it they're kept."""
    carets: list[int] = []

    def walk(held: object) -> None:
        if isinstance(held, dict):
            if held.get("text") == words and isinstance(held.get("caret"), int):
                carets.append(held["caret"])
            for value in held.values():
                walk(value)
        elif isinstance(held, list):
            for value in held:
                walk(value)

    for kept in (STATE / "quickshell/by-shell").glob("*/launcher-drafts.json"):
        with contextlib.suppress(ValueError):
            walk(json.loads(kept.read_text()))
    return carets


def talk_opened(shell: Shell) -> None:
    """Open Listening Quest's talk again from a shut launcher, typing as it opens."""
    found(shell, "listening quest", "Listening Quest")
    shell.send_key("ret")
    shell.launcher_until(lambda shown: shown["asking"])


def talk_cleared(shell: Shell) -> None:
    """Empty Listening Quest's talk whatever a failure left it in, so no case after inherits it.

    The talk opens again typing however it was left, and Ctrl+U clears to the caret, which
    five Rights take past what any case here types.
    """
    shell.close_launcher()
    talk_opened(shell)
    for _ in range(5):
        shell.send_key("right")
    shell.send_key("ctrl-u")
    shell.launcher_until(lambda shown: shown["typed"] == "")


@pytest.mark.usefixtures("at_shots_scale")
def test_a_talk_shows_under_its_field_sent_then_waiting_then_answered(
    shell: Shell, shoot: Callable[[str], None]
) -> None:
    """What Joel says to a session stands under the field until it answers, as the line's do.

    Talk opened over empty rows and said nothing once sent (Joel at 19:30 on 2026-11-13: "no
    feedback, they need to feel more like the void"). Before any talk the card is the field
    and its foot alone; his words then stand with sent, then delivered as the session's row
    says it, and its answer in their place, which is told read once shown two seconds. How
    each stage reads is tests/talk-line.qml's.
    """
    words, answer = "is the tide in yet", "Rising, and full by six."
    inbox = STEER / "LISTEN"
    replies = inbox / "replies"
    replies.mkdir(parents=True, exist_ok=True)
    steered = set(inbox.glob("*.json"))
    talk: Path | None = None

    def now() -> str:
        return datetime.now(UTC).isoformat(timespec="milliseconds").replace("+00:00", "Z")

    try:
        found(shell, "listening quest", "Listening Quest")
        shell.send_key("ret")
        shell.launcher_until(lambda shown: shown["asking"] and shown["talks"] == [])
        card = shell.showing()["card"]
        assert card["foot"] - card["top"] == card["drawn"], card
        shoot("talk-empty")
        shell.send_chars(words)
        shell.launcher_until(lambda shown: shown["typed"] == words)
        shoot("talk-typed")
        since = shell.mark()
        shell.send_key("ret")

        def standing(shown: Json) -> tuple[object, ...]:
            talks = cast("list[Json]", shown["talks"] or [{}])
            first = talks[0]
            return (len(talks), first.get("line"), first.get("stands"), first.get("waiting"),
                first.get("answers"), first.get("by"))  # fmt: skip

        shell.launcher_until(lambda shown: standing(shown) == (1, words, "sent", "", [], ""))
        card = shell.showing()["card"]
        assert card["foot"] - card["top"] > card["drawn"], card
        shown_after_return(shell, since)
        shoot("talk-sent")
        # The session's inbox took it, and its talk's own file, named by the id the launcher
        # gave it and opened by the node, says what became of it, as pi says it.
        talk = replies / f"{said_to(inbox, steered, words)}.jsonl"
        delivered = {"stage": "delivered", "at": now(), "session": "s-listen", "from": "joel",
            "text": words}  # fmt: skip
        talk.write_text(json.dumps(delivered) + "\n")
        waits = "delivered"
        shell.launcher_until(lambda shown: standing(shown) == (1, words, "", waits, [], ""))
        shoot("talk-waiting")
        replied = {"stage": "replied", "at": now(), "text": answer}
        with talk.open("a") as lines:
            lines.write(json.dumps(replied) + "\n")
        shell.launcher_until(
            lambda shown: standing(shown) == (1, words, "", "", [answer], "Listening Quest")
        )
        shoot("talk-answered")
        # Shown while it's open, the answer is heard, which the daemon says in its file.
        deadline = time.monotonic() + 10
        while '"stage":"read"' not in talk.read_text():
            assert time.monotonic() < deadline, talk.read_text()
            time.sleep(0.1)
    finally:
        if talk is not None:
            talk.unlink(missing_ok=True)
        for one in set(inbox.glob("*.json")) - steered:
            one.unlink()


def said_to(inbox: Path, steered: set[Path], words: str) -> str:
    """Give the id of the talk these words went in as, once their steer is in the inbox."""
    deadline = time.monotonic() + 10
    while True:
        for one in set(inbox.glob("*.json")) - steered:
            steer = json.loads(one.read_text())
            if steer.get("text") == words and str(steer.get("id", "")).startswith("talk-"):
                return str(steer["id"])
        assert time.monotonic() < deadline, sorted(inbox.glob("*.json"))
        time.sleep(0.1)


def shown_after_return(shell: Shell, since: int) -> None:
    """Say how long words given took from their Return to the frame showing them, as said."""
    deadline = time.monotonic() + 10
    while not (
        took := re.search(
            r"words given shown (\d+) ms", shell.log.read_bytes()[since:].decode(errors="replace")
        )
    ):
        assert time.monotonic() < deadline, shell.tail()
        time.sleep(0.1)
    logging.getLogger(__name__).info("words given were shown %s ms after Return", took[1])


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
    usage_dated_today("LISTEN")
    row = found(shell, "listening quest", "Listening Quest")
    # Found by typing, whom it's under leads, before what's its own.
    assert row["side"].endswith(" \u00b7 84% full \u00b7 listening"), row
    assert row["hue"] == "warning", row
    # Its drawer's glance gives its last turn and how full its context is, the figures that
    # move as it works, and folds the rest under z a by gen_ai's names (VKCF3Z, 2b03e43c).
    moving = ["Last Turn", "Context"]
    folded = ["Model", "Turns Today", "Tokens", "Cache"]
    shell.launcher_until(
        lambda shown: (
            [fact for fact in shown["drawer"]["facts"] if fact in moving] == moving
            and shown["drawer"]["nerd"][: len(folded)] == folded
        )
    )
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
                and "Context Rebuilt" in shown["drawer"]["nerd"]
            )
        )
    finally:
        SPEND.write_text(json.dumps({"v": 2, "agentsToday": "$214"}))


def shown_first(drawer: Json) -> list[str]:
    """Give the facts a drawer's first screen shows, each laid above its foot.

    Its more line counts what runs below its words as well as its facts, so the facts are
    read by where each is laid rather than by that count.
    """
    facts = cast("list[str]", drawer["facts"])
    lines = cast("list[float]", drawer["lines"]["facts"])
    foot = cast("float", drawer["foot"])
    return [fact for fact, line in zip(facts, lines, strict=True) if line < foot]


@pytest.mark.usefixtures("at_shots_scale")
def test_a_sessions_drawer_glances_at_six_facts_at_most_and_folds_the_rest(
    shell: Shell, shoot: Callable[[str], None]
) -> None:
    """Its glance says its last turn and how full, all on its first screen, its quest's prose under.

    How it stands is its row's, so with no step it says no Doing. What its quest is about
    is the drawer's prose, not a fact, since it runs long (VKCF3Z, a2101591), and leaves out
    the title the drawer already says, drawn whole rather than cut, since z a opens the fold
    rather than reading on (VKCF3Z, e3bb38e5). Its glance holds six facts at most, in
    VKCF3Z's order (2b03e43c); the rest, its quest and machine among them, is folded under
    z a after its figures, so none is lost.
    """
    usage_dated_today("LISTEN")
    found(shell, "listening quest", "Listening Quest")
    glance = ["Last Turn", "Context"]
    # Its figures folded too, so a drawer from before its usage was laid again isn't read.
    shell.launcher_until(
        lambda shown: (
            shown["drawer"]["shown"]
            and shown["drawer"]["facts"] == glance
            and "Cache" in shown["drawer"]["nerd"]
        )
    )
    drawer = shell.showing()["drawer"]
    assert shown_first(drawer) == glance, f"its first screen hides some: {drawer}"
    summary = WORKS_ON.removeprefix("Listening Quest: ")
    assert drawer["body"] == summary, drawer
    assert drawer["bodyDrawn"] == summary, drawer
    unfolded(shell, shoot, "agents-drawer-glance")
    nerd = drawer["nerd"]
    assert "Doing" not in nerd, drawer
    assert nerd.index("Quest") > nerd.index("Cache"), drawer
    # Nor does its row's note, under the same title; found by typing, whom it's under leads.
    note = shell.showing()["selected"]["note"]
    assert note == f"under BOSSES \u00b7 {summary}", note
    # One that landed something folds what it landed last, out of its glance.
    found(shell, "paused quest", "Paused Quest")
    shell.launcher_until(
        lambda shown: (
            shown["drawer"]["shown"]
            and "Last Landing" not in shown["drawer"]["facts"]
            and "Last Landing" in shown["drawer"]["nerd"]
        )
    )


def test_za_shows_a_sessions_fold_above_its_more_line_its_prose_giving_way(shell: Shell) -> None:
    """Opened by z a, the fold's line and its first bit stand above the more line.

    A session's prose runs past its card, so opened, the fold would stand under it, out of
    sight, and z a would change nothing seen. The prose gives up whole lines from its end for
    the fold instead, its rest counted under the more line (VKCF3Z, bfdad751).
    """
    found(shell, "listening quest", "Listening Quest")
    shell.launcher_until(lambda shown: shown["drawer"]["shown"] and shown["drawer"]["nerd"])
    shell.normal()
    shell.send_chars("za")

    # Read once every bit is made, which the repeater does after it counts them.
    def laid(shown: Json) -> bool:
        drawer = shown["drawer"]
        lasts = drawer["lines"]["nerdLast"]
        return drawer["folded"] is False and len(lasts) == len(drawer["nerd"]) and None not in lasts

    shell.launcher_until(laid)
    opened = shell.showing()["drawer"]
    shell.send_chars("za")
    shell.launcher_until(lambda shown: shown["drawer"]["folded"] is True)
    shut = shell.showing()["drawer"]
    shell.on_find()
    # The last line it keeps fades at its end, as a long name does as it
    # reads on, so it says more follows rather than read as a broken
    # sentence; shut, its prose is whole to its ellipsis (VKCF3Z, 448f6cc6).
    fade = opened.get("wordsFade")
    assert fade, f"its kept prose ends with nothing to say more follows: {opened}"
    assert fade["from"] < fade["to"], f"its fade covers nothing: {opened}"
    # Its words are laid by how wide Qt draws them, so its lines break where
    # Qt's do and its fade ends on the ink: the face's metrics measured a
    # line of the serif 236 pixels wide where Qt drew it 229.
    widths = cast("list[int]", opened["laidWidths"])
    assert len(widths) == opened["bodyLines"], f"its words laid in lines Qt's aren't: {opened}"
    assert abs(widths[fade["line"]] - fade["to"]) <= 1, (
        f"its words laid by a measure wider than Qt draws them: {opened}"
    )
    # Drawn only where effects draw: Qt drawing in software draws none, and
    # a mask it can't draw would take the whole line with it, so there the
    # line stands whole to its last word (VKCF3Z, f24de250).
    effects = os.environ.get("QT_QUICK_BACKEND") != "software"
    assert opened.get("fadeDrawn") is effects, (
        f"its fade not drawn just where effects are: {opened}"
    )
    assert shut.get("wordsFade") is None, f"shut, its prose still fades: {shut}"
    _, *bits = cast("list[float]", opened["lines"]["nerd"])
    foot = cast("float", opened["foot"])
    assert bits, f"its fold opens on no bits: {opened}"
    assert bits[0] < foot, f"its fold's line and first bit stand below its foot: {opened}"
    # A bit is shown whole or not at all: a value's second line cut at the
    # foot showed the tops of its letters alone.
    lasts = cast("list[float]", opened["lines"]["nerdLast"])
    cut = [(first, last) for first, last in zip(bits, lasts, strict=True) if first < foot < last]
    assert not cut, f"a bit stands across its foot: {opened}"
    assert opened["moreSays"] != "", f"it hides its prose's rest unsaid: {opened}"


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
    names = into_the_fold(shell)
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


# The fold's sessions, in the order it lists them.
FOLDED = ("Listening Quest", "Paused Quest")


def into_the_fold(shell: Shell) -> tuple[str, str]:
    """Step into the orchestration's fold, and give its sessions' names once both are listed."""
    into_orchestration(shell)
    shell.down_to(FOLD)
    shell.step_in()
    shell.launcher_until(lambda shown: all(name in shown["rows"] for name in FOLDED))
    return FOLDED


def rested_on(name: str) -> Callable[[Json], bool]:
    """Test that the session named is selected, its drawer out and listing its acts."""
    return lambda shown: (
        (shown["selected"] or {}).get("name") == name
        and bool((shown["drawer"] or {}).get("shown"))
        and bool((shown["drawer"] or {}).get("acts"))
    )


def test_a_sessions_drawer_beside_the_one_drawn_comes_with_its_key(shell: Shell) -> None:
    """A key to the session beside the one drawn finds its drawer held, drawn with the key.

    Agents watched a session's drawer only once it was drawn, so an arrow to a session's row
    whose drawer the shell didn't hold yet left it a daemon's round trip behind: all ten drawers
    Joel's journal said came "a round trip after its row" from 18 to 19 November were
    sessions', 23 to 71 ms each. So the drawers of the rows beside the one drawn are watched
    too, as Landed's are.
    """
    # A drawer once watched stays held, so the case before's arrows left Paused's held, drawn
    # in the key's frame whether its row was watched ahead or not. Started again, the shell
    # holds none, as on the first walk after a switch.
    shell.restart()
    names = into_the_fold(shell)
    rows = cast("list[str]", shell.showing()["rows"])
    assert rows.index(names[1]) == rows.index(names[0]) + 1, f"the fold's rows: {rows}"
    shell.down_to(names[0])
    shell.launcher_until(rested_on(names[0]))
    # Room for Paused's drawer, watched ahead as Listening's is drawn, to come before the arrow,
    # since nothing reports it held. On main it's watched only once drawn, so no wait holds it.
    time.sleep(PAST_A_ROUND_TRIP)
    since = shell.mark()
    shell.send_key("down")
    shell.launcher_until(rested_on(names[1]))
    row = shell.showing()["drawer"]["row"]
    # Each state the launcher passed through on the way, as it reports each: any whose drawer
    # followed the row with nothing to draw for it yet waited a round trip. One still drawing
    # the row before is a follow's frame, which isn't this.
    reports, _ = shell.said_since(since)
    passed = [json.loads(said.removeprefix("showing ")) for said in reports
              if said.startswith("showing ")]  # fmt: skip
    short = [shown["drawer"] for shown in passed
             if (shown["drawer"] or {}).get("row") == row
             and not shown["drawer"].get("wanted")]  # fmt: skip
    assert not short, f"{names[1]}'s row was drawn before its drawer: {short}"


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


def test_a_sessions_drawer_marks_return_on_the_act_return_does(shell: Shell) -> None:
    """Each session's drawer marks Return on the act its Return does, whatever its state.

    The drawer marked the first of the shell's own acts, Talk or Go To, while Return did the
    daemon's act for its state, Resume for one paused (2026-11-19), so the drawer said one act
    and Return did another; Joel answers a session in any state with one Return.
    """
    for typed, name, act in (("paused quest", "Paused Quest", "Resume"),
                             ("listening quest", "Listening Quest", "Talk")):  # fmt: skip
        row = found(shell, typed, name)
        shell.launcher_until(rested_on(name))
        acts = shell.showing()["drawer"]["acts"]
        marked = [one for one in acts if one.startswith("\u21b5 ")]
        assert marked == [f"\u21b5 {act}"], (row["verb"], acts)
        shell.close_launcher()


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


def test_a_sessions_close_asks_saying_what_stays_and_return_steers_it_closed(
    shell: Shell,
) -> None:
    """Close, last among a session's acts, asks what it leaves, and Return steers it a close.

    The ask is answered as an armed row is (Joel, 2026-11-13): a Return more does it, its foot
    saying so, and any other key, y among them, lets it go without reaching the line.
    """
    into_orchestration(shell)
    rows = shell.showing()["rows"]
    assert rows[: rows.index(FOLD)][-1:] == ["Close"], rows
    shell.down_to("Close")
    inbox = STEER / "BOSSES"

    def asked_to_close() -> None:
        """Return writes the line Close is, and a second runs it, which asks."""
        shell.send_key("ret")
        shell.launcher_until(
            lambda shown: (
                (shown["command"] or {}).get("line") == "close" and not shown["linedFresh"]
            )
        )
        shell.send_key("ret")
        shell.launcher_until(lambda shown: shown["takesAnswer"])

    asked_to_close()
    asking = shell.showing()
    assert asking["question"] == "Close BOSSES? Its folder stays.", asking
    assert asking["hints"] == ["\u21b5 confirm"], asking
    # y once asked closed it; now it lets the ask go, typing nothing, and nothing is steered.
    shell.send_chars("y")
    shell.launcher_until(
        lambda shown: shown["open"] and shown["question"] == "" and shown["typed"] == ""
    )
    assert not any(inbox.glob("*.json")), "y closed the session"
    asked_to_close()
    shell.send_key("ret")
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
    shell.launcher_until(lambda shown: shown["rows"][:2] == [STOPPED, TRY_FIRST])
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
    picks = walked_into_landed(shell)
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
    climbed_out_of_landed(shell, picks)
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
    made_through(shell, lambda: lay("BOSSES", ORCHESTRATING, {**boss, "usage": FULL}))
    shell.down_to("Landed")
    shell.step_in()
    shell.launcher_until(lambda shown: shown["rows"][:2] == [STOPPED, TRY_FIRST])
    # A session changing while Landed is held remakes none of its landings. Counted against
    # Agents' top, a change measured as one pass there and two here failed 1 run in 4, since
    # the top is laid out once for each source that hears it; so the landings are named by key
    # (Provider.qml's tally keeps the last rows made). Remaking them made 26 rows to the top's 9.
    inside = made_through(shell, lambda: lay("BOSSES", ORCHESTRATING, boss))
    remade = landings_made_last(shell, inside)
    assert not remade, f"a change remade {len(remade)} of Landed's landings: {remade}"


def test_a_step_into_landed_weighs_the_keys_of_one_of_each_run_alike(shell: Shell) -> None:
    """Stepping into Landed weighs the keys of one row of each run saying the same, not each.

    Weighing each of Joel's 2,600 landings for the foot took 270 to 355 ms of every step into
    Landed, most of For You's To Try Return, which steps through it (2026-11-14).
    """
    opened(shell)
    shell.launcher_until(lambda shown: "Landed" in shown["rows"])
    shell.down_to("Landed")
    before = launcher_on_niri.weighed_held_still(shell)
    shell.step_in()
    shell.launcher_until(lambda shown: shown["rows"][:2] == [STOPPED, TRY_FIRST])
    weighed = launcher_on_niri.weighed_held_still(shell) - before
    assert weighed <= WEIGHED_INTO_LANDED, f"a step into Landed weighed {weighed} rows' keys"
    shell.close_launcher()


def test_landed_read_again_with_nothing_changed_looks_at_no_landing(shell: Shell) -> None:
    """Stepping into Landed, then into a landing there, looks at Landed's landings once.

    Landed's rows are read again at each step through it, and each read looked at every one
    of Joel's 2,600 landings again, 27 to 39 ms a time, four times in To Try's Return
    (2026-11-14), though nothing they're made from had changed.
    """
    opened(shell)
    shell.launcher_until(lambda shown: "Landed" in shown["rows"])
    shell.down_to("Landed")
    launcher_on_niri.weighed_held_still(shell)
    before = landings_looked(shell)
    shell.step_in()
    shell.launcher_until(lambda shown: shown["rows"][:2] == [STOPPED, TRY_FIRST])
    # Return would read it, as it goes nowhere; Tab steps into its verbs alone.
    shell.step_in("tab")
    shell.launcher_until(lambda shown: "Say What You Make of It" in shown["rows"])
    launcher_on_niri.weighed_held_still(shell)
    looked = landings_looked(shell) - before
    assert looked <= len(LANDINGS), f"two steps through Landed looked at {looked} landings"
    shell.close_launcher()


# The days within which When.js names a day by its weekday.
LATELY_DAYS = 6


def earlier_days() -> list[tuple[str, int]]:
    """Give Landed's days, newest first, as each is named, and how many each holds.

    A day holds the landings over a day old that landed on it, as the daemon moves them; those
    of the last day are Landed's own, whatever their date.
    """
    now = datetime.now().astimezone()
    today = now.date()
    counted: dict[tuple[str, str], int] = {}
    for one in LANDINGS:
        at = datetime.fromisoformat(one["at"]).astimezone()
        day = at.date()
        if "core.nix#" in one["ref"] and now - at >= timedelta(days=1):
            before = (today - day).days
            named = (
                "Yesterday"
                if before == 1
                else day.strftime("%A")
                if before <= LATELY_DAYS
                else f"{day.day} {day.strftime('%b')}"
            )
            key = (day.isoformat(), named)
            counted[key] = counted.get(key, 0) + 1
    return [(named, count) for (_, named), count in sorted(counted.items(), reverse=True)]


def test_landed_lists_a_row_a_day_before_today_whose_landings_come_only_inside(
    shell: Shell,
) -> None:
    """Landed lists the last day's landings, then a row for each day before, with its count.

    Joel's shell held every one of the past 30 days' landings, 4,270 and 3.95 MB of them, and
    made each a row as it started, about 45% of the rows it made (WCJQSG, bea3796d; 19
    November). A day's landings reach the shell only once he steps into it, so it keeps only
    the last day's as it starts, and the day lists its own, newest first. The shell takes
    whatever its daemon tells it, so the bound on what it kept holds the daemon to it.
    """
    days = earlier_days()
    shell.reported(
        "landings kept as the shell started: ",
        lambda rest: int(rest) < EARLIER,
        shell.launched,
    )
    opened(shell)
    shell.launcher_until(lambda shown: "Landed" in shown["rows"])
    shell.down_to("Landed")
    shell.step_in()
    shell.launcher_until(lambda shown: shown["rows"][-len(days) :] == [day for day, _ in days])
    rows = shell.showing()["rows"]
    assert not any(row.startswith("Earlier change") for row in rows), rows
    newest, count = days[0]
    shell.down_to(newest)
    assert shell.showing()["selected"]["side"] == f"{count} landed", shell.showing()
    shell.step_in()
    # What the launcher shows says only its first rows, so each of those is the day's, newest
    # first, and the side said how many it holds.
    shell.launcher_until(
        lambda shown: (
            shown["held"][-1] == newest
            and shown["rows"][:1] == ["Earlier change 0."]
            and all(row.startswith("Earlier change") for row in shown["rows"])
        )
    )
    shell.close_launcher()


def walked_into_landed(shell: Shell) -> dict[str, Json]:
    """Return on To Try's row, which walks into it in Landed, and give the picks kept before.

    The walk passes through Agents and Landed unlisted: listing Joel's 2,600 landings on the
    way took about 40 of its 84 ms (2026-11-14), so it searches once, for its last step.
    """
    searches, picks = launcher_on_niri.searched(shell), picks_kept()
    shell.send_key("ret")
    shell.launcher_until(lambda shown: shown["held"] == ["agents", "Landed", TRY_FIRST])
    searched = launcher_on_niri.searched(shell) - searches
    assert searched == 1, f"To Try's Return searched {searched} times"
    return picks


def climbed_out_of_landed(shell: Shell, picks: dict[str, Json]) -> None:
    """Climb back by - through each step the walk passed, each listed then, onto its row.

    It starts from the words given on the landing, the step they were asked in. Nothing
    typed led there, so no step on the way learned a pick (`picks` were kept before).
    """
    shell.normal()
    shell.send_chars("-")
    shell.launcher_until(lambda shown: shown["held"] == ["agents", "Landed", TRY_FIRST])
    shell.send_chars("-")
    shell.launcher_until(
        lambda shown: (
            shown["held"] == ["agents", "Landed"]
            and shown["rows"][:2] == [STOPPED, TRY_FIRST]
            and (shown["selected"] or {}).get("name") == TRY_FIRST
        )
    )
    shell.send_chars("-")
    shell.launcher_until(
        lambda shown: (
            shown["held"] == ["agents"]
            and "Landed" in shown["rows"]
            and (shown["selected"] or {}).get("name") == "Landed"
        )
    )
    assert picks_kept() == picks, f"walking to it learned {picks_kept()}, from {picks}"


def picks_kept() -> dict[str, Json]:
    """Give what the launcher has learned was meant by what was typed, as it keeps it."""
    said: dict[str, Json] = {}
    for path in STATE.glob("quickshell/by-shell/*/launcher-picks.json"):
        if path.read_text().strip():
            said.update(cast("dict[str, Json]", json.loads(path.read_text())))
    return said


def landings_made_last(shell: Shell, made: int) -> list[str]:
    """Give the keys of the landings among the last `made` rows Agents made."""
    assert made <= launcher_on_niri.LATELY_KEPT, f"Agents made {made} rows: {made_lately(shell)}"
    lately = shell.made()["lately"]["agents"]
    return [key for key in (lately[-made:] if made else []) if key.startswith("agents:landed:")]


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


def unfolded(shell: Shell, shoot: Callable[[str], None], name: str) -> None:
    """Shoot a drawer as it stands, then with its fold opened on za, and fold it again.

    za is normal mode's, typed on the find otherwise, so it's pressed from there and the line
    is left typing again.
    """
    shoot(name)
    shell.normal()
    shell.send_chars("za")
    shell.launcher_until(lambda shown: shown["drawer"]["folded"] is False)
    shoot(f"{name}-open")
    shell.send_chars("za")
    shell.launcher_until(lambda shown: shown["drawer"]["folded"] is True)
    shell.on_find()


@pytest.mark.usefixtures("at_shots_scale")
def test_agents_top_level_is_the_rows_its_daemon_serves_ready(
    shell: Shell, shoot: Callable[[str], None]
) -> None:
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
            and drawer["facts"] + drawer["nerd"] != []
            and all(label != "" for label in drawer["facts"] + drawer["nerd"])
        )

    shell.launcher_until(worded)
    assert not shell.showing()["drawer"]["eyebrowDrawn"].startswith("[object"), shell.showing()
    unfolded(shell, shoot, "agents-drawer-orchestrator")


def test_an_orchestrators_acts_split_evenly_none_alone_on_a_line(shell: Shell) -> None:
    """Acts too wide for one line split evenly over two, none alone on one.

    The orchestrator's five acts left Close alone under the other four at 288 pixels, a wrap
    as gross as any (VKCF3Z, 448f6cc6).
    """
    opened(shell)
    shell.launcher_until(lambda shown: shown["drawer"]["shown"] and shown["drawer"]["acts"])
    drawer = shell.showing()["drawer"]
    laid = cast("list[int]", drawer["actsLaid"])
    assert sum(laid) == len(drawer["acts"]), f"its acts aren't all laid: {drawer}"
    # Its step into its sessions, last, is too wide to share a line with
    # Close, so it stands alone; the five before it split evenly over two,
    # Return's act leading the longer.
    assert laid == [3, 2, 1], f"its acts aren't split evenly, one alone only as it must: {drawer}"


def test_an_identifier_in_a_fold_is_never_broken_its_name_standing_above_it(shell: Shell) -> None:
    """A quest's id too wide for the fold's value column stands whole under its name.

    At twenty letters, QEST-20261107-BOSSES won't fit beside Quest in a 288-pixel drawer and
    broke at its hyphen; it stands under its name at the full measure instead, as the
    glance's facts do (VKCF3Z, bfdad751).
    """
    opened(shell)
    shell.launcher_until(
        lambda shown: shown["drawer"]["shown"] and "Quest" in shown["drawer"]["nerd"]
    )
    shell.normal()
    shell.send_chars("za")
    shell.launcher_until(lambda shown: shown["drawer"]["folded"] is False)
    drawer = shell.showing()["drawer"]
    shell.send_chars("za")
    shell.launcher_until(lambda shown: shown["drawer"]["folded"] is True)
    shell.on_find()
    bits = dict(zip(drawer["nerd"], drawer["nerdDrawn"], strict=True))
    assert bits["Quest"] == {"lines": 1, "under": True}, f"its quest's id breaks: {bits}"


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
    # Forgetting ends what's offered, so Return writes :forget, which runs at once on a
    # session whose folder has gone, since it couldn't be reopened anyway.
    shell.send_key("ret")
    shell.launcher_until(
        lambda shown: (shown["command"] or {}).get("line") == "forget" and not shown["linedFresh"]
    )
    shell.send_key("ret")
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


def test_no_words_however_long_leave_their_room_or_lie_on_anothers(shell: Shell) -> None:
    """Words of any length stay inside their room and the card, and off each other's ink.

    A talk Joel sent with what he was looking at attached ran its lines down over the rows,
    and its words leftward over the row's name and out of the card (Joel at 19:00 on
    2026-11-15: "somehting like this should never be possible"). So each step from the
    session down to its talk's parts, with the path, the rows, their sides and the drawer
    filled with long words and a word too long for any line, asks the launcher which words
    broke out (Contained.js), and none may. Where words are drawn needs no system service,
    so it's proven here rather than in a VM (docs/testing.md, The Pyramid).
    """
    unbroken = "whydidthewindowjumpwheneverynameranon" * 4
    asked = (
        "why did the launcher let a talk run past its card when i asked it about our agentic "
        f"harness and the {unbroken} around this?\n\nAttached:\n- window: kitty \u00b7 ssh"
    )
    answer = f"A step drew whole: {unbroken}, and the side beside it drew whole too."
    replies = STEER / "LISTEN" / "replies"
    replies.mkdir(parents=True, exist_ok=True)
    talk = replies / "talk-1794786327974.jsonl"
    lines = [
        {"stage": "delivered", "at": "2026-11-15T19:00:00.000Z", "session": "s-listen",
         "from": "joel", "text": asked},
        {"stage": "replied", "at": "2026-11-15T19:01:00.000Z", "text": answer},
    ]  # fmt: skip
    talk.write_text("".join(json.dumps(line) + "\n" for line in lines))

    def none_broke_out(where: str) -> None:
        broke, drawn = broke_out_once_laid(shell, where)
        if broke:
            said = {fault["said"] for fault in broke} | {fault.get("with") for fault in broke}
            who = [each for surface in drawn for each in surface if each["said"] in said]
            assert broke == [], f"{where}: {broke}\ndrawn by: {who}"

    try:
        found(shell, "listening quest", "Listening Quest")
        shell.launcher_until(lambda shown: bool(shown["drawer"]))
        none_broke_out("the session found, its drawer out")
        shell.step_in("ctrl-bracketright")
        shell.launcher_until(
            lambda shown: any(row.startswith("why did the launcher") for row in shown["rows"])
        )
        name = next(
            row for row in shell.showing()["rows"] if row.startswith("why did the launcher")
        )
        none_broke_out("inside the session")
        shell.down_to(name)
        none_broke_out("its talk selected, its drawer out")
        shell.step_in()
        shell.launcher_until(lambda shown: shown["rows"][:2] == ["You Said", "How It Went"])
        none_broke_out("its parts, what Joel said selected")
        shell.down_to("How It Went")
        none_broke_out("its parts, how it went selected")
    finally:
        talk.unlink()


# How long the words on the card and its drawer must hold still before a case asks which broke
# out, and how long they have to come to rest and be drawn.
LAID_STILL = 0.5
LAID_WITHIN = 10.0


def broke_out_once_laid(shell: Shell, where: str) -> tuple[list[Json], list[list[Json]]]:
    """Give the words that broke out once the card and its drawer are laid, and every word.

    Qt lays a row or a column out again only before its next frame, and a loaded builder can be
    slow to draw one, so words read between a change and that frame stand where they were:
    the drawer's '1 more' over its dot, a name over the words below it. So the words must hold
    still for LAID_STILL, then through two frames asked for, and read the same after the
    launcher says which broke out as before it. What broke out only at the first read is
    logged, so an overlap that really draws for a moment leaves a trace.
    """
    deadline = time.monotonic() + LAID_WITHIN
    first = json.loads(shell.as_account(shell.ipc("contained")))
    drawn, moved = shell.as_account(shell.ipc("drawnTexts")), time.monotonic()
    while True:
        assert time.monotonic() < deadline, f"{where}: the words never held still: {drawn}"
        time.sleep(launcher_on_niri.POLL)
        now = shell.as_account(shell.ipc("drawnTexts"))
        if now != drawn:
            drawn, moved = now, time.monotonic()
            continue
        if time.monotonic() - moved < LAID_STILL:
            continue
        framed(shell, deadline, where)
        broke = json.loads(shell.as_account(shell.ipc("contained")))
        now = shell.as_account(shell.ipc("drawnTexts"))
        if now == drawn:
            if first and not broke:
                logging.getLogger(__name__).warning(
                    "%s: broke out before it was laid: %s", where, first
                )
            return broke, json.loads(drawn)
        drawn, moved = now, time.monotonic()


def framed(shell: Shell, deadline: float, where: str) -> None:
    """Ask the launcher for a frame twice, each once the last is drawn.

    The first may have been under way before it was asked for, as the launcher's priming says,
    so only the second is sure to lay out what changed before the ask.
    """
    for _ in range(2):
        asked = int(shell.as_account(shell.ipc("drawAFrame")))
        while shell.frames() <= asked:
            assert time.monotonic() < deadline, f"{where}: no frame drawn when asked for"
            time.sleep(launcher_on_niri.POLL)


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


HEADINGS = {"Repositories", "Quests", "Groups", "Machines"}


def presented_after(shell: Shell, key: str) -> int:
    """Press a key and give how long it took to the frame niri presented with it, in ms."""
    before = len(json.loads(shell.as_account(shell.ipc("keysPresented"))))
    shell.send_key(key)
    deadline = time.monotonic() + launcher_on_niri.WAITING.total_seconds()
    presented: list[Json] = []
    while len(presented) <= before:
        assert time.monotonic() < deadline, f"{key} was never presented: {presented}"
        presented = json.loads(shell.as_account(shell.ipc("keysPresented")))
        time.sleep(launcher_on_niri.POLL)
    return round(cast("float", presented[before]["at"]) - cast("float", presented[before]["from"]))


@pytest.mark.usefixtures("at_shots_scale")
def test_networks_is_one_row_after_the_tree_its_kinds_under_headings_never_selected(
    shell: Shell, shoot: Callable[[str], None]
) -> None:
    """One Networks row after the tree steps into every other network, under its kind's heading.

    A heading is never selected: the selection lands on the first network, j and k step over
    headings, and the gutter numbers only the rows (VKCF3Z, e1eb0ebe; Y2YBTS, fc1b8044).
    """
    opened(shell)
    shell.down_to("Networks")
    took = presented_after(shell, "ret")
    logging.getLogger(__name__).info("LAB the step into Networks was presented %s ms on", took)
    shell.launcher_until(
        lambda shown: shown["rows"][:4] == ["Repositories", "core.nix", "Groups", "harbour"]
    )
    assert shell.showing()["selected"]["name"] == "core.nix", shell.showing()
    shoot("agents-networks")
    # The gutter numbers the rows in view and no heading, one to the last: all of them while
    # they fit, as at 1, and fewer on a screen at 2, where the card holds three rows' room.
    shown = shell.showing()
    numbered = [number for number in shown["gutter"]["numbers"] if number != ""]
    rows = [row for row in shown["rows"] if row not in HEADINGS]
    assert numbered == [str(at) for at in range(1, len(numbered) + 1)], shown["gutter"]
    whole = shown["inView"]["bottom"] >= len(shown["rows"]) - 1
    assert len(numbered) == len(rows) if whole else len(numbered) < len(rows), shown
    shell.normal()
    shell.send_chars("j")
    shell.launcher_until(lambda shown: shown["selected"]["name"] == "harbour")
    shoot("agents-networks-stepped")
    shell.send_chars("k")
    shell.launcher_until(lambda shown: shown["selected"]["name"] == "core.nix")
    # G lands on the last network, the list scrolled so the Machines heading shows above it.
    shell.send_chars("G")
    shell.launcher_until(lambda shown: shown["selected"]["name"] == "machines")
    shoot("agents-networks-scrolled")
    shell.send_chars("gg")
    shell.launcher_until(lambda shown: shown["selected"]["name"] == "core.nix")
    # Return on a network steps into its cells, each its session's own row.
    shell.step_in()
    shell.launcher_until(lambda shown: sorted(shown["rows"]) == ["Listening Quest", "Paused Quest"])
    shell.close_launcher()


# A network's drawer's glance, in VKCF3Z's order, what none says left out (e1eb0ebe).
NETWORK_GLANCE = [
    "Standing",
    "Spent Today",
    "Busiest",
    "Machines",
    "Branched From",
    "Last Changed",
]


def test_a_networks_drawer_glances_in_its_order_and_y_copies_its_short_name(
    shell: Shell,
) -> None:
    """A network's drawer says how it stands and when it changed, and y copies its short name.

    It's served in ready-rows' own drawer shape, so it's drawn as any ready-rows drawer is,
    titled by its row's name, its short name folded under z a (VKCF3Z, e1eb0ebe).
    """
    opened(shell)
    shell.down_to("Networks")
    shell.step_in()
    shell.launcher_until(lambda shown: shown["selected"]["name"] == "core.nix")
    shell.normal()
    shell.send_chars("j")
    shell.launcher_until(
        lambda shown: (
            shown["selected"]["name"] == "harbour"
            and bool((shown["drawer"] or {}).get("shown"))
            and shown["drawer"]["title"] == "harbour"
        )
    )
    drawer = shell.showing()["drawer"]
    facts = cast("list[str]", drawer["facts"])
    assert facts == [fact for fact in NETWORK_GLANCE if fact in facts], drawer
    assert {"Standing", "Last Changed"} <= set(facts), drawer
    assert "Short Name" in drawer["nerd"], drawer
    since = shell.mark()
    shell.send_chars("yy")
    shell.reported("yanked 1", lambda rest: rest == "", since)
    assert shell.copied() == "harbour"
    shell.close_launcher()


# How near niri's scale must come to the one asked for, as niri rounds a fractional scale to the
# nearest it can draw at.
SCALE_NEAR = 0.01


def scaled(shell: Shell, scales: Mapping[str, float | str]) -> None:
    """Set each output named to its scale, and wait until the shell has heard its new height.

    niri answers once it has taken a scale, before the shell hears the screen's new size, and
    the launcher keeps the height it opened at until it closes (Launcher.qml's tall). Opened in
    between, Networks stood as tall as the screen at 1 on the screen at 2: on 19 and 20
    November it rested on all three rows whole, its card 493.5 tall and its top 53 above a
    screen 388 high (the net, at d1061758 and ed3c8438). The height is read once niri says the
    focused output is at its new scale, so it's never the old one read before niri took it.
    """
    for name, scale in scales.items():
        shell.as_account(f"niri msg output {name} scale {scale}")
    deadline = time.monotonic() + launcher_on_niri.WAITING.total_seconds()
    while True:
        focused = json.loads(shell.as_account("niri msg --json focused-output"))
        wanted = scales.get(focused["name"])
        if wanted is None or abs(float(focused["logical"]["scale"]) - float(wanted)) < SCALE_NEAR:
            break
        assert time.monotonic() < deadline, f"niri never took the scale {wanted}: {focused}"
        time.sleep(launcher_on_niri.POLL)
    height = focused["logical"]["height"]
    try:
        # Within a pixel, as Qt and niri may round a fractional scale's height apart.
        shell.launcher_until(lambda shown: abs(shown["card"]["screen"] - height) <= 1)
    except AssertionError as missed:
        failed = f"the shell never heard the screen {height} high at {dict(scales)}"
        raise AssertionError(failed) from missed


@pytest.fixture
def at_twice(shell: Shell) -> Iterator[None]:
    """Hold niri's screen at twice its size, where the card holds three rows' room."""
    outputs = json.loads(shell.as_account("niri msg --json outputs"))
    # Put back however setting it ends, so a wait that times out leaves no case after it at 2.
    try:
        scaled(shell, dict.fromkeys(outputs, 2))
        yield
    finally:
        scaled(shell, {name: output["logical"]["scale"] for name, output in outputs.items()})


# The rows Networks rests on as it's stepped into, each numbered and neither cut.
TOP_ROWS = ("core.nix", "harbour")


@pytest.mark.usefixtures("at_twice")
def test_networks_never_rests_with_a_row_or_a_heading_cut(shell: Shell) -> None:
    """Where Networks' rows overflow the card, each step leaves no row or heading cut.

    A view rests on a row's top and draws only what's wholly in it, a heading only with its
    first row (VKCF3Z, c09d8222), and the gutter numbers only the rows it draws. A row is whole
    when what it draws fits: its ground selected, its words otherwise (VKCF3Z, 889f9a7d), so
    harbour, whose words fit above the foot though its ground wouldn't, is drawn at the top.
    """
    opened(shell)
    shell.down_to("Networks")
    shell.step_in()
    shell.launcher_until(lambda shown: shown["selected"]["name"] == "core.nix")
    shell.normal()
    # Read once it rests on its two rows, so a frame caught as the step in lays them out isn't
    # what's asserted. The three it rested on in the net's failures were a card kept from the
    # screen at 1, which at_twice now waits out (scaled).
    shell.launcher_until(
        lambda shown: (
            shown["restsOnARow"]
            and shown["drawnRows"] == len(shown["gutter"]["numbers"]) == len(TOP_ROWS)
        )
    )
    top = shell.showing()
    # core.nix and harbour, each numbered, neither cut.
    assert top["drawnRows"] == len(top["gutter"]["numbers"]) == len(TOP_ROWS), top
    assert [name for name in top["rows"] if name in TOP_ROWS] == list(TOP_ROWS), top
    assert top["cut"] == [], top
    steps = [("j", "harbour"), ("j", "machines"), ("k", "harbour"), ("k", "core.nix")]
    for key, name in [*steps, ("G", "machines"), ("gg", "core.nix")]:

        def rested(shown: Json, name: str = name) -> bool:
            return bool(
                shown["selected"]["name"] == name
                and shown["cut"] == []
                and shown["restsOnARow"]
                and len(shown["gutter"]["numbers"]) == shown["drawnRows"]
            )

        shell.send_chars(key)
        shell.launcher_until(rested)
        assert rested(shell.showing()), shell.showing()
    shell.close_launcher()


def test_talk_to_a_network_opens_agents_saying_once_it_has_no_one_lead(shell: Shell) -> None:
    """TalkTo and a network's name opens Agents, its foot saying it didn't talk, and why.

    A network has no one lead to say anything to (VKCF3Z, e1eb0ebe), so nothing asks for words.
    Core.nix's network is among the rows the Networks row holds, not the tree's (KP0L6A,
    2fed5a19), and is found there all the same.
    """
    did = followed(shell, "talkTo", "repo.core-nix")
    assert did == {"done": False, "said": "didn't talk: a network has no one lead"}, did
    shell.launcher_until(
        lambda shown: (
            shown["open"]
            and shown["held"] == ["agents"]
            # The foot keeps each phrase whole with no-break spaces.
            and "a network has no one lead" in " ".join(shown["saying"]).replace("\xa0", " ")
        )
    )
    assert not shell.showing()["asking"], shell.showing()
    shell.close_launcher()


def test_network_opens_on_its_row_in_networks_selected_and_on_nothing_none_has(
    shell: Shell,
) -> None:
    """Network and a network's name opens Agents' Networks, that network's row selected.

    A mycelium: link to a network on the realm's map asks it (XUBK2E, 2026-11-15 [2b5c927a]),
    landing where a step into Networks and a walk down to it would. Only the first word names
    one; a name no network has is said as one the launcher hasn't, and opens nothing.
    """

    def on(name: str) -> Callable[[Json], bool]:
        return lambda shown: (
            shown["open"]
            and shown["held"] == ["agents", "Networks"]
            and (shown["selected"] or {}).get("name") == name
        )

    for said, name in (("repo.core-nix", "core.nix"), ("machines go", "machines")):
        did = followed(shell, "network", said)
        assert did["done"], did
        shell.launcher_until(on(name))
        shell.close_launcher()
    did = followed(shell, "network", "repo.gone", quiet=False)
    assert did == {"error": "the launcher has nothing called repo.gone"}, did
    assert not shell.showing()["open"], shell.showing()


def test_machine_opens_on_its_row_in_machines_selected_and_on_nothing_none_has(
    shell: Shell,
) -> None:
    """Machine and a machine's name opens Machines, that machine's row selected.

    The system page's Open in the Launcher asks it (BMDYSH and XUBK2E, 2026-11-17), since a
    page can't act. Only the first word names one; a name no node lists opens Machines, its
    foot saying Machines doesn't list it, and why. Here one node lists only this machine,
    which Machines lists first, so the case proves the call reaches Machines, not which row it
    lands on: modules/quickshell/tests/machines.qml holds which row a link names, over several
    realms' machines, by the one key Machines gives its rows.
    """
    name = here()

    def on(wanted: str) -> Callable[[Json], bool]:
        return lambda shown: (
            shown["open"]
            and shown["held"] == ["machines"]
            and (shown["selected"] or {}).get("name") == wanted
        )

    for said in (name, f"{name} update"):
        did = followed(shell, "machine", said)
        assert did == {"done": True, "said": f"opening Machines \u203a {name}"}, did
        shell.launcher_until(on(name))
        shell.close_launcher()
    # The foot says the miss in Machines' own words, never the name, which a page chose.
    did = followed(shell, "machine", "skiff.gone")
    assert did == {
        "done": False,
        "said": "the link named a machine Machines doesn't list: no node of yours has said it"
        " holds it, or none has answered since the desktop last started",
    }, did
    shell.launcher_until(
        lambda shown: (
            shown["open"]
            and shown["held"] == ["machines"]
            # The foot keeps each phrase whole with no-break spaces.
            and "none has answered since the desktop last started"
            in " ".join(shown["saying"]).replace("\xa0", " ")
            and "skiff" not in " ".join(shown["saying"])
        )
    )
    shell.close_launcher()


def test_a_link_opens_nothing_over_the_launcher_or_the_line_joel_has_up(shell: Shell) -> None:
    """A link while Joel has the launcher or the line up opens nothing: talkTo, network, machine.

    A page picks when its link is followed, so one followed while he's typing would take his
    place and his words (ISXEG8, 2026-11-15 [dd375f5e]): what he's typed and where he is stay
    as they are, and the link is told the launcher's in use. A machine no node lists is asked
    too, since its miss opens Machines and says why, and must not do so over him either.
    """
    in_use = {"done": False, "said": "the launcher is in use, so the link opened nothing"}
    links = (
        ("talkTo", "LISTEN"),
        ("network", "repo.core-nix"),
        ("machine", here()),
        ("machine", "skiff.gone"),
    )
    opened(shell)
    shell.type_in("listening")
    was = shell.showing()
    for verb, said in links:
        assert followed(shell, verb, said, quiet=False) == in_use, (verb, said)
    shown = shell.showing()
    kept = ("open", "held", "typed", "mode", "asking")
    assert [shown[one] for one in kept] == [was[one] for one in kept], (was, shown)
    assert not shown["line"]["up"], shown
    shell.close_launcher()
    shell.as_account(shell.ipc("line"))
    shell.launcher_until(lambda shown: shown["line"]["up"] and shown["line"]["keys"])
    try:
        for verb, said in links:
            assert followed(shell, verb, said, quiet=False) == in_use, (verb, said)
        assert not shell.showing()["open"], shell.showing()
    finally:
        shell.as_account(shell.ipc("line"))
        shell.launcher_until(lambda shown: not shown["line"]["up"])


def test_what_a_link_opens_drops_keys_at_first_and_takes_no_return_before_a_key(
    shell: Shell,
) -> None:
    """What a link opens drops every key for a moment, then takes Return only after a key.

    Joel may be mid-sentence in another window when a page's link opens Talk, so his next
    keys and his Return would go to whoever the link named (ISXEG8, 2026-11-15 [dd375f5e]).
    Keys in its first moment are dropped, and a Return before he types sends nothing, not even
    the talk's kept words. The void's line holds back the same, a case of its own once the void is.
    """
    inbox = STEER / "LISTEN"
    steered = set(inbox.glob("*.json"))

    def sent() -> list[Path]:
        return sorted(set(inbox.glob("*.json")) - steered)

    def talking(shown: Json) -> bool:
        return bool(
            shown["open"]
            and shown["asking"]
            and shown["keys"]
            and shown["held"] == ["agents", "Listening Quest"]
        )

    try:
        # Under load keys can land past the quiet however soon they're sent, so each try asks
        # how many it dropped, as the void's case does.
        for _ in range(LINK_TRIES):
            try:
                dropped, typed = typed_after_a_link(
                    shell, "LISTEN", late=False, field=lambda shown: shown, up=talking
                )
            finally:
                emptied_and_put_away(shell)
            assert typed == LINK_KEYS[dropped:], (dropped, typed)
            if dropped:
                break
        else:
            pytest.fail(f"no key landed inside the link's quiet in {LINK_TRIES} tries")
        # Typed past the quiet, every key is taken, and closing keeps them as the talk's words.
        late = typed_after_a_link(shell, "LISTEN", late=True, field=lambda shown: shown, up=talking)
        assert late == (0, LINK_KEYS), late
        shell.close_launcher()
        # An empty field's Return sends nothing anyway, so the Return that must wait for a key
        # is pressed on those kept words, past the quiet of the link that opened them.
        did = followed(shell, "talkTo", "LISTEN")
        assert did["done"], did
        shell.launcher_until(lambda shown: talking(shown) and shown["typed"] == LINK_KEYS)
        shell.send_key("ret")
        # A Return taken would send at once; this gives it the time to.
        time.sleep(0.3)
        assert shell.showing()["asking"], shell.showing()
        assert sent() == [], sent()
        # A key typed lets Return send, the kept words with it, and only once. The key goes in
        # at the kept caret, which the Escape that closed the launcher stepped back one.
        caret = shell.showing()["caret"]
        words = LINK_KEYS[:caret] + "x" + LINK_KEYS[caret:]
        shell.send_chars("x")
        shell.launcher_until(lambda shown: shown["typed"] == words)
        shell.send_key("ret")
        deadline = time.monotonic() + 10
        while not sent():
            assert time.monotonic() < deadline, "nothing reached the session's inbox"
            time.sleep(0.1)
        assert [json.loads(one.read_text())["text"] for one in sent()] == [words]
        shell.close_launcher()
    finally:
        for one in sent():
            one.unlink()


def test_a_links_new_work_says_whom_it_names_while_joel_checks_its_words(shell: Shell) -> None:
    """New work a link offers opens From a Link, its foot saying whom the page names past a key.

    Joel checks the offered words before his Return starts them, so the foot keeps saying a
    page offered them, and which session it says they're from, while he types, rather than
    giving way at his first key (Y2YBTS, 2026-11-20). The caret starts where he reads from.
    """
    offered = "tidy the rows"

    def told(shown: Json) -> bool:
        # The foot keeps each phrase whole with no-break spaces.
        said = " ".join(shown["saying"]).replace("\xa0", " ")
        return "saying it's from Listening Quest" in said

    # The words and the cell come as mycelium-link hands them on, each after its own prefix.
    asked = shell.ipc("newWork", shlex.quote(f"words:{offered}"), "from:LISTEN")
    did = cast("Json", json.loads(shell.as_account(asked)))
    assert did["done"], did
    assert did["from"] == "LISTEN", did
    time.sleep(LINK_QUIET)
    shell.launcher_until(
        lambda shown: (
            shown["open"]
            and shown["asking"]
            and shown["typed"] == offered
            and shown["caret"] == 0
            and told(shown)
        )
    )
    shell.send_chars("x")
    shell.launcher_until(lambda shown: shown["typed"] == "x" + offered)
    assert told(shell.showing()), shell.showing()
    # Stray keys type in at the start, so their Return, the caret short of the words' end,
    # starts nothing a page wrote (council F40).
    shell.send_key("ret")
    time.sleep(LINK_QUIET)
    shown = shell.showing()
    assert shown["asking"], shown
    assert shown["typed"] == "x" + offered, shown
    # Another word in the foot, as q's in normal, takes the offer's place; a Return held after
    # it says the offer's foot again, so it is never dropped unexplained (council F51, F57).
    shell.send_key("esc")
    shell.send_chars("q")
    shell.launcher_until(lambda shown: shown["open"] and not told(shown))
    shell.send_key("ret")
    shell.launcher_until(lambda shown: shown["asking"] and told(shown))
    assert shell.showing()["typed"] == "x" + offered, shell.showing()
    # Back and forward again, the line comes back with none of the page's words, since
    # neither its foot nor Return's wait comes back with it (council F68).
    shell.send_key("ctrl-o")
    shell.launcher_until(lambda shown: shown["open"] and not shown["asking"])
    shell.send_key("ctrl-i")
    shell.launcher_until(lambda shown: shown["asking"])
    back = shell.showing()
    assert back["typed"] == "", back
    assert not told(back), back
    shell.close_launcher()
    # A line reached later at the same depth is no page's, so its foot says nothing of one
    # (council F37).
    talked = followed(shell, "talkTo", "LISTEN")
    assert talked["done"], talked
    shell.launcher_until(lambda shown: shown["open"] and shown["asking"])
    assert not told(shell.showing()), shell.showing()
    shell.close_launcher()


def typed_after_a_link(
    shell: Shell,
    said: str,
    *,
    late: bool,
    field: Callable[[Json], Json],
    up: Callable[[Json], bool],
) -> tuple[int, str]:
    """Open a talk to `said` by a link and type LINK_KEYS into it, late or at once.

    `field` finds in the showing the field the link opened, and `up` says it's ready for keys.
    It gives how many keys the field says the link's quiet dropped, and what it took of them,
    waiting until the two have every key between them, so a key that lands late is counted.
    """
    did = followed(shell, "talkTo", said, quiet=late)
    assert did["done"], did
    # What's typed already is read from the showing the wait passes on, since asking again
    # would spend the quiet on a round trip.
    was = [""]

    def ready(shown: Json) -> bool:
        was[0] = field(shown)["typed"]
        return up(field(shown))

    def heard(typed: Json) -> int:
        return int(typed["quietDropped"]) + len(typed["typed"]) - len(was[0])

    shell.launcher_until(ready)
    shell.send_chars(LINK_KEYS)
    # At least, so a key heard twice fails on what was typed rather than waiting out the wait.
    shell.launcher_until(lambda shown: heard(field(shown)) >= len(LINK_KEYS))
    typed = field(shell.showing())
    assert heard(typed) == len(LINK_KEYS), (was[0], typed)
    assert typed["typed"].startswith(was[0]), (was[0], typed)
    return typed["quietDropped"], typed["typed"][len(was[0]) :]


def emptied_and_put_away(shell: Shell) -> None:
    """Past the quiet, rub out what's typed in the launcher's field, then close it.

    Rubbed out, so a talk's kept words don't carry what a try typed into the next.
    """
    time.sleep(LINK_QUIET)
    for _ in shell.showing()["typed"]:
        shell.send_key("backspace")
    shell.launcher_until(lambda shown: shown["typed"] == "")
    shell.close_launcher()


def test_recent_holds_the_sessions_closed_this_boot_after_landed_and_before_start(
    shell: Shell,
) -> None:
    """Recent holds a session closed this boot, never one to reopen; Return reopens it."""
    # When it closed, as its row would say it before any of Recent is made, since a run begun
    # before midnight reads it after, saying its day too (XY9H5C, 1b5eff6c).
    before = closed_side("CLOSED")
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
    # By its time alone only on the day it closed: as before the row was made, or as now,
    # should the day turn in between.
    row = shell.showing()["selected"]
    assert row["side"] in {before, closed_side("CLOSED")}, row
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
    the Watch shows only while the watch is down, so spend-served.qml beside this holds its case.
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
        since = shell.mark()
        shell.send_key("ret")
        shell.launcher_until(lambda shown: shown["footer"] == "told Listening Quest")
        shown_after_return(shell, since)
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


def test_a_folder_filling_the_disk_says_its_session_as_agents_says_it(shell: Shell) -> None:
    """A folder's session waiting on Joel's yes says so on its Talk row, in Agents' own words.

    Disk worded how a session stands itself, from its own copy of the agents daemon's rule, so
    one asking for his approval read "needs you" there while Agents said "needs your yes"
    (2026-11-19); a session stands one way, in one order, wherever the launcher shows it.
    """
    folder = STATE / "yeses"
    folder.mkdir(parents=True, exist_ok=True)
    named = f"~/{folder.relative_to(Path.home())}"
    status = Path(os.environ["XDG_RUNTIME_DIR"]) / "disk-guard" / "status.json"
    wants = {"kind": "approval", "since": datetime.now(UTC).isoformat(), "said": "May I?"}
    lay("YESES", "Asking Quest", {"state": "listening", "busy": False, "wants": wants})
    guard_holds(
        status,
        {
            "path": str(folder),
            "name": named,
            "bytes": 12_300_000_000,
            "clears": 0,
            "grew": None,
            "owner": {"kind": "worktree", "name": "yeses"},
            "clearable": False,
            "why": None,
            "kept": None,
            "says": f"{named} holds 11.5 GiB.",
        },
    )
    try:
        shell.answer("disk")
        shell.step_in()
        shell.type_in("filling", selecting="What's Filling It")
        shell.step_in()
        held = f"machine:disk:holder:{folder}"
        shell.launcher_until(lambda shown: shown["things"][0]["is"] == held)
        shell.step_in()
        shell.normal()
        shell.launcher_until(lambda shown: "Talk to Its Session" in shown["rows"])
        shell.down_to("Talk to Its Session")
        side = "Asking Quest \u00b7 needs your yes"
        shell.launcher_until(lambda shown: shown["selected"]["side"] == side)
    finally:
        shell.close_launcher()
        status.unlink()
        (PRESENCE / "yeses.json").unlink()
        # A session asking for Joel's yes, left for the cases after, would stand first in
        # For You and Agents, so it's gone from the node before they begin.
        deadline = time.monotonic() + 10
        while "session:s-yeses" in held_by_node():
            assert time.monotonic() < deadline, "the node still holds the asking session"
            time.sleep(0.1)


@pytest.mark.usefixtures("at_shots_scale")
def test_a_session_waiting_on_joel_is_drawn_in_his_mark_not_the_agents(
    shell: Shell, shoot: Callable[[str], None]
) -> None:
    """A row waiting on Joel wears his mark in the alert's ink, so a narrow row says it unworded.

    The word at a row's right is the first thing cut, so the mark alone has to say it waits
    on him (Joel, 2026-11-20: "no clue how to see from that view that a thing is waiting").
    """
    wants = {"kind": "approval", "since": datetime.now(UTC).isoformat(), "said": "May I?"}
    lay("ASKING", "Waiting Quest", {"state": "listening", "busy": False, "wants": wants})
    lay("IDLING", "Idle Quest", {"state": "listening", "busy": False})
    names = ("Waiting Quest", "Idle Quest")
    try:
        shell.as_account(shell.ipc("collection", "agents"))
        shell.launcher_ready()
        shell.launcher_until(lambda shown: all(name in shown["rows"] for name in names))
        shell.down_to("Waiting Quest")
        assert shell.showing()["selected"]["hue"] == "alert", shell.showing()["selected"]
        laid = json.loads(shell.as_account(shell.ipc("laidOut")))
        # A row's first word is its mark.
        marks = {
            name: row["words"][0]
            for row in laid["rows"]
            for name in names
            if any(word["text"] == name for word in row["words"])
        }
        assert set(marks) == set(names), laid["rows"]
        waiting, idle = marks["Waiting Quest"], marks["Idle Quest"]
        assert waiting["text"] != idle["text"], f"a waiting row wears the agent's mark: {marks}"
        assert waiting["ink"] != idle["ink"], f"a waiting row's mark is a live one's ink: {marks}"
        shoot("launcher-agents-waiting-on-joel")
    finally:
        shell.close_launcher()
        for address in ("asking", "idling"):
            (PRESENCE / f"{address}.json").unlink()
        # A session asking for Joel's yes, left for the cases after, would stand first in
        # For You and Agents, so it's gone from the node before they begin.
        deadline = time.monotonic() + 10
        while {"session:s-asking", "session:s-idling"} & set(held_by_node()):
            assert time.monotonic() < deadline, "the node still holds the sessions"
            time.sleep(0.1)


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
    return shell.rows_made("agents")


def landings_looked(shell: Shell) -> int:
    """Give how many records Agents has looked at since the shell started, its landings."""
    return int(json.loads(shell.as_account(shell.ipc("made")))["looked"]["agents"])


def sessions_sought(shell: Shell) -> int:
    """Give how many sessions Agents has looked through to find its landings' own."""
    return int(json.loads(shell.as_account(shell.ipc("made")))["sought"]["agents"])


def made_since(shell: Shell, before: int) -> list[str]:
    """Give the keys of the rows Agents made after it had made `before`, oldest first."""
    return shell.made_since("agents", before)


def made_lately(shell: Shell) -> str:
    """Say the last rows Agents made, each marked shut or walked as the launcher was then."""
    return shell.made_lately("agents")


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
    # Warming wraps every session's row shut, a new one's too, so it fell inside this count as
    # an arrival's rows (`agent:ARRIVE shut` made from warmNext, 2026-11-13).
    launcher_on_niri.warmed(shell)
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


def test_a_step_into_agents_from_home_makes_its_rows_once(shell: Shell) -> None:
    """A step into Agents by arrow, a session moved since it was last shown, makes its rows once.

    Home makes none of Agents' rows with nothing typed, since Agents says what it holds itself,
    so the step makes them before it's held. Held, Agents found them moved from what it said
    when last wanted, and the launcher made them again before the step's frame: each of Joel's
    steps into Agents by \u2193 and Return waited 16 to 25 ms for its frame, where other steps
    as long waited a median 2 (his journal, 2026-11-19 and 20).
    """
    shell.launcher_closed()
    arrived_shut(shell, lambda: lay("STEPIN", "Stepped Into Quest", {"state": "listening"}))
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    shell.down_to("Agents")
    # Return comes once the arrow's frames are drawn, as Joel's hand comes after his arrow.
    assert shell.held_still(), "the launcher never held still after the arrow"
    since = shell.mark()
    shell.step_in()
    shell.launcher_until(lambda shown: "Stepped Into Quest" in shown["rows"])
    # The step's frame and any made after it are drawn before what they said is read.
    assert shell.held_still(), "the launcher never held still after the step"
    said, _ = shell.said_since(since)
    again = [line for line in said if line.startswith("act moved in agents")]
    assert not again, f"the step into Agents made its rows again: {again}"
    # A precondition, not the claim: Agents did say its rows moved as the step was held, so
    # the case stands where Joel's steps did rather than passing on a change never said.
    assert "agents said only what its step read" in said, f"Agents said no change: {said}"
    # Only the step's own change is let by: one Agents says once it's held still shows.
    lay("AFTERSTEP", "After the Step Quest", {"state": "listening"})
    shell.launcher_until(lambda shown: "After the Step Quest" in shown["rows"])


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


def test_the_first_letter_typed_at_home_makes_no_rows_after_one_arrived_while_shut(
    shell: Shell,
) -> None:
    """A session arriving while the launcher's shut is wrapped as it opens, so typing makes none.

    Its coming moves every row after it, but only in order, so they keep their wraps; its own rows
    are made the frame after home is drawn, not by the letter. Joel's first letter once made 176 to
    185 rows, 11 to 17 ms of letters that took 56 to 81 (RCTRF8, 2026-11-12), and then the arriving
    one's five. That home makes no row unseen while things change is vm-desktop-launcher-keeps-up's
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
    assert made == 0, f"typing at home made {made} of Agents' rows: {made_lately(shell)}"


def test_a_step_out_of_now_makes_none_of_agents_rows_as_home_is_held(shell: Shell) -> None:
    """- out of Now, opened by its chord, makes none of Agents' rows as home is held.

    Joel's - out of Now held 72, 53 and 47 ms, Agents making 6 rows within each, where every
    other step out held 2 to 4 ms with none (his journal, 2026-11-21 13:13). Home names what
    Agents holds from the daemon's rows, never from rows made of them, so a row made as it's held
    is made for nobody (Y2YBTS, 2026-11-21 [8c306245]). The step's line counts them, as
    stepHolding says.
    """
    shell.launcher_closed()
    # Home read first, as Joel's had been, so what's counted is the step's alone, whichever
    # cases ran before this one.
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    made_settled(shell)
    shell.close_launcher()
    shell.as_account(shell.ipc("collection", "on"))
    shell.launcher_ready()
    shell.launcher_until(lambda shown: shown["held"] == ["now"] and "For You" in shown["rows"])
    made_settled(shell)
    before = made_by_agents(shell)
    shell.normal()
    shell.send_chars("-")
    shell.launcher_until(lambda shown: not shown["held"])
    holding = json.loads(shell.as_account(shell.ipc("laidOut")))["stepHolding"]
    made_settled(shell)
    after = made_by_agents(shell) - before
    lately = made_lately(shell)
    shell.close_launcher()
    assert holding is not None, "- out of Now counted nothing as it held"
    made = holding["rows"].get("agents", 0)
    assert made == 0, f"- out of Now made {made} of Agents' rows as it held: {lately}"
    # Nor in the turns after, where a remake put off from the step's hold would land.
    assert after == 0, f"- out of Now made {after} of Agents' rows after it held: {lately}"


def test_sessions_to_reopen_in_for_you_follows_what_is_to_reopen_as_now_is_held(
    shell: Shell,
) -> None:
    """For You's Sessions to Reopen changes as what's to reopen does, while Now is held.

    Now is the source named "on" (home.nix), and Agents reads Sessions to Reopen there. Asked
    for as "now", it was never found, and the row For You drew stood as it was last made at
    home or in Agents.
    """
    in_for_you(shell)
    shell.launcher_until(lambda shown: "Sessions to Reopen" in shown["rows"])
    shell.down_to("Sessions to Reopen")
    before = shell.showing()["selected"]["side"]
    kept = RESTORE.read_text()
    said = json.loads(kept)
    said["sessions"].append(
        {
            "id": "01b4-tide",
            "address": "TIDERN",
            "title": "Tide Return",
            "cwd": str(KEPT),
            "lostAt": 1,
        }
    )
    try:
        RESTORE.write_text(json.dumps(said))
        shell.launcher_until(
            lambda shown: (
                shown["held"] == ["now", "For You"]
                and (shown["selected"] or {}).get("name") == "Sessions to Reopen"
                and (shown["selected"] or {}).get("side") != before
            )
        )
    finally:
        RESTORE.write_text(kept)
        shell.close_launcher()


def test_a_session_landing_while_now_is_held_is_counted_in_landed_once_stepped_out(
    shell: Shell,
) -> None:
    """A landing heard while Now is held makes no Landed there, and - out of Now makes it.

    Agents keeps each of its own rows while what it's made from is as it was, and Now reads
    only Sessions to Reopen of them, so a part left out of what Landed is kept by would leave
    its count as it stood before the landing, for as long as nothing else it's made from
    changed.
    """
    # Not by opened(), whose first rows the sessions the cases before this one laid move.
    shell.as_account(shell.ipc("collection", "agents"))
    shell.launcher_ready()
    shell.launcher_until(lambda shown: shown["held"] == ["agents"] and "Landed" in shown["rows"])
    shell.down_to("Landed")
    before = shell.showing()["selected"]["side"]
    shell.close_launcher()
    shell.as_account(shell.ipc("collection", "on"))
    shell.launcher_ready()
    shell.launcher_until(lambda shown: shown["held"] == ["now"])
    landing = {"at": datetime.now(UTC).isoformat(), "ref": "core.nix#6037", "what": "Counted"}
    try:
        made = made_by_agents(shell)
        # Heard whole as one arriving shut is, though Now is held.
        arrived_shut(
            shell,
            lambda: lay("NOWLND", "Landed in Now", {"state": "listening", "landings": [landing]}),
        )
        in_now = made_since(shell, made)
        assert "agents:landed" not in in_now, f"Now made Landed as the landing came: {in_now}"
        made = made_by_agents(shell)
        shell.normal()
        shell.send_chars("-")
        shell.launcher_until(lambda shown: not shown["held"])
        made_settled(shell)
        at_home = made_since(shell, made)
        assert "agents:landed" in at_home, f"- out of Now made no Landed: {at_home}"
        shell.close_launcher()
        shell.as_account(shell.ipc("collection", "agents"))
        shell.launcher_ready()
        shell.launcher_until(
            lambda shown: shown["held"] == ["agents"] and "Landed" in shown["rows"]
        )
        shell.down_to("Landed")
        shell.launcher_until(lambda shown: (shown["selected"] or {}).get("side") != before)
    finally:
        shell.close_launcher()
        (PRESENCE / "nowlnd.json").unlink(missing_ok=True)


def made_settled(shell: Shell) -> None:
    """Wait until Agents has made no row for a while, the opening's work done."""
    shell.made_settled("agents")


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


@pytest.fixture
def at_shots_scale(shell: Shell) -> Iterator[None]:
    """Hold niri's screen at SHOTS_SCALE through a case pictured there, then as it was."""
    scale = os.environ.get("SHOTS_SCALE", "")
    if not scale:
        yield
        return
    outputs = json.loads(shell.as_account("niri msg --json outputs"))
    try:
        scaled(shell, dict.fromkeys(outputs, scale))
        yield
    finally:
        scaled(shell, {name: output["logical"]["scale"] for name, output in outputs.items()})


@pytest.fixture(scope="module")
def said_over(shell: Shell) -> Iterator[str]:
    """Give the title of a window Super+S's line is said over, with the void standing at the top."""
    if os.environ.get("SHOTS_SCALE"):
        outputs = json.loads(shell.as_account("niri msg --json outputs"))
        scaled(shell, dict.fromkeys(outputs, os.environ["SHOTS_SCALE"]))
    lay("VOID", "Void Quest", {"state": "listening", "standsAtTop": {"name": "The Void"}})
    # Only that Agents has heard of the void, whatever else earlier cases laid beside it.
    shell.as_account(shell.ipc("collection", "agents"))
    shell.launcher_ready()
    shell.launcher_until(lambda shown: "The Void" in shown["rows"])
    shell.close_launcher()
    # A pi window's title as the windows daemon names it, without pi's letter leading it,
    # which its entries.rs drops and proves; this sandbox runs no windows daemon, so the
    # shell reads niri's title as it is.
    title = "Tune the launcher so it fits the way he works"
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
    line is the launcher's frame, centred on the screen, over his window: what's typed
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
    # the shell; the foot names who hears it, in words rather than a stray name (VKCF3Z,
    # 2026-11-13 [00c78048]).
    shell.launcher_until(
        lambda shown: [row["text"] for row in shown["line"]["attached"]] == [said_over]
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
    # The launcher's frame, its top and its foot, so the two are one box in one place (Joel,
    # 2026-11-14 09:44: "i want them to feel like more of part of the same").
    framed = [shown["line"]["top"], shown["line"]["bottom"]]
    assert framed == [shown["card"]["top"], shown["card"]["foot"]], (shown["line"], shown["card"])
    assert_centred(shown["line"])
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
    # Typing again where it left off, after the last letter, not on it as normal rested.
    assert shell.showing()["line"]["caret"] == len(words), shell.showing()["line"]
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


def assert_centred(line: dict[str, Any]) -> None:
    """Fail unless the line stands as far from the screen's foot as from its top, to a pixel.

    Joel, 2026-11-14 09:44: "does not have co-equal top and bottom margins"; VKCF3Z ruled it
    centred to the device pixel, an odd one going below (4643352a).
    """
    scale = float(os.environ.get("SHOTS_SCALE", "1"))
    above = round(line["top"] * scale)
    below = round((line["screen"] - line["bottom"]) * scale)
    assert below - above in {0, 1}, (above, below, line)


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
    shown = said_over
    attach(shell, "*", [("window", shown), ("selection", "the cut title")])
    attach(
        shell,
        "+",
        [
            ("window", shown),
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


def complete_take(shell: Shell, shoot: Callable[[str], None]) -> None:
    """Type : in Super+S's line, see its commands listed, and complete :take with Tab.

    Its : lists what it runs, under what's typed, as every other command line does, and Tab
    completes what's typed (Joel, 2026-11-14 16:25: "the void lacks any Ex command completion
    like other windows do"), all of it over the foot, and nothing chosen before Tab.
    """
    menu = ["take", "q[uit]"]
    shell.send_chars(":")
    shell.launcher_until(
        lambda shown: shown["line"]["menu"] == menu and shown["line"]["menuDrawn"] == len(menu)
    )
    listed = shell.showing()["line"]
    assert listed["menuEnds"] < listed["footAt"], listed
    assert listed["menuChosen"] == -1, listed
    shoot("commands")
    shell.send_chars("t")
    shell.launcher_until(lambda shown: shown["line"]["menu"] == menu[:1])
    shell.send_key("tab")
    shell.launcher_until(lambda shown: shown["line"]["command"] == "take")
    taken = shell.showing()["line"]
    assert taken["menuChosen"] == 0, taken
    shoot("completed")


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
        lambda shown: (
            shown["line"]["talk"] == "answered"
            and [":take", "take it back"] in shown["line"]["footKeys"]
        )
    )
    assert shell.showing()["line"]["standing"].startswith("\u2192 Delta\n"), shell.showing()
    shoot("answered")
    # :take in normal tells the void to undo it, and its undone reads taken back, in the line.
    shell.send_key("esc")
    shell.launcher_until(lambda shown: shown["line"]["mode"] == "normal")
    complete_take(shell, shoot)
    shell.send_key("ret")
    shell.launcher_until(lambda shown: shown["line"]["talk"] == "taking")
    replied(steered(lambda text: text == "undo 04:58"), "undone")
    shell.launcher_until(lambda shown: shown["line"]["standing"] == "taken back")
    assert [":take", "take it back"] not in shell.showing()["line"]["footKeys"], shell.showing()
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
        lambda shown: [row["text"] for row in shown["line"]["attached"]] == [said_over]
    )
    # Its talks kept: the case before's newest answer.
    standing = shell.showing()["line"]["standing"]
    assert standing == "The cat's clips, and the void.", shell.showing()["line"]
    shell.as_account(shell.ipc("line"))
    shell.launcher_until(lambda shown: not shown["line"]["up"])


@pytest.mark.usefixtures("said_over")
def test_talk_to_the_voids_address_opens_super_s_line_as_its_talk_does(shell: Shell) -> None:
    """TalkTo and the void's address opens Super+S's line, the void's one place, as Talk does."""
    did = followed(shell, "talkTo", "void")
    assert did["done"], did
    shell.launcher_until(
        lambda shown: not shown["open"] and shown["line"]["up"] and shown["line"]["keys"]
    )
    shell.as_account(shell.ipc("line"))
    shell.launcher_until(lambda shown: not shown["line"]["up"])


def test_a_link_to_the_void_opens_nothing_over_the_launcher_and_its_line_drops_keys_at_first(
    shell: Shell,
) -> None:
    """A link to the void holds back as one to a session does, on Super+S's line.

    With the launcher up it opens nothing, and the line it opens from closed drops what's
    typed in its first moment (ISXEG8, 2026-11-15 [dd375f5e]).
    """
    # Agents holds the void by now, so its rows aren't those opened waits on.
    shell.as_account(shell.ipc("collection", "agents"))
    shell.launcher_ready()
    shell.launcher_until(lambda shown: shown["held"] == ["agents"])
    did = followed(shell, "talkTo", "void", quiet=False)
    assert did == {"done": False, "said": "the launcher is in use, so the link opened nothing"}
    assert not shell.showing()["line"]["up"], shell.showing()
    shell.close_launcher()
    # Typed past the quiet, every key is taken.
    assert typed_into_the_void(shell, late=True) == (0, LINK_KEYS)
    # Under load keys can land past the quiet however soon they're sent, so each try asks how
    # many it dropped: those are the first, and the rest are typed. Tries run under the same
    # load, so there are enough of them that one lands inside the 700 ms on a busy builder.
    for _ in range(LINK_TRIES):
        dropped, typed = typed_into_the_void(shell, late=False)
        assert typed == LINK_KEYS[dropped:], (dropped, typed)
        if dropped:
            return
    pytest.fail(f"no key landed inside the link's quiet in {LINK_TRIES} tries")


def typed_into_the_void(shell: Shell, *, late: bool) -> tuple[int, str]:
    """Open the void's line by a link and type LINK_KEYS, late or at once, and empty it again.

    It gives what typed_after_a_link does, of the line.
    """
    try:
        return typed_after_a_link(
            shell,
            "void",
            late=late,
            field=lambda shown: shown["line"],
            up=lambda line: bool(line["up"] and line["keys"]),
        )
    finally:
        try:
            # Past the quiet, so the key that empties the line isn't dropped as well, and the
            # cases after this one find it as they would.
            time.sleep(LINK_QUIET)
            shell.send_key("ctrl-u")
            shell.launcher_until(lambda shown: shown["line"]["typed"] == "")
        finally:
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


def test_super_s_line_fits_the_screen_and_scrolls_a_line_at_a_time(
    shell: Shell, shoot: Callable[[str], None]
) -> None:
    """Super+S's line stands inside the screen however many lines show, and j and k pick.

    In normal, j and k pick a line, scrolled into view, and Return opens it as its thread
    (2026-11-13 17:48, [851e94b2]).


    Joel's line showed so many lines with answers he hadn't seen that it ran off the bottom
    of the screen, with no way to reach the rest (2026-11-13 23:39, [ff6c6197]). They stand
    no taller than the screen has room for, and j and k in normal go down and up them.
    """
    replies = STEER / "VOID" / "replies"
    # The Super+S cases before it make these, but a missing folder isn't what this one checks.
    replies.mkdir(parents=True, exist_ok=True)
    (STATE / "void").mkdir(parents=True, exist_ok=True)
    now = datetime.now().astimezone()
    events = []
    said = [f"line {back}" for back in range(20)]
    for back, words in enumerate(said):
        at = now - timedelta(minutes=40 + back)
        clock = at.strftime("%H:%M")
        lines = [
            {"stage": "delivered", "at": at.isoformat(), "session": "s-void", "from": "joel",
             "text": words},
            {"stage": "replied", "at": at.isoformat(),
             "text": f"Your orchestrator has it \u00b7 undo {clock}"},
        ]  # fmt: skip
        laid = replies / f"talk-{int(at.timestamp() * 1000)}.jsonl.tmp"
        laid.write_text("".join(json.dumps(line) + "\n" for line in lines))
        laid.rename(laid.with_suffix(""))
        key = f"{at.strftime('%Y-%m-%d')} {clock}"
        events += [
            {"line": key, "msg": f"r{back}", "who": "your orchestrator", "event": "routed"},
            {"line": key, "msg": f"a{back}", "who": "Delta", "event": "answered",
             "at": clock, "text": "Done, merged, and live after the next switch. " * 4},
        ]  # fmt: skip
    with (STATE / "void" / "answers.jsonl").open("a") as file:
        file.writelines(json.dumps(event) + "\n" for event in events)
    line = shell.ipc("line")
    shell.as_account(line)
    shell.launcher_until(
        lambda shown: (
            shown["line"]["keys"] and {talk["line"] for talk in shown["line"]["talks"]} >= set(said)
        )
    )
    # However many lines it holds, it stands in the launcher's frame, its foot the launcher's.
    shown = shell.showing()
    stands = shown["line"]
    assert [stands["top"], stands["bottom"]] == [shown["card"]["top"], shown["card"]["foot"]], (
        stands,
        shown["card"],
    )
    assert_centred(stands)
    # A key of another keymap first, since the line's first key was read by the last keymap
    # and lost, its Escape at 1.5x leaving the line in insert. F24 does nothing here.
    shell.send_key("F24")
    shell.send_key("esc")
    shell.launcher_until(lambda shown: shown["line"]["mode"] == "normal")
    # j picks the newest, and a line picked far down is scrolled into view.
    shell.send_key("j")
    shell.launcher_until(lambda shown: shown["line"]["picked"] == 0)
    # Past the lines the screen has room for at either scale: at 1.5x the ninth still fits.
    far = 15
    shell.send_chars(f"{far}j")
    shell.launcher_until(
        lambda shown: shown["line"]["picked"] == far and shown["line"]["scrolled"] > 0
    )
    # Return opens it as its thread, ready to type; after Escape, za opens its nerdy bits,
    # and - comes back to it picked.
    picked = shell.showing()["line"]["talks"][far]["line"]
    shell.send_key("ret")
    shell.launcher_until(
        lambda shown: (
            (shown["line"]["thread"] or {}).get("line") == picked
            and shown["line"]["mode"] == "insert"
        )
    )
    shell.send_key("esc")
    shell.launcher_until(lambda shown: shown["line"]["mode"] == "normal")
    shell.send_chars("za")
    # Pictured once niri has given it the fold's size, so a surface still at the last is not.
    shell.launcher_until(lambda shown: shown["line"]["nerdy"] and shown["line"]["sized"])
    shoot("thread")
    shell.send_chars("-")
    shell.launcher_until(
        lambda shown: shown["line"]["thread"] is None and shown["line"]["picked"] == far
    )
    shell.send_chars(f"{far + 1}k")
    shell.launcher_until(
        lambda shown: shown["line"]["picked"] == 0 and shown["line"]["scrolled"] == 0
    )
    # Pictured once the keys have come back round, so a surface still taking its size is not.
    shoot("many")
    shell.as_account(line)
    shell.launcher_until(lambda shown: not shown["line"]["up"])


@pytest.mark.usefixtures("said_over")
def test_super_s_line_sends_what_is_typed_in_an_open_thread_as_its_reply(shell: Shell) -> None:
    """What's typed in an open thread goes to the void as its reply, shown in it at once.

    Joel wanted to "continue those threads of conversation" (2026-11-13 17:48, [851e94b2]).
    A reply's first line names its thread by its key, which the void sends on to whoever has
    it and writes as `replied` (QCXZEB [5eaea432], 8GYNJ6 [0543317b]).

    Picked by Tab while typing, Return opens the line with no Escape first, the foot saying
    each key as it's wanted, and the reply stands as sent under it (Joel, 2026-11-21 10:23
    and 10:27: "I have no clue how to step into a thread", "enter does nothing").
    """
    inbox = STEER / "VOID"
    replies = inbox / "replies"
    replies.mkdir(parents=True, exist_ok=True)
    (STATE / "void").mkdir(parents=True, exist_ok=True)
    at = datetime.now().astimezone()
    clock = at.strftime("%H:%M")
    key = f"{at.strftime('%Y-%m-%d')} {clock}"
    asked = "is the thread fix in?"

    def lay(talk: str, said: str) -> None:
        """Lay what the void kept of a talk, as its inbox's replies hold it."""
        lines = [
            {"stage": "delivered", "at": at.isoformat(), "session": "s-void", "from": "joel",
             "text": said},
            {"stage": "replied", "at": at.isoformat(),
             "text": f"Your orchestrator has it \u00b7 undo {clock}"},
        ]  # fmt: skip
        laid = replies / f"{talk}.jsonl.tmp"
        laid.write_text("".join(json.dumps(line) + "\n" for line in lines))
        laid.rename(laid.with_suffix(""))

    def steered(words: str) -> dict[str, Any]:
        """Give the steer these words went to the void in, once it's in the inbox."""
        deadline = time.monotonic() + 10
        while not (
            sent := [
                steer
                for one in set(inbox.glob("*.json")) - before
                if str((steer := json.loads(one.read_text())).get("text", "")).endswith(words)
            ]
        ):
            assert time.monotonic() < deadline, "the void's inbox never had the reply"
            time.sleep(0.1)
        return cast("dict[str, Any]", sent[0])

    lay(f"talk-{int(at.timestamp() * 1000)}", asked)
    events = [
        {"msg": "t1", "owner": "WCJQSG", "who": "Your orchestrator", "event": "routed"},
        {"msg": "t2", "re": "t1", "who": "Your orchestrator", "event": "answered",
         "text": "Not yet; tonight."},
    ]  # fmt: skip
    with (STATE / "void" / "answers.jsonl").open("a") as file:
        file.writelines(json.dumps({"line": key, "at": clock} | one) + "\n" for one in events)
    before = set(inbox.glob("*.json"))
    line = shell.ipc("line")
    shell.as_account(line)
    shell.launcher_until(
        lambda shown: (
            shown["line"]["keys"] and asked in [one["line"] for one in shown["line"]["talks"]]
        )
    )
    # Typing, the foot says Tab picks a line; once one is picked, that Return opens it.
    shell.launcher_until(
        lambda shown: (
            shown["line"]["mode"] == "insert"
            and shown["line"]["footKeys"][:1] == [["Tab", "picks a line"]]
        )
    )
    at_line = [one["line"] for one in shell.showing()["line"]["talks"]].index(asked)
    # A key of another keymap first, as the cases before do, so the first Tab isn't lost.
    shell.send_key("F24")
    pick_by_tab(shell, at_line)
    typing_lets_the_pick_go(shell, at_line)
    shell.send_key("ret")
    # It opens ready to type, so the reply's first letter is its first letter, no i before.
    shell.launcher_until(
        lambda shown: (
            (shown["line"]["thread"] or {}).get("key") == key and shown["line"]["mode"] == "insert"
        )
    )
    words = "and the reply?"
    shell.send_chars(words)
    shell.launcher_until(lambda shown: shown["line"]["typed"] == words)
    shell.send_key("ret")
    # It shows in the thread as his at once, before the void has written it, sent under it,
    # and the foot says how to come back.
    shell.launcher_until(
        lambda shown: (
            [[step["text"], step["mine"]] for step in (shown["line"]["thread"] or {})["steps"]][-2:]
            == [[words, True], ["sent", False]]
            and shown["line"]["footKeys"][:1] == [["Esc -", "back to the lines"]]
        )
    )
    sent = steered(words)
    assert sent["text"] == f"reply {key}\n{words}", sent
    # Said now, so the void reads it into the run under way rather than after it.
    assert sent.get("now") is True, sent
    comes_back_picked(shell, asked)
    shell.as_account(line)
    shell.launcher_until(lambda shown: not shown["line"]["up"])


def pick_by_tab(shell: Shell, at_line: int) -> None:
    """Pick Super+S's line at `at_line` by Tab, typing, till the foot says Return opens it."""
    for _ in range(at_line + 1):
        shell.send_key("tab")
    shell.launcher_until(
        lambda shown: (
            shown["line"]["picked"] == at_line
            and shown["line"]["mode"] == "insert"
            and shown["line"]["footKeys"][:1] == [["\u21b5", "opens it"]]
        )
    )


def typing_lets_the_pick_go(shell: Shell, at_line: int) -> None:
    """Type a letter, which lets a pick go so Return would send it, then pick again."""
    shell.send_chars("x")
    shell.launcher_until(
        lambda shown: (
            shown["line"]["picked"] == -1
            and shown["line"]["footKeys"][:1] == [["Tab", "picks a line"]]
        )
    )
    shell.send_key("backspace")
    shell.launcher_until(lambda shown: shown["line"]["typed"] == "")
    pick_by_tab(shell, at_line)


def comes_back_picked(shell: Shell, asked: str) -> None:
    """Come back from a thread by Esc -, its line still picked, a reply's clearing aside.

    The line is told by its words, not where it stood, since the list may move it while its
    thread is open.
    """

    def picked(shown: dict[str, Any]) -> str:
        at, talks = shown["line"]["picked"], shown["line"]["talks"]
        return talks[at]["line"] if 0 <= at < len(talks) else ""

    shell.send_key("esc")
    shell.launcher_until(lambda shown: shown["line"]["mode"] == "normal")
    shell.send_chars("-")
    shell.launcher_until(
        lambda shown: (
            shown["line"]["thread"] is None
            and picked(shown) == asked
            and shown["line"]["footKeys"][:1] == [["\u21b5", "opens it"]]
        )
    )


@pytest.mark.usefixtures("said_over")
def test_super_s_line_shows_a_reply_the_void_wrote_once_in_its_thread_and_not_as_a_line(
    shell: Shell, shoot: Callable[[str], None]
) -> None:
    """A reply the void kept and wrote as `replied` shows once, in its thread, as his.

    The void keeps a reply as a talk of its own, whose words name its thread first, and
    writes it to the thread's key as `replied` (QCXZEB [5eaea432], 8GYNJ6 [0543317b]); so
    the line shows it in its thread, never as a line of its own beside it.
    """
    replies = STEER / "VOID" / "replies"
    replies.mkdir(parents=True, exist_ok=True)
    (STATE / "void").mkdir(parents=True, exist_ok=True)
    # Minutes before the case before it, so the two threads keep keys apart.
    at = datetime.now().astimezone() - timedelta(minutes=3)
    clock = at.strftime("%H:%M")
    key = f"{at.strftime('%Y-%m-%d')} {clock}"
    asked = "is the reply fix in?"
    words = "and after the switch?"

    def lay(since: datetime, said: str) -> None:
        """Lay what the void kept of a talk, as its inbox's replies hold it."""
        lines = [
            {"stage": "delivered", "at": since.isoformat(), "session": "s-void",
             "from": "joel", "text": said},
            {"stage": "replied", "at": since.isoformat(),
             "text": f"Your orchestrator has it \u00b7 undo {since.strftime('%H:%M')}"},
        ]  # fmt: skip
        laid = replies / f"talk-{int(since.timestamp() * 1000)}.jsonl.tmp"
        laid.write_text("".join(json.dumps(line) + "\n" for line in lines))
        laid.rename(laid.with_suffix(""))

    def mine(shown: dict[str, Any]) -> list[list[Any]]:
        """Give the open thread's steps in these words, whether his, and their fold."""
        steps = (shown["line"]["thread"] or {}).get("steps", [])
        return [[step["text"], step["mine"], step["nerd"]] for step in steps
                if step["text"] == words]  # fmt: skip

    lay(at, asked)
    lay(at + timedelta(seconds=20), f"reply {key}\n{words}")
    events = [
        {"msg": "w1", "owner": "WCJQSG", "who": "Your orchestrator", "event": "routed"},
        {"msg": "w2", "re": "w1", "who": "Your orchestrator", "event": "answered",
         "text": "Tonight, with the rest."},
        {"msg": "w3", "re": "w2", "owner": "WCJQSG", "who": "Your orchestrator",
         "event": "replied", "text": words},
    ]  # fmt: skip
    with (STATE / "void" / "answers.jsonl").open("a") as file:
        file.writelines(json.dumps({"line": key, "at": clock} | one) + "\n" for one in events)
    line = shell.ipc("line")
    shell.as_account(line)
    shell.launcher_until(
        lambda shown: (
            shown["line"]["keys"] and asked in [one["line"] for one in shown["line"]["talks"]]
        )
    )
    talks = [one["line"] for one in shell.showing()["line"]["talks"]]
    assert not [one for one in talks if one.startswith("reply ") or one == words], talks
    # A key of another keymap first, as the cases before do, so Escape isn't lost.
    shell.send_key("F24")
    shell.send_key("esc")
    shell.launcher_until(lambda shown: shown["line"]["mode"] == "normal")
    shell.send_chars(f"{talks.index(asked) + 1}j")
    shell.launcher_until(lambda shown: shown["line"]["picked"] == talks.index(asked))
    shell.send_key("ret")
    fold = "replied \u00b7 msg w3 \u00b7 re w2 \u00b7 to WCJQSG"
    # Written, it waits on whoever has the thread, by name and never the owner's address.
    shell.launcher_until(
        lambda shown: (
            mine(shown) == [[words, True, fold]]
            and [step["text"] for step in shown["line"]["thread"]["steps"]][-1:]
            == ["waiting on your orchestrator"]
        )
    )
    shell.send_key("esc")
    shell.launcher_until(lambda shown: shown["line"]["mode"] == "normal")
    shell.send_chars("za")
    shell.launcher_until(lambda shown: shown["line"]["nerdy"] and shown["line"]["sized"])
    shoot("replied")
    shell.as_account(line)
    shell.launcher_until(lambda shown: not shown["line"]["up"])


# How long before Return a session's work turns, in seconds, is how long the nudge's line took
# from its file to what the launcher shows that very turn, the same way through the node and
# Now, and AHEAD more. A fixed lead couldn't follow the builders: a turn took about 0.85 s to the
# shell on 18 November and under 0.3 s on 21 November, when every turn of one run was heard
# before Return, and the same lead was heard 14 to 345 ms after Return's work from one turn to
# the next (2026-11-19). Return's own key takes a while to be worked, so AHEAD starts half of
# Pacing.keyed's 50 ms early and is steered: each turn heard after Return moves it earlier by as
# much as it was late, and each heard before moves it later by AHEAD_BACK, twice as far for
# each such turn in a row, within AHEAD_MOST, so the turns close in on the race on a builder
# fast or slow. The way's time counts the poll that sees it too, which only steering earns
# back, so the steps back grow rather than leave a slow poll beyond their reach. The launcher
# says a turn only within a second of Return's work, so one heard a second or more after reads
# as one heard before; AHEAD_LEAST keeps the lead within half a second of the way, from which a
# turn would need half a second longer than the nudge took to read so, past the 331 ms the same
# lead's turns spread over on 19 November.
FIRST_AHEAD = -0.025
AHEAD_BACK = 0.04
AHEAD_LEAST = -0.5
AHEAD_MOST = 1.0


def steered(ahead: float, back: float, late: int | None) -> tuple[float, float]:
    """AHEAD and the next step back after a turn heard `late` ms after Return, or None."""
    if late is None:
        ahead, back = ahead - back, back * 2
    else:
        ahead, back = ahead + (late - KEYED_WITHIN / 2) / 1000, AHEAD_BACK
    return min(max(ahead, AHEAD_LEAST), AHEAD_MOST), back


# How many turns the case lays at most, and how many heard beside Return are enough.
TURNS = 36
ENOUGH_BESIDE = 3
# How long the shell says nothing more of a turn before the turn is taken as told.
TOLD_QUIET = 1.0
# Within how long of a key's work the launcher owns a change as the key's, Launcher.qml's
# keyedWithin, and how it says one it heard after a key but didn't draw as the key's act: Now's,
# or the notices' a turn updates a session's notice in, which Now no longer says when it moves
# nothing For You shows (2026-11-19). Since the fix, the notices' path says nothing as it moves
# nothing, so Now's own, heard from the same turn, is what shows the turn landed beside Return;
# that the case reaches the notices' path there is shown by its failing with Now's fix taken out.
KEYED_WITHIN = 50
HEARD_AFTER = re.compile(r"heard (?:on|notices) (\d+) ms after a key's work")


def test_a_session_turning_beside_return_into_for_you_draws_nothing(shell: Shell) -> None:
    """A session's work turning just after Return into For You, moving none of it, draws nothing.

    Joel's journal said "what Return changed in on took 76 ms to its frame: its work took 13 ms;
    work told it 63 ms after the key" on 15 November, after 73 of 81 steps into For You. Now last
    said what it shows with nothing held below it, as its home rows moved; the first session line
    after Return asked again with For You held, read its rows too, and so said Now changed though
    no row had. The launcher made the very rows again, drew them and timed them as Return's act.
    Now comparing what For You shows is tests/now-moment.qml's; this is the path, the realm's
    node to Now to the launcher, a line laid each turn as long before Return as that way takes
    until one lands beside it.
    """
    # A session asking Joel a question is a row in For You, and its work turns each time.
    asking = {
        "state": "listening",
        "wants": {"kind": "question", "since": datetime.now(UTC).isoformat(), "said": "Which?"},
    }
    lay("TICKER", "Ticking Quest", {**asking, "busy": False})
    # What the shell said of each lead's turn, and the leads whose turn it heard within
    # KEYED_WITHIN of Return's work and rightly didn't draw as its act, so the case is seen to
    # reach the race it's about rather than pass with every turn told early or late.
    told: dict[str, list[str]] = {}
    beside: list[float] = []
    try:
        shell.as_account(shell.ipc("collection", "on"))
        shell.launcher_ready()
        shell.launcher_until(lambda shown: shown["held"] == ["now"] and "For You" in shown["rows"])
        shell.normal()
        ahead = FIRST_AHEAD
        back = AHEAD_BACK
        for turn in range(TURNS):
            # Another session asking, or done asking, moves For You's own row, so Now says what
            # it shows with nothing held below it, as Joel's home rows had before each step in.
            ask = asking if turn % 2 == 0 else {"state": "busy", "busy": True}
            was = shell.showing()["seen"]["sides"][0]
            laid = time.monotonic()
            lay("NUDGER", "Nudging Quest", ask)
            shell.launcher_until(partial(moved, was))
            way = time.monotonic() - laid
            # No lead is laid after Return, so one AHEAD would put after it is laid as none.
            lead = max(0.0, way + ahead)
            key = f"turn {turn}, {lead:.3f} s before ({way:.3f} s way, {ahead:+.3f} s ahead)"
            shell.down_to("For You")
            step = {
                "kind": "tool",
                "name": "bash",
                "detail": f"turn {turn}",
                "since": datetime.now(UTC).isoformat(),
            }
            lay(
                "TICKER",
                "Ticking Quest",
                {**asking, "busy": turn % 2 == 0, "step": f"bash turn {turn}", "stepDetail": step},
            )
            time.sleep(lead)
            since = shell.mark()
            shell.send_key("ret")
            shell.launcher_until(lambda shown: shown["held"] == ["now", "For You"])
            said = told_quietly(shell, since)
            shown = [report for report in said if report.startswith("act shown in on")]
            # What the shell said of each act and change it heard, the act's "act moved in on"
            # with it, so a failure says what was drawn: the one on 16 November said only that
            # something was. The launcher's whole state is left out.
            heard = [report for report in said if report.startswith(("act ", "heard "))]
            assert not shown, f"{key} was drawn as Return's act: {heard}"
            told[key] = heard
            after = [int(found[1]) for report in said if (found := HEARD_AFTER.match(report))]
            if any(ago < KEYED_WITHIN for ago in after):
                beside.append(lead)
            shell.send_chars("-")
            shell.launcher_until(lambda shown: shown["held"] == ["now"])
            if len(beside) >= ENOUGH_BESIDE:
                break
            # Only a turn that moves a row is heard, every other one, so only those steer. Each
            # steers from no further back than its own way: below it, steps would move AHEAD
            # but not the lead, spending turns climbing back. Floored on a turn that doesn't
            # steer, a way shorter than most would put AHEAD later with nothing to bring it back.
            if turn % 2 == 0:
                ahead, back = steered(max(ahead, -way), back, min(after, default=None))
        print(f"turns heard within {KEYED_WITHIN} ms of Return, by their leads: {beside}")
        assert beside, (
            f"no turn was heard within {KEYED_WITHIN} ms of Return (one heard before it, or a "
            f"second or more after, says nothing): {told}"
        )
    finally:
        shell.close_launcher()
        lay("TICKER", "Ticking Quest", {"state": "idle"}, closed=True)
        lay("NUDGER", "Nudging Quest", {"state": "idle"}, closed=True)


def moved(was: str, shown: Json) -> bool:
    """Whether For You's own row says otherwise than it did, `was`."""
    return bool(shown["seen"]["sides"][0] != was)


def told_quietly(shell: Shell, since: int) -> list[str]:
    """Give what the shell said since the mark, once it says nothing more for a while."""
    said, mark = shell.said_since(since)
    quiet = time.monotonic()
    while time.monotonic() - quiet < TOLD_QUIET:
        time.sleep(0.1)
        more, mark = shell.said_since(mark)
        if more:
            said += more
            quiet = time.monotonic()
    return said


# The carries the node ordered, as it keeps them (mycelium's carry::ordered), in its state.
CARRIES = STATE / "mycelium/carries.jsonl"


def here() -> str:
    """Give the name the node goes by, as it recorded it for every command on the machine."""
    return (STATE / "mycelium/machine").read_text().strip()


def touched(address: str) -> None:
    """Lay a session's presence again, its state said to have begun now, so the node reads anew.

    The node reads its carries again only as a session it holds changes, and a presence's
    `at` alone is none of a session's, so its `since` moves.
    """
    record = laid_record(address)
    record["at"] = datetime.now(UTC).isoformat()
    record["since"] = record["at"]
    laid_whole(address, record)


def kept(*carries: Json) -> None:
    """Keep these carries whole, as the node keeps those it ordered, and have it read them."""
    laid = CARRIES.with_suffix(".tmp")
    laid.parent.mkdir(parents=True, exist_ok=True)
    laid.write_text("".join(f"{json.dumps(carry)}\n" for carry in carries))
    laid.rename(CARRIES)
    touched("LISTEN")


def ordered(address: str, ago: int, **said: str) -> Json:
    """Give a carry of a session the node ordered `ago` seconds ago, by its orchestrator."""
    at = datetime.now(UTC) - timedelta(seconds=ago)
    return {
        "at": at.isoformat().replace("+00:00", "Z"),
        "by": "BOSSES",
        "session": f"s-{address.lower()}",
        "address": address,
        "quest": f"QEST-20261107-{address}",
        **said,
    }


def in_agents(shell: Shell) -> None:
    """Open Agents as it stands this late in the run, whatever sessions the cases before laid."""
    shell.as_account(shell.ipc("collection", "agents"))
    shell.launcher_ready()
    shell.launcher_until(lambda shown: shown["held"] == ["agents"])


@pytest.fixture
def carried() -> Iterator[None]:
    """Keep two carries for one case: Listening Quest's arrived here, Paused Quest's to go.

    Each case's own, so no other case's rows count a carry. The node may keep the carry back
    it was asked for after, so the next case lays its own file whole rather than wait on none.
    """
    kept(
        ordered("LISTEN", 120, **{"from": "tern", "to": here(), "why": "it needs the camera"}),
        ordered("PAUSED", 60, **{"from": here(), "to": "tern", "why": "tern sits idle"}),
    )
    yield
    CARRIES.unlink(missing_ok=True)
    touched("LISTEN")


@pytest.mark.usefixtures("carried", "at_shots_scale")
def test_carried_lists_each_carry_newest_first_and_carries_an_arrived_one_back_in_one_act(
    shell: Shell, shoot: Callable[[str], None]
) -> None:
    """Carried lists each carry where it went, by whom and why; Return carries one back.

    An arrived carry offers Carry Back alone, asked of the node; one about to go only opens,
    offering no Carry Back, and Carry Back says so in place while the node is asked (DCU5J4,
    items 1 and 2).
    """
    in_agents(shell)
    shell.launcher_until(lambda shown: "Carried" in shown["rows"])
    shell.down_to("Carried")
    # The group counts ordered and carrying as one, on their way.
    assert shell.showing()["selected"]["side"] == "1 on its way \u00b7 1 arrived", shell.showing()
    took = presented_after(shell, "ret")
    logging.getLogger(__name__).info("LAB the step into Carried was presented %s ms on", took)
    shell.launcher_until(lambda shown: shown["rows"][:2] == ["Paused Quest", "Listening Quest"])
    going = shell.showing()["selected"]
    assert going["note"] == "carried to tern by Orchestrating Quest \u00b7 tern sits idle", going
    assert not [act for act in going["told"] if act.endswith("carry back")], going
    shoot("agents-carried")
    shell.down_to("Listening Quest")
    arrived = shell.showing()["selected"]
    assert arrived["note"] == (
        f"carried to {here()} by Orchestrating Quest \u00b7 it needs the camera"
    ), arrived
    assert arrived["told"][:1] == ["\u21b5 carry back"], arrived
    # Carry Back changes where a session runs, so Return from the find only lands on it
    # (Launcher.qml's lands) and normal's Return asks the node, the row becoming "carrying
    # back" while it's asked. The node here answers tern, no machine of its, at once, and the
    # row reads as it did again before a poll may see it (the whole run, 2026-11-16), so the
    # shell's own reports say it: the row becoming, then not, in the node's words, which only
    # its answer gives. How each answer ends it is tests/carry-back.qml's.
    shell.normal()
    # An act's Return is no key the launcher times to its frame (Launcher.qml's keyTimed), so
    # this is the most it took: when the case read the report, its polls of the log in.
    since = shell.mark()
    began = time.monotonic()
    shell.send_key("ret")
    shell.reported(f"becoming {arrived['is']}", lambda rest: rest == "", since)
    took = round((time.monotonic() - began) * 1000)
    logging.getLogger(__name__).info("LAB Carry Back was reported becoming %s ms on", took)
    shell.reported(f"not become {arrived['is']}: ", bool, since)


def in_to_read(shell: Shell) -> None:
    """Open For You and step into To Read, where what asks nothing of Joel waits."""
    in_for_you(shell)
    shell.launcher_until(lambda shown: "To Read" in shown["rows"])
    shell.down_to("To Read")
    shell.step_in()
    shell.launcher_until(lambda shown: shown["held"] == ["now", "For You", "To Read"])


def test_a_carry_under_way_waits_quietly_in_for_you_and_goes_as_it_arrives(
    shell: Shell,
) -> None:
    """A carry under way is a thing to read in For You, and goes from it once it arrives.

    Only one about to go or on its way is there, never one arrived (DCU5J4, item 3).
    """
    kept(
        ordered("AFLOAT", 90, **{"from": "tern", "to": here(), "why": "it needs the camera"}),
        ordered("PAUSED", 60, **{"from": here(), "to": "tern", "why": "tern sits idle"}),
    )
    try:
        # Served first, so a For You without them is For You's own miss: these two, not the
        # case before's, which the node may not yet have read gone.
        in_agents(shell)
        shell.launcher_until(lambda shown: "Carried" in shown["rows"])
        shell.down_to("Carried")
        shell.launcher_until(lambda shown: shown["selected"]["side"] == "2 on their way")
        shell.close_launcher()
        in_to_read(shell)
        # Afloat Quest's session runs nowhere yet, so its carry is on its way.
        shell.launcher_until(
            lambda shown: {"Paused Quest", "A Session on Its Way"} <= set(shown["rows"])
        )
        shell.close_launcher()
        lay("AFLOAT", "Afloat Quest", {"orchestratedBy": "BOSSES", "state": "listening"})
        in_to_read(shell)
        shell.launcher_until(
            lambda shown: (
                "Paused Quest" in shown["rows"]
                and "A Session on Its Way" not in shown["rows"]
                and "Afloat Quest" not in shown["rows"]
            )
        )
    finally:
        CARRIES.unlink(missing_ok=True)
        lay("AFLOAT", "Afloat Quest", {"state": "idle"}, closed=True)
        touched("LISTEN")


def test_a_landing_read_finds_each_landings_session_in_one_look(shell: Shell) -> None:
    """A landing read keeps Landed's rows again looking through the sessions once, not each time.

    Reading one changes the landings, so each is kept again, and each looked for its session
    among all of Joel's: 2,600 landings by about 120 sessions took 40 of the 87 ms To Try's
    Return spent to its frame (2026-11-15). It's the module's last case, since the landing
    it reads, two days old, then reads as read.
    """
    # The cases before laid sessions of their own, so Agents' first rows are theirs by now.
    shell.as_account(shell.ipc("collection", "agents"))
    shell.launcher_ready()
    shell.launcher_until(lambda shown: shown["held"] == ["agents"] and "Landed" in shown["rows"])
    shell.down_to("Landed")
    shell.step_in()
    # It's two days old, so it's in its day.
    day, _ = earlier_days()[0]
    shell.launcher_until(lambda shown: day in shown["rows"])
    shell.down_to(day)
    shell.step_in()
    earliest = "Earlier change 0."
    shell.launcher_until(lambda shown: earliest in shown["rows"])
    shell.down_to(earliest)
    launcher_on_niri.weighed_held_still(shell)
    looked, sought = landings_looked(shell), sessions_sought(shell)
    # It goes nowhere, so normal's Return reads it and keeps the launcher.
    shell.normal()
    shell.send_key("ret")
    deadline = time.monotonic() + 10
    while landings_looked(shell) == looked:
        assert time.monotonic() < deadline, "reading a landing kept no landing again"
        time.sleep(0.1)
    launcher_on_niri.weighed_held_still(shell)
    # Each keeping looks at every landing and through the sessions once, where looking for
    # each landing's own looked through them all again, once a landing at least. The row
    # selected looks its own up live a few times besides, as its verbs are read: 4 times
    # once, when a walk soon after midnight left 7 landings in the day (2026-11-19). The day
    # holds all of EARLIER now, so half of them leaves room both ways.
    looked = landings_looked(shell) - looked
    sought = sessions_sought(shell) - sought
    times = sought / len(held_by_node())
    assert looked >= EARLIER, f"reading a landing kept {looked} landings, not its day's"
    assert times < looked / 2, (
        f"keeping {looked} landings looked through the sessions {times}x, not under half"
    )


def test_a_parked_sessions_return_lands_on_wake_and_a_second_asks_the_wake(shell: Shell) -> None:
    """A parked session's Return lands on Wake, its drawer marking it; a second asks the wake.

    Joel answers a session in any state with one Return (his ask of 19 November). A parked
    session stands apart from the fixture's, so it's laid here and taken away after, leaving
    every other case's rows as they were. It's parked as pi parks one, its process gone and
    its mark laid, so the node holds it parked and the launcher waits for it to wake. Only its
    holder wakes it, and no holder keeps this one, so the node refuses the wake. It's fenced,
    and the shell brings no fenced session back by itself, so the launcher says why on its
    foot at once, its row reading parked again rather than woken.
    """
    PARKS.mkdir(parents=True, exist_ok=True)
    (PARKS / "s-parked.json").write_text("{}")
    lay("PARKED", "Parked Quest", {"state": "parked", "fence": "fence"}, closed=True)
    try:
        # It's listed beside the fixture's rows, so Agents is opened on it here, not by opened.
        shell.as_account(shell.ipc("collection", "agents"))
        shell.launcher_ready()
        shell.launcher_until(lambda shown: "Parked Quest" in shown["rows"])
        shell.type_in("parked quest", selecting="Parked Quest")
        row = cast("Json", shell.showing()["selected"])
        assert row["verb"] == "wake", row
        assert row["told"][:1] == ["\u21b5 wake"], row
        asleep = row["side"]
        shell.launcher_until(rested_on("Parked Quest"))
        acts = shell.showing()["drawer"]["acts"]
        marked = [one for one in acts if one.startswith("\u21b5 ")]
        assert marked == ["\u21b5 Wake"], acts
        first = shell.mark()
        shell.send_key("ret")
        shell.launcher_until(
            lambda shown: (
                shown["mode"] == "normal"
                and (shown["selected"] or {}).get("told", [])[:1] == ["\u21b5 wake"]
                and not shown["landedFresh"]
            )
        )
        # A wake asked is said as becoming in the same call that asks it, before anything is
        # drawn (Agents' wakeNow), so none said by now shows the first Return only landed.
        landed, second = shell.said_since(first)
        assert not [one for one in landed if one.startswith("becoming agent:PARKED")], landed
        shell.send_key("ret")
        # Said without the address the node's refusal opens with, which means nothing to Joel
        # and which the row already stands for (SessionRow.reasonOf).
        unheld = "isn't parked with a holder to wake it this boot"
        # Fenced, with no holder, it's reopened by its own id under its fence instead, and
        # with no session file of its own that's refused too (Sessions' reopensUnwoken).
        shell.reported(
            "couldn't reopen PARKED: ",
            lambda why: why == "no session file or folder of its own",
            first,
        )
        # Its row, drawn as woken from the second Return, is taken back at once, not after
        # the half minute a wake is waited for.
        shell.reported(
            "not become agent:PARKED: ",
            lambda why: why == unheld,
            first,
            timeout=timedelta(seconds=5),
        )
        shell.launcher_until(
            lambda shown: (
                any(unheld in said.replace("\u00a0", " ") for said in shown["saying"])
                and not any("PARKED" in said for said in shown["saying"])
                and (shown["selected"] or {}).get("side") == asleep
            )
        )
        # The second Return asked the one wake: one becoming, one refusal.
        said, _ = shell.said_since(second)
        becoming = [one for one in said if one.startswith("becoming agent:PARKED")]
        assert len(becoming) == 1, said
    finally:
        (PRESENCE / "parked.json").unlink(missing_ok=True)
        (PARKS / "s-parked.json").unlink(missing_ok=True)
        shell.close_launcher()
        left_the_node("s-parked")


def test_a_wake_left_asleep_for_its_fence_is_taken_back_at_once_by_its_name(
    shell: Shell,
) -> None:
    """A Wake the shell won't do, since the session names no fence, is taken back at once.

    Its record names no fence it could come back under, so reopening it might bring it back
    looser than it ran, and the shell leaves it asleep (Sessions' reopensUnwoken). The row
    drawn as woken from Joel's Return is taken back as a refused Wake's is, rather than
    reading listening for the half minute a wake is waited for, and the foot says why by the
    session's name, never its address.
    """
    PARKS.mkdir(parents=True, exist_ok=True)
    (PARKS / "s-unsaid.json").write_text("{}")
    lay("UNSAID", "Unsaid Quest", {"state": "parked"}, closed=True)
    try:
        shell.as_account(shell.ipc("collection", "agents"))
        shell.launcher_ready()
        shell.launcher_until(lambda shown: "Unsaid Quest" in shown["rows"])
        shell.type_in("unsaid quest", selecting="Unsaid Quest")
        row = cast("Json", shell.showing()["selected"])
        assert row["verb"] == "wake", row
        asleep = row["side"]
        shell.launcher_until(rested_on("Unsaid Quest"))
        first = shell.mark()
        shell.send_key("ret")
        shell.launcher_until(
            lambda shown: (
                shown["mode"] == "normal"
                and (shown["selected"] or {}).get("told", [])[:1] == ["\u21b5 wake"]
                and not shown["landedFresh"]
            )
        )
        shell.send_key("ret")
        shell.reported(
            "left UNSAID asleep, ",
            lambda why: why == "whose record names no fence it can come back under",
            first,
        )
        sandbox = "it might come back outside its sandbox"
        shell.reported(
            "not become agent:UNSAID: ",
            lambda why: why == sandbox,
            first,
            timeout=timedelta(seconds=5),
        )
        shell.launcher_until(
            lambda shown: (
                any("Unsaid Quest" in said.replace("\u00a0", " ") for said in shown["saying"])
                and any(sandbox in said.replace("\u00a0", " ") for said in shown["saying"])
                and not any("UNSAID" in said for said in shown["saying"])
                and (shown["selected"] or {}).get("side") == asleep
            )
        )
    finally:
        (PRESENCE / "unsaid.json").unlink(missing_ok=True)
        (PARKS / "s-unsaid.json").unlink(missing_ok=True)
        shell.close_launcher()
        left_the_node("s-unsaid")


def left_the_node(session: str) -> None:
    """Wait until the node no longer holds a session laid for one case, as the next begins."""
    deadline = time.monotonic() + 10
    while f"session:{session}" in held_by_node():
        assert time.monotonic() < deadline, f"the node still holds {session}"
        time.sleep(0.1)


def test_an_asking_sessions_return_goes_to_it_with_its_question_in_sight(shell: Shell) -> None:
    """A session asking Joel a question is gone to, where it asks, on one Return.

    Its Return is Go To, marked so in its drawer, and what it asked is its row's note, in sight
    before he presses it, so he knows what he's going to answer. This one runs in no window the
    sandbox can reach, so Go To says so on the next opening, which shows Return did go to it;
    raising a window is the desktop VM tests'.
    """
    wants = {"kind": "question", "since": datetime.now(UTC).isoformat(), "said": "Which way?"}
    lay("ASKING", "Asking Quest", {"state": "listening", "busy": False, "wants": wants})
    try:
        shell.as_account(shell.ipc("collection", "agents"))
        shell.launcher_ready()
        shell.launcher_until(lambda shown: "Asking Quest" in shown["rows"])
        shell.type_in("asking quest", selecting="Asking Quest")
        row = cast("Json", shell.showing()["selected"])
        assert row["verb"] == "go to", row
        assert row["told"][:1] == ["\u21b5 go to"], row
        assert row["note"] == "Which way?", f"the question is out of sight: {row}"
        shell.launcher_until(rested_on("Asking Quest"))
        shown = shell.showing()
        marked = [one for one in shown["drawer"]["acts"] if one.startswith("\u21b5 ")]
        assert marked == ["\u21b5 Go To"], shown["drawer"]
        assert shown["selected"]["note"] == "Which way?", shown["selected"]
        # In sight: drawn on its row, whole, not only held in the row's data.
        laid = json.loads(shell.as_account(shell.ipc("laidOut")))["rows"]
        asked = [
            word
            for one in laid
            for word in one["words"]
            if re.sub("<[^>]*>", "", word["text"]) == "Which way?"
        ]
        assert asked, laid
        assert not any(word["cut"] for word in asked), asked
        shell.send_key("ret")
        shell.launcher_until(lambda shown: not shown["open"])
        shell.as_account(shell.ipc("collection", "agents"))
        shell.launcher_ready()
        # Named by its row's name, not its address, which means nothing to Joel.
        shell.launcher_until(
            lambda shown: (
                any(
                    "Asking Quest not opened" in said.replace("\xa0", " ")
                    for said in shown["saying"]
                )
                and not any("ASKING" in said for said in shown["saying"])
            )
        )
    finally:
        (PRESENCE / "asking.json").unlink(missing_ok=True)
        shell.close_launcher()
        left_the_node("s-asking")

"""An agent's row in the launcher does, on Return, the act its state makes harmless, on niri.

modules/quickshell/checks.nix runs this through pytest inside tests/nested-niri.sh. The sessions
are the realm's own mycelium node's, which reads the presence records laid here as pi lays
them, each owned by this test's own process so the node judges it alive; the shell knows them
from the node alone. A listening session's Return asks what to say to it, and a paused one's
lands on Resume, which a second Return sends it; Ctrl+] still steps into all of either's acts.
A drawer's eyebrow names the repo a record says, rather than the folder it works in. Both
sit under an orchestrator at work, so they fold into one row after its own, which Return steps
into, and typing still finds each. Of two sessions the last boot left to reopen, the one whose
folder is gone says so on its row before Return, holds only Forget It, and Reopen Them All
counts only the other. Home's Agents row, and the row "agents" finds, say how they stand
together and what they spent today, as spend-watch sums it. Two orchestrators the harness lists,
closed, stand dimmed at the tree's end, the one last live first, each saying what its
orchestration spent today only where it spent anything, Return reopening it.
"""

from __future__ import annotations

import json
import os
import socket
import subprocess
import time
from datetime import UTC, datetime
from pathlib import Path
from typing import TYPE_CHECKING, cast

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


def lay(address: str, title: str, said: Json) -> None:
    """Lay a session's presence record whole, by rename, as pi does."""
    record = {
        "session": f"s-{address.lower()}",
        "addresses": [address],
        "title": f"\u03c0 - {title}",
        "questInfo": {"id": f"QEST-20261107-{address}", "title": title, "short": address,
                      "status": "active"},
        "cwd": str(STATE / address.lower()),
        "owner": owner(),
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
    lay("LISTEN", "Listening Quest", {**under, "state": "listening", "repo": "core.nix"})
    lay("PAUSED", "Paused Quest", {**under, "state": "paused"})
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
    started = launcher_on_niri.started()
    yield started
    started.stop()
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


def test_agents_says_how_its_sessions_stand_together_at_home_and_found_by_name(
    shell: Shell,
) -> None:
    """Home's Agents row and the row "agents" finds read "1 working \u00b7 $214 today"."""
    together = "1 working \u00b7 $214 today"
    shell.close_launcher()
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    shell.launcher_until(
        lambda shown: any(
            thing["is"] == "collection:agents"
            and thing["reads"].startswith(f"Agents \u00b7 {together}")
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
    shell.launcher_until(
        lambda shown: shown["selected"]["side"] == "closed \u00b7 $14.20 in all"
    )
    row = shell.showing()["selected"]
    assert (row["hue"], row["note"], row["verb"]) == ("muted", "Keep the lamp lit", "reopen"), row
    assert row["told"][:1] == ["\u21b5 reopen"], row
    shell.down_to("Beacon")
    assert shell.showing()["selected"]["side"] == "closed", shell.showing()["selected"]
    # Its acts: Reopen, then its talks, of which it has none.
    shell.step_in("ctrl-bracketright")
    shell.launcher_until(lambda shown: shown["rows"] == ["Reopen"])

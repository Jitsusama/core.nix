"""Live sessions served as Joel's Agents are, for the keys lab's LAB_AGENTS.

Pi lays each session's presence, the realm's node reads it, and the agents daemon makes
Agents' rows from the node, as the account's units run them, so a step into Agents lays as many
agent rows as his. The launcher's checks start the shell (launcher_on_niri.py); this starts
what serves its Agents beside it.
"""

from __future__ import annotations

import contextlib
import json
import os
import socket
import subprocess
import time
from datetime import UTC, datetime, timedelta
from pathlib import Path
from typing import TYPE_CHECKING

import launcher_on_niri

if TYPE_CHECKING:
    from collections.abc import Iterator

    from launcher_on_niri import Json

# The realm the sessions are the node's in, and where pi lays each one's presence.
REALM = "harbour"

# How each session stands, in turn, as Joel's Agents holds some of each.
STANDINGS = ("busy", "listening", "paused", "busy", "listening")

# What each session works on, about as long as a real one's summary, 290 letters.
WORKING_ON = (
    "{title}: A builder under the owner's plan, which keeps the launcher's agents quiet, told"
    " and easy to read: every session says how it stands, what it does, what it spent and what"
    " it needs from Joel, from the node, with nothing guessed and nothing said twice, held to"
    " the launcher's family and its keys."
)

# How full each session's context is and its turns today, as the harness's usage keeps them.
USAGE = {
    "gen_ai.response.model": "claude-opus-5-5",
    "last": {"gen_ai.usage.output_tokens": 4100},
    "today": {
        "gen_ai.usage.input_tokens": 786,
        "gen_ai.usage.output_tokens": 272470,
        "gen_ai.usage.cache_read.input_tokens": 124620036,
        "gen_ai.usage.cache_creation.input_tokens": 1194960,
    },
    "context": {"used": 442000, "limit": 1000000},
    "turns": {"today": 359, "last": {"duration_ms": 38000, "first_token_ms": 900}},
}

# How long the node and the agents daemon may take to listen, in seconds.
LISTENING = 30

# The id the node's sessions are asked for under, after its hello, so its answer is known.
LISTED = 2


def presence(state: Path, nth: int) -> Json:
    """Give the nth session as pi lays its presence, owned by this process, so alive."""
    stat = Path("/proc/self/stat").read_text()
    ticks = int(stat[stat.rindex(")") + 2 :].split()[19])
    address = f"LAB{nth:03d}"
    title = f"Lab Quest {nth}"
    now = datetime.now(UTC)
    landings = [
        {"at": (now - timedelta(minutes=at)).isoformat(), "ref": f"core.nix#{4000 + at}",
         "what": f"Lab change {at}", "person": "Press d on a session."}
        for at in range(3)
    ]  # fmt: skip
    return {
        "session": f"s-{address.lower()}",
        "addresses": [address],
        "title": f"\u03c0 - {title}",
        "questInfo": {"id": f"QEST-20261120-{address}", "title": title, "short": address,
                      "status": "active"},
        "cwd": str(state / address.lower()),
        "owner": {"pid": os.getpid(),
                  "bootId": Path("/proc/sys/kernel/random/boot_id").read_text().strip(),
                  "startTicks": ticks, "startedAt": int(time.time() * 1000)},
        "at": now.isoformat(),
        "state": STANDINGS[nth % len(STANDINGS)],
        "busy": STANDINGS[nth % len(STANDINGS)] == "busy",
        "repo": "core.nix",
        "groups": ["#repo/core-nix", f"#lab-{nth}"],
        "usage": {**USAGE, "day": datetime.now().astimezone().date().isoformat()},
        "worksOn": WORKING_ON.format(title=title),
        "landings": landings,
    }  # fmt: skip


def held_by_node(node: Path) -> int:
    """Give how many sessions the node holds alive, asked over its daemon interface."""
    asks = [{"id": 1, "ask": "hello"}, {"id": LISTED, "ask": "list", "kind": "session"}]
    with socket.socket(socket.AF_UNIX) as asked:
        asked.connect(str(node))
        asked.sendall("".join(json.dumps(ask) + "\n" for ask in asks).encode())
        said = asked.makefile()
        while True:
            answer = json.loads(said.readline())
            if answer.get("id") == LISTED:
                records = answer.get("records", [])
                return sum(1 for record in records if record.get("state") != "ended")


def listening(path: Path, process: subprocess.Popen[bytes], log: Path) -> None:
    """Wait until a daemon listens on `path`, failing with its log if it ends first."""
    deadline = time.monotonic() + LISTENING
    while not path.exists():
        if process.poll() is not None:
            raise AssertionError(log.read_text(errors="replace"))
        if time.monotonic() >= deadline:
            refused = f"nothing listened on {path}"
            raise AssertionError(refused)
        time.sleep(launcher_on_niri.POLL)


@contextlib.contextmanager
def agents_served(count: int) -> Iterator[None]:
    """Lay `count` sessions and serve them as Joel's Agents are served; none given 0.

    His step took 9 agent rows in 10 to 37 ms, where a program's 9 take 2 to 5 (2026-11-20).
    """
    if count == 0:
        yield
        return
    state = Path(os.environ["XDG_STATE_HOME"])
    runtime = Path(os.environ["XDG_RUNTIME_DIR"])
    logs = Path(os.environ["LOGS"])
    logs.mkdir(parents=True, exist_ok=True)
    laid = state / "pi/agentic-harness.pi/mesh-workflow/presence"
    laid.mkdir(parents=True, exist_ok=True)
    for nth in range(count):
        (laid / f"lab{nth:03d}.json").write_text(json.dumps(presence(state, nth)))
    steer = state / "pi/agentic-harness.pi/steer"
    steer.mkdir(parents=True, exist_ok=True)
    serves = logs / "serves.json"
    serves.write_text(json.dumps({"talk": {"talks": str(steer)}}))
    node_socket = runtime / "mycelium" / f"{REALM}.sock"
    rows = runtime / "daemons" / "agents.sock"
    rows.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
    spend = runtime / "spend-watch" / "collection.json"
    spend.parent.mkdir(parents=True, exist_ok=True)
    spend.write_text(json.dumps({"v": 2, "agentsToday": "$214"}))
    with contextlib.ExitStack() as stack:
        node_log = stack.enter_context((logs / "mycelium.log").open("wb"))
        node = subprocess.Popen(
            [os.environ["MYCELIUM"], "--realm", REALM, "node", "--serves", str(serves)],
            stdout=node_log,
            stderr=subprocess.STDOUT,
        )
        stack.callback(node.wait)
        stack.callback(node.terminate)
        listening(node_socket, node, logs / "mycelium.log")
        deadline = time.monotonic() + LISTENING
        while held_by_node(node_socket) < count:
            if time.monotonic() >= deadline:
                refused = f"the node holds {held_by_node(node_socket)}"
                raise AssertionError(refused)
            time.sleep(launcher_on_niri.POLL)
        agents_log = stack.enter_context((logs / "agents.log").open("wb"))
        workflow = state / "pi/agentic-harness.pi/quest-workflow"
        served = subprocess.Popen(
            [os.environ["AGENTS"], "--vocabulary", os.environ["WORDS"],
             "--node", str(node_socket), "--to-do", str(rows.parent / "to-do.sock"),
             "--windows", str(rows.parent / "windows.sock"), "--spend", str(spend),
             "--orchestrators", str(workflow / "orchestrators.json"),
             "--restore", str(state / "pi/restore.json"),
             "--kept", str(state / "agents/reopen.json"),
             "--landed", str(state / "agents/landed.json"), "--steer", str(steer),
             "--shells", str(state / "quickshell/by-shell"), "--socket", str(rows)],
            stdout=agents_log,
            stderr=subprocess.STDOUT,
        )  # fmt: skip
        stack.callback(served.wait)
        stack.callback(served.terminate)
        listening(rows, served, logs / "agents.log")
        yield

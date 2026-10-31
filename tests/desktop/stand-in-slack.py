"""Stand in for Slack's daemon, slack-unreads, on the socket it keeps (modules/slack/src/daemon.rs).

It speaks the daemon interface, version 1, as the real one does: hello says Slack is signed in,
a search answers the Slack messages found for its words, and a get of a message's key answers
the messages around it. Each search is written down in slack-searched beside the socket, one
line of words each, so a test can count how often Slack was asked, and is held until the
runtime folder's slack-answers file exists, so a test sees what the launcher shows while Slack
is asked. "launcher" finds two messages, Ada's in #launcher and Joel's own to her; "busy" is
refused as Slack's rate limit refuses; anything else finds nothing. Nothing else is answered.
Each hello is written down in slack-hellos, so a test knows the shell has reached it.
"""

import json
import socket
import sys
import threading
import time
from pathlib import Path

SOCKET = Path(sys.argv[1])
RUNTIME = SOCKET.parent.parent
SEARCHED = RUNTIME / "slack-searched"
ANSWERS = RUNTIME / "slack-answers"
HELLOS = RUNTIME / "slack-hellos"
# How often a held search looks for leave to answer, in seconds.
LOOK = 0.05
# Milliseconds in an hour, as the daemon gives times.
HOUR = 3_600_000


def found(now: int) -> list[dict[str, object]]:
    """Give what searching "launcher" finds, as the daemon says each message (src/search.rs)."""
    return [
        {
            "key": "slack-message:C1:1730300000.000100",
            "kind": "slack-message",
            "place": "#launcher",
            "who": "ada",
            "mine": False,
            "first": "The launcher is fast",
            "text": "The launcher is fast\nand never waits",
            "at": now - HOUR,
            "link": "https://slack.example/archives/C1/p1730300000000100",
            "team": "T1",
        },
        {
            "key": "slack-message:D2:1730200000.000200",
            "kind": "slack-message",
            "place": "Ada Lovelace",
            "who": "joel",
            "mine": True,
            "first": "Launcher notes for Friday",
            "text": "Launcher notes for Friday",
            "at": now - 2 * HOUR,
            "link": "https://slack.example/archives/D2/p1730200000000200",
            "team": "T1",
        },
    ]


def around(key: str, now: int) -> dict[str, object]:
    """Give the messages around one found, oldest first, the one asked about marked."""
    return {
        "key": key,
        "kind": "slack-message",
        "around": [
            {
                "mine": True,
                "text": "Is the launcher fast yet?",
                "at": now - 2 * HOUR,
                "asked": False,
            },
            {"mine": False, "text": "The launcher is fast", "at": now - HOUR, "asked": True},
        ],
    }


def answer(said: dict[str, object]) -> dict[str, object]:
    """Answer one ask as the daemon would."""
    asked = said.get("id")
    now = int(time.time() * 1000)
    if said.get("ask") == "hello":
        with HELLOS.open("a") as hellos:
            hellos.write("hello\n")
        return {
            "id": asked,
            "version": 1,
            "domain": "slack",
            "state": "answering",
            "sources": {"slack": "answering"},
        }
    if said.get("ask") == "search":
        words = str(said.get("words", "")).strip()
        with SEARCHED.open("a") as searched:
            searched.write(f"{words}\n")
        while not ANSWERS.exists():
            time.sleep(LOOK)
        if words == "busy":
            return {"id": asked, "error": {"code": "too-many", "message": "Slack answered 429"}}
        return {"id": asked, "records": found(now) if words == "launcher" else []}
    keys = said.get("keys")
    if said.get("ask") == "get" and isinstance(keys, list):
        return {"id": asked, "records": [around(str(key), now) for key in keys]}
    return {"id": asked, "error": {"code": "unknown-ask", "message": "not asked of Slack"}}


def asking(said: dict[str, object], connection: socket.socket, lock: threading.Lock) -> None:
    """Answer an ask on a thread of its own, as the daemon does, so a held search holds no other."""
    line = f"{json.dumps(answer(said))}\n".encode()
    with lock:
        connection.sendall(line)


def serve(connection: socket.socket) -> None:
    """Answer each line a client says, until it goes."""
    lock = threading.Lock()
    with connection, connection.makefile("r") as lines:
        for line in lines:
            said = json.loads(line)
            threading.Thread(target=asking, args=(said, connection, lock), daemon=True).start()


def main() -> None:
    """Listen on the socket the shell asks, as the daemon's unit makes it."""
    SOCKET.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
    SOCKET.unlink(missing_ok=True)
    listening = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    listening.bind(str(SOCKET))
    listening.listen()
    while True:
        connection, _ = listening.accept()
        threading.Thread(target=serve, args=(connection,), daemon=True).start()


main()

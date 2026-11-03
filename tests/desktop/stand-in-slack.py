"""Stand in for Slack's daemon, slack-unreads, on the socket it keeps (modules/slack/src/daemon.rs).

It speaks the daemon interface, version 1, as the real one does. What Slack says is unread,
read last and how it stands is the runtime folder's slack-says.json, { state, conversations,
recent }, which a test writes and then sends SIGHUP, as slack-unreads' rounds would tell it;
with none, Slack answers with nothing unread. It's held as the daemon's records, slack-unread,
slack-recent and slack-account, listed and watched, each watch told what changed and went and
how Slack stands as that changes; each conversation is given a timestamp falling with its place,
so the order a test lists them in is Slack's. Hello says how Slack stands, a search answers
the Slack messages found for its words, and a get of a message's key answers the messages
around it. Each search is written down in slack-searched beside the socket, one
line of words each, so a test can count how often Slack was asked, and is held until the
runtime folder's slack-answers file exists, so a test sees what the launcher shows while Slack
is asked. "launcher" finds two messages, Ada's in #launcher and Joel's own to her; "busy" is
refused as Slack's rate limit refuses; anything else finds nothing. Nothing else is answered.
Each hello is written down in slack-hellos, so a test knows the shell has reached it.
"""

import contextlib
import json
import signal
import socket
import sys
import threading
import time
from dataclasses import dataclass
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Callable

SOCKET = Path(sys.argv[1])
RUNTIME = SOCKET.parent.parent
SEARCHED = RUNTIME / "slack-searched"
ANSWERS = RUNTIME / "slack-answers"
HELLOS = RUNTIME / "slack-hellos"
SAYS = RUNTIME / "slack-says.json"
# The kinds it holds, and slack-message, which it holds none of but answers for, as Search
# Slack watches it to hear how Slack stands.
KINDS = ("slack-unread", "slack-recent", "slack-account", "slack-message")
# The daemon's word for how Slack stands, by what slack-says.json says.
WORDS = {"ok": "answering", "signedOut": "signed-out", "unreachable": "refused"}
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


@dataclass
class Line:
    """One client's connection, which one thread at a time writes a line to."""

    connection: socket.socket
    lock: threading.Lock

    def say(self, said: dict[str, object]) -> None:
        """Write one line to the client, or nothing once it has gone."""
        line = f"{json.dumps(said)}\n".encode()
        with self.lock, contextlib.suppress(OSError):
            self.connection.sendall(line)


@dataclass
class Watch:
    """A client's watch of some kinds, by the id it asked with."""

    line: Line
    id: object
    kinds: set[str]


@dataclass
class Held:
    """What Slack says, as the daemon holds it: its records in Slack's order, and its watches."""

    state: str
    records: dict[str, dict[str, object]]
    watches: list[Watch]
    heard: int = 0


HELD = Held(state="ok", records={}, watches=[])
HOLDING = threading.Lock()


def saying() -> tuple[str, dict[str, dict[str, object]]]:
    """Read slack-says.json as how Slack stands and the things it holds by key, in its order."""
    try:
        says = json.loads(SAYS.read_text())
    except OSError, ValueError:
        says = {}
    state = str(says.get("state", "ok"))
    things: dict[str, dict[str, object]] = {}
    if state == "ok":
        for kind, field, listed in (
            ("slack-unread", "latest", says.get("conversations", [])),
            ("slack-recent", "read", says.get("recent", [])),
        ):
            for place, one in enumerate(listed):
                thing = {name: one[name] for name in ("id", "name", "kind", "link")}
                if kind == "slack-unread":
                    thing["count"] = one["count"]
                thing[field] = f"{1730000000 - place}.000100"
                things[f"{kind}:{one['id']}"] = thing
    things["slack-account:slack"] = {"standing": state}
    return state, things


def retell() -> None:
    """Hold what slack-says.json says, telling each watch what changed, as the daemon does.

    Each watch is told the records that changed and went, and how Slack stands once that
    changes, as the daemon tells it after a round of asking Slack.
    """
    state, things = saying()
    with HOLDING:
        was = HELD.records
        HELD.records = {}
        changed = []
        for key, thing in things.items():
            kind = key.split(":", 1)[0]
            before = was.get(key)
            if before is not None and before[kind] == thing:
                HELD.records[key] = before
                continue
            HELD.heard += 1
            record = {"key": key, "kind": kind, "heard": HELD.heard, kind: thing}
            HELD.records[key] = record
            changed.append(record)
        gone = [key for key in was if key not in HELD.records]
        stood = HELD.state
        HELD.state = state
        for watch in HELD.watches:
            seen = [record for record in changed if record["kind"] in watch.kinds]
            went = [key for key in gone if key.split(":", 1)[0] in watch.kinds]
            if seen or went:
                watch.line.say({"id": watch.id, "changed": seen, "gone": went})
            if stood != state:
                watch.line.say({"id": watch.id, "sources": {"slack": WORDS.get(state, "")}})


def kinds(said: dict[str, object]) -> set[str] | None:
    """Give the kinds an ask names, every kind when it names none, or None for one not held."""
    asked = said.get("kind")
    if asked is None:
        named = set(KINDS)
    elif isinstance(asked, str):
        named = {asked}
    elif isinstance(asked, list):
        named = {str(kind) for kind in asked}
    else:
        return None
    return named if named <= set(KINDS) else None


def unknown(asked: object) -> dict[str, object]:
    """Refuse an ask of a kind it doesn't hold, as the daemon does."""
    return {"id": asked, "error": {"code": "unknown-kind", "message": "not one of Slack's"}}


def watch(said: dict[str, object], line: Line) -> dict[str, object] | None:
    """Tell a watch what's held while holding it, and only then add it, as the daemon does."""
    asked = said.get("id")
    watched = kinds(said)
    if watched is None:
        return unknown(asked)
    with HOLDING:
        records = [record for record in HELD.records.values() if record["kind"] in watched]
        line.say({"id": asked, "records": records})
        HELD.watches = [
            each for each in HELD.watches if not (each.line is line and each.id == asked)
        ]
        HELD.watches.append(Watch(line=line, id=asked, kinds=watched))
    return None


def hello(said: dict[str, object], _line: Line) -> dict[str, object]:
    """Say how Slack stands, writing the hello down."""
    with HELLOS.open("a") as hellos:
        hellos.write("hello\n")
    with HOLDING:
        state = HELD.state
    return {
        "id": said.get("id"),
        "version": 1,
        "domain": "slack",
        "state": "answering",
        "sources": {"slack": WORDS.get(state, "")},
    }


def listing(said: dict[str, object], _line: Line) -> dict[str, object]:
    """List what's held of the kinds asked, in Slack's order."""
    listed = kinds(said)
    if listed is None:
        return unknown(said.get("id"))
    with HOLDING:
        records = [record for record in HELD.records.values() if record["kind"] in listed]
    return {"id": said.get("id"), "records": records}


def unwatch(said: dict[str, object], line: Line) -> dict[str, object]:
    """Let a client's watch go, by the id it asked with."""
    asked = said.get("id")
    with HOLDING:
        HELD.watches = [
            each for each in HELD.watches if not (each.line is line and each.id == asked)
        ]
    return {"id": asked, "unwatched": True}


def search(said: dict[str, object], _line: Line) -> dict[str, object]:
    """Answer a search once the test gives leave, writing its words down."""
    asked = said.get("id")
    words = str(said.get("words", "")).strip()
    with SEARCHED.open("a") as searched:
        searched.write(f"{words}\n")
    while not ANSWERS.exists():
        time.sleep(LOOK)
    if words == "busy":
        return {"id": asked, "error": {"code": "too-many", "message": "Slack answered 429"}}
    now = int(time.time() * 1000)
    return {"id": asked, "records": found(now) if words == "launcher" else []}


def get(said: dict[str, object], _line: Line) -> dict[str, object]:
    """Give the messages around each message asked about."""
    keys = said.get("keys")
    if not isinstance(keys, list):
        return refused(said)
    now = int(time.time() * 1000)
    return {"id": said.get("id"), "records": [around(str(key), now) for key in keys]}


def refused(said: dict[str, object], _line: Line | None = None) -> dict[str, object]:
    """Refuse an ask the daemon doesn't answer."""
    return {"id": said.get("id"), "error": {"code": "unknown-ask", "message": "not asked of Slack"}}


# What answers each ask, by its name.
ASKS: dict[str, Callable[[dict[str, object], Line], dict[str, object] | None]] = {
    "hello": hello,
    "watch": watch,
    "list": listing,
    "unwatch": unwatch,
    "search": search,
    "get": get,
}


def answer(said: dict[str, object], line: Line) -> dict[str, object] | None:
    """Answer one ask as the daemon would, or None once a watch was told already."""
    return ASKS.get(str(said.get("ask")), refused)(said, line)


def asking(said: dict[str, object], line: Line) -> None:
    """Answer an ask on a thread of its own, as the daemon does, so a held search holds no other."""
    answered = answer(said, line)
    if answered is not None:
        line.say(answered)


def serve(connection: socket.socket) -> None:
    """Answer each line a client says, until it goes, and let its watches go with it."""
    line = Line(connection=connection, lock=threading.Lock())
    with connection, connection.makefile("r") as lines:
        for said in lines:
            threading.Thread(target=asking, args=(json.loads(said), line), daemon=True).start()
    with HOLDING:
        HELD.watches = [each for each in HELD.watches if each.line is not line]


def hearing(told: threading.Event) -> None:
    """Hold what slack-says.json says again each time a test says it changed."""
    while True:
        told.wait()
        told.clear()
        retell()


def main() -> None:
    """Listen on the socket the shell asks, as the daemon's unit makes it."""
    # Heard before the socket is made, so a test that sees the socket can say a change.
    told = threading.Event()
    signal.signal(signal.SIGHUP, lambda *_: told.set())
    retell()
    threading.Thread(target=hearing, args=(told,), daemon=True).start()
    SOCKET.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
    SOCKET.unlink(missing_ok=True)
    listening = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    listening.bind(str(SOCKET))
    listening.listen()
    while True:
        connection, _ = listening.accept()
        threading.Thread(target=serve, args=(connection,), daemon=True).start()


main()

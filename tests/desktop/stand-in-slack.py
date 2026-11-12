"""Stand in for Slack's daemon, slack-unreads, on the socket it keeps (modules/slack/src/daemon.rs).

It speaks the daemon interface, version 1, as the real one does. What Slack says is unread,
read last, each message wanting Joel and how it stands is the runtime folder's slack-says.json,
{ state, conversations, recent, messages }, which a test writes and then sends SIGHUP, as
slack-unreads' rounds would tell it; with none, Slack answers with nothing unread. It's held
as the daemon's records, slack-unread, slack-recent, slack-message and slack-account, the
account saying it holds every message once slack-says.json lists any, and each conversation's
row of Unread and Recent, row:<place>:<id>, as src/rows.rs serves it ready, its order its
place, and each message's row, newest first, with its drawer a record of its own, as
src/message_rows.rs serves them, listed and watched, a watch of rows narrowed by its place,
each watch told what changed and went and how Slack stands as that changes; each conversation
is given a timestamp falling with its place, so the order a test lists them in is Slack's.
Hello says how Slack stands, a search answers the Slack messages found for its words, and a
get of a message's key answers the messages around it. Each search is written down in
slack-searched beside the socket, one line of words each, so a test can count how often Slack
was asked, and is held until the runtime folder's slack-answers file exists, so a test sees
what the launcher shows while Slack is asked. "launcher" finds two messages, Ada's in
#launcher and Joel's own to her; "busy" is refused as Slack's rate limit refuses; anything
else finds nothing. Nothing else is answered. Each hello is written down in slack-hellos, so a
test knows the shell has reached it.
"""

import contextlib
import json
import signal
import socket
import sys
import threading
import time
from dataclasses import dataclass
from datetime import UTC, datetime
from pathlib import Path
from typing import TYPE_CHECKING, cast

if TYPE_CHECKING:
    from collections.abc import Callable

SOCKET = Path(sys.argv[1])
RUNTIME = SOCKET.parent.parent
SEARCHED = RUNTIME / "slack-searched"
ANSWERS = RUNTIME / "slack-answers"
HELLOS = RUNTIME / "slack-hellos"
SAYS = RUNTIME / "slack-says.json"
# The kinds it holds: slack-message for each message wanting Joel, which Search Slack watches
# too, to hear how Slack stands, and row for each conversation's row as the launcher shows it.
# slack-seen, a conversation's read mark, it keeps none of, but a watch naming it is answered
# as the daemon's is, or the shell's one watch of Slack (SlackUnreads.qml) is refused whole and
# hears nothing at all, as it did once SlackUnreads.qml asked for it (2026-11-11, #3440).
KINDS = (
    "slack-unread",
    "slack-recent",
    "slack-seen",
    "slack-account",
    "slack-message",
    "row",
    "drawer",
)
# A conversation's note by its kind, and what's beside one counted, in the launcher's words
# (words.json's slack:conversation), as the daemon is handed them.
NOTES = {
    "dm": "a direct message",
    "mention": "mentions you",
    "thread": "replies in threads",
    "unread": "a channel",
    "channel": "a channel",
}
COUNTED = "{count} to read"
# The order the ready-rows crate gives the first row of a place, and the room between two.
SPACED = 1024
# The daemon's word for how Slack stands, by what slack-says.json says.
WORDS = {"ok": "answering", "signedOut": "signed-out", "unreachable": "refused"}
# How often a held search looks for leave to answer, in seconds.
LOOK = 0.05
# Milliseconds in an hour and a minute, as the daemon gives times.
HOUR = 3_600_000
MINUTE = 60_000


def found(now: int) -> list[dict[str, object]]:
    """Give what searching "launcher" finds, as the daemon says each message (src/search.rs)."""
    return [
        {
            "key": "slack-message:C1:1730300000.000100",
            "kind": "slack-message",
            "place": "#launcher",
            "who": "Ada Lovelace",
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
    places: set[str] | None = None


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
                row = conversation_row(kind, one, place)
                things[str(row["key"])] = row
        things.update(messages(says.get("messages", [])))
    account: dict[str, object] = {"standing": state}
    if "messages" in says:
        account["messages"] = "whole"
    things["slack-account:slack"] = account
    return state, things


def conversation_row(kind: str, one: dict[str, object], at: int) -> dict[str, object]:
    """Give a conversation's row in Unread or Recent, at its place, as the daemon (src/rows.rs)."""
    count = int(cast("int", one.get("count", 0)))
    return {
        "key": f"row:{kind}:{one['id']}",
        "thing": f"slack:conversation:{one['id']}",
        "place": kind,
        "kind": one["kind"],
        "order": (at + 1) * SPACED,
        "name": one["name"],
        "glyph": "chat",
        "hue": "program",
        "depth": 0,
        "side": [{"said": COUNTED.replace("{count}", str(count))}] if count > 0 else [],
        "note": NOTES.get(str(one["kind"]), ""),
        "acts": [{"key": "verb:slack:open", "open": one["link"]}],
        "held": {"id": one["id"], "count": count},
    }


def link(channel: str, ts: str) -> str:
    """Give where Slack shows a message on the web, as the daemon makes it (src/messages.rs)."""
    return f"https://slack.example/archives/{channel}/p{ts.replace('.', '')}"


def messages(listed: list[dict[str, object]]) -> dict[str, dict[str, object]]:
    """Hold each message wanting Joel as the daemon does (src/messages.rs), by its key.

    Each says its channel, ts, place, who, first, text, link and team, how many minutes ago
    it was said, and done, { how, minutes }, once it's read in Slack.
    """
    now = int(time.time() * 1000)
    held: dict[str, dict[str, object]] = {}
    for one in listed:
        message = {name: one[name] for name in ("channel", "ts", "place", "who", "first")}
        message |= {
            "mine": False,
            "text": one.get("text", one["first"]),
            "at": now - int(cast("int", one["minutes"])) * MINUTE,
            "link": one.get("link", link(str(one["channel"]), str(one["ts"]))),
            "team": one.get("team", "T1"),
        }
        done = cast("dict[str, object] | None", one.get("done"))
        if done is not None:
            # The daemon says when it was done in ISO 8601, in UTC, where it
            # says when it was said in milliseconds (src/messages.rs).
            ended = now - int(cast("int", done["minutes"])) * MINUTE
            at = datetime.fromtimestamp(ended / 1000, tz=UTC).isoformat()
            message["done"] = {"how": done["how"], "at": at.replace("+00:00", "Z")}
        held[f"slack-message:{one['channel']}:{one['ts']}"] = message
    newest = sorted(
        held.values(), key=lambda one: (int(cast("int", one["at"])), str(one["ts"])), reverse=True
    )
    for place, message in enumerate(newest):
        row, drawer = message_row(message, place)
        held[str(row["key"])] = row
        held[str(row["drawer"])] = drawer
    return held


def message_row(one: dict[str, object], at: int) -> tuple[dict[str, object], dict[str, object]]:
    """Give a message's row at its place, newest first, and its drawer, as src/message_rows.rs."""
    id_ = f"{one['channel']}:{one['ts']}"
    drawer = f"drawer:slack-messages:{id_}"
    direct = str(one["channel"]).startswith("D")
    acts = [
        {"key": "verb:slack:message:open", "open": one["link"]},
        {"key": "verb:slack:message:link", "copy": one["link"]},
    ]
    row = {
        "key": f"row:slack-messages:{id_}",
        "thing": f"slack:message:{id_}",
        "place": "slack-messages",
        "kind": "message",
        "order": (at + 1) * SPACED,
        "name": one["who"],
        "glyph": "chat",
        "hue": "program",
        "depth": 0,
        "side": [{"at": one["at"], "as": "came"}],
        "note": one["first"],
        "drawer": drawer,
        "acts": acts,
        "held": {name: one.get(name) for name in ("channel", "ts", "place", "mine", "at", "done")},
    }
    drawn = {
        "eyebrow": "direct message" if direct else f"in {one['place']} \u00b7 message",
        "eyebrowNames": not direct,
        "title": one["who"],
        "facts": [{"name": "Sent", "at": one["at"], "as": "moment"}],
        "body": one["text"],
        "prose": True,
    }
    return row, drawn


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
            told = [
                record for record in changed if seen(watch.kinds, watch.places, str(record["key"]))
            ]
            went = [key for key in gone if seen(watch.kinds, watch.places, key)]
            if told or went:
                watch.line.say({"id": watch.id, "changed": told, "gone": went})
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


def narrowed(said: dict[str, object]) -> set[str] | None:
    """Give the places an ask narrows its rows to, one or a list, or None for every place."""
    place = said.get("place")
    if isinstance(place, str):
        return {place}
    if isinstance(place, list):
        return {str(one) for one in place}
    return None


def seen(kinds: set[str], places: set[str] | None, key: str) -> bool:
    """Say whether a watch of these kinds and places hears the record of `key`, as the daemon."""
    kind, _, rest = key.partition(":")
    place = rest.split(":", 1)[0]
    return kind in kinds and (kind != "row" or places is None or place in places)


def unknown(asked: object) -> dict[str, object]:
    """Refuse an ask of a kind it doesn't hold, as the daemon does."""
    return {"id": asked, "error": {"code": "unknown-kind", "message": "not one of Slack's"}}


def watch(said: dict[str, object], line: Line) -> dict[str, object] | None:
    """Tell a watch what's held while holding it, and only then add it, as the daemon does."""
    asked = said.get("id")
    watched = kinds(said)
    if watched is None:
        return unknown(asked)
    places = narrowed(said)
    with HOLDING:
        records = [record for key, record in HELD.records.items() if seen(watched, places, key)]
        line.say({"id": asked, "records": records})
        HELD.watches = [
            each for each in HELD.watches if not (each.line is line and each.id == asked)
        ]
        HELD.watches.append(Watch(line=line, id=asked, kinds=watched, places=places))
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
    places = narrowed(said)
    with HOLDING:
        records = [record for key, record in HELD.records.items() if seen(listed, places, key)]
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

"""Stand in for a daemon holding more than one kind of record, as Google's does.

It answers the shell's daemon interface (RSCH-20261029-0E8050) on the socket named as its one
argument: hello, and a watch, answered with one record for each key asked, or else one for each
kind asked, `<kind>:one`. Each record says the kinds its watch asked for, so tests/daemon.qml can
tell the client sent its list of kinds in every watch, the one ahead too, and not one kind alone,
and the pace it asked for (`every`), so it can tell a watch whose pace changed was asked again.
A watch of sessions is answered as mycelium's node answers one: a record whole, then a change
saying only the fields that differ, one dropped as null; its presence, which hello offers deep,
says only what moved inside it to a watch asking it deep, and comes whole to one that doesn't.
A watch of marks is answered whole,
then told a change, so tests/daemon.qml can tell which of the two is heard under its line.
A watch of what waits is answered with
two records the first time, and the connection then dropped, and with one of them alone after,
as the node's whole answer is once one went in a gap. A watch of what lapses is answered the
same way with an ask's row and an entry, and with neither after, as the passwords child answers
once it starts again, its asks' rows whole and its entries not yet listed. One of verdicts is
answered with none, and one of what's inherited with records keyed "__proto__" and "constructor",
names every object inherits, and "toString" said gone, never held, so the test can tell a client
holds them as any other and says no such key let go. An ask of steps
is answered in three, as wifid answers a join: two saying they aren't done, then the last.
An unwatch is answered as wifid answers one, and each record says how many watches ahead its
connection has unwatched, so the test can tell an emptied watch ahead was let go.
"""

import json
import socket
import sys
import threading
from pathlib import Path

# How many watches of each kind that loses records in a gap have been answered, so the first is
# told them all and the rest only what stood through it.
WAITED = {"waiting": 0, "lapsing": 0}
WAITED_LOCK = threading.Lock()

# What each kind that loses records in a gap holds before it, and what it holds after.
LAPSING = {
    "waiting": (
        ["waiting:a", "waiting:b"],
        ["waiting:b"],
    ),
    "lapsing": (
        ["lapsing:ask", "lapsing:entry"],
        [],
    ),
}


# What each kind answered the same every time holds: verdicts none, and what's inherited the
# records keyed by names every object inherits, with one such name said gone though never held.
FIXED: dict[str, list[dict[str, object]]] = {
    "verdict": [],
    "inherited": [
        *(
            {"key": key, "kind": "inherited", "heard": 1}
            for key in ("__proto__", "constructor")
        ),
        {"key": "toString", "kind": "inherited", "gone": True},
    ],
}


def kinds_of(asked: dict[str, object]) -> list[str]:
    """Read the kinds a watch names, one or a list, as Google's daemon reads them."""
    kind = asked.get("kind")
    if isinstance(kind, str):
        return [kind]
    if isinstance(kind, list):
        return [each for each in kind if isinstance(each, str)]
    return []


def told_after_whole(asked: dict[str, object]) -> list[dict[str, object]] | None:
    """Answer a watch of a kind answered whole and then told a change, or None for another."""
    if asked.get("kind") == "session":
        whole = {"key": "session:a", "kind": "session", "heard": 1}
        was = {"at": 1, "cwd": "~/src/skiff", "owner": "kestrel"}
        asked_deep = asked.get("deep")
        deep = isinstance(asked_deep, list) and "presence" in asked_deep
        presence = {"at": 2, "owner": None} if deep else {"at": 2}
        return [
            {
                "id": asked.get("id"),
                "records": [{**whole, "state": "idle", "step": "x", "presence": was}],
            },
            {
                "id": asked.get("id"),
                "changed": [
                    {"key": "session:a", "state": "busy", "step": None, "presence": presence}
                ],
            },
        ]
    if asked.get("kind") == "mark":
        mark = {"key": "mark:one", "kind": "mark"}
        return [
            {"id": asked.get("id"), "records": [{**mark, "heard": 1}]},
            {"id": asked.get("id"), "changed": [{**mark, "heard": 2}]},
        ]
    return None


def answer(asked: dict[str, object], unwatched: int) -> list[dict[str, object]]:
    """Answer one ask, in the lines a daemon sends for it, its connection unwatched so often."""
    if asked.get("ask") == "hello":
        offers = {"session": {"live": [], "deep": ["presence"]}}
        return [
            {
                "id": asked.get("id"),
                "version": 1,
                "domain": "kinds",
                "state": "answering",
                "kinds": offers,
            }
        ]
    told = told_after_whole(asked)
    if told is not None:
        return told
    kind = asked.get("kind")
    if isinstance(kind, str) and kind in LAPSING:
        with WAITED_LOCK:
            WAITED[kind] += 1
            first = WAITED[kind] == 1
        before, after = LAPSING[kind]
        waits = before if first else after
        waiting = [{"key": key, "kind": kind, "heard": 1} for key in waits]
        return [{"id": asked.get("id"), "records": waiting}]
    if asked.get("ask") == "steps":
        stages = ["joining", "gettingAddress", "online"]
        return [
            {"id": asked.get("id"), "stage": stage, "done": stage == "online"} for stage in stages
        ]
    if isinstance(kind, str) and kind in FIXED:
        return [{"id": asked.get("id"), "records": FIXED[kind]}]
    kinds = kinds_of(asked)
    keys = asked.get("keys")
    named = (
        [key for key in keys if isinstance(key, str)]
        if isinstance(keys, list)
        else [f"{kind}:one" for kind in kinds]
    )
    records = [
        {
            "key": key,
            "kind": key.split(":", 1)[0],
            "heard": 1,
            "asked": kinds,
            "every": asked.get("every"),
            "unwatched": unwatched,
        }
        for key in named
    ]
    return [{"id": asked.get("id"), "records": records}]


def serve(client: socket.socket) -> None:
    """Answer one client, a line for each line, until it goes."""
    unwatched = 0
    with client, client.makefile("rw") as lines:
        for line in lines:
            try:
                asked = json.loads(line)
            except json.JSONDecodeError:
                continue
            if isinstance(asked, dict):
                if asked.get("ask") == "unwatch":
                    if asked.get("id") == "ahead":
                        unwatched += 1
                    lines.write(json.dumps({"id": asked.get("id"), "unwatched": True}) + "\n")
                    lines.flush()
                    continue
                for said in answer(asked, unwatched):
                    lines.write(json.dumps(said) + "\n")
                lines.flush()
                # The first watch of a kind that loses records is followed by a gap.
                kind = asked.get("kind")
                if isinstance(kind, str) and WAITED.get(kind) == 1:
                    return


def main() -> None:
    """Listen on the socket given for as long as it runs."""
    path = Path(sys.argv[1])
    path.unlink(missing_ok=True)
    listening = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    listening.bind(str(path))
    listening.listen()
    while True:
        client, _ = listening.accept()
        threading.Thread(target=serve, args=(client,), daemon=True).start()


if __name__ == "__main__":
    main()

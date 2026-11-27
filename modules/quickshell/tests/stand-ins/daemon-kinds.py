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
answered with none. An ask of steps
is answered in three, as wifid answers a join: two saying they aren't done, then the last.
An unwatch is answered as wifid answers one, and each record says how many watches ahead its
connection has unwatched, so the test can tell an emptied watch ahead was let go.
An ask to keep is refused for now the first time, as the agents daemon refuses an act it does as
Joel while it can't yet tell the shell asked, and taken after; an ask of what was asked to keep
is answered with each value in turn, so the test can tell a refused ask was asked again as it
was then given, and a taken one no more. An ask to keep through a drop is answered the first
time by the connection dropped, unanswered, as a daemon restarting at a switch drops it, and
taken after. An ask to hold is refused for now twice, so two given at once are both refused,
and taken after. An ask to refuse is refused at every try, so the test can tell one given up
on is said. An ask to forget is refused twice, so two given as words are each asked again; an
ask to linger is refused three times, so one kept for an older try's few would be given up on.
Each of those is answered, as asked, with every value asked of it in turn.
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

# Each value asked to keep, in the order asked, the first refused.
KEPT: list[object] = []
KEPT_LOCK = threading.Lock()

# Each value asked of the asks that drop or refuse more than once, by ask, in the order asked,
# and how many of each first go unanswered or refused.
TRIED: dict[str, list[object]] = {
    "keep-dropped": [],
    "hold": [],
    "refuse": [],
    "forget": [],
    "linger": [],
}
UNTAKEN = {"keep-dropped": 1, "hold": 2, "refuse": sys.maxsize, "forget": 2, "linger": 3}

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


def refused(asked: dict[str, object]) -> list[dict[str, object]]:
    """Refuse an ask for now, as the agents daemon refuses one it can't yet tell the shell asked."""
    return [{"id": asked.get("id"), "error": {"code": "refused", "message": "not yet"}}]


def taken(asked: dict[str, object]) -> list[dict[str, object]]:
    """Take an ask, saying the value it was given."""
    return [{"id": asked.get("id"), "kept": asked.get("value")}]


def keeping(asked: dict[str, object]) -> list[dict[str, object]] | None:
    """Answer an ask to keep or hold, or of what one was asked, or None for another ask."""
    ask = asked.get("ask")
    if ask == "keep":
        with KEPT_LOCK:
            KEPT.append(asked.get("value"))
            first = len(KEPT) == 1
        return refused(asked) if first else taken(asked)
    if ask == "kept":
        with KEPT_LOCK:
            return [{"id": asked.get("id"), "asked": list(KEPT)}]
    if isinstance(ask, str) and ask in TRIED:
        with KEPT_LOCK:
            TRIED[ask].append(asked.get("value"))
            untaken = len(TRIED[ask]) <= UNTAKEN[ask]
        if not untaken:
            return taken(asked)
        # A drop leaves the ask unanswered; serve hangs up after it.
        return [] if ask == "keep-dropped" else refused(asked)
    if isinstance(ask, str) and ask.startswith("tried:"):
        with KEPT_LOCK:
            return [{"id": asked.get("id"), "asked": list(TRIED.get(ask[6:], []))}]
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
    for answering in (told_after_whole, keeping):
        said = answering(asked)
        if said is not None:
            return said
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
    if asked.get("kind") == "verdict":
        return [{"id": asked.get("id"), "records": []}]
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
                # So is the first ask to keep through a drop, left unanswered.
                if asked.get("ask") == "keep-dropped" and len(TRIED["keep-dropped"]) == 1:
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

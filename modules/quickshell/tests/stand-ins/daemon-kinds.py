"""Stand in for a daemon holding more than one kind of record, as Google's does.

It answers the shell's daemon interface (RSCH-20261029-0E8050) on the socket named as its one
argument: hello, and a watch, answered with one record for each key asked, or else one for each
kind asked, `<kind>:one`. Each record says the kinds its watch asked for, so tests/daemon.qml can
tell the client sent its list of kinds in every watch, the one ahead too, and not one kind alone.
A watch of sessions is answered as mycelium's node answers one: a record whole, then a change
saying only the fields that differ, one dropped as null. A watch of what waits is answered with
two records the first time, and the connection then dropped, and with one of them alone after,
as the node's whole answer is once one went in a gap; one of verdicts with none. An ask of steps
is answered in three, as wifid answers a join: two saying they aren't done, then the last.
"""

import json
import socket
import sys
import threading
from pathlib import Path

# How many watches of what waits have been answered, so the first is told both and the rest one.
WAITED = {"count": 0}
WAITED_LOCK = threading.Lock()


def kinds_of(asked: dict[str, object]) -> list[str]:
    """Read the kinds a watch names, one or a list, as Google's daemon reads them."""
    kind = asked.get("kind")
    if isinstance(kind, str):
        return [kind]
    if isinstance(kind, list):
        return [each for each in kind if isinstance(each, str)]
    return []


def answer(asked: dict[str, object]) -> list[dict[str, object]]:
    """Answer one ask, in the lines a daemon sends for it."""
    if asked.get("ask") == "hello":
        return [{"id": asked.get("id"), "version": 1, "domain": "kinds", "state": "answering"}]
    if asked.get("kind") == "session":
        whole = {"key": "session:a", "kind": "session", "heard": 1}
        return [
            {"id": asked.get("id"), "records": [{**whole, "state": "idle", "step": "x"}]},
            {
                "id": asked.get("id"),
                "changed": [{"key": "session:a", "state": "busy", "step": None}],
            },
        ]
    if asked.get("kind") == "waiting":
        with WAITED_LOCK:
            WAITED["count"] += 1
            first = WAITED["count"] == 1
        waits = ["waiting:a", "waiting:b"] if first else ["waiting:b"]
        waiting = [{"key": key, "kind": "waiting", "heard": 1} for key in waits]
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
        {"key": key, "kind": key.split(":", 1)[0], "heard": 1, "asked": kinds} for key in named
    ]
    return [{"id": asked.get("id"), "records": records}]


def serve(client: socket.socket) -> None:
    """Answer one client, a line for each line, until it goes."""
    with client, client.makefile("rw") as lines:
        for line in lines:
            try:
                asked = json.loads(line)
            except json.JSONDecodeError:
                continue
            if isinstance(asked, dict):
                for said in answer(asked):
                    lines.write(json.dumps(said) + "\n")
                lines.flush()
                # The first watch of what waits is followed by a gap.
                if asked.get("kind") == "waiting" and WAITED["count"] == 1:
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

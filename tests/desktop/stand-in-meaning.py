"""Stand in for the meaning daemon on the socket it keeps (modules/meaning).

    stand-in-meaning SOCKET MEANS

It speaks the daemon interface, version 1, as the real one does
(modules/dictation/src/bin/meaning/known.rs), for kind thing: hello, a watch, which holds
nothing, a know, which takes the launcher's things and says how many, and a search, which
gives the things MEANS names for the words asked, if the launcher told of them. MEANS is a JSON
object of keys by the words that mean them, read again at every search, so a test writes it
rather than loading a model. Each connection, each know with how many things it told, and each
search are written down in meaning-asked in the runtime folder, a line each, so a test can see
what the shell asked and when; the keys told last are in meaning-known, a line each, so a test
can name things the launcher holds.
"""

import json
import socket
import sys
import threading
from pathlib import Path

SOCKET = Path(sys.argv[1])
MEANS = Path(sys.argv[2])
ASKED = SOCKET.parent.parent / "meaning-asked"
KNOWN_KEYS = SOCKET.parent.parent / "meaning-known"
KIND = "thing"
WRITING = threading.Lock()
# What the launcher told of last, by key, as the daemon keeps it between connections.
KNOWN: set[str] = set()


def written(line: str) -> None:
    """Write down one thing asked, whole, as the test reads it."""
    with WRITING, ASKED.open("a") as asked:
        asked.write(f"{line}\n")


def means() -> dict[str, list[str]]:
    """Give the keys each text means, or nothing before a test writes it."""
    try:
        said = json.loads(MEANS.read_text())
    except OSError, ValueError:
        return {}
    return {str(words): [str(key) for key in keys] for words, keys in said.items()}


def search(words: str) -> list[dict[str, object]]:
    """Give the known things the words mean, nearest first, as the daemon does."""
    keys = [key for key in means().get(words, []) if key in KNOWN]
    return [
        {"key": key, "kind": KIND, "near": round(0.9 - 0.01 * rank, 3)}
        for rank, key in enumerate(keys)
    ]


def answer(said: dict[str, object]) -> dict[str, object]:
    """Answer one ask as the daemon would."""
    asked = said.get("id")
    match said.get("ask"):
        case "hello":
            return {"id": asked, "version": 1, "domain": "meaning", "state": "answering"}
        case "watch":
            return {"id": asked, "records": []}
        case "know":
            things = said.get("things")
            told = things if isinstance(things, list) else []
            KNOWN.clear()
            KNOWN.update(str(thing.get("key")) for thing in told if isinstance(thing, dict))
            with WRITING:
                KNOWN_KEYS.write_text("".join(f"{key}\n" for key in sorted(KNOWN)))
            written(f"know {len(KNOWN)}")
            return {"id": asked, "known": len(KNOWN), "waiting": 0}
        case "search":
            words = str(said.get("words", ""))
            written(f"search {words}")
            return {"id": asked, "records": search(words), "whole": True}
        case _:
            return {"id": asked, "error": {"code": "unknown-ask", "message": "not asked here"}}


def serve(connection: socket.socket) -> None:
    """Answer each line a client says, in order, until it goes, and write down that it went."""
    written("connected")
    with connection, connection.makefile("r") as lines:
        for said in lines:
            connection.sendall(f"{json.dumps(answer(json.loads(said)))}\n".encode())
    written("gone")


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

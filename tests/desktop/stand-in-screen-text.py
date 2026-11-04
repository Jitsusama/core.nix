"""Stand in for the screen-text daemon on the socket it keeps (modules/screen-text).

    stand-in-screen-text SOCKET TEXTS

It speaks the daemon interface, version 1, as the real one does
(modules/dictation/src/bin/screen-text/shelf.rs), for kind screenshot: hello, a watch, which
holds what its keys name and so nothing when they name none, and a search, which gives the
screenshots whose text holds every word asked, those holding them all in one line first, each
with the line that matched. What each screenshot says is TEXTS, a JSON object of lines by
picture path, read again at every search, so a test writes it as it saves a picture rather
than waiting for one to be read. Each connection and each search is written down in
screen-text-asked in the runtime folder, a line each, so a test can count how often it was
asked, and on how many connections.
"""

import json
import socket
import sys
import threading
from pathlib import Path

SOCKET = Path(sys.argv[1])
TEXTS = Path(sys.argv[2])
ASKED = SOCKET.parent.parent / "screen-text-asked"
KIND = "screenshot"
WRITING = threading.Lock()


def written(line: str) -> None:
    """Write down one thing asked, whole, as the test reads it."""
    with WRITING, ASKED.open("a") as asked:
        asked.write(f"{line}\n")


def texts() -> dict[str, list[str]]:
    """Give what each screenshot says, by its path, or nothing before a test writes it."""
    try:
        said = json.loads(TEXTS.read_text())
    except OSError, ValueError:
        return {}
    return {str(path): [str(line) for line in lines] for path, lines in said.items()}


def matched(lines: list[str], words: list[str]) -> tuple[bool, str] | None:
    """Say whether a screenshot's lines hold every word, all in one line or not, and which line."""

    def holds(text: str) -> bool:
        return all(word in text.lower() for word in words)

    if not holds("\n".join(lines)):
        return None
    whole = next((line for line in lines if holds(line)), None)
    if whole is not None:
        return True, whole
    return False, next(line for line in lines if words[0] in line.lower())


def search(words: str) -> list[dict[str, object]]:
    """Give the screenshots holding every word, those in one line first, as the daemon does."""
    asked = words.lower().split()
    if not asked:
        return []
    found = []
    for path, lines in texts().items():
        match = matched(lines, asked)
        if match is not None:
            one, line = match
            record = {"key": f"{KIND}:{path}", "kind": KIND, "path": path, "taken": 0}
            found.append((not one, {**record, "text": "\n".join(lines), "line": line}))
    return [record for _, record in sorted(found, key=lambda each: each[0])]


def answer(said: dict[str, object]) -> dict[str, object]:
    """Answer one ask as the daemon would."""
    asked = said.get("id")
    match said.get("ask"):
        case "hello":
            return {"id": asked, "version": 1, "domain": "screenshots", "state": "answering"}
        case "watch":
            keys = said.get("keys")
            named = keys if isinstance(keys, list) else None
            held = [
                {"key": f"{KIND}:{path}", "kind": KIND, "path": path}
                for path in texts()
                if named is None or f"{KIND}:{path}" in named
            ]
            return {"id": asked, "records": held}
        case "search":
            words = str(said.get("words", ""))
            written(f"search {words}")
            return {"id": asked, "records": search(words)}
        case _:
            return {"id": asked, "error": {"code": "unknown-ask", "message": "not asked here"}}


def serve(connection: socket.socket) -> None:
    """Answer each line a client says, in order, until it goes."""
    written("connected")
    with connection, connection.makefile("r") as lines:
        for said in lines:
            connection.sendall(f"{json.dumps(answer(json.loads(said)))}\n".encode())


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

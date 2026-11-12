"""Stand in for the eero daemon on the socket it keeps (modules/eero/src/listeners.rs).

The real one asks eero's cloud, which the VM can't reach. This speaks the daemon interface,
version 1, as the real one does: hello, a watch of the home network answered with its one
record, kind and key `home-network`, the model whole under `network`, a watch ahead of it
answered the same, an unwatch, and an ask to read, answered at once. The model is the file
named as its second argument, the one the daemon's parsers are tested against (modules/eero),
so the Home Network has a whole three-node network to show. A watch of rows, one naming the
row kind or a place, is answered with the records named as its third argument, those the daemon
serves of that model, each row ordered and the place's own drawer beside them
(modules/eero/tests/rows.json, which a cargo test holds to the daemon's), or only those it
names. Each such watch is said on its output, with the words it asks in, so a test sees the
shell watch them. It never changes, so nothing is told after a
watch's answer.
"""

import json
import socketserver
import sys
from pathlib import Path

SOCKET = Path(sys.argv[1])
KIND = "home-network"
RECORD = {"key": KIND, "kind": KIND, "network": json.loads(Path(sys.argv[2]).read_text())}
ROW = "row"
# The kinds served beside each other, as served.rs's KINDS: the rows, and the place's drawer.
KINDS = (ROW, "drawer")
PLACE = "eero"
# The rows are named as a third argument, which a check that draws no row may leave out.
ROWS = [record for named in sys.argv[3:4] for record in json.loads(Path(named).read_text())]


def asks_rows(said: dict[str, object]) -> bool:
    """Whether a watch is of rows, as the daemon's served.rs tells: the row kind or a place."""
    kind = said.get("kind")
    return kind == ROW or (isinstance(kind, list) and ROW in kind) or "place" in said


def watched(said: dict[str, object]) -> dict[str, object]:
    """Answer a watch with the records it asks for, or say why its rows aren't kept here."""
    keys = said.get("keys")
    named = None if not isinstance(keys, list) else set(keys)
    if not asks_rows(said):
        shown = named is None or KIND in named
        return {"id": said.get("id"), "records": [RECORD] if shown else []}
    kind = said.get("kind")
    kinds = kind if isinstance(kind, list) else [] if kind is None else [kind]
    place = said.get("place", PLACE)
    if any(one not in KINDS for one in kinds) or place != PLACE:
        message = f"no kind {kinds} or place {place} here"
        return {"id": said.get("id"), "error": {"code": "unknown-kind", "message": message}}
    records = [record for record in ROWS if named is None or record["key"] in named]
    print(f"watched {PLACE}'s rows in {said.get('words', 'no words')}", flush=True)
    return {"id": said.get("id"), "records": records}


def answer(said: dict[str, object]) -> dict[str, object]:
    """Answer one ask as the daemon does."""
    asked = said.get("ask")
    if asked == "hello":
        return {"id": said.get("id"), "state": "answering", "version": 1}
    if asked == "watch":
        return watched(said)
    if asked == "unwatch":
        return {"id": said.get("id"), "unwatched": True}
    if asked == "read":
        return {"id": said.get("id")}
    return {"id": said.get("id"), "error": "the eero daemon doesn't know that ask"}


class Connection(socketserver.StreamRequestHandler):
    """One client, each line it says answered in order until it goes."""

    def handle(self) -> None:
        """Answer each line as it comes."""
        for said in self.rfile:
            self.wfile.write(f"{json.dumps(answer(json.loads(said)))}\n".encode())


SOCKET.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
SOCKET.unlink(missing_ok=True)
# Each client on a thread of its own, as the daemon serves each, so the shell's held connection
# never keeps a test's own ask waiting.
with socketserver.ThreadingUnixStreamServer(str(SOCKET), Connection) as server:
    server.daemon_threads = True
    server.serve_forever()

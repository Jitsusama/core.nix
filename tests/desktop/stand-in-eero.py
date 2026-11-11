"""Stand in for the eero daemon on the socket it keeps (modules/eero/src/listeners.rs).

The real one asks eero's cloud, which the VM can't reach. This speaks the daemon interface,
version 1, as the real one does: hello, a watch of the home network answered with its one
record, kind and key `home-network`, the model whole under `network`, a watch ahead of it
answered the same, an unwatch, and an ask to read, answered at once. The model is the file
named as its second argument, the one the daemon's parsers are tested against (modules/eero),
so the Home Network has a whole three-node network to show. It never changes, so nothing is
told after a watch's answer.
"""

import json
import socketserver
import sys
from pathlib import Path

SOCKET = Path(sys.argv[1])
KIND = "home-network"
RECORD = {"key": KIND, "kind": KIND, "network": json.loads(Path(sys.argv[2]).read_text())}


def answer(said: dict[str, object]) -> dict[str, object]:
    """Answer one ask as the daemon does."""
    asked = said.get("ask")
    if asked == "hello":
        return {"id": said.get("id"), "state": "answering", "version": 1}
    if asked == "watch":
        keys = said.get("keys")
        shown = keys is None or (isinstance(keys, list) and KIND in keys)
        return {"id": said.get("id"), "records": [RECORD] if shown else []}
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

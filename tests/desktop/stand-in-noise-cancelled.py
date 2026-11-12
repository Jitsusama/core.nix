"""Stand in for the noise-cancelled daemon's trial, on the socket it keeps.

The real one records the default microphone and cleans it with each cleaner, which the VM's
stand-in microphone has nothing worth cleaning for. This speaks the daemon interface, version
1, and the trial protocol (QEST-20261111-JCWNFD's trial-protocol.md) as the real one does:
hello, a watch of kind `trial` answered with its one record, `trial:now`, and each act answered
`taken`, the record told whole to every watch as it changes. A recording runs a second, then
cleans for a moment, so a walk through it takes seconds; the clips aren't shuffled, so a test
knows what it picked: A is RNNoise, B DeepFilterNet gentle, C DeepFilterNet full, D none.
"""

import json
import socketserver
import sys
import threading
import time
from pathlib import Path

SOCKET = Path(sys.argv[1])
KIND = "trial"
KEY = "trial:now"
SECONDS = 1
CLEANING = 0.5
REVEALED = {"A": "rnnoise", "B": "deepfilternet-gentle", "C": "deepfilternet", "D": "raw"}
COSTS = {"raw": 0, "rnnoise": 0.012, "deepfilternet": 0.31, "deepfilternet-gentle": 0.3}

lock = threading.Lock()
watchers: list[Connection] = []
record: dict[str, object] = {}


def idle() -> dict[str, object]:
    """Give the record as it stands with nothing under way."""
    return {
        "key": KEY,
        "kind": KIND,
        "heard": int(time.time() * 1000),
        "stage": "idle",
        "why": None,
        "microphone": None,
        "seconds": SECONDS,
        "until": None,
        "clips": [],
        "playing": None,
        "picked": None,
        "revealed": None,
        "costs": None,
        "running": record.get("running", "raw") if record else "raw",
    }


def change(fields: dict[str, object]) -> None:
    """Lay these fields over the record and tell every watch it whole."""
    with lock:
        record.update(fields, heard=int(time.time() * 1000))
        line = f"{json.dumps({'changed': [record]})}\n".encode()
        for watcher in list(watchers):
            try:
                watcher.wfile.write(line)
                watcher.wfile.flush()
            except OSError:
                watchers.remove(watcher)


def recorded() -> None:
    """Finish a recording, clean it, then offer the clips."""
    time.sleep(SECONDS)
    change({"stage": "cleaning", "until": None})
    time.sleep(CLEANING)
    change({"stage": "ready", "clips": sorted(REVEALED)})


def act(said: dict[str, object]) -> dict[str, object]:
    """Take one act as the protocol says, or refuse it in plain words."""
    name = said.get("act")
    clip = said.get("clip")
    stage = record["stage"]
    if name == "record" and stage in {"idle", "picked", "refused"}:
        change(
            {
                "stage": "recording",
                "microphone": "stand-in microphone",
                "until": int(time.time() * 1000) + SECONDS * 1000,
            }
        )
        threading.Thread(target=recorded, daemon=True).start()
    elif name == "play" and stage == "ready" and clip in REVEALED:
        change({"playing": clip})
    elif name == "stop" and stage == "ready":
        change({"playing": None})
    elif name == "pick" and stage == "ready" and clip in REVEALED:
        change(
            {
                "stage": "picked",
                "clips": [],
                "playing": None,
                "picked": {"clip": clip, "cleaner": REVEALED[str(clip)]},
                "revealed": REVEALED,
                "costs": COSTS,
                "running": REVEALED[str(clip)],
            }
        )
    elif name == "discard":
        change(idle())
    else:
        return {"id": said.get("id"), "error": {"code": "refused", "message": "not now"}}
    return {"id": said.get("id"), "taken": True}


class Connection(socketserver.StreamRequestHandler):
    """One client, each line it says answered in order until it goes."""

    def handle(self) -> None:
        """Answer each line as it comes, and let its watch go once it does."""
        try:
            for line in self.rfile:
                self.answer(json.loads(line))
        finally:
            with lock:
                if self in watchers:
                    watchers.remove(self)

    def answer(self, said: dict[str, object]) -> None:
        """Answer one ask as the daemon does."""
        asked = said.get("ask")
        if asked == "hello":
            said_back = {
                "id": said.get("id"),
                "state": "answering",
                "version": 1,
                "domain": "noise cancelling",
            }
        elif asked == "watch":
            with lock:
                if self not in watchers:
                    watchers.append(self)
            said_back = {"id": said.get("id"), "records": [record]}
        elif asked == "unwatch":
            with lock:
                if self in watchers:
                    watchers.remove(self)
            said_back = {"id": said.get("id"), "unwatched": True}
        elif asked == "act":
            said_back = act(said)
        else:
            said_back = {"id": said.get("id"), "error": {"code": "unknown", "message": "no"}}
        with lock:
            self.wfile.write(f"{json.dumps(said_back)}\n".encode())
            self.wfile.flush()


record.update(idle())
SOCKET.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
SOCKET.unlink(missing_ok=True)
# Each client on a thread of its own, as the daemon serves each, so the shell's held connection
# never keeps a test's own ask waiting.
with socketserver.ThreadingUnixStreamServer(str(SOCKET), Connection) as server:
    server.daemon_threads = True
    server.serve_forever()

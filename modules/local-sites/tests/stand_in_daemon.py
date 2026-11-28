"""A stand-in daemon for a page's test, speaking version 1 of the daemon interface.

A page's test runs the real sites' server beside this, in place of the daemon
its feed reads, so the page is read as it will be on the machine without the
daemon or its machine. It answers hello, list, watch and health on a unix
socket from records it is handed by kind, and the test moves it as a real
daemon moves: `tell` sends a change to every watch of a kind, `leave` closes
every connection and the socket, as a daemon stopping does, and `start` opens
it again. `withhold` keeps a kind's first answer back until `answer`, as a
node slow to read one kind is. docs/daemons.md says what each answer holds.
"""

import json
import socket
import threading
from typing import TYPE_CHECKING, Self

if TYPE_CHECKING:
    from collections.abc import Iterable, Iterator, Mapping
    from pathlib import Path

type Record = dict[str, object]


def standing_in(path: Path, domain: str, reading: Path) -> Iterator[StandIn]:
    """Stand in on `path` with the records by kind `reading` holds, then leave, as a fixture."""
    stand_in = StandIn(path, domain, json.loads(reading.read_text())["kinds"]).start()
    yield stand_in
    stand_in.leave()


class StandIn:
    """A daemon on `path` holding `records` by kind, for as long as a test needs."""

    def __init__(self, path: Path, domain: str, records: Mapping[str, Iterable[Record]]) -> None:
        """Hold the records; nothing listens until `start`."""
        self.path = path
        self.domain = domain
        self.records = {kind: {str(r["key"]): r for r in rs} for kind, rs in records.items()}
        self.lock = threading.Lock()
        self.watches: list[tuple[socket.socket, object, str]] = []
        # The watches of each kind held back, waiting for their first answer.
        self.withheld: dict[str, list[tuple[socket.socket, object]]] = {}
        self.connections: list[socket.socket] = []
        self.server: socket.socket | None = None

    def start(self) -> Self:
        """Listen on the socket, as the daemon starting does."""
        self.path.unlink(missing_ok=True)
        server = socket.socket(socket.AF_UNIX)
        server.bind(str(self.path))
        server.listen()
        self.server = server
        threading.Thread(target=self._accept, args=(server,), daemon=True).start()
        return self

    def leave(self) -> None:
        """Close every connection and the socket, as the daemon stopping does."""
        with self.lock:
            if self.server is not None:
                self.server.close()
                self.server = None
            for conn in self.connections:
                # A reader's open file holds the socket open past close, so
                # the connection is shut first, which the other end hears.
                conn.shutdown(socket.SHUT_RDWR)
                conn.close()
            self.connections.clear()
            self.watches.clear()
        self.path.unlink(missing_ok=True)

    def tell(self, kind: str, changed: Iterable[Record] = (), gone: Iterable[str] = ()) -> None:
        """Mend the records and tell every watch of `kind` only what moved."""
        changed, gone = list(changed), list(gone)
        with self.lock:
            held = self.records.setdefault(kind, {})
            for change in changed:
                record = held.setdefault(str(change["key"]), {"key": change["key"]})
                for field, value in change.items():
                    if value is None:
                        record.pop(field, None)
                    else:
                        record[field] = value
            for key in gone:
                held.pop(key, None)
            for conn, ask, watched in list(self.watches):
                if watched == kind:
                    self._send(conn, {"id": ask, "changed": changed, "gone": gone})

    def withhold(self, kind: str) -> None:
        """Keep each watch of `kind` from its first answer until `answer` sends it."""
        with self.lock:
            self.withheld.setdefault(kind, [])

    def answer(self, kind: str) -> None:
        """Send each watch of `kind` held back its first answer, and answer the rest at once."""
        with self.lock:
            waiting = self.withheld.pop(kind, [])
            records = list(self.records.get(kind, {}).values())
            for conn, ask in waiting:
                self.watches.append((conn, ask, kind))
                self._send(conn, {"id": ask, "records": records})

    def _accept(self, server: socket.socket) -> None:
        while True:
            try:
                conn, _ = server.accept()
            except OSError:
                # The socket was closed by leave, which is how this ends.
                return
            with self.lock:
                self.connections.append(conn)
            threading.Thread(target=self._serve, args=(conn,), daemon=True).start()

    def _serve(self, conn: socket.socket) -> None:
        try:
            for line in conn.makefile(encoding="utf-8"):
                self._answer(conn, json.loads(line))
        except OSError, ValueError:
            # A connection closed under it, by leave or by the server going,
            # ends its thread; nothing is owed to it.
            return

    def _answer(self, conn: socket.socket, ask: dict[str, object]) -> None:
        ask_id, verb, kind = ask.get("id"), ask.get("ask"), str(ask.get("kind"))
        if verb == "hello":
            kinds: dict[str, dict[str, list[str]]] = {
                k: {"live": [], "deep": []} for k in self.records
            }
            hello = {"version": 1, "domain": self.domain, "state": "answering", "kinds": kinds}
            self._send(conn, {"id": ask_id, **hello})
        elif verb == "health":
            self._send(conn, {"id": ask_id, "state": "answering"})
        elif verb in {"list", "watch"} and kind in self.records:
            with self.lock:
                records = list(self.records[kind].values())
                if verb == "watch" and kind in self.withheld:
                    self.withheld[kind].append((conn, ask_id))
                    return
                if verb == "watch":
                    self.watches.append((conn, ask_id, kind))
            self._send(conn, {"id": ask_id, "records": records})
        else:
            refused = {"code": "unknown-kind", "message": f"no {kind} here"}
            self._send(conn, {"id": ask_id, "error": refused})

    def _send(self, conn: socket.socket, answer: Mapping[str, object]) -> None:
        try:
            conn.sendall((json.dumps(answer) + "\n").encode())
        except OSError:
            # A watch whose page has gone is passed over; its thread ends it.
            return

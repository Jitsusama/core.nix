"""Stand in for the google program, which would ask Google, on the socket it keeps.

It speaks the program's protocol (modules/google/src/state.rs and ask.rs): the whole state as one
line of JSON, left in google.json beside the socket, which the shell reads, and told to a shell
that connects. The account is ready, with one meeting, Standup, half an hour from when the
stand-in starts, joined on Meet. An answer a shell writes is written down in google-asked beside
the socket, for the test to read, and taken as Google would take it, so the file and every shell
say the meeting answered.
"""

import json
import socket
import sys
import threading
import time
from pathlib import Path

# Milliseconds in a minute, as the program counts times.
MINUTE = 60_000
# How far ahead the meeting starts, and how long it lasts.
AHEAD = 30 * MINUTE
LASTS = 30 * MINUTE


class Account:
    """The account as the program would hold it, the file it leaves, and the shells it tells."""

    def __init__(self, told: Path, asked: Path) -> None:
        """Hold the one meeting, answered yes, with no shell yet, and leave it in the file."""
        now = int(time.time() * 1000)
        self.told = told
        self.asked = asked
        self.lock = threading.Lock()
        self.shells: list[socket.socket] = []
        self.meeting = {
            "id": "standup",
            "title": "Standup",
            "start": now + AHEAD,
            "end": now + AHEAD + LASTS,
            "allDay": False,
            "answer": "yes",
            "link": "https://meet.google.com/abc-defg-hij",
            "kind": "meet",
            "page": "https://calendar.google.com/calendar/event?eid=standup",
        }
        self.heard = now
        self.leave()

    def line(self) -> bytes:
        """Give the state the program would say."""
        said = {"account": "ready", "meetings": [self.meeting], "heard": self.heard}
        return f"{json.dumps(said)}\n".encode()

    def leave(self) -> None:
        """Leave the state in the file, replacing it whole as the program does."""
        new = self.told.with_name(f"{self.told.name}.new")
        new.write_bytes(self.line())
        new.replace(self.told)

    def welcome(self, shell: socket.socket) -> None:
        """Tell a shell the state and keep it for each change, unless it's already gone."""
        with self.lock:
            try:
                shell.sendall(self.line())
            except OSError:
                return
            self.shells.append(shell)

    def ask(self, line: str) -> None:
        """Write down what a shell asked, and answer the meeting it names."""
        with self.asked.open("a") as asked:
            asked.write(f"{line}\n")
        answer = json.loads(line).get("answer", {})
        with self.lock:
            if answer.get("id") == self.meeting["id"]:
                self.meeting["answer"] = answer["with"]
            self.leave()
            for shell in list(self.shells):
                try:
                    shell.sendall(self.line())
                except OSError:
                    self.shells.remove(shell)


def keep(account: Account, shell: socket.socket) -> None:
    """Tell a shell the state, then hear what it asks until it goes."""
    try:
        account.welcome(shell)
        for line in shell.makefile("r", encoding="utf-8"):
            account.ask(line.strip())
    except OSError, ValueError:
        pass
    finally:
        shell.close()


def main() -> None:
    """Leave the state beside the socket the program is given, and tell each shell that connects."""
    path = Path(sys.argv[1])
    account = Account(path.with_suffix(".json"), path.with_name("google-asked"))
    # One left by a stand-in before it would refuse the name.
    path.unlink(missing_ok=True)
    server = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    server.bind(str(path))
    server.listen()
    while True:
        shell, _ = server.accept()
        threading.Thread(target=keep, args=(account, shell), daemon=True).start()


if __name__ == "__main__":
    main()

"""Stand in for the google program, which would ask Google, on the socket it keeps.

It speaks the program's protocol (modules/google/src/state.rs and ask.rs): the whole state as one
line of JSON, left in google.json beside the socket, which the shell reads, and told to a shell
that connects. The account is ready, with two meetings on Meet from when the stand-in starts:
Design Review five minutes on, about to start, Joel's own, and Standup half an hour on, Ada's in
Room 4, with Grace not going. An answer a shell
writes is written down in google-asked beside the socket, for the test to read, and taken as
Google would take it, so the file and every shell say the meeting answered. Two pieces of mail
want Joel, Lunch on Friday and Quarterly Numbers, with three unread in the inbox; one marked read
or archived is written down too, and taken out of the mail that wants him, as Gmail would.
"""

import json
import socket
import sys
import threading
import time
from pathlib import Path

# Milliseconds in a minute, as the program counts times.
MINUTE = 60_000
# How far ahead each meeting starts, and how long one lasts.
SOON = 5 * MINUTE
AHEAD = 30 * MINUTE
LASTS = 30 * MINUTE


def meeting(key: str, title: str, start: int, call: str, **more: object) -> dict[str, object]:
    """Give a meeting as the program says one, answered yes, its call on Meet, Joel's own."""
    return {
        "id": key,
        "title": title,
        "start": start,
        "end": start + LASTS,
        "allDay": False,
        "answer": "yes",
        "link": f"https://meet.google.com/{call}",
        "kind": "meet",
        "page": f"https://calendar.google.com/calendar/event?eid={key}",
        "place": None,
        "about": None,
        "holds": True,
        "organizer": None,
        "guests": [],
        **more,
    }


class Account:
    """The account as the program would hold it, the file it leaves, and the shells it tells."""

    def __init__(self, told: Path, asked: Path) -> None:
        """Hold the meetings, answered yes, with no shell yet, and leave them in the file."""
        now = int(time.time() * 1000)
        self.told = told
        self.asked = asked
        self.lock = threading.Lock()
        self.shells: list[socket.socket] = []
        self.meetings = [
            meeting("review", "Design Review", now + SOON, "pqr-stuv-wxy"),
            meeting(
                "standup",
                "Standup",
                now + AHEAD,
                "abc-defg-hij",
                place="Room 4",
                about="What we did yesterday,\nwhat we do today.",
                holds=False,
                organizer={"name": "Ada Lovelace", "email": "ada@example.com"},
                guests=[{"name": "Grace Hopper", "email": "grace@example.com", "answer": "no"}],
            ),
        ]
        self.mail = [
            {
                "id": "lunch",
                "from": "Ada Lovelace",
                "subject": "Lunch on Friday",
                "at": now - 2 * MINUTE,
                "page": "https://mail.google.com/mail/u/0/#inbox/lunch",
            },
            {
                "id": "numbers",
                "from": "Grace Hopper",
                "subject": "Quarterly Numbers",
                "at": now - 5 * MINUTE,
                "page": "https://mail.google.com/mail/u/0/#inbox/numbers",
            },
        ]
        self.unread = 3
        self.heard = now
        self.leave()

    def line(self) -> bytes:
        """Give the state the program would say."""
        said = {
            "account": "ready",
            "meetings": self.meetings,
            "mail": self.mail,
            "unread": self.unread,
            "heard": self.heard,
        }
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
        """Write down what a shell asked, and answer the meeting or tidy the mail it names."""
        with self.asked.open("a") as asked:
            asked.write(f"{line}\n")
        said = json.loads(line)
        answer = said.get("answer", {})
        tidied = (said.get("read") or said.get("archive") or {}).get("id")
        with self.lock:
            for one in self.meetings:
                if answer.get("id") == one["id"]:
                    one["answer"] = answer["with"]
            held = [mail for mail in self.mail if mail["id"] != tidied]
            if len(held) < len(self.mail):
                self.mail = held
                self.unread -= 1
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

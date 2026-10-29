"""Stand in for the google program, which would ask Google, on the socket it keeps.

It speaks the program's protocol (modules/google/src/state.rs and ask.rs): the whole state as one
line of JSON, left in google.json beside the socket, which the shell reads, and told to a shell
that connects. The account is ready, unless a test signs it out, with three meetings on Meet
from when the stand-in starts: Design Review five minutes on, Joel's own, reminding him a
minute before, or nearer or further as a test asks; Standup half an hour on, Ada's in Room 4,
with Grace not going; and Planning two hours on, Joel's own with Grace asked. Joel may change
his own, and Standup on Ada's calendar, which she shares with him. An answer a shell writes is
written down in google-asked beside the socket, for the test to read, and taken as Google would
take it, so the file and every shell say the meeting answered or moved; a meeting Joel holds
that's cancelled is written down and let go, as Google would. Two pieces of mail want Joel,
Lunch on Friday from Ada and Quarterly Numbers from Grace, each to Joel and saying a line,
with three unread in the inbox; one marked read or archived is written down too, and taken
out of the mail that wants him, as Gmail would, and one read and then marked unread again
wants him once more. Joel looked at two documents lately: Ada's Quarterly Plan in Docs, and
Team Budget in Sheets, on a shared drive.
"""

import json
import os
import socket
import sys
import threading
import time
from pathlib import Path

# Milliseconds in a minute, as the program counts times.
MINUTE = 60_000
# How far ahead each meeting starts, and how long one lasts. A test may set
# Design Review nearer or further, in milliseconds, through the user
# manager's environment, to see it want Joel and be said.
SOON = int(os.environ.get("GOOGLE_STAND_IN_SOON", str(5 * MINUTE)))
# Whether the account is ready, or signed out as a test asks, when it holds
# nothing, as the program says it (state.rs).
SIGNED_OUT = os.environ.get("GOOGLE_STAND_IN_ACCOUNT") == "signedOut"
AHEAD = 30 * MINUTE
LATER = 120 * MINUTE
LASTS = 30 * MINUTE


def meeting(key: str, title: str, start: int, call: str, **more: object) -> dict[str, object]:
    """Give a meeting as the program says one: answered yes, on Meet, Joel's, with no reminder."""
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
        "through": "primary",
        "organizer": None,
        "guests": [],
        "reminders": [],
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
            meeting("review", "Design Review", now + SOON, "pqr-stuv-wxy", reminders=[1]),
            meeting(
                "standup",
                "Standup",
                now + AHEAD,
                "abc-defg-hij",
                place="Room 4",
                about="What we did yesterday,\nwhat we do today.",
                holds=False,
                through="ada@example.com",
                organizer={"name": "Ada Lovelace", "email": "ada@example.com"},
                guests=[{"name": "Grace Hopper", "email": "grace@example.com", "answer": "no"}],
            ),
            meeting(
                "planning",
                "Planning",
                now + LATER,
                "klm-nopq-rst",
                guests=[{"name": "Grace Hopper", "email": "grace@example.com", "answer": "yes"}],
            ),
        ]
        self.mail = [
            {
                "id": "lunch",
                "from": "Ada Lovelace",
                "address": "ada@example.com",
                "to": "Joel Gerber",
                "subject": "Lunch on Friday",
                "at": now - 2 * MINUTE,
                "page": "https://mail.google.com/mail/u/0/#inbox/lunch",
                "body": "Shall we try the place on the corner?\n\nAda",
                "attachments": ["menu.pdf"],
            },
            {
                "id": "numbers",
                "from": "Grace Hopper",
                "address": "grace@example.com",
                "to": "Joel Gerber, Ada Lovelace",
                "subject": "Quarterly Numbers",
                "at": now - 5 * MINUTE,
                "page": "https://mail.google.com/mail/u/0/#inbox/numbers",
                "body": "The numbers are in the sheet.",
                "attachments": [],
            },
        ]
        self.unread = 3
        # Mail marked read, which marking it unread again brings back.
        self.read: list[dict[str, object]] = []
        self.documents = [
            {
                "id": "plan",
                "title": "Quarterly Plan",
                "kind": "document",
                "link": "https://docs.google.com/document/d/plan/edit",
                "viewed": now - 10 * MINUTE,
                "changed": now - 60 * MINUTE,
                "owner": "Ada Lovelace",
            },
            {
                "id": "budget",
                "title": "Team Budget",
                "kind": "spreadsheet",
                "link": "https://docs.google.com/spreadsheets/d/budget/edit",
                "viewed": now - 20 * MINUTE,
                "changed": now - 30 * MINUTE,
                "owner": None,
            },
        ]
        self.heard = now
        self.leave()

    def line(self) -> bytes:
        """Give the state the program would say."""
        if SIGNED_OUT:
            gone: dict[str, object] = {
                "account": "signedOut",
                "meetings": [],
                "mail": [],
                "unread": None,
                "documents": [],
                "heard": None,
            }
            return f"{json.dumps(gone)}\n".encode()
        said = {
            "account": "ready",
            "meetings": self.meetings,
            "mail": self.mail,
            "unread": self.unread,
            "documents": self.documents,
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
        """Write down what a shell asked, and take it: a meeting answered, moved or cancelled.

        Or mail marked read or archived, or marked unread again after it was read.
        """
        with self.asked.open("a") as asked:
            asked.write(f"{line}\n")
        said = json.loads(line)
        answer = said.get("answer", {})
        tidied = (said.get("read") or said.get("archive") or {}).get("id")
        unread = said.get("unread", {}).get("id")
        cancelled = said.get("cancel", {}).get("id")
        moved = said.get("move", {})
        with self.lock:
            for one in self.meetings:
                if answer.get("id") == one["id"]:
                    one["answer"] = answer["with"]
                if moved.get("id") == one["id"] and one["through"]:
                    one["start"], one["end"] = moved["start"], moved["end"]
            self.meetings = [
                one for one in self.meetings if not (one["id"] == cancelled and one["holds"])
            ]
            held = [mail for mail in self.mail if mail["id"] != tidied]
            if len(held) < len(self.mail):
                if "read" in said:
                    self.read += [mail for mail in self.mail if mail["id"] == tidied]
                self.mail = held
                self.unread -= 1
            back = [mail for mail in self.read if mail["id"] == unread]
            if back:
                self.read = [mail for mail in self.read if mail["id"] != unread]
                self.mail = sorted(self.mail + back, key=lambda mail: -int(str(mail["at"])))
                self.unread += 1
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

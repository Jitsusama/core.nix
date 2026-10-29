"""Stand in for the google program, which would ask Google, on the socket it keeps.

It speaks the program's protocol (modules/google/src/state.rs and ask.rs): the whole state as one
line of JSON, left in google.json beside the socket, which the shell reads, and told to a shell
that connects. The account is ready, unless a test signs it out, with three meetings on Meet
from when the stand-in starts: Design Review five minutes on, Joel's own, reminding him a
minute before, or nearer or further as a test asks; Standup half an hour on, Ada's in Room 4
every weekday, with Grace not going and Ada's Quarterly Plan and an agenda attached; and
Planning two hours on, Joel's own with Grace asked. Joel may change his own, and Standup on
Ada's calendar, which she shares with him. An answer a shell writes is written down in
google-asked beside the socket, for the test to read, and taken as Google would take it, so the
file and every shell say the meeting answered or moved; a meeting Joel holds that's cancelled
is written down and let go, as Google would. Two pieces of mail want Joel,
Lunch on Friday from Ada and Quarterly Numbers from Grace, each to Joel and saying a line,
with three unread in the inbox; one marked read or archived is written down too, and taken
out of the mail that wants him, as Gmail would, and one read and then marked unread again
wants him once more. Lunch on Friday carries a menu, which a shell may ask to be fetched, to
open, kept in google-files beside the socket, or to save, where the shell says or else in
Downloads, said as fetched as the program says it. Joel looked at two documents lately: Ada's
Quarterly Plan in Docs, and Team Budget in Sheets, on a shared drive. His sign-in grants Meet's
scope, so a shell may ask for a new Meet meeting, which is made at once as new-abcd-efg.
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
        "attachments": [],
        "dial": None,
        "repeats": None,
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
                repeats={
                    "every": "week",
                    "interval": 1,
                    "days": [{"day": day, "nth": None} for day in range(1, 6)],
                    "monthDays": [],
                    "times": None,
                    "until": None,
                },
                about="What we did yesterday,\nwhat we do today.",
                holds=False,
                through="ada@example.com",
                organizer={"name": "Ada Lovelace", "email": "ada@example.com"},
                guests=[{"name": "Grace Hopper", "email": "grace@example.com", "answer": "no"}],
                attachments=[
                    {
                        "title": "Quarterly Plan",
                        "link": "https://docs.google.com/document/d/plan/edit",
                        "id": "plan",
                        "kind": "document",
                    },
                    {
                        "title": "Agenda.pdf",
                        "link": "https://drive.google.com/file/d/agenda/view",
                        "id": None,
                        "kind": None,
                    },
                ],
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
                "cc": "",
                "others": [],
                "subject": "Lunch on Friday",
                "at": now - 2 * MINUTE,
                "page": "https://mail.google.com/mail/u/0/#inbox/lunch",
                "body": "Shall we try the place on the corner?\n\nAda",
                "attachments": [
                    {"name": "menu.pdf", "kind": "application/pdf", "size": 4, "runs": False}
                ],
            },
            {
                "id": "numbers",
                "from": "Grace Hopper",
                "address": "grace@example.com",
                "to": "Joel Gerber, Ada Lovelace",
                "cc": "",
                "others": ["ada@example.com"],
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
        # The file last asked to be fetched, as the program says it (src/files.rs).
        self.fetched: dict[str, object] | None = None
        # The Meet meeting last asked to be made, as the program says it (src/state.rs).
        self.made: dict[str, object] | None = None
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
                "fetched": None,
                "meet": {"granted": False, "made": None},
            }
            return f"{json.dumps(gone)}\n".encode()
        said = {
            "account": "ready",
            "meetings": self.meetings,
            "mail": self.mail,
            "unread": self.unread,
            "documents": self.documents,
            "heard": self.heard,
            "fetched": self.fetched,
            "meet": {"granted": True, "made": self.made},
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

        Or mail marked read or archived, or marked unread again after it was read, or a file it
        carries fetched, or a new Meet meeting made, as new-abcd-efg.
        """
        with self.asked.open("a") as asked:
            asked.write(f"{line}\n")
        said = json.loads(line)
        with self.lock:
            if "fetch" in said:
                self.fetch(said["fetch"])
            if "make" in said:
                self.made = {
                    "asked": int(time.time() * 1000),
                    "link": "https://meet.google.com/new-abcd-efg",
                    "failed": False,
                }
            self.schedule(said)
            self.tidy(said)
            self.leave()
            for shell in list(self.shells):
                try:
                    shell.sendall(self.line())
                except OSError:
                    self.shells.remove(shell)

    def schedule(self, said: dict[str, dict[str, object]]) -> None:
        """Take what a shell asked of the meetings: one answered, moved or cancelled."""
        answer = said.get("answer", {})
        cancelled = said.get("cancel", {}).get("id")
        moved = said.get("move", {})
        for one in self.meetings:
            if answer.get("id") == one["id"]:
                one["answer"] = answer["with"]
            if moved.get("id") == one["id"] and one["through"]:
                one["start"], one["end"] = moved["start"], moved["end"]
        self.meetings = [
            one for one in self.meetings if not (one["id"] == cancelled and one["holds"])
        ]

    def tidy(self, said: dict[str, dict[str, object]]) -> None:
        """Take what a shell asked of the mail: marked read, archived or marked unread again."""
        tidied = (said.get("read") or said.get("archive") or {}).get("id")
        unread = said.get("unread", {}).get("id")
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

    def fetch(self, asked: dict[str, object]) -> None:
        """Write a file a piece of mail carries where the program would, and say where."""
        mail = next((one for one in self.mail + self.read if one["id"] == asked["id"]), None)
        files = mail["attachments"] if mail else []
        at = int(str(asked["file"]))
        file = files[at] if isinstance(files, list) and at < len(files) else None
        path, failed = None, None
        if not isinstance(file, dict):
            failed = "gone"
        elif file["runs"] and asked["how"] == "open":
            failed = "runs"
        else:
            folder = (
                self.told.with_name("google-files") / str(asked["id"])
                if asked["how"] == "open"
                else Path.home() / "Downloads"
            )
            path = folder / str(file["name"])
            if asked["how"] == "save" and "to" in asked:
                path = Path(str(asked["to"]))
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(b"menu")
        self.fetched = {
            "mail": asked["id"],
            "file": at,
            "how": asked["how"],
            "asked": int(time.time() * 1000),
            "path": str(path) if path else None,
            "failed": failed,
        }


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

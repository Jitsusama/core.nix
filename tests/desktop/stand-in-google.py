"""Stand in for the google program, which would ask Google, on its daemon's socket.

It speaks the program's daemon interface (modules/google/src/daemon.rs), on the socket systemd
hands it or else the one it's named: each meeting, piece of mail and document a record, and the
account one more, each told to a watch as it changes, and what a shell asks taken as an act, a
line of ask.rs. The account is ready, unless a test signs it out, with three
meetings on Meet from when the stand-in starts: Design Review five minutes on, Joel's own,
reminding him a minute before, or nearer or further as a test asks; Standup half an hour on,
Ada's in Room 4 every weekday, with Grace not going and Ada's Quarterly Plan and an agenda
attached; and Planning two hours on, Joel's own with Grace asked. Joel may change his own, and
Standup on Ada's calendar, which she shares with him. An answer a shell writes is written down
in the runtime folder's google-asked, for the test to read, and taken as Google would take it,
so every watch hears the meeting answered or moved; a meeting Joel holds that's cancelled
is written down and let go, as Google would. Two pieces of mail want Joel,
Lunch on Friday from Ada and Quarterly Numbers from Grace, each to Joel and saying a line,
with three unread in the inbox; one marked read or archived is written down too, and taken
out of the mail that wants him, as Gmail would, and one read and then marked unread again
wants him once more. Lunch on Friday carries a menu, which a shell may ask to be fetched, to
open, kept in the runtime folder's google-files, or to save, where the shell says or else in
Downloads, said as fetched as the program says it. Joel looked at two documents lately: Ada's
Quarterly Plan in Docs, and Team Budget in Sheets, on a shared drive, each also served as
Documents' ready row and drawer, as the daemon makes them, to a watch of their place. Each
meeting is served as Calendar's ready rows and drawers too, made by the program's own code
(src/bin/google-rows.rs, named after the socket, with the launcher's words), and made again
each minute, as the program looks, so Return turns to joining ten minutes before. His
sign-in grants Meet's
scope, so a shell may ask for a new Meet meeting, which is made at once as new-abcd-efg. His
Drive holds a Trips folder, with an itinerary in it, the menu and the Quarterly Plan; Team
Budget is shared with him, and the Team shared drive is empty. A shell may ask what a folder
holds, and for a file in one, to open or pick, kept in the cache home's google/drive as the
program keeps it, or to save, Google's own as their office files.
Drive's own places hold the menu, which he opened last, then the two documents, as its Recent;
the Trips folder, as Starred; Team Budget, as Shared with Me; and the Team shared drive.
What a Recent holds: Joel held Retro a day before the stand-in starts, Ada's on Meet with Grace
going, and once before that, a week earlier, as one series; he read Grace's Trip Notes three
hours before it starts, and an hour before that sent Ada, Grace and Alan Notes on the Plan.
"""

import copy
import json
import os
import socket
import subprocess
import sys
import threading
import time
from pathlib import Path
from typing import cast

# The account's one record, as src/daemon.rs keys it.
ACCOUNT = "account:google"
# The descriptor systemd hands a socket over as, the first after the standard three.
HANDED = 3
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
# How long before the stand-in starts Retro was held, and Trip Notes read, and how long before
# them Retro was held the time before and Notes on the Plan sent.
HELD = 26 * 60 * MINUTE
READ = 3 * 60 * MINUTE
WEEK = 7 * 24 * 60 * MINUTE
SENT = 60 * MINUTE
LATER = 120 * MINUTE
LASTS = 30 * MINUTE
# What Drive's copies kept here take, and the most they may before the oldest go, in bytes.
KEPT_HELD = 412_000_000
KEPT_MOST = 2_000_000_000


def held(key: str, name: str, kind: str, mime: str, size: int | None) -> dict[str, object]:
    """Give a file in Drive as the program lists one (src/drive.rs), changed an hour ago."""
    return {
        "id": key,
        "name": name,
        "kind": kind,
        "mime": mime,
        "size": size,
        "changed": int(time.time() * 1000) - 60 * MINUTE,
        "owner": "Ada Lovelace",
        "link": f"https://drive.google.com/file/d/{key}/view",
        "runs": False,
    }


FOLDER = "application/vnd.google-apps.folder"
# What each Drive folder holds, as the program lists it: folders first, then by name.
FOLDERS = {
    "root": [
        held("trips", "Trips", "folder", FOLDER, None),
        held("menu", "menu.pdf", "file", "application/pdf", 4),
        held("plan", "Quarterly Plan", "document", "application/vnd.google-apps.document", None),
    ],
    "shared": [
        held(
            "budget", "Team Budget", "spreadsheet", "application/vnd.google-apps.spreadsheet", None
        ),
    ],
    "drives": [held("0Team", "Team", "folder", FOLDER, None)],
    "trips": [held("itinerary", "itinerary.pdf", "file", "application/pdf", 9)],
    "0Team": [],
}
# How many downloads and folders are told, as the program tells them.
DOWNLOADS_KEPT = 20
LISTINGS_KEPT = 4
# The ending each of Google's own kinds is downloaded with, as their office program's file.
EXPORTED = {"document": ".docx", "spreadsheet": ".xlsx", "presentation": ".pptx"}


# Drive's places, as the program says them while it holds nothing.
NO_PLACES = {
    "recent": [],
    "starred": {"files": [], "more": False},
    "shared": {"files": [], "more": False},
    "drives": {"files": [], "more": False},
}


def placed(key: str, title: str, kind: str, mime: str, now: int) -> dict[str, object]:
    """Give a file in Drive's places as the program says one, Ada's, changed an hour ago."""
    return {
        "id": key,
        "title": title,
        "kind": kind,
        "link": f"https://drive.google.com/file/d/{key}/view",
        "viewed": None,
        "changed": now - 60 * MINUTE,
        "owner": "Ada Lovelace",
        "mime": mime,
        "size": None,
        "created": now - 7 * 24 * 60 * MINUTE,
        "changedBy": "Grace Hopper",
        "changedByMe": None,
        "sharedWithMe": None,
        "sharedBy": None,
        "shared": False,
        "starred": False,
        "drive": None,
        "about": None,
        "across": None,
        "down": None,
        "lasts": None,
    }


def meeting(key: str, title: str, start: int, call: str, **more: object) -> dict[str, object]:
    """Give a meeting as the program says one: answered yes, on Meet, Joel's, with no reminder."""
    given = {
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
    # How many are on it besides Joel, each address once, as the program counts them.
    organizer = given["organizer"]
    holder = [] if given["holds"] or not isinstance(organizer, dict) else [organizer]
    guests = given["guests"] if isinstance(given["guests"], list) else []
    addresses = {str(one["email"]).lower() for one in [*holder, *guests] if one.get("email")}
    return {**given, "guestsCounted": len(addresses)}


# Each of a document's kinds as the daemon words it (src/document_rows.rs): its glyph, the
# program whose acts open and copy it, and its kind in the drawer's eyebrow.
DOCUMENT_KINDS = {
    "document": ("googleDocument", "docs", "Document"),
    "spreadsheet": ("googleSpreadsheet", "sheets", "Spreadsheet"),
    "presentation": ("googlePresentation", "slides", "Presentation"),
}
# What a file of any other kind is, Drive's.
DRIVE_FILE = ("path", "drive", "File")
# How far apart rows' orders are told, as the ready-rows crate spaces them (src/order.rs).
SPACING = 1024


def document_row(document: dict[str, object], at: int) -> dict[str, object]:
    """Give a document Joel opened lately as its ready row, as src/document_rows.rs makes it."""
    key = str(document["id"])
    glyph, program, _ = DOCUMENT_KINDS.get(str(document["kind"]), DRIVE_FILE)
    viewed = document.get("viewed")
    link = document["link"]
    return {
        "key": f"row:documents:{key}",
        "thing": f"gdoc:{key}",
        "place": "documents",
        "kind": document["kind"],
        "name": document["title"],
        "glyph": glyph,
        "hue": "program",
        "depth": 0,
        "side": [] if viewed is None else [{"at": viewed, "as": "opened", "keep": 1}],
        "drawer": f"drawer:documents:{key}",
        "acts": [
            # Opened through google-opens and copied by the shell, as rows.rs's opens serves
            # them: as asks, which no daemon takes, Return said it couldn't.
            {"key": f"verb:{program}:open", "open": link, "in": "google-opens"},
            {"key": f"verb:{program}:link", "copy": link},
        ],
        "held": {"link": link, "viewed": viewed, "mime": document.get("mime", "")},
        "order": (at + 1) * SPACING,
    }


def document_drawer(document: dict[str, object]) -> dict[str, object]:
    """Give a document's drawer as src/document_rows.rs makes it, of what the stand-in holds."""
    _, _, kind = DOCUMENT_KINDS.get(str(document["kind"]), DRIVE_FILE)
    facts: list[dict[str, object]] = []
    if document.get("changed") is not None:
        facts.append({"name": "Changed", "at": document["changed"], "as": "moment"})
    if document.get("owner"):
        facts.append({"name": "Owner", "said": document["owner"]})
    facts.append({"name": "ID", "said": document["id"], "identifies": True})
    return {
        "eyebrow": f"in My Drive \u00b7 {kind}",
        "title": document["title"],
        "facts": facts,
        "body": "",
    }


def named_in(asked: object) -> set[str] | None:
    """Read what a watch names of one field, one or a list, or none for all of them."""
    if isinstance(asked, str):
        return {asked}
    if isinstance(asked, list):
        return {each for each in asked if isinstance(each, str)}
    return None


class Watch:
    """One watch a shell holds: its id, what it sees, and the shell it's told on."""

    def __init__(self, shell: socket.socket, id_: object, asked: dict[str, object]) -> None:
        """Hold what the watch asked for: its kinds, and the places of the rows it sees."""
        self.shell = shell
        self.id = id_
        self.kinds = named_in(asked.get("kind"))
        self.places = named_in(asked.get("place"))

    def sees(self, key: str) -> bool:
        """Say whether a record is one this watch asked for, a row or drawer by its place."""
        kind, _, rest = key.partition(":")
        place = rest.partition(":")[0]
        return (self.kinds is None or kind in self.kinds) and (
            kind not in {"row", "drawer"} or self.places is None or place in self.places
        )

    def replaces(self, other: Watch) -> bool:
        """Say whether this watch is asked again on the same shell by the same id."""
        return other.shell is self.shell and other.id == self.id

    def say(self, said: dict[str, object]) -> bool:
        """Tell the shell one line, saying whether it's still there to hear it."""
        try:
            self.shell.sendall(f"{json.dumps(said)}\n".encode())
        except OSError:
            return False
        return True


class Account:
    """The account as the program would hold it, and the watches it tells."""

    def __init__(self, runtime: Path) -> None:
        """Hold the meetings, answered yes, with no shell yet."""
        now = int(time.time() * 1000)
        self.files = runtime / "google-files"
        self.asked = runtime / "google-asked"
        self.lock = threading.Lock()
        self.watches: list[Watch] = []
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
        # What a Recent holds: the meetings held in the month before today, and the mail Joel
        # sent or read lately, as the program keeps them (src/state.rs).
        self.held = [
            meeting(
                "retro",
                "Retro",
                now - HELD,
                "rtr-ospe-ctv",
                series="retro",
                holds=False,
                through="ada@example.com",
                organizer={"name": "Ada Lovelace", "email": "ada@example.com"},
                guests=[{"name": "Grace Hopper", "email": "grace@example.com", "answer": "yes"}],
            ),
            # The time before, of the same series by Google's id for it, so Retro is listed once.
            meeting(
                "retro_20261020T150000Z",
                "Retro",
                now - HELD - WEEK,
                "rtr-ospe-ctv",
                series="retro",
                holds=False,
                through="ada@example.com",
                organizer={"name": "Ada Lovelace", "email": "ada@example.com"},
                guests=[{"name": "Grace Hopper", "email": "grace@example.com", "answer": "yes"}],
            ),
        ]
        self.lately = [
            {
                "id": "trip",
                "from": "Grace Hopper",
                "address": "grace@example.com",
                "to": "Joel Gerber",
                "cc": "",
                "others": [],
                "subject": "Trip Notes",
                "at": now - READ,
                "page": "https://mail.google.com/mail/u/0/#all/trip",
                "body": "The train leaves at nine.",
                "attachments": [],
            },
            {
                "id": "notes",
                "from": "Sam Rivers",
                "address": "sam@example.com",
                "to": "Ada Lovelace, Grace Hopper",
                "cc": "Alan Turing",
                "others": [],
                "subject": "Notes on the Plan",
                "at": now - READ - SENT,
                "page": "https://mail.google.com/mail/u/0/#all/notes",
                "body": "A few thoughts on the plan.",
                "attachments": [],
                # Sent by Joel, as Gmail's SENT label tells the daemon, which names who it
                # went to, never him: three, so its row gives way by whole names.
                "sent": True,
                "went": ["Ada Lovelace", "Grace Hopper", "Alan Turing"],
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
        # Drive's own places, each file as the program says one (src/drive.rs's Document).
        trips = placed("trips", "Trips", "folder", FOLDER, now)
        trips["link"] = "https://drive.google.com/drive/folders/trips"
        trips["starred"] = True
        menu = placed("menu", "menu.pdf", "file", "application/pdf", now)
        menu["viewed"] = now - 5 * MINUTE
        menu["size"] = 4
        team = placed("0Team", "Team", "drive", "", now)
        team["link"] = "https://drive.google.com/drive/folders/0Team"
        documents = [
            {**placed(str(one["id"]), str(one["title"]), str(one["kind"]), "", now), **one}
            for one in self.documents
        ]
        self.places = {
            "recent": [menu, *documents],
            "starred": {"files": [trips], "more": False},
            "shared": {"files": documents[1:], "more": False},
            "drives": {"files": [team], "more": False},
        }
        # What Drive's copies here take and may, in bytes (src/drive.rs's Kept).
        self.kept = {"held": KEPT_HELD, "most": KEPT_MOST}
        self.heard = now
        # The file last asked to be fetched, as the program says it (src/files.rs).
        self.fetched: dict[str, object] | None = None
        # The Meet meeting last asked to be made, as the program says it (src/state.rs).
        self.made: dict[str, object] | None = None
        # The Drive folders listed and the downloads asked lately, each newest first.
        self.listings: list[dict[str, object]] = []
        self.downloads: list[dict[str, object]] = []
        # What every watch was last told of, by key.
        self.said = self.records()

    def state(self) -> dict[str, object]:
        """Give the state the program would say."""
        if SIGNED_OUT:
            return {
                "account": "signedOut",
                "meetings": [],
                "mail": [],
                "held": [],
                "lately": [],
                "unread": None,
                "documents": [],
                "heard": None,
                "fetched": None,
                "meet": {"granted": False, "made": None},
                "drive": {"listings": [], "downloads": [], "uploads": [], "places": NO_PLACES},
            }
        return {
            "account": "ready",
            "meetings": self.meetings,
            "mail": self.mail,
            "held": self.held,
            "lately": self.lately,
            "unread": self.unread,
            "documents": self.documents,
            "heard": self.heard,
            "fetched": self.fetched,
            "meet": {"granted": True, "made": self.made},
            "drive": {
                "listings": self.listings,
                "downloads": self.downloads,
                "uploads": [],
                "places": self.places,
                "kept": self.kept,
            },
        }

    def looks(self) -> None:
        """Tell each watch what the clock moved each minute, as the program looks each minute."""
        while True:
            # A fixed wait, since the program's own look is a minute's tick (src/main.rs).
            time.sleep(MINUTE / 1000)
            with self.lock:
                self.tell()

    def records(self) -> dict[str, dict[str, object]]:
        """Give each record the daemon would hold, by key, as src/daemon.rs makes them."""
        # A copy, since what's held changes in place, and what each watch was
        # last told must stay as it was to find what changed.
        state = copy.deepcopy(self.state())
        made: dict[str, dict[str, object]] = {}
        for kind, things in (
            ("meeting", "meetings"),
            ("mail", "mail"),
            ("file", "documents"),
            ("held", "held"),
            ("lately", "lately"),
        ):
            for thing in cast("list[dict[str, object]]", state[things]):
                key = f"{kind}:{thing['id']}"
                made[key] = {"key": key, "kind": kind, "heard": self.heard, kind: thing}
        # Documents' rows and drawers, made ready as the daemon makes them once it has words.
        documents = cast("list[dict[str, object]]", state["documents"])
        for at, document in enumerate(documents):
            row = document_row(document, at)
            made[str(row["key"])] = {"key": row["key"], "kind": "row", "row": row}
            key = str(row["drawer"])
            made[key] = {"key": key, "kind": "drawer", "drawer": document_drawer(document)}
        # Calendar's rows and drawers, made by the program's own code.
        for record in calendar_records(state["meetings"], state["held"]):
            made[str(record["key"])] = record
        account = {name: state[name] for name in ("unread", "fetched", "meet", "drive")} | {
            "standing": state["account"]
        }
        made[ACCOUNT] = {
            "key": ACCOUNT,
            "kind": "account",
            "heard": state["heard"],
            "account": account,
        }
        return made

    def tell(self) -> None:
        """Tell each watch the records that changed and those gone, since it was last told."""
        now = self.records()
        changed = [record for key, record in now.items() if self.said.get(key) != record]
        gone = [key for key in self.said if key not in now]
        self.said = now
        for watch in list(self.watches):
            seen = [record for record in changed if watch.sees(str(record["key"]))]
            went = [key for key in gone if watch.sees(key)]
            if (seen or went) and not watch.say({"id": watch.id, "changed": seen, "gone": went}):
                self.watches.remove(watch)

    def answer(self, shell: socket.socket, asked: dict[str, object]) -> dict[str, object] | None:
        """Answer one ask, or give none once a watch's answer is sent."""
        id_ = asked.get("id")
        with self.lock:
            if asked.get("ask") == "hello":
                return {
                    "id": id_,
                    "version": 1,
                    "domain": "google",
                    "state": "answering",
                    "since": self.heard,
                    "sources": {"google": "signed-out" if SIGNED_OUT else "answering"},
                }
            if asked.get("ask") == "watch":
                watch = Watch(shell, id_, asked)
                self.watches = [one for one in self.watches if not watch.replaces(one)]
                records = [record for key, record in self.said.items() if watch.sees(key)]
                # Sent before the watch is added, so no change comes before it.
                if watch.say({"id": id_, "records": records}):
                    self.watches.append(watch)
                return None
            if asked.get("ask") == "unwatch":
                self.watches = [
                    one for one in self.watches if not (one.shell is shell and one.id == id_)
                ]
                return {"id": id_, "unwatched": True}
        if asked.get("ask") == "act" and isinstance(asked.get("act"), dict):
            self.act(cast("dict[str, dict[str, object]]", asked["act"]))
            return {"id": id_, "taken": True}
        return {"id": id_, "error": {"code": "unknown-ask", "message": "not here"}}

    def act(self, said: dict[str, dict[str, object]]) -> None:
        """Write down what a shell asked, and take it: a meeting answered, moved or cancelled.

        Or mail marked read or archived, or marked unread again after it was read, or a file it
        carries fetched, or a new Meet meeting made, as new-abcd-efg, or a Drive folder listed
        or a file in one downloaded. It's written down as the shell's line, as compact.
        """
        with self.asked.open("a") as asked:
            asked.write(f"{json.dumps(said, separators=(',', ':'))}\n")
        with self.lock:
            if "fetch" in said:
                self.fetch(said["fetch"])
            if "list" in said:
                self.look_in(str(said["list"]["folder"]))
            if "download" in said:
                self.download(said["download"])
            if "make" in said:
                self.made = {
                    "asked": int(time.time() * 1000),
                    "link": "https://meet.google.com/new-abcd-efg",
                    "failed": False,
                }
            self.schedule(said)
            self.tidy(said)
            self.tell()

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
                self.files / str(asked["id"])
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

    def look_in(self, folder: str) -> None:
        """Say what a Drive folder holds, or that it's gone, first among those asked lately."""
        files = FOLDERS.get(folder)
        self.listings = [one for one in self.listings if one["folder"] != folder]
        self.listings.insert(
            0,
            {
                "folder": folder,
                "asked": int(time.time() * 1000),
                "files": files or [],
                "more": False,
                "failed": None if files is not None else "gone",
            },
        )
        del self.listings[LISTINGS_KEPT:]

    def download(self, asked: dict[str, object]) -> None:
        """Write a Drive file where the program would, and say where, newest first."""
        file = next(
            (one for files in FOLDERS.values() for one in files if one["id"] == asked["file"]),
            None,
        )
        path, failed = None, None
        if file is None or file["kind"] == "folder":
            failed = "gone"
        else:
            name = f"{file['name']}{EXPORTED.get(str(file['kind']), '')}"
            path = Path.home() / ".cache" / "google" / "drive" / str(file["id"]) / name
            if asked["how"] == "save":
                path = Path(str(asked.get("to") or Path.home() / "Downloads" / name))
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(str(file["id"]).encode())
        self.downloads.insert(
            0,
            {
                "file": asked["file"],
                "how": asked["how"],
                "asked": int(time.time() * 1000),
                "path": str(path) if path else None,
                "failed": failed,
            },
        )
        del self.downloads[DOWNLOADS_KEPT:]


def keep(account: Account, shell: socket.socket) -> None:
    """Answer what a shell asks until it goes, its watches with it."""
    try:
        for line in shell.makefile("r", encoding="utf-8"):
            asked = json.loads(line)
            if not isinstance(asked, dict):
                continue
            answer = account.answer(shell, asked)
            if answer is not None:
                with account.lock:
                    shell.sendall(f"{json.dumps(answer)}\n".encode())
    except OSError, ValueError:
        pass
    finally:
        with account.lock:
            account.watches = [one for one in account.watches if one.shell is not shell]
        shell.close()


def calendar_records(meetings: object, held: object) -> list[dict[str, object]]:
    """Give Calendar's ready records as the program makes them, at this moment."""
    asked = json.dumps({"meetings": meetings, "held": held, "now": int(time.time() * 1000)})
    # The program and its words are the VM's own, from Nix (tests/desktop.nix).
    made = subprocess.run(  # noqa: S603
        [sys.argv[2], sys.argv[3]], input=asked, capture_output=True, text=True, check=True
    )
    return cast("list[dict[str, object]]", json.loads(made.stdout))


def listening(path: Path) -> socket.socket:
    """Give the socket systemd hands over, as it does google's, or else one bound at `path`."""
    if os.environ.get("LISTEN_PID") == str(os.getpid()) and os.environ.get("LISTEN_FDS"):
        return socket.socket(fileno=HANDED)
    path.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
    # One left by a stand-in before it would refuse the name.
    path.unlink(missing_ok=True)
    server = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    server.bind(str(path))
    server.listen()
    return server


def main() -> None:
    """Serve the daemon interface on the socket given, in the runtime folder's daemons."""
    path = Path(sys.argv[1])
    account = Account(path.parent.parent)
    server = listening(path)
    threading.Thread(target=account.looks, daemon=True).start()
    while True:
        shell, _ = server.accept()
        threading.Thread(target=keep, args=(account, shell), daemon=True).start()


if __name__ == "__main__":
    main()

"""Whether open-with chooses a file in the shell, or in GTK's portal while the shell can't.

The files-are-chosen-with-or-without-the-picker check runs it on a session bus of its own, as
`files-chosen.py <open-with> <a qs that refuses> <a qs that answers> <the path it answers>`.
A stand-in for GTK's portal answers every file chooser call with a file of its own. open-with is
started with each qs in turn and asked to open a file: given the qs that refuses, as a shell
with no picker does, the answer has to be GTK's, for the call GTK was given; given the one that
answers, it has to be the shell's path, with GTK never asked.
"""

import os
import subprocess
import sys
import threading
import time
from pathlib import Path

from jeepney import DBusAddress, message_bus, new_method_call, new_method_return
from jeepney.io.blocking import DBusConnection, open_dbus_connection
from jeepney.low_level import HeaderFields, Message

LAUNCHER = "org.freedesktop.impl.portal.desktop.launcher"
GTK = "org.freedesktop.impl.portal.desktop.gtk"
CHOOSER = "org.freedesktop.impl.portal.FileChooser"
PATH = "/org/freedesktop/portal/desktop"
HANDLE = "/org/freedesktop/portal/desktop/request/1_1/files"
TITLE = "Open a note"
GTKS_FILE = "file:///from/gtk.md"


class StandInGtk:
    """GTK's portal's file chooser, answering each call with its own file."""

    def __init__(self) -> None:
        """Take GTK's portal's name on the bus, and answer on a thread of its own."""
        self.connection = open_dbus_connection(bus="SESSION")
        self.connection.send_and_get_reply(message_bus.RequestName(GTK))
        self.asked: list[tuple] = []
        threading.Thread(target=self.answer, daemon=True).start()

    def answer(self) -> None:
        """Answer OpenFile with GTK's file, keeping what each call was given."""
        while True:
            message = self.connection.receive()
            if message.header.fields.get(HeaderFields.member) == "OpenFile":
                self.asked.append(message.body)
                results = {"uris": ("as", [GTKS_FILE])}
                self.connection.send(new_method_return(message, "ua{sv}", (0, results)))


def owned(connection: DBusConnection, name: str) -> bool:
    """Whether a name has an owner on the bus."""
    reply = connection.send_and_get_reply(message_bus.NameHasOwner(name))
    return bool(reply.body[0])


def opened(connection: DBusConnection, open_with: str, qs: str) -> tuple[int, dict]:
    """What open-with answers to opening a file, run with this qs."""
    # Both are the check's own, from Nix.
    portal = subprocess.Popen([open_with, qs], env=os.environ)  # noqa: S603
    try:
        for _ in range(100):
            if owned(connection, LAUNCHER):
                break
            time.sleep(0.1)
        else:
            sys.exit("open-with never took its name on the bus.")
        address = DBusAddress(PATH, bus_name=LAUNCHER, interface=CHOOSER)
        call = new_method_call(
            address, "OpenFile", "osssa{sv}", (HANDLE, "notes", "", TITLE, {})
        )
        reply: Message = connection.send_and_get_reply(call, timeout=30)
        return reply.body
    finally:
        portal.terminate()
        portal.wait(timeout=10)
        while owned(connection, LAUNCHER):
            time.sleep(0.1)


def main(argv: list[str]) -> None:
    """Fail, saying why, unless a file is chosen by whichever can choose it."""
    open_with, refusing, answering, answered = argv
    Path(os.environ["XDG_RUNTIME_DIR"]).mkdir(mode=0o700, exist_ok=True)
    gtk = StandInGtk()
    connection = open_dbus_connection(bus="SESSION")

    code, results = opened(connection, open_with, refusing)
    if (code, results.get("uris")) != (0, ("as", [GTKS_FILE])):
        sys.exit(f"With no picker in the shell, GTK's portal didn't choose: {code} {results}")
    if [(asked[0], asked[3]) for asked in gtk.asked] != [(HANDLE, TITLE)]:
        sys.exit(f"GTK's portal wasn't given the call as it was asked: {gtk.asked}")

    code, results = opened(connection, open_with, answering)
    if (code, results.get("uris")) != (0, ("as", [f"file://{answered}"])):
        sys.exit(f"The shell's pick didn't reach the program: {code} {results}")
    if len(gtk.asked) != 1:
        sys.exit("GTK's portal was asked as well as the shell.")


if __name__ == "__main__":
    main(sys.argv[1:])

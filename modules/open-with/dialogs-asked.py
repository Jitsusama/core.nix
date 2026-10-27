"""Whether a GTK 3 program's own file dialog asks the portal, as dialogs-ask-the-portal has it.

The gtk-3-dialogs-ask-the-portal check runs it on a session bus and a screen of its own, with
GTK3_MODULES naming the module and GTK_USE_PORTAL set, as
`dialogs-asked.py <dialog-of-its-own> <a folder>`. A stand-in for the portal answers each call to
choose a file with a file of its own, or cancels it, keeping what each was given. A dialog to
open a file, one to save a draft, and one the portal cancels must each never show, and answer as
its accept button would with the portal's file, or as closed.
"""

import subprocess
import sys
import threading
from typing import Any

from jeepney import DBusAddress, MessageType, message_bus, new_error, new_method_return, new_signal
from jeepney.io.blocking import DBusConnection, open_dbus_connection
from jeepney.low_level import HeaderFields, Message

PORTAL = "org.freedesktop.portal.Desktop"
CHOOSER = "org.freedesktop.portal.FileChooser"
REQUEST = "org.freedesktop.portal.Request"
OPENED = "/from/the/portal/picture.png"
SAVED = "/from/the/portal/saved.svg"
# GTK's answers: its accept button's, and a dialog closed.
ACCEPT = -3
OK = -5
CLOSED = -4


class StandInPortal:
    """The portal's file chooser, answering each call with a file of its own, or cancelling."""

    def __init__(self) -> None:
        """Take the portal's name on the bus, and answer on a thread of its own."""
        self.connection: DBusConnection = open_dbus_connection(bus="SESSION")
        self.connection.send_and_get_reply(message_bus.RequestName(PORTAL))
        self.asked: list[tuple[str, str, dict[str, Any]]] = []
        self.cancelling = False
        threading.Thread(target=self.answer, daemon=True).start()

    def answer(self) -> None:
        """Answer OpenFile and SaveFile on their request's path, as the portal does.

        Any other call is refused at once, as a portal without it would, since GTK waits on
        the portal's settings as it starts.
        """
        while True:
            message = self.connection.receive()
            if message.header.message_type != MessageType.method_call:
                continue
            member = message.header.fields.get(HeaderFields.member)
            interface = message.header.fields.get(HeaderFields.interface)
            if member in {"OpenFile", "SaveFile"} and interface == CHOOSER:
                self.chosen(message, member)
            elif member == "Get" and message.body[0] == CHOOSER:
                self.connection.send(new_method_return(message, "v", (("u", 4),)))
            else:
                refused = "org.freedesktop.DBus.Error.UnknownMethod"
                self.connection.send(new_error(message, refused))

    def chosen(self, message: Message, member: str) -> None:
        """Say what was chosen on the request's path the caller is listening on."""
        _parent, title, options = message.body
        self.asked.append((member, title, options))
        caller = message.header.fields[HeaderFields.sender]
        token = options["handle_token"][1]
        named = caller.lstrip(":").replace(".", "_")
        handle = f"/org/freedesktop/portal/desktop/request/{named}/{token}"
        self.connection.send(new_method_return(message, "o", (handle,)))
        file = SAVED if member == "SaveFile" else OPENED
        # xdg-desktop-portal always says uris, none when cancelled, and GTK 3 reads them unasked.
        uris = [] if self.cancelling else [f"file://{file}"]
        said = (1 if self.cancelling else 0, {"uris": ("as", uris)})
        request = DBusAddress(handle, interface=REQUEST)
        # GTK listens for its answer without a match rule, so the portal sends it to the caller
        # alone, as xdg-desktop-portal does.
        response = new_signal(request, "Response", "ua{sv}", said)
        response.header.fields[HeaderFields.destination] = caller
        self.connection.send(response)


def opened(dialog: str, *words: str) -> list[str]:
    """Run the program with its own dialog with these words, and return its lines."""
    # The program is the check's own, from Nix; what it warns of goes to the check's log.
    said = subprocess.run(  # noqa: S603
        [dialog, *words], stdout=subprocess.PIPE, text=True, timeout=60, check=True
    )
    return said.stdout.split("\n")[:-1]


def main(argv: list[str]) -> None:
    """Fail, saying why, unless every dialog asks the portal and says what it chose."""
    dialog, folder = argv
    portal = StandInPortal()

    said = opened(dialog, "open", folder)
    if said != [f"answered {ACCEPT} {OPENED}"]:
        sys.exit(f"A dialog to open a file didn't answer with the portal's: {said} {portal.asked}")
    if [(member, title) for member, title, _ in portal.asked] != [("OpenFile", "Open a picture")]:
        sys.exit(f"The portal wasn't asked to open a file as the dialog would: {portal.asked}")

    said = opened(dialog, "save", folder, "draft.svg")
    if said != [f"answered {OK} {SAVED}"]:
        sys.exit(f"A dialog to save a file didn't answer with the portal's: {said}")
    member, title, options = portal.asked[1]
    named = options.get("current_name", ("s", ""))[1]
    within = bytes(options.get("current_folder", ("ay", b""))[1]).rstrip(b"\0").decode()
    if (member, title, named, within) != ("SaveFile", "Save a draft", "draft.svg", folder):
        sys.exit(f"The portal wasn't asked to save the draft as the dialog would: {portal.asked}")

    portal.cancelling = True
    said = opened(dialog, "open", folder)
    if said != [f"answered {CLOSED} -"]:
        sys.exit(f"A dialog the portal cancelled didn't answer as closed: {said}")


if __name__ == "__main__":
    main(sys.argv[1:])

"""Ask the desktop portal for a file to open or save, in a folder, as a program does.

    choose-a-file open|save FOLDER [NAME]

It stays on the bus until the portal answers, since the portal drops a request whose caller
has gone. Its answer, 0 for chosen and 1 for refused, then each file's link, lands in the
runtime folder's portal-chose, written whole.
"""

import os
import sys
from pathlib import Path

from jeepney import DBusAddress, MatchRule, message_bus, new_method_call
from jeepney.io.blocking import Proxy, open_dbus_connection

# The request's handle, which the portal's answer is sent on.
HANDLE = "chosen"
# How long the portal may take to answer, in seconds: someone has to choose.
ANSWERING = 120


def say(line: str) -> None:
    """Say what happened, for the test's log."""
    print(line, file=sys.stderr, flush=True)


def main(argv: list[str]) -> None:
    """Ask the portal for a file in the folder named, and write down its answer."""
    mode, folder, *named = argv
    said = Path(os.environ["XDG_RUNTIME_DIR"]) / "portal-chose"
    connection = open_dbus_connection(bus="SESSION")
    sender = connection.unique_name[1:].replace(".", "_")
    rule = MatchRule(
        type="signal",
        interface="org.freedesktop.portal.Request",
        member="Response",
        path=f"/org/freedesktop/portal/desktop/request/{sender}/{HANDLE}",
    )
    Proxy(message_bus, connection).AddMatch(rule)
    portal = DBusAddress(
        "/org/freedesktop/portal/desktop",
        bus_name="org.freedesktop.portal.Desktop",
        interface="org.freedesktop.portal.FileChooser",
    )
    # The portal takes a folder as bytes ending in a nul, as a path is on disk.
    options = {
        "handle_token": ("s", HANDLE),
        "current_folder": ("ay", folder.encode() + b"\0"),
    }
    if named:
        options["current_name"] = ("s", named[0])
    method = "SaveFile" if mode == "save" else "OpenFile"
    title = "Save the Notes" if mode == "save" else "Open the Notes"
    asked = new_method_call(portal, method, "ssa{sv}", ("", title, options))
    with connection.filter(rule) as heard:
        handle = connection.send_and_get_reply(asked)
        say(f"asked, {handle.header.message_type} {handle.body}")
        answer = connection.recv_until_filtered(heard, timeout=ANSWERING)
        say(f"answered {answer.body}")
    code, results = answer.body
    uris = results.get("uris", ("as", []))[1]
    part = said.with_name(said.name + ".part")
    part.write_text("".join(f"{line}\n" for line in [str(code), *uris]))
    part.rename(said)


if __name__ == "__main__":
    main(sys.argv[1:])

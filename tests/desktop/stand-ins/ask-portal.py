"""Ask the desktop portal to open a file, always asking what with, as a program does.

    ask-portal FILE

It stays on the bus until the portal answers, since the portal drops a request whose caller
has gone. Its answer, 0 for opened and 1 for refused, lands in the runtime folder's
portal-said, written whole.
"""

import os
import sys
from pathlib import Path

from jeepney import DBusAddress, MatchRule, message_bus, new_method_call
from jeepney.io.blocking import Proxy, open_dbus_connection

# The request's handle, which the portal's answer is sent on.
HANDLE = "asked"
# How long the portal may take to answer, in seconds: someone has to choose.
ANSWERING = 120


def say(line: str) -> None:
    """Say what happened, for the test's log."""
    print(line, file=sys.stderr, flush=True)


def main(argv: list[str]) -> None:
    """Ask the portal to open the file named, and write down its answer."""
    said = Path(os.environ["XDG_RUNTIME_DIR"]) / "portal-said"
    connection = open_dbus_connection(bus="SESSION", enable_fds=True)
    say(f"on {os.environ.get('DBUS_SESSION_BUS_ADDRESS')} as {connection.unique_name}")
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
        interface="org.freedesktop.portal.OpenURI",
    )
    options = {"ask": ("b", True), "handle_token": ("s", HANDLE)}
    # The portal opens a file only as one already open, by OpenFile, and refuses its path as
    # a file:// link.
    with Path(argv[0]).open("rb") as opened:
        asked = new_method_call(portal, "OpenFile", "sha{sv}", ("", opened, options))
        with connection.filter(rule) as heard:
            handle = connection.send_and_get_reply(asked)
            say(f"asked, {handle.header.message_type} {handle.body}")
            answer = connection.recv_until_filtered(heard, timeout=ANSWERING)
            say(f"answered {answer.body[0]}")
    part = said.with_name(said.name + ".part")
    part.write_text(f"{answer.body[0]}\n")
    part.rename(said)


if __name__ == "__main__":
    main(sys.argv[1:])

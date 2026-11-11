"""Stand in for systemd, saying how the disk guards' units stand, for the disk piece's check.

The machine daemon asks a silent guard's unit how it stands over systemd's bus, the account's
on the session bus and the machine's on the system bus. This owns systemd's name on both of
the check's own buses and answers each unit's ActiveState and Result from a file the case
writes, as `systemctl show -p ActiveState,Result` prints them. It says it's ready on a fifo
once it owns both names, as
`systemd-units.py <ready fifo> <session address> <account file> <system address> <machine file>`.
"""

import sys
import threading
from pathlib import Path

from jeepney import MessageType, message_bus, new_error, new_method_return
from jeepney.io.blocking import DBusConnection, open_dbus_connection
from jeepney.low_level import HeaderFields, Message

NAME = "org.freedesktop.systemd1"
# Each interface's one property the daemon asks for, by the key the file holds it under.
PROPERTIES = {
    ("org.freedesktop.systemd1.Unit", "ActiveState"): "ActiveState",
    ("org.freedesktop.systemd1.Service", "Result"): "Result",
}


def said(file: Path) -> dict[str, str]:
    """Read what a case wrote of a unit, or nothing before the first case writes it."""
    fields: dict[str, str] = {}
    if not file.exists():
        return fields
    for line in file.read_text().splitlines():
        key, _, value = line.partition("=")
        fields[key] = value
    return fields


def answer(asked: Message, file: Path) -> Message:
    """Answer one call: a property Get the file answers, or an error."""
    fields = asked.header.fields
    if fields.get(HeaderFields.member) != "Get":
        return new_error(asked, "org.freedesktop.DBus.Error.UnknownMethod", "s", ("only Get",))
    key = PROPERTIES.get(tuple(asked.body))
    value = said(file).get(key) if key else None
    if value is None:
        return new_error(asked, "org.freedesktop.DBus.Error.UnknownProperty", "s", ("not here",))
    return new_method_return(asked, "v", (("s", value),))


def answering(connection: DBusConnection, file: Path) -> None:
    """Answer every call on one bus until the check ends."""
    while True:
        asked = connection.receive()
        if asked.header.message_type == MessageType.method_call:
            connection.send(answer(asked, file))


def main() -> int:
    """Own systemd's name on both buses, say so, and answer until killed."""
    ready, session, account, system, machine = sys.argv[1:6]
    buses = []
    for address, written in ((session, account), (system, machine)):
        connection = open_dbus_connection(bus=address)
        connection.send_and_get_reply(message_bus.RequestName(NAME))
        buses.append((connection, Path(written)))
    for connection, file in buses:
        threading.Thread(target=answering, args=(connection, file), daemon=True).start()
    Path(ready).write_text("ready\n")
    threading.Event().wait()
    return 0


if __name__ == "__main__":
    sys.exit(main())

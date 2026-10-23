"""Stand in for whatever shows notices.

It takes the notification server's name on the session bus and writes each
notice it's asked to show to the file named first, its program, summary and
body a line each.
"""

import asyncio
import sys
from pathlib import Path
from typing import Annotated

from dbus_fast.aio import MessageBus
from dbus_fast.annotations import DBusDict, DBusInt32, DBusSignature, DBusStr, DBusUInt32
from dbus_fast.service import ServiceInterface, dbus_method

DBusStrs = Annotated[list[str], DBusSignature("as")]
ServerInformation = Annotated[list[str], DBusSignature("ssss")]


class Notices(ServiceInterface):
    """The notification server's interface, keeping what it's told."""

    def __init__(self, told: Path) -> None:
        """Keep each notice in the file told."""
        super().__init__("org.freedesktop.Notifications")
        self.told = told

    @dbus_method(name="Notify")
    def notify(
        self,
        app: DBusStr,
        _replaces: DBusUInt32,
        _icon: DBusStr,
        summary: DBusStr,
        body: DBusStr,
        _actions: DBusStrs,
        _hints: DBusDict,
        _timeout: DBusInt32,
    ) -> DBusUInt32:
        """Keep the notice, and name it 1, as every one is."""
        with self.told.open("a") as told:
            told.write(f"{app}\n{summary}\n{body}\n")
        return 1

    @dbus_method(name="GetCapabilities")
    def capabilities(self) -> DBusStrs:
        """Say a notice may have a body."""
        return ["body"]

    @dbus_method(name="GetServerInformation")
    def server_information(self) -> ServerInformation:
        """Name this stand-in, its vendor and versions."""
        return ["stand-in", "core.nix", "1", "1.2"]

    @dbus_method(name="CloseNotification")
    def close(self, _notice: DBusUInt32) -> None:
        """Close nothing, since nothing is shown."""


async def serve(told: Path) -> None:
    """Serve the interface until stopped."""
    bus = await MessageBus().connect()
    bus.export("/org/freedesktop/Notifications", Notices(told))
    await bus.request_name("org.freedesktop.Notifications")
    await asyncio.Future()


if __name__ == "__main__":
    asyncio.run(serve(Path(sys.argv[1])))

"""Stand in for whatever shows notices.

It takes the notification server's name on the session bus and writes each
notice it's asked to show to the file named first, its program, summary and
body a line each. A test chooses a notice's act, as a click would, with the
stand-in's own Invoke: given a summary and an act, it says that act was
invoked on the newest notice with that summary, and that it closed.
"""

import asyncio
import sys
from pathlib import Path
from typing import Annotated

from dbus_fast.aio import MessageBus
from dbus_fast.annotations import DBusDict, DBusInt32, DBusSignature, DBusStr, DBusUInt32
from dbus_fast.service import ServiceInterface, dbus_method, dbus_signal

DBusStrs = Annotated[list[str], DBusSignature("as")]
ServerInformation = Annotated[list[str], DBusSignature("ssss")]
Invoked = Annotated[tuple[int, str], DBusSignature("us")]
Closed = Annotated[tuple[int, int], DBusSignature("uu")]
# Why a notice closed, by the specification's numbers.
DISMISSED = 2
CLOSED = 3


class Notices(ServiceInterface):
    """The notification server's interface, keeping what it's told."""

    def __init__(self, told: Path) -> None:
        """Keep each notice in the file told."""
        super().__init__("org.freedesktop.Notifications")
        self.told = told
        # Each notice's number, newest last for its summary, so an act
        # chosen on one never answers another's wait.
        self.shown: dict[str, int] = {}
        self.count = 0

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
        """Keep the notice, giving it a number of its own."""
        with self.told.open("a") as told:
            told.write(f"{app}\n{summary}\n{body}\n")
        self.count += 1
        self.shown[summary] = self.count
        return self.count

    @dbus_method(name="Invoke")
    def invoke(self, summary: DBusStr, act: DBusStr) -> None:
        """Choose an act on the newest notice with that summary, which then closes."""
        notice = self.shown[summary]
        self.action_invoked(notice, act)
        self.notification_closed(notice, DISMISSED)

    @dbus_signal(name="ActionInvoked")
    def action_invoked(self, notice: int, act: str) -> Invoked:
        """Say an act was chosen on a notice."""
        return (notice, act)

    @dbus_signal(name="NotificationClosed")
    def notification_closed(self, notice: int, reason: int) -> Closed:
        """Say a notice closed, and why."""
        return (notice, reason)

    @dbus_method(name="GetCapabilities")
    def capabilities(self) -> DBusStrs:
        """Say a notice may have a body and acts."""
        return ["actions", "body"]

    @dbus_method(name="GetServerInformation")
    def server_information(self) -> ServerInformation:
        """Name this stand-in, its vendor and versions."""
        return ["stand-in", "core.nix", "1", "1.2"]

    @dbus_method(name="CloseNotification")
    def close(self, notice: DBusUInt32) -> None:
        """Say the notice closed, since nothing is shown to close."""
        self.notification_closed(notice, CLOSED)


async def serve(told: Path) -> None:
    """Serve the interface until stopped."""
    bus = await MessageBus().connect()
    bus.export("/org/freedesktop/Notifications", Notices(told))
    await bus.request_name("org.freedesktop.Notifications")
    await asyncio.Future()


if __name__ == "__main__":
    asyncio.run(serve(Path(sys.argv[1])))

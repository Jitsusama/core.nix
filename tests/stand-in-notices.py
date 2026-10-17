# Stands in for whatever shows notices: takes the notification server's name
# on the session bus and writes each notice it's asked to show to the file
# named first, its program, summary and body a line each.
import asyncio
import sys

from dbus_fast.aio import MessageBus
from dbus_fast.service import ServiceInterface, method


class Notices(ServiceInterface):
    def __init__(self):
        super().__init__("org.freedesktop.Notifications")

    @method()
    def Notify(
        self,
        app: "s",
        replaces: "u",
        icon: "s",
        summary: "s",
        body: "s",
        actions: "as",
        hints: "a{sv}",
        timeout: "i",
    ) -> "u":
        with open(sys.argv[1], "a") as told:
            told.write(f"{app}\n{summary}\n{body}\n")
        return 1

    @method()
    def GetCapabilities(self) -> "as":
        return ["body"]

    @method()
    def GetServerInformation(self) -> "ssss":
        return ["stand-in", "core.nix", "1", "1.2"]

    @method()
    def CloseNotification(self, id: "u"):
        pass


async def main():
    bus = await MessageBus().connect()
    bus.export("/org/freedesktop/Notifications", Notices())
    await bus.request_name("org.freedesktop.Notifications")
    await asyncio.Future()


asyncio.run(main())

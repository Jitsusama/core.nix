"""A classic Bluetooth keyboard, for a VM test to pair and type with.

It runs on a machine of its own, over BlueZ started without its input plugin,
which would otherwise take the two HID channels for itself. `serve` makes the
adapter a keyboard anyone may pair with: it says so in its service record,
answers every pairing, and holds the channels a host opens once paired.
`type ADDRESS TEXT` does what a real keyboard does at a key press after a
restart: it opens both channels to the host it was paired with and sends the
text as key presses, Enter included for a newline.
"""

import asyncio
import contextlib
import socket
import sys
import time
from typing import Annotated

from dbus_fast import BusType, Message, MessageType, Variant
from dbus_fast.aio import MessageBus
from dbus_fast.annotations import DBusSignature, DBusStr, DBusUInt32
from dbus_fast.service import ServiceInterface, dbus_method

DBusPath = Annotated[str, DBusSignature("o")]

HID = "00001124-0000-1000-8000-00805f9b34fb"
CONTROL = 17
INTERRUPT = 19

# A boot keyboard's report descriptor, report 1: a byte of modifiers, a
# reserved byte and six keys, which is what each report below sends.
DESCRIPTOR = bytes.fromhex(
    "05010906a101850175019508050719e029e715002501810295017508810395067508150025650507190029658100c0"
)

# The SDP record a host reads to learn it's a keyboard and how to read it.
RECORD = f"""<?xml version="1.0" encoding="UTF-8" ?>
<record>
  <attribute id="0x0001"><sequence><uuid value="0x1124" /></sequence></attribute>
  <attribute id="0x0004">
    <sequence>
      <sequence><uuid value="0x0100" /><uint16 value="0x{CONTROL:04x}" /></sequence>
      <sequence><uuid value="0x0011" /></sequence>
    </sequence>
  </attribute>
  <attribute id="0x0005"><sequence><uuid value="0x1002" /></sequence></attribute>
  <attribute id="0x0009">
    <sequence><sequence><uuid value="0x1124" /><uint16 value="0x0100" /></sequence></sequence>
  </attribute>
  <attribute id="0x000d">
    <sequence>
      <sequence>
        <sequence><uuid value="0x0100" /><uint16 value="0x{INTERRUPT:04x}" /></sequence>
        <sequence><uuid value="0x0011" /></sequence>
      </sequence>
    </sequence>
  </attribute>
  <attribute id="0x0100"><text value="Stand-in Keyboard" /></attribute>
  <attribute id="0x0201"><uint16 value="0x0111" /></attribute>
  <attribute id="0x0202"><uint8 value="0x40" /></attribute>
  <attribute id="0x0203"><uint8 value="0x00" /></attribute>
  <attribute id="0x0204"><boolean value="false" /></attribute>
  <attribute id="0x0205"><boolean value="true" /></attribute>
  <attribute id="0x0206">
    <sequence><sequence>
      <uint8 value="0x22" /><text encoding="hex" value="{DESCRIPTOR.hex()}" />
    </sequence></sequence>
  </attribute>
  <attribute id="0x0207">
    <sequence><sequence><uint16 value="0x0409" /><uint16 value="0x0100" /></sequence></sequence>
  </attribute>
  <attribute id="0x020b"><uint16 value="0x0100" /></attribute>
  <attribute id="0x020c"><uint16 value="0x0c80" /></attribute>
  <attribute id="0x020d"><boolean value="false" /></attribute>
  <attribute id="0x020e"><boolean value="true" /></attribute>
</record>
"""

SHIFT = 0x02


# Each character the stand-in types, as a US keyboard's modifiers and key.
USAGES = (
    {chr(ord("a") + n): (0, 0x04 + n) for n in range(26)}
    | {chr(ord("A") + n): (SHIFT, 0x04 + n) for n in range(26)}
    | {chr(ord("1") + n): (0, 0x1E + n) for n in range(9)}
    | {"0": (0, 0x27), "\n": (0, 0x28), " ": (0, 0x2C), "-": (0, 0x2D)}
)


def usage_of(character: str) -> tuple[int, int]:
    """Give a character's modifiers and key usage on a US keyboard."""
    if character not in USAGES:
        message = f"the stand-in can't type {character!r}"
        raise ValueError(message)
    return USAGES[character]


def report(modifiers: int, key: int) -> bytes:
    """Give an input report on the interrupt channel: DATA | INPUT, report 1."""
    return bytes([0xA1, 0x01, modifiers, 0x00, key, 0, 0, 0, 0, 0])


class Agent(ServiceInterface):
    """Answers yes to every pairing, as a keyboard with no screen does."""

    def __init__(self) -> None:
        """Be BlueZ's agent interface."""
        super().__init__("org.bluez.Agent1")

    @dbus_method(name="Release")
    def release(self) -> None:
        """Let go, since BlueZ no longer needs the agent."""

    @dbus_method(name="AuthorizeService")
    def authorize_service(self, _device: DBusPath, _uuid: DBusStr) -> None:
        """Let a paired host use any service."""

    @dbus_method(name="RequestAuthorization")
    def request_authorization(self, _device: DBusPath) -> None:
        """Let any host pair."""

    @dbus_method(name="RequestConfirmation")
    def request_confirmation(self, _device: DBusPath, _passkey: DBusUInt32) -> None:
        """Take any passkey shown as right."""

    @dbus_method(name="Cancel")
    def cancel(self) -> None:
        """Forget a request that was withdrawn."""


class Profile(ServiceInterface):
    """The HID profile, registered only for its service record."""

    def __init__(self) -> None:
        """Be BlueZ's profile interface."""
        super().__init__("org.bluez.Profile1")

    @dbus_method(name="Release")
    def release(self) -> None:
        """Let go, since BlueZ no longer needs the profile."""


def listening(psm: int) -> socket.socket:
    """Open an L2CAP channel for a host to connect to."""
    channel = socket.socket(socket.AF_BLUETOOTH, socket.SOCK_SEQPACKET, socket.BTPROTO_L2CAP)
    channel.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    channel.bind((socket.BDADDR_ANY, psm))
    channel.listen(1)
    channel.setblocking(False)  # noqa: FBT003
    return channel


async def held(channel: socket.socket) -> None:
    """Hold each channel a host opens, answering its control requests."""
    loop = asyncio.get_running_loop()
    # The loop keeps only a weak reference to a task, so they're kept here.
    answering: set[asyncio.Task[None]] = set()
    while True:
        # An L2CAP peer's address starts with its own; after it Python
        # gives the PSM and, in the Python here, more besides.
        connection, peer = await loop.sock_accept(channel)
        address = peer[0]
        print(f"{address} opened channel {channel.getsockname()[1]}", flush=True)
        task = loop.create_task(answered(connection))
        answering.add(task)
        task.add_done_callback(answering.discard)


async def answered(connection: socket.socket) -> None:
    """Answer a host's requests on a channel with success until it closes."""
    loop = asyncio.get_running_loop()
    with contextlib.closing(connection):
        while data := await loop.sock_recv(connection, 64):
            # SET_PROTOCOL, SET_IDLE and the like each want a handshake.
            if data[0] >> 4 in (0x7, 0x9):
                await loop.sock_sendall(connection, b"\x00")


async def asked(
    bus: MessageBus, path: str, interface: str, member: str, signature: str, *body: object
) -> None:
    """Call one of BlueZ's methods, stopping with its error if it refuses."""
    reply = await bus.call(
        Message(
            destination="org.bluez",
            path=path,
            interface=interface,
            member=member,
            signature=signature,
            body=list(body),
        )
    )
    if reply is None or reply.message_type == MessageType.ERROR:
        details = reply.body if reply else "no reply"
        message = f"BlueZ refused {member}: {details}"
        raise RuntimeError(message)


async def serve() -> None:
    """Make the adapter a keyboard to pair with, and hold its channels."""
    bus = await MessageBus(bus_type=BusType.SYSTEM).connect()
    bus.export("/stand_in/agent", Agent())
    bus.export("/stand_in/profile", Profile())
    agents = "org.bluez.AgentManager1"
    await asked(
        bus, "/org/bluez", agents, "RegisterAgent", "os", "/stand_in/agent", "NoInputNoOutput"
    )
    await asked(bus, "/org/bluez", agents, "RequestDefaultAgent", "o", "/stand_in/agent")
    await asked(
        bus,
        "/org/bluez",
        "org.bluez.ProfileManager1",
        "RegisterProfile",
        "osa{sv}",
        "/stand_in/profile",
        HID,
        {
            "ServiceRecord": Variant("s", RECORD),
            "Role": Variant("s", "server"),
            "RequireAuthentication": Variant("b", False),  # noqa: FBT003
            "RequireAuthorization": Variant("b", False),  # noqa: FBT003
        },
    )
    for name, value in (
        ("Alias", Variant("s", "Stand-in Keyboard")),
        ("Powered", Variant("b", True)),  # noqa: FBT003
        ("Pairable", Variant("b", True)),  # noqa: FBT003
        ("DiscoverableTimeout", Variant("u", 0)),
        ("Discoverable", Variant("b", True)),  # noqa: FBT003
    ):
        await asked(
            bus,
            "/org/bluez/hci0",
            "org.freedesktop.DBus.Properties",
            "Set",
            "ssv",
            "org.bluez.Adapter1",
            name,
            value,
        )
    print("ready", flush=True)
    await asyncio.gather(held(listening(CONTROL)), held(listening(INTERRUPT)))


def typed(address: str, text: str) -> None:
    """Reconnect to a paired host, as at a key press, and type the text."""
    channels = []
    for psm in (CONTROL, INTERRUPT):
        channel = socket.socket(socket.AF_BLUETOOTH, socket.SOCK_SEQPACKET, socket.BTPROTO_L2CAP)
        channel.connect((address, psm))
        channels.append(channel)
    _, interrupt = channels
    # The host starts its input device once both channels are open.
    time.sleep(1)
    for character in text:
        modifiers, key = usage_of(character)
        interrupt.send(report(modifiers, key))
        time.sleep(0.02)
        interrupt.send(report(0, 0))
        time.sleep(0.02)
    # Long enough for the host to take the last report before the link drops.
    time.sleep(1)
    for channel in reversed(channels):
        channel.close()


def main() -> None:
    """Serve, or type."""
    match sys.argv[1:]:
        case ["serve"]:
            asyncio.run(serve())
        case ["type", address, text]:
            typed(address, text.encode().decode("unicode_escape"))
        case _:
            sys.exit("usage: bluetooth-keyboard-stand-in serve | type ADDRESS TEXT")


if __name__ == "__main__":
    main()

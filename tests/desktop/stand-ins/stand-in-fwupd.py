"""Stand in for fwupd on the system bus, saying what a test wrote, as fwupd would.

    stand-in-fwupd FOLDER DEVICES

The machine daemon asks fwupd over the bus for its devices, what it offers each and what it
has installed. The VM has no firmware to update, so this answers in fwupd's place from the
files a test writes in FOLDER, each as `fwupdmgr <command> --json` gives it: get-devices.json,
get-updates.json and get-history.json. With no get-devices.json it says DEVICES, the laptop's
two. Each time a file there changes it says Changed, as fwupd does once an update goes in.
Its security level is the laptop's four attributes in SECURITY, one failing at level 2 for
a firmware setting and one about what's running, which lowers it.
"""

import json
import sys
import time
from pathlib import Path
from typing import TYPE_CHECKING, Any

from jeepney import (
    DBusAddress,
    HeaderFields,
    MessageType,
    new_error,
    new_method_return,
    new_signal,
)
from jeepney.bus_messages import message_bus
from jeepney.io.blocking import DBusConnection, Proxy, open_dbus_connection

if TYPE_CHECKING:
    from jeepney.low_level import Message

NAME = "org.freedesktop.fwupd"
PROPERTIES = "org.freedesktop.DBus.Properties"
# fwupd's device flags by their names, as libfwupd numbers them, for those a test writes.
FLAGS = {
    "internal": 0,
    "updatable": 1,
    "require-ac": 3,
    "needs-reboot": 8,
    "updatable-hidden": 37,
}
# The fields fwupd sends as 32-bit numbers; every other number is 64-bit.
NARROW = {"UpdateState", "Urgency", "InstallDuration", "VersionFormat"}
# What fwupd says of itself.
SAID = {
    "DaemonVersion": ("s", "2.1.6"),
    "HostProduct": ("s", "Stand-in Laptop"),
    "HostSecurityId": ("s", "HSI:1! (v2.1.6)"),
}
# fwupd's security attribute flags by their bits, as libfwupd numbers them.
SUCCESS = 1 << 0
RUNTIME_ISSUE = 1 << 10
CONFIG_FW = 1 << 12
CONFIG_OS = 1 << 13
# What fwupd found of an attribute, by its number.
ENABLED, NOT_ENABLED, FOUND = 1, 2, 11
# The attributes the laptop's level is made of, as GetHostSecurityAttrs gives them: its
# numbers are 32-bit but Flags and Created, as fwupd sends them.
SECURITY = [
    {
        "AppstreamId": ("s", "org.fwupd.hsi.Tpm.Version20"),
        "Name": ("s", "TPM v2.0"),
        "Summary": ("s", "TPM 2.0 Present"),
        "Description": ("s", "A TPM securely stores measurements of the firmware."),
        "Plugin": ("s", "tpm"),
        "HsiLevel": ("u", 1),
        "HsiResult": ("u", FOUND),
        "HsiResultSuccess": ("u", FOUND),
        "Flags": ("t", SUCCESS),
        "Created": ("t", 1_700_000_000),
    },
    {
        "AppstreamId": ("s", "org.fwupd.hsi.Iommu"),
        "Name": ("s", "IOMMU"),
        "Summary": ("s", "IOMMU Protection"),
        "Description": ("s", "The IOMMU keeps devices from reading memory they weren't given."),
        "Plugin": ("s", "iommu"),
        "HsiLevel": ("u", 1),
        "HsiResult": ("u", ENABLED),
        "HsiResultSuccess": ("u", ENABLED),
        "Flags": ("t", SUCCESS),
        "Created": ("t", 1_700_000_000),
    },
    {
        "AppstreamId": ("s", "org.fwupd.hsi.Bios.RollbackProtection"),
        "Name": ("s", "BIOS rollback protection"),
        "Summary": ("s", "BIOS Rollback Protection"),
        "Description": ("s", "Rollback protection keeps older firmware from being put back."),
        "Plugin": ("s", "bios"),
        "HsiLevel": ("u", 2),
        "HsiResult": ("u", NOT_ENABLED),
        "HsiResultSuccess": ("u", ENABLED),
        "Flags": ("t", CONFIG_FW),
        "BiosSettingId": ("s", "AllowBiosDowngrade"),
        "BiosSettingCurrentValue": ("s", "Enabled"),
        "BiosSettingTargetValue": ("s", "Disabled"),
        "Created": ("t", 1_700_000_000),
    },
    {
        "AppstreamId": ("s", "org.fwupd.hsi.Kernel.Lockdown"),
        "Name": ("s", "Linux kernel lockdown"),
        "Summary": ("s", "Linux Kernel Lockdown"),
        "Description": ("s", "Lockdown keeps even root from changing the running kernel."),
        "Plugin": ("s", "linux_lockdown"),
        "HsiResult": ("u", NOT_ENABLED),
        "HsiResultSuccess": ("u", ENABLED),
        "Flags": ("t", RUNTIME_ISSUE | CONFIG_OS),
        "KernelCurrentValue": ("s", "none"),
        "KernelTargetValue": ("s", "integrity"),
        "Created": ("t", 1_700_000_000),
    },
]
# How often the folder is looked at for a change, in seconds.
LOOK_EVERY = 0.25

type Variant = tuple[str, Any]


def say(line: str) -> None:
    """Say what happened, for the test's log."""
    print(line, file=sys.stderr, flush=True)


def variant(key: str, value: object) -> Variant:
    """Give a field of fwupd's as the variant fwupd sends it in."""
    if key == "Flags" and isinstance(value, list):
        return ("t", sum(1 << FLAGS[flag] for flag in value if flag in FLAGS))
    if isinstance(value, bool):
        return ("b", value)
    if isinstance(value, int):
        return ("u" if key in NARROW else "t", value)
    if isinstance(value, list) and all(isinstance(each, str) for each in value):
        return ("as", value)
    if isinstance(value, list):
        return ("aa{sv}", [dictionary(each) for each in value])
    return ("s", str(value))


def dictionary(said: dict[str, object]) -> dict[str, Variant]:
    """Give a device or release as fwupd sends it: its releases are Release on the bus."""
    return {
        ("Release" if key == "Releases" else key): variant(key, value)
        for key, value in said.items()
    }


class Fwupd:
    """What the test wrote, read afresh for each ask."""

    def __init__(self, folder: Path, devices: Path) -> None:
        """Answer from FOLDER, saying DEVICES where the test wrote none."""
        self.folder = folder
        self.devices = devices

    def read(self, command: str) -> list[dict[str, object]]:
        """Give the devices a command's answer lists, or none where it isn't written."""
        path = self.folder / f"{command}.json"
        if command == "get-devices" and not path.exists():
            path = self.devices
        try:
            said = json.loads(path.read_text())
        except OSError, ValueError:
            return []
        devices: list[dict[str, object]] = said.get("Devices", [])
        return devices

    def upgrades(self, device: str) -> list[dict[str, object]]:
        """Give the releases offered to a device, newest first."""
        for offered in self.read("get-updates"):
            releases = offered.get("Releases")
            if offered.get("DeviceId") == device and isinstance(releases, list):
                return releases
        return []

    def seen(self) -> tuple[tuple[str, int, int], ...]:
        """Give what the folder holds, to tell when it changes."""
        if not self.folder.is_dir():
            return ()
        return tuple(
            sorted(
                (path.name, path.stat().st_mtime_ns, path.stat().st_size)
                for path in self.folder.iterdir()
            )
        )


def answer(connection: DBusConnection, fwupd: Fwupd, asked: Message) -> None:
    """Answer one method call as fwupd does, or refuse one it doesn't know."""
    fields = asked.header.fields
    interface = fields.get(HeaderFields.interface)
    member = fields.get(HeaderFields.member)
    if interface == PROPERTIES and member == "Get":
        _, name = asked.body
        if name in SAID:
            connection.send(new_method_return(asked, "v", (SAID[name],)))
            return
    elif interface == PROPERTIES and member == "GetAll":
        connection.send(new_method_return(asked, "a{sv}", (SAID,)))
        return
    elif member == "GetDevices":
        devices = [dictionary(device) for device in fwupd.read("get-devices")]
        connection.send(new_method_return(asked, "aa{sv}", (devices,)))
        return
    elif member == "GetHostSecurityAttrs":
        connection.send(new_method_return(asked, "aa{sv}", (SECURITY,)))
        return
    elif member == "GetHistory":
        history = [dictionary(entry) for entry in fwupd.read("get-history")]
        connection.send(new_method_return(asked, "aa{sv}", (history,)))
        return
    elif member == "GetUpgrades":
        releases = fwupd.upgrades(asked.body[0])
        if releases:
            offered = [dictionary(release) for release in releases]
            connection.send(new_method_return(asked, "aa{sv}", (offered,)))
        else:
            refused = new_error(asked, f"{NAME}.NothingToDo", "s", ("No releases found",))
            connection.send(refused)
        return
    said = f"{interface}.{member} isn't one fwupd's stand-in answers"
    connection.send(new_error(asked, "org.freedesktop.DBus.Error.UnknownMethod", "s", (said,)))


def main(argv: list[str]) -> None:
    """Hold fwupd's name on the system bus and answer for it until stopped."""
    fwupd = Fwupd(Path(argv[0]), Path(argv[1]))
    connection = open_dbus_connection(bus="SYSTEM")
    Proxy(message_bus, connection).RequestName(NAME)
    say(f"holding {NAME} as {connection.unique_name}")
    emitter = DBusAddress("/", interface=NAME)
    seen = fwupd.seen()
    looked = time.monotonic()
    while True:
        try:
            asked = connection.receive(timeout=LOOK_EVERY)
        except TimeoutError:
            asked = None
        if asked is not None and asked.header.message_type == MessageType.method_call:
            answer(connection, fwupd, asked)
        if time.monotonic() - looked >= LOOK_EVERY:
            looked = time.monotonic()
            now = fwupd.seen()
            if now != seen:
                seen = now
                connection.send(new_signal(emitter, "Changed"))
                say("said Changed")


if __name__ == "__main__":
    main(sys.argv[1:])

"""The desk's lights are told what to do again when they're plugged back in.

tests/desk-lights.nix runs this. Each stand-in logs the reports it's sent as hex. The Litra's
power command is 11 ff 04 1c, then 01 for on and 00 for off; the Pebble's lighting command is
03 6a 3a 02 00 25, then the same. Both come up as they were when plugged in again under their old
hidraw names, so their programs must notice the device behind the name is new.
"""

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from driver_names import machine, subtest

STAND_IN = "@standIn@"
HOLDERS = "/run/test-holders"
LOCK = "/run/test-locked"
IDS = {"litra": "0003:0000046D:0000C900", "pebble": "0003:0000041E:0000329A"}
LITRA_ON = "11ff041c01"
LITRA_OFF = "11ff041c00"
PEBBLE_ON = "036a3a02002501"
PEBBLE_OFF = "036a3a02002500"


def sent(kind: str) -> str:
    """Give where a stand-in logs what it's sent."""
    return f"/run/{kind}.log"


def plug_in(kind: str) -> None:
    """Plug a stand-in in, and wait until hidraw names it by its ids."""
    machine.succeed(f"systemd-run --unit={kind} python3 {STAND_IN} {kind} {sent(kind)}")
    machine.wait_until_succeeds(
        f"grep -l HID_ID={IDS[kind]} /sys/class/hidraw/*/device/uevent", timeout=30
    )


def replug(kind: str) -> None:
    """Pull a stand-in out, wait until hidraw lets it go, clear its log and plug it in again."""
    machine.succeed(f"systemctl stop {kind}.service")
    machine.wait_until_fails(
        f"grep -l HID_ID={IDS[kind]} /sys/class/hidraw/*/device/uevent", timeout=30
    )
    machine.succeed(f": > {sent(kind)}")
    plug_in(kind)


def told(kind: str, report: str) -> None:
    """Wait until a stand-in has been sent a report since its log was last cleared."""
    machine.wait_until_succeeds(f"grep -q '^{report}' {sent(kind)}", timeout=15)


machine.wait_for_unit("multi-user.target")
machine.succeed(f"mkdir -p {HOLDERS}")
plug_in("litra")
plug_in("pebble")
machine.succeed(f"systemd-run --unit=litra-follows litra follow {HOLDERS}")
lights = machine.succeed(
    "grep -oP 'ExecStart=\\K\\S+' /etc/systemd/user/pebble-lights.service"
).strip()
machine.succeed(f"systemd-run --unit=pebble-follows {lights} {LOCK}")

with subtest("a camera streaming lights the Litra"):
    machine.succeed(f"echo firefox > '{HOLDERS}/MX Brio'")
    told("litra", LITRA_ON)

with subtest("plugged back in while the camera streams, the Litra lights again"):
    replug("litra")
    told("litra", LITRA_ON)

with subtest("the camera stopping puts the Litra out"):
    machine.succeed(f"rm '{HOLDERS}/MX Brio'")
    told("litra", LITRA_OFF)

with subtest("the Pebble's lighting comes on when someone's there"):
    told("pebble", PEBBLE_ON)

with subtest("plugged back in, the Pebble's lighting still goes out at a lock"):
    replug("pebble")
    machine.succeed(f"echo locked > {LOCK}")
    told("pebble", PEBBLE_OFF)

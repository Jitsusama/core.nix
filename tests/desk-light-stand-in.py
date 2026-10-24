"""A desk light made of uhid, for tests/desk-lights-vm-test.py to plug in and pull out.

The kernel's uhid makes a HID device of whatever a program describes. Given `litra`, this is a
Litra Glow, with its USB ids and a Logitech long report. Given `pebble`, it's a Pebble X Plus's
control channel, with its ids and its vendor collection, which is how pebble-lights tells it
from the speakers' other HID collections. Each report written to it is logged as hex to the file
named as the second argument, and answered as the device answers. Stopping it, by SIGTERM or
SIGINT, takes the device away, as pulling its cable does.
"""

import os
import signal
import struct
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Callable

UHID_DESTROY = 1
UHID_OUTPUT = 6
UHID_CREATE2 = 11
UHID_INPUT2 = 12
BUS_USB = 0x03
EVENT = 4380


def litra_answer(report: bytes) -> bytes:
    """Answer as the Litra does: its header and the command echoed back, 20 bytes."""
    return report[:20].ljust(20, b"\0")


def pebble_answer(report: bytes) -> bytes:
    """Answer as the Pebble does: a lighting command acknowledged as taken, 64 bytes."""
    # The command's operation, after the report id, the transport, the command and its length.
    operation = report[5:6] or b"\0"
    return (bytes([0x03, 0x6A, 0x02, 0x03, 0x00, 0x3A, 0x00]) + operation).ljust(64, b"\0")


@dataclass(frozen=True)
class Device:
    """A device's name, its USB ids, its report descriptor, and how it answers."""

    name: bytes
    vendor: int
    product: int
    descriptor: bytes
    answer: Callable[[bytes], bytes]


DEVICES = {
    # Report 0x11, 19 bytes each way on a vendor page, as Logitech's long reports are.
    "litra": Device(
        b"Litra Glow stand-in",
        0x046D,
        0xC900,
        bytes.fromhex("0643ff0a0206a101 8511 7508 9513 1500 26ff00 0904 8100 0904 9100 c0"),
        litra_answer,
    ),
    # Report 0x03, 63 bytes each way in the vendor collection 0xFF01.
    "pebble": Device(
        b"Pebble X Plus stand-in",
        0x041E,
        0x329A,
        bytes.fromhex("0601ff 0901 a101 8503 7508 953f 1500 26ff00 0901 8100 0901 9100 c0"),
        pebble_answer,
    ),
}


def create(device: Device) -> bytes:
    """Give the event that makes the device."""
    return struct.pack(
        "<I128s64s64sHHIIII4096s",
        UHID_CREATE2,
        device.name,
        b"",
        b"stand-in",
        len(device.descriptor),
        BUS_USB,
        device.vendor,
        device.product,
        0,
        0,
        device.descriptor,
    )


def main() -> None:
    """Make the device, log and answer every report, and take it away when stopped."""
    device = DEVICES[sys.argv[1]]
    log = Path(sys.argv[2])
    uhid = os.open("/dev/uhid", os.O_RDWR)
    os.write(uhid, create(device))

    def stop(_signal: int, _frame: object) -> None:
        os.write(uhid, struct.pack("<I", UHID_DESTROY))
        os.close(uhid)
        sys.exit(0)

    signal.signal(signal.SIGTERM, stop)
    signal.signal(signal.SIGINT, stop)
    while True:
        event = os.read(uhid, EVENT)
        (kind,) = struct.unpack_from("<I", event)
        if kind != UHID_OUTPUT:
            continue
        data, size = struct.unpack_from("<4096sH", event, 4)
        report = data[:size]
        with log.open("a") as written:
            written.write(report.hex() + "\n")
        reply = device.answer(report)
        os.write(uhid, struct.pack("<IH4096s", UHID_INPUT2, len(reply), reply))


if __name__ == "__main__":
    main()

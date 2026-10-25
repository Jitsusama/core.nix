"""The desk's lights are told what to do again when they're plugged back in.

tests/desk-lights.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py). Each
stand-in logs the reports it's sent as hex. The Litra's power command is 11 ff 04 1c, then 01
for on and 00 for off; the Pebble's lighting command is 03 6a 3a 02 00 25, then the same. Both
come up as they were when plugged in again under their old hidraw names, so their programs
must notice the device behind the name is new. Each test asks for the state it starts from as
a fixture, the lights plugged in and followed, then a camera streaming, so any test runs alone
with `-k`. The stand-in's path is desk-lights.nix's, given as the `values` fixture.
"""

from __future__ import annotations

import datetime as dt
from typing import TYPE_CHECKING, Any

import pytest

if TYPE_CHECKING:
    from test_driver.machine import BaseMachine

HOLDERS = "/run/test-holders"
LOCK = "/run/test-locked"
IDS = {"litra": "0003:0000046D:0000C900", "pebble": "0003:0000041E:0000329A"}
LITRA_ON = "11ff041c01"
LITRA_OFF = "11ff041c00"
PEBBLE_ON = "036a3a02002501"
PEBBLE_OFF = "036a3a02002500"
# How long hidraw may take to name a stand-in, or let it go.
PLUGGING = dt.timedelta(seconds=30)
# How long a light's program may take to tell it something.
TELLING = dt.timedelta(seconds=15)


class Lights:
    """The stand-in lights, and what their programs send them."""

    def __init__(self, machine: BaseMachine, stand_in: str) -> None:
        """Act on the stand-ins in that machine, each run from that script."""
        self.machine = machine
        self.stand_in = stand_in

    @staticmethod
    def sent(kind: str) -> str:
        """Give where a stand-in logs what it's sent."""
        return f"/run/{kind}.log"

    def named(self, kind: str) -> str:
        """Give a shell test of hidraw naming a stand-in by its ids."""
        return f"grep -l HID_ID={IDS[kind]} /sys/class/hidraw/*/device/uevent"

    def plug_in(self, kind: str) -> None:
        """Plug a stand-in in, and wait until hidraw names it by its ids."""
        self.machine.succeed(
            f"systemd-run --unit={kind} python3 {self.stand_in} {kind} {self.sent(kind)}"
        )
        self.machine.wait_until_succeeds(self.named(kind), timeout=PLUGGING)

    def replug(self, kind: str) -> None:
        """Pull a stand-in out, wait until hidraw lets it go, clear its log and plug it in."""
        self.machine.succeed(f"systemctl stop {kind}.service")
        self.machine.wait_until_fails(self.named(kind), timeout=PLUGGING)
        self.machine.succeed(f": > {self.sent(kind)}")
        self.plug_in(kind)

    def told(self, kind: str, report: str) -> None:
        """Wait until a stand-in has been sent a report since its log was last cleared."""
        self.machine.wait_until_succeeds(f"grep -q '^{report}' {self.sent(kind)}", timeout=TELLING)


@pytest.fixture(scope="module")
def lights(machine: BaseMachine, values: dict[str, Any]) -> Lights:
    """Plug both lights in, and start the Litra's and the Pebble's programs following them."""
    machine.wait_for_unit("multi-user.target")
    machine.succeed(f"mkdir -p {HOLDERS}")
    plugged = Lights(machine, values["standIn"])
    plugged.plug_in("litra")
    plugged.plug_in("pebble")
    machine.succeed(f"systemd-run --unit=litra-follows litra follow {HOLDERS}")
    program = machine.succeed(
        "grep -oP 'ExecStart=\\K\\S+' /etc/systemd/user/pebble-lights.service"
    ).strip()
    machine.succeed(f"systemd-run --unit=pebble-follows {program} {LOCK}")
    return plugged


@pytest.fixture(scope="module")
def streaming(lights: Lights) -> Lights:
    """Have a camera stream, and wait until the Litra is told to light."""
    lights.machine.succeed(f"echo firefox > '{HOLDERS}/MX Brio'")
    lights.told("litra", LITRA_ON)
    return lights


def test_a_camera_streaming_lights_the_litra(streaming: Lights) -> None:
    """A camera streaming lights the Litra."""
    streaming.told("litra", LITRA_ON)


def test_the_litra_plugged_back_in_while_a_camera_streams_lights_again(
    streaming: Lights,
) -> None:
    """Plugged back in while the camera streams, the Litra lights again."""
    streaming.replug("litra")
    streaming.told("litra", LITRA_ON)


def test_the_camera_stopping_puts_the_litra_out(streaming: Lights) -> None:
    """The camera stopping puts the Litra out."""
    streaming.machine.succeed(f"rm '{HOLDERS}/MX Brio'")
    streaming.told("litra", LITRA_OFF)


def test_the_pebble_s_lighting_comes_on_when_someone_s_there(lights: Lights) -> None:
    """The Pebble's lighting comes on when someone's there."""
    lights.told("pebble", PEBBLE_ON)


def test_the_pebble_plugged_back_in_still_goes_out_at_a_lock(lights: Lights) -> None:
    """Plugged back in, the Pebble's lighting still goes out at a lock."""
    lights.replug("pebble")
    lights.machine.succeed(f"echo locked > {LOCK}")
    lights.told("pebble", PEBBLE_OFF)

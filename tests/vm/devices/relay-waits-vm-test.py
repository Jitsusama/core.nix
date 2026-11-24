"""How long the relay waits for a camera's frames, and what it does when none come.

relay-waits.nix, beside this, runs this through pytest inside the driver (tests/lib/vm_pytest.py),
and its opening comment says what has to hold. Each relay is started by the test that reads it,
with a client reading its loopback, and stopped as that test ends, so any test runs alone.
"""

from __future__ import annotations

import re
from datetime import timedelta
from typing import TYPE_CHECKING

import pytest

if TYPE_CHECKING:
    from collections.abc import Iterator

    from test_driver.machine import BaseMachine

# What the relay says as it makes the camera's pipeline again, with how long the client waited.
REMADE = re.compile(r"a client waited (\d+) s with no frame from the camera; starting it again")
# What it says as it ends, the pipeline made again having given no frame either.
ENDING = "the camera still gives no frames; ending so it starts afresh"
# How long a client waits for a first frame, and for the next once frames have stopped, before
# the pipeline is made again: the patch's STALL_SECONDS and FLOWING_SECONDS.
STALL = 90
FLOWING = 10
# How long the camera that stops gives frames for each time it starts: 60 at 30 a second.
FRAMING = 2
# How many times that camera is to be made again, to show the wait is the same each time.
AGAIN = 2
# How late the relay may see a wait has run out: it looks every two seconds, and counts in
# whole seconds, so a wait it says can run past its bound by as much.
LOOKING = 3
# How long a test gives the relay to say what it says, beyond the wait it's saying it after:
# the loopback's client takes a moment to be counted on a busy builder.
SAYING = timedelta(seconds=30)
# How long udev, and the relay's first placeholder, may take with a new loopback on a busy
# builder; each wait ends as soon as it's done.
SETTLING = timedelta(minutes=2)


class Relay:
    """One of the machine's relays, read by a client from its loopback."""

    def __init__(self, machine: BaseMachine, name: str) -> None:
        """Act on the relay of that input."""
        self.machine = machine
        self.unit = f"relay-{name}.service"

    def start(self) -> None:
        """Start the relay, then a client reading its loopback until it's stopped."""
        self.machine.succeed(f"systemctl start {self.unit}")
        device = self.machine.succeed(
            f"cat /run/{self.unit.removesuffix('.service')}/device"
        ).strip()
        self.machine.succeed(f"udevadm wait --timeout {SETTLING.seconds} {device}")
        # The loopback is exclusive, as the shared cameras' are, so it offers a picture to
        # read only once the relay writes its placeholder, which a first start of GStreamer,
        # reading its plugins, can put off for seconds.
        self.machine.wait_until_succeeds(
            f"v4l2-ctl -d {device} --info | sed -n '/Device Caps/,$p' | grep -q 'Video Capture'",
            timeout=SETTLING.seconds,
        )
        # The client streams for as long as the relay gives it frames, and waits through the
        # gaps, as a call's camera does.
        self.machine.succeed(
            f"systemd-run --unit=client-{self.unit} -p Type=exec"
            f" v4l2-ctl -d {device} --stream-mmap --stream-count=0 --stream-to=/dev/null"
        )

    def stop(self) -> None:
        """Stop the client and the relay, whichever are still running."""
        self.machine.execute(f"systemctl stop client-{self.unit} {self.unit}")
        self.machine.execute(f"systemctl reset-failed client-{self.unit} {self.unit}")

    def saying(self) -> str:
        """Give the command that prints what the relay has said since it last started."""
        return (
            "journalctl -o cat"
            f" _SYSTEMD_INVOCATION_ID=$(systemctl show -P InvocationID {self.unit})"
        )

    def said(self) -> str:
        """Give what the relay has said since it last started, an ended one included."""
        return self.machine.succeed(self.saying())

    def waits(self) -> list[int]:
        """Give how long the client waited before each time the pipeline was made again."""
        return [int(waited) for waited in REMADE.findall(self.said())]

    def remade(self, times: int, within: timedelta) -> list[int]:
        """Wait until the pipeline has been made again that many times, giving each wait."""
        self.machine.wait_until_succeeds(
            f"[ $({self.saying()} | grep -c 'starting it again') -ge {times} ]",
            timeout=within.seconds,
        )
        return self.waits()


@pytest.fixture
def never(machine: BaseMachine) -> Iterator[Relay]:
    """Give the relay whose camera never hands on a frame, started and read."""
    relay = Relay(machine, "never")
    relay.start()
    yield relay
    relay.stop()


@pytest.fixture
def stops(machine: BaseMachine) -> Iterator[Relay]:
    """Give the relay whose camera gives two seconds of frames and stops, started and read."""
    relay = Relay(machine, "stops")
    relay.start()
    yield relay
    relay.stop()


def test_a_camera_that_never_frames_is_waited_on_90_s_then_ended(never: Relay) -> None:
    """A camera that never frames is made again after 90 s, and ended 90 s after that."""
    waits = never.remade(1, timedelta(seconds=STALL + LOOKING) + SAYING)
    assert waits, f"The relay never made its camera again:\n{never.said()}"
    assert STALL <= waits[0] <= STALL + LOOKING, f"The relay waited {waits[0]} s, not {STALL}"
    never.machine.wait_until_succeeds(
        f"! systemctl is-active --quiet {never.unit}",
        timeout=(timedelta(seconds=STALL + LOOKING) + SAYING).seconds,
    )
    said = never.said()
    assert ENDING in said, f"The relay ended without saying why:\n{said}"
    result = never.machine.succeed(f"systemctl show -P Result {never.unit}").strip()
    assert result == "exit-code", f"The relay ended as {result}, not with its failure"


def test_a_camera_that_stops_is_waited_on_10_s_each_time(stops: Relay) -> None:
    """A camera whose frames stop is made again after 10 s, each time, and never ended."""
    # Its frames, then the wait, each time, the first time included.
    waits = stops.remade(AGAIN, AGAIN * timedelta(seconds=FRAMING + FLOWING + LOOKING) + SAYING)
    assert len(waits) >= AGAIN, (
        f"The relay made its camera again {len(waits)} times:\n{stops.said()}"
    )
    for waited in waits:
        assert FLOWING <= waited <= FLOWING + LOOKING, f"The relay waited {waited} s, not {FLOWING}"
    assert ENDING not in stops.said(), "The relay ended a camera that framed again"
    stops.machine.succeed(f"systemctl is-active --quiet {stops.unit}")

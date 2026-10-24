"""A stand-in IR camera relayed to the programs that read it, and to no others.

tests/ir-camera.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), and
its opening comment says what has to hold. Each test asks for the state it starts from as a
fixture: the relay booted, or the stand-in camera sending its pattern. A test that reads what
the relay said counts only what came after it began, so any test runs alone with `-k`. The
two that change the machine, stopping the stand-in and taking the loopback away, come last.
The frames' size, the one value the tests need from Nix, is the `values` fixture.
"""

from __future__ import annotations

import datetime as dt
from typing import TYPE_CHECKING, Any

import pytest

if TYPE_CHECKING:
    from test_driver.machine import BaseMachine

RELAYED = "/dev/ir-camera"
# The relay's device as systemd names it, which the relay is bound to.
DEVICE = "'dev-ir\\x2dcamera.device'"
# How long the relay may take to let the camera go once its reader stops, or a relay whose
# loopback went to be left waiting for it.
LETTING_GO = dt.timedelta(seconds=30)
# How long a paused camera is given to hand a reader a stale frame.
PAUSED_READ = dt.timedelta(seconds=3)
# How long the next reader is given to read once the camera has stopped sending.
NEXT_READ = dt.timedelta(seconds=5)
# How long the holders watcher takes for the two silent looks after it says nobody: it looks
# once a second.
TWO_LOOKS = dt.timedelta(seconds=2)
# How many frames a reader takes, enough for the relay's first ones to pass.
FRAMES = 10


class Camera:
    """The stand-in camera, sending its pattern to a loopback the relay reads."""

    def __init__(self, machine: BaseMachine, source: str, relay: str) -> None:
        """Act on the stand-in at source, which the relay with that process id reads."""
        self.machine = machine
        self.source = source
        self.holds = f"readlink /proc/{relay}/fd/* | grep -q -x -F {source}"

    def read(self, name: str, count: int = FRAMES) -> list[str]:
        """Read that many frames from the relay into /tmp's file of that name, giving each."""
        self.machine.succeed(
            f"v4l2-ctl -d {RELAYED} --stream-mmap --stream-count={count} --stream-to=/tmp/{name}"
        )
        return self.machine.succeed(f"frames /tmp/{name}").split()

    def said(self, line: str) -> int:
        """Give how many times the relay has logged the line this boot."""
        return int(
            self.machine.succeed(
                f"journalctl -b -o cat -u ir-camera.service | grep -c -F '{line}' || true"
            )
        )


@pytest.fixture(scope="module")
def booted(machine: BaseMachine) -> BaseMachine:
    """Boot the machine with the relay running and its device made."""
    machine.wait_for_unit("ir-camera.service")
    machine.wait_for_file(RELAYED)
    return machine


@pytest.fixture(scope="module")
def camera(booted: BaseMachine) -> Camera:
    """Start the stand-in camera on a loopback of its own, which the relay finds by its name."""
    source = booted.succeed("v4l2loopback-ctl add -n 'Stand-in IR' -x 1 -b 2").strip()
    booted.succeed(f"udevadm wait --timeout 10 {source}")
    booted.succeed(f"systemd-run --unit=stand-in -p Type=exec stand-in-camera {source}")
    booted.wait_for_file("/run/stand-in-camera")
    relay = booted.succeed("systemctl show -P MainPID ir-camera.service").strip()
    return Camera(booted, source, relay)


def test_the_relay_waits_for_udev(booted: BaseMachine) -> None:
    """The relay waits for udev to give it the loopback."""
    restarts = booted.succeed("systemctl show -P NRestarts ir-camera.service").strip()
    assert restarts == "0"
    booted.fail("journalctl -b -o cat -u ir-camera.service | grep -q 'Permission denied'")


def test_the_camera_stays_closed_until_a_program_reads(camera: Camera) -> None:
    """The camera stays closed until a program reads."""
    camera.machine.fail(camera.holds)


def test_programs_get_grey_frames_not_a_camera(camera: Camera, values: dict[str, Any]) -> None:
    """Programs get grey frames, and browsers don't take it for a camera."""
    video_format = camera.machine.succeed(f"v4l2-ctl -d {RELAYED} --get-fmt-video")
    assert "'GREY'" in video_format
    assert f"{values['width']}/{values['height']}" in video_format
    capabilities = camera.machine.succeed(f"v4l2-ctl -d {RELAYED} --info")
    assert "Video Output" in capabilities.split("Device Caps")[1]


def test_a_reader_gets_the_frames_in_grey(camera: Camera) -> None:
    """A reader gets the camera's frames in grey."""
    names = camera.read("read")
    print(f"The reader got: {' '.join(names)}")
    assert "other" not in names
    assert names[-1] == "pattern", " ".join(names)


def test_a_paused_cameras_next_reader_gets_no_old_frame(camera: Camera) -> None:
    """A paused camera's next reader gets no frame from before it opened."""
    camera.machine.succeed("touch /run/stand-in-paused; sleep 1")
    try:
        seconds = int(PAUSED_READ.total_seconds())
        camera.machine.execute(
            f"timeout {seconds} v4l2-ctl -d {camera.source} --stream-mmap --stream-count=1"
            " --stream-to=/tmp/stale"
        )
        stale = int(camera.machine.succeed("stat -c %s /tmp/stale 2>/dev/null || echo 0"))
        assert stale == 0, "A reader opening a paused camera got the frame from before it"
    finally:
        camera.machine.succeed("rm -f /run/stand-in-paused")


def test_the_camera_closes_when_the_reader_stops(camera: Camera) -> None:
    """The camera closes when the reader stops."""
    stopped = camera.said("The reader stopped")
    camera.read("closing")
    camera.machine.wait_until_succeeds(f"! {camera.holds}", timeout=LETTING_GO)
    camera.machine.wait_until_succeeds(
        f"[ $(journalctl -b -o cat -u ir-camera.service | grep -c -F 'The reader stopped')"
        f" -gt {stopped} ]",
        timeout=LETTING_GO,
    )


def test_the_next_reader_never_gets_the_last_ones_frames(camera: Camera) -> None:
    """The next reader never gets the last one's frames.

    It stops the stand-in camera, so it comes after every test that reads from it.
    """
    assert camera.read("last")[-1] == "pattern", "The last reader got no pattern to leave"
    camera.machine.succeed("systemctl stop stand-in.service")
    seconds = int(NEXT_READ.total_seconds())
    camera.machine.execute(
        f": > /tmp/next; timeout {seconds} v4l2-ctl -d {RELAYED} --stream-mmap"
        " --stream-count=1 --stream-to=/tmp/next"
    )
    names = camera.machine.succeed("frames /tmp/next").split()
    assert "pattern" not in names, "The next reader got the last one's frames"


def test_a_relay_whose_loopback_has_gone_waits_for_it(booted: BaseMachine) -> None:
    """A relay started with its loopback gone waits for it rather than retrying.

    It takes the loopback away and brings it back, so it comes last.
    """
    booted.succeed("systemctl stop ir-camera.service")
    booted.succeed("touch /tmp/before-gone")
    booted.succeed("v4l2loopback-ctl delete $(readlink /run/ir-camera-loopback/sink)")
    # systemd hears the device go from udev a moment after the delete, and a start before
    # then runs the relay on a device about to vanish rather than waiting for it.
    booted.wait_until_succeeds(f"! systemctl is-active --quiet {DEVICE}", timeout=LETTING_GO)
    booted.succeed("systemctl start --no-block ir-camera.service")
    # Bound to the loopback's device, the relay's start waits for it, so the relay itself
    # never runs to fail.
    booted.wait_until_succeeds(
        "systemctl list-jobs --no-legend ir-camera.service | grep -q waiting",
        timeout=LETTING_GO,
    )
    # The holders watcher says nobody as the node goes, then looks for it once a second
    # without a word, so two looks follow the saying.
    booted.wait_until_succeeds(
        "test /run/camera-holders/ir-camera -nt /tmp/before-gone", timeout=LETTING_GO
    )
    booted.sleep(TWO_LOOKS)
    restarts = booted.succeed("systemctl show -P NRestarts ir-camera.service").strip()
    assert restarts == "0", "The relay kept retrying a loopback that had gone"
    holders = booted.succeed("systemctl show -P NRestarts ir-camera-holders.service").strip()
    assert holders == "0", "Saying who holds the camera failed while it was gone"
    booted.succeed("systemctl restart ir-camera-loopback.service ir-camera.service")
    booted.wait_for_unit("ir-camera.service")
    booted.wait_for_unit("ir-camera-holders.service")
    assert booted.succeed("cat /run/camera-holders/ir-camera") == ""

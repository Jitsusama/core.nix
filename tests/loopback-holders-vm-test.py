"""Which programs have a loopback camera open, as the file naming them says.

tests/loopback-holders.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py),
and its opening comment says what has to hold. Each test starts from the stand-in camera up
and nobody named, and stops what it started and points the holders back at the stand-in as
it ends, so any runs alone. The file naming the holders and the programs that hold the
camera are Nix's, given as the `values` fixture.
"""

from __future__ import annotations

from datetime import timedelta
from typing import TYPE_CHECKING, Any

import pytest

if TYPE_CHECKING:
    from collections.abc import Callable, Iterator

    from test_driver.machine import BaseMachine

# Where the stand-in's service writes the loopback it made, which the holders watch.
DEVICE = "/run/stand-in/device"
# How long the file may take to follow an open or a close: it's looked at on each, and every
# two seconds while anyone holds the camera.
FOLLOWING = timedelta(seconds=10)
# How long something that shouldn't be named is given to be named anyway: two of the looks
# made while the camera is held.
LOOKING = timedelta(seconds=4)
# How long the holders are given to see an open they'd name, which they look at as it happens.
NOTICING = timedelta(seconds=1)


class Camera:
    """The stand-in camera on the machine, the file naming its holders, and who holds it."""

    def __init__(self, machine: BaseMachine, values: dict[str, Any]) -> None:
        """Act on the stand-in in that machine, with the programs Nix built to hold it."""
        self.machine = machine
        self.file: str = values["file"]
        self.bash: str = values["bash"]
        self.sleep: str = values["sleep"]
        # A program as Nix wraps one, which waits until it's stopped.
        self.doze: str = values["doze"]
        # Takes the name it's given, then holds the camera it's given open.
        self.renamed: str = values["renamed"]

    def device(self) -> str:
        """Give the loopback the stand-in's service made, once udev has it."""
        device = self.machine.succeed(f"cat {DEVICE}").strip()
        self.machine.succeed(f"udevadm wait --timeout 10 {device}")
        return device

    def holders(self) -> list[str]:
        """Give the names the file holds, one a line."""
        return self.machine.succeed(f"cat {self.file}").split("\n")[:-1]

    def holding(self, program: str, camera: str) -> str:
        """Give how to run a program with the camera it's given held open."""
        return f"{self.bash} -c 'exec 3< {camera}; exec {program} 300'"

    def named(self, name: str) -> None:
        """Wait until the file names that program."""
        self.machine.wait_until_succeeds(f"grep -q -x {name} {self.file}", timeout=FOLLOWING)

    def nobody(self) -> None:
        """Wait until the file names nobody."""
        self.machine.wait_until_succeeds(f"! test -s {self.file}", timeout=FOLLOWING)


@pytest.fixture(scope="module")
def camera(machine: BaseMachine, values: dict[str, Any]) -> Camera:
    """Give the stand-in camera with its holders watched, and nobody holding it yet."""
    machine.wait_for_unit("stand-in-holders.service")
    camera = Camera(machine, values)
    machine.wait_for_file(camera.file)
    camera.device()
    return camera


@pytest.fixture
def hold(camera: Camera) -> Iterator[Callable[..., None]]:
    """Give a way to run a command as someone else, in a unit of its own; each goes at the end.

    Whatever someone still runs goes too, as a holder's child can outlive its unit, and the
    next test starts once the file names nobody.
    """
    started: list[str] = []

    def holding(unit: str, command: str, extra: str = "") -> None:
        camera.machine.succeed(
            f"systemd-run --unit={unit} --uid=someone -p Type=exec {extra} -- {command}"
        )
        started.append(unit)

    yield holding
    for unit in started:
        camera.machine.succeed(f"systemctl stop {unit}.service || true")
    camera.machine.succeed("pkill -u someone || true")
    camera.nobody()


@pytest.fixture
def pointed(camera: Camera) -> Iterator[Callable[[str], None]]:
    """Give a way to point the holders at another device; they're pointed back at the end."""
    own = camera.machine.succeed(f"cat {DEVICE}").strip()

    def point(device: str) -> None:
        camera.machine.succeed(
            "systemctl stop stand-in-holders.service",
            f"echo {device} > {DEVICE}",
            "systemctl start stand-in-holders.service",
        )

    yield point
    point(own)
    camera.machine.wait_for_unit("stand-in-holders.service")


def test_the_relay_holding_the_camera_isnt_named(camera: Camera) -> None:
    """The relay holding the camera isn't named, since the holders leave out its unit."""
    device = camera.device()
    relay = camera.machine.succeed("systemctl show -P MainPID stand-in.service").strip()
    camera.machine.wait_until_succeeds(
        f"readlink /proc/{relay}/fd/* | grep -q -x -F {device}", timeout=FOLLOWING
    )
    camera.machine.sleep(duration=NOTICING)
    assert camera.holders() == []


def test_each_program_another_account_has_it_open_with_is_named(
    camera: Camera, hold: Callable[..., None]
) -> None:
    """Each program another account has it open with is named, in a file anyone may read."""
    device = camera.device()
    hold("held", camera.holding(camera.sleep, device))
    hold("dozing", camera.holding(camera.doze, device))
    camera.named("doze")
    assert camera.holders() == ["doze", "sleep"]
    assert camera.machine.succeed(f"stat -c %a {camera.file}").strip() == "644"


def test_it_empties_once_they_close_it(camera: Camera, hold: Callable[..., None]) -> None:
    """It empties once they close it."""
    device = camera.device()
    hold("held", camera.holding(camera.sleep, device))
    camera.named("sleep")
    camera.machine.succeed("systemctl stop held.service")
    camera.nobody()


def test_a_program_named_as_the_relay_is_named_and_a_name_is_one_line(
    camera: Camera, hold: Callable[..., None]
) -> None:
    """A program named as the relay is named, and a name is one line however it's written."""
    device = camera.device()
    hold("posing", f"{camera.renamed} stand-in-relay {device}")
    hold("odd", f"'{camera.renamed}' 'a\\nb\\xff' {device}")
    camera.named("stand-in-relay")
    camera.machine.wait_until_succeeds(f"grep -q '^a?b' {camera.file}", timeout=FOLLOWING)
    named = camera.holders()
    assert "stand-in-relay" in named, named
    others = [name for name in named if name != "stand-in-relay"]
    assert len(others) == 1, named
    assert others[0].startswith("a?b"), named


def test_a_holder_that_leaves_the_camera_to_its_child_is_replaced_by_it(
    camera: Camera, hold: Callable[..., None]
) -> None:
    """A holder that leaves the camera to its child is replaced by it, with no open or close."""
    device = camera.device()
    hold(
        "forking",
        f"{camera.bash} -c 'exec 3< {device}; {camera.doze} & {camera.sleep} 1'",
        # So the child outlives the parent's unit, as it would its shell.
        "-p KillMode=process",
    )
    camera.named("doze")
    camera.machine.wait_until_succeeds(f"! grep -q -x bash {camera.file}", timeout=FOLLOWING)
    assert camera.holders() == ["doze"]


def test_its_folder_outlives_the_cameras_service_and_says_nobody_while_its_down(
    camera: Camera, hold: Callable[..., None]
) -> None:
    """Its folder outlives the camera's service, and says nobody while it's down."""
    folder = camera.machine.succeed("stat -c %i /run/camera-holders").strip()
    hold("stopping", camera.holding(camera.sleep, camera.device()))
    camera.named("sleep")
    # The camera stops while the program still holds it, so only the service's stopping can
    # empty the file.
    camera.machine.succeed("systemctl stop stand-in.service")
    camera.machine.wait_until_fails(
        "systemctl is-active stand-in-holders.service", timeout=FOLLOWING
    )
    assert camera.machine.succeed(f"cat {camera.file}") == ""
    camera.machine.succeed("systemctl stop stopping.service", "systemctl start stand-in.service")
    camera.machine.wait_for_unit("stand-in-holders.service")
    assert camera.machine.succeed("stat -c %i /run/camera-holders").strip() == folder


def test_it_follows_the_camera_to_the_next_loopback(
    camera: Camera, hold: Callable[..., None]
) -> None:
    """It follows the camera to the next loopback the camera's service makes."""
    camera.machine.succeed("systemctl restart stand-in.service")
    camera.machine.wait_for_unit("stand-in-holders.service")
    hold("again", camera.holding(camera.sleep, camera.device()))
    camera.named("sleep")


def test_another_cameras_loopback_isnt_taken_for_it(
    camera: Camera, hold: Callable[..., None], pointed: Callable[[str], None]
) -> None:
    """Another camera's loopback isn't taken for it, even pointed at, since its name differs."""
    other = camera.machine.succeed("v4l2loopback-ctl add -n 'Other'").strip()
    camera.machine.succeed(f"udevadm wait --timeout 10 {other}")
    pointed(other)
    hold("elsewhere", camera.holding(camera.sleep, other))
    camera.machine.sleep(duration=LOOKING)
    assert camera.holders() == []
    camera.machine.succeed("systemctl stop elsewhere.service", f"v4l2loopback-ctl delete {other}")


def test_it_waits_while_the_camera_isnt_there_and_finds_it_when_it_is(
    camera: Camera, hold: Callable[..., None], pointed: Callable[[str], None]
) -> None:
    """It waits while the camera isn't there, rather than failing, and finds it when it is."""
    device = camera.device()
    pointed("/dev/video-not-yet")
    camera.machine.sleep(duration=LOOKING)
    camera.machine.succeed("systemctl is-active stand-in-holders.service")
    restarts = camera.machine.succeed("systemctl show -P NRestarts stand-in-holders.service")
    assert restarts.strip() == "0"
    assert camera.holders() == []
    camera.machine.succeed(f"echo {device} > {DEVICE}")
    hold("found", camera.holding(camera.sleep, device))
    camera.named("sleep")

"""A switch bringing in the MX Brio's sharing takes hold of a Brio already plugged in.

mx-brio-at-switch.nix, beside this, runs this through pytest inside the driver
(tests/lib/vm_pytest.py), and its opening comment says what has to hold. The machine boots
without the sharing, with a stand-in Brio and someone at the seat, and the module's fixture
switches it in once, so every test reads the machine the switch left. The tests run in the
order written: the last two run the take-hold again, and the last unplugs the stand-in. The
cameras' names are Nix's, given as the `values` fixture.
"""

from __future__ import annotations

from datetime import timedelta
from typing import TYPE_CHECKING, Any

import pytest
from test_driver.errors import RequestedAssertionFailed

if TYPE_CHECKING:
    from test_driver.machine import BaseMachine

# How long udev may take to finish with the stand-in. It's done at once on a quiet host, but a
# busy builder has taken minutes to boot a machine, its udev far behind; the wait ends as soon
# as udev is done, so a long bound costs nothing when it's quick.
SETTLING = timedelta(minutes=2)
# How long the relay may take to start once its node is named: systemd starts it as the node
# appears, so it's a moment on a quiet host, and the bound is for a busy one.
STARTING = timedelta(seconds=60)


def the_stand_in(machine: BaseMachine, card: str) -> str:
    """Give the stand-in Brio's node, once udev is done with it."""
    node = machine.succeed(
        f"for n in /sys/class/video4linux/*; do grep -qx '{card}' $n/name && basename $n; done"
    ).strip()
    assert node, f"no loopback is named {card}"
    machine.succeed(f"udevadm wait --timeout {SETTLING.seconds} /dev/{node}")
    return f"/dev/{node}"


def the_old_watch(node: str) -> str:
    """Name the seat's camera watch on the node, which a relayed camera doesn't keep."""
    return f"usb-camera-holders@{node.removeprefix('/dev/')}.service"


def the_seat_can_open(machine: BaseMachine, node: str) -> bool:
    """Say whether the one at the seat has been handed the node."""
    return "user:someone:" in machine.succeed(f"getfacl -p {node}")


@pytest.fixture(scope="module")
def switched(machine: BaseMachine, values: dict[str, Any]) -> str:
    """Switch the sharing in over a Brio plugged in and handed to the seat, giving its node."""
    machine.wait_for_unit("multi-user.target")
    try:
        machine.wait_for_unit("stand-in-brio.service")
    except RequestedAssertionFailed:
        # What the stand-in said, which a failed unit's state alone doesn't.
        print(machine.execute("journalctl -b -u stand-in-brio --no-pager")[1])
        raise
    machine.wait_for_unit("getty@tty1.service")
    machine.wait_until_succeeds("loginctl show-user someone -p State --value | grep -qx active")
    node = the_stand_in(machine, values["card"])
    other = the_stand_in(machine, values["other"])
    for camera in (node, other):
        machine.wait_until_succeeds(f"getfacl -p {camera} | grep -q user:someone:", timeout=30)
    machine.wait_for_unit(the_old_watch(node))
    assert machine.execute("test -e /dev/mx-brio")[0] != 0, (
        "the Brio was named for its relay before the sharing was switched in"
    )
    shared = machine.succeed("readlink -f /run/current-system/specialisation/shared").strip()
    machine.succeed(f"{shared}/bin/switch-to-configuration test")
    return node


def test_the_switch_names_the_brio_for_its_relay(machine: BaseMachine, switched: str) -> None:
    """The relay's name for the Brio points at the one plugged in before the switch."""
    named = machine.succeed("readlink -e /dev/mx-brio || true").strip()
    assert named == switched, f"/dev/mx-brio is {named or 'missing'}, not the Brio at {switched}"


def test_the_switch_starts_the_relay(machine: BaseMachine, switched: str) -> None:
    """The relay runs once the switch is done, with nobody plugging the Brio in again."""
    del switched
    machine.wait_for_unit("mx-brio.service", timeout=STARTING.seconds)


def test_the_switch_takes_the_brio_back_from_the_seat(machine: BaseMachine, switched: str) -> None:
    """Nobody at the seat keeps what logind handed them of the Brio before the switch."""
    assert not the_seat_can_open(machine, switched), (
        f"someone at the seat can still open the Brio: {machine.succeed(f'getfacl -p {switched}')}"
    )


def test_the_switch_leaves_every_other_camera_with_the_seat(
    machine: BaseMachine, switched: str, values: dict[str, Any]
) -> None:
    """Only the Brio is taken back: another camera stays the seat's."""
    del switched
    other = the_stand_in(machine, values["other"])
    assert the_seat_can_open(machine, other), (
        f"the seat lost another camera: {machine.succeed(f'getfacl -p {other}')}"
    )


def test_udev_no_longer_hands_the_brio_to_the_seat(machine: BaseMachine, switched: str) -> None:
    """Udev keeps no uaccess tag on the Brio, which logind would hand the seat again."""
    tags = machine.succeed(f"udevadm info --query=property --property=TAGS --value {switched}")
    assert ":uaccess:" not in tags, f"udev still tags the Brio for the seat: {tags.strip()}"


def test_the_switch_ends_the_brios_old_watch(machine: BaseMachine, switched: str) -> None:
    """The watch started as the seat's camera ends, leaving the loopback's watch to say who."""
    state = machine.execute(f"systemctl is-active {the_old_watch(switched)}")[1].strip()
    assert state != "active", f"the Brio's old watch is still {state}"


def test_a_later_run_leaves_the_relay_running(machine: BaseMachine, switched: str) -> None:
    """Run again over a Brio already held, as at boot, the take-hold leaves its relay be.

    The relay is bound to the Brio's device, so the device never going away means the relay was
    never stopped; read from the device, it holds however the relay fares on a stand-in.
    """
    relay = "systemctl show -p InactiveEnterTimestampMonotonic --value 'dev-mx\\x2dbrio.device'"
    before = machine.succeed(relay).strip()
    machine.succeed("systemctl restart mx-brio-take-hold.service")
    said = machine.succeed("journalctl -b -u mx-brio-take-hold -o cat --no-pager")
    assert "already held" in said, f"the take-hold didn't find the Brio held: {said}"
    assert machine.succeed(relay).strip() == before, (
        "the take-hold took the Brio away from its relay"
    )
    assert not the_seat_can_open(machine, switched), "the seat got the Brio back"


def test_with_no_brio_the_take_hold_does_nothing(machine: BaseMachine, switched: str) -> None:
    """With the Brio gone, as at a boot without it, the take-hold succeeds and says so."""
    machine.succeed("systemctl stop mx-brio.service")
    machine.succeed(f"v4l2loopback-ctl delete {switched}")
    machine.succeed("systemctl restart mx-brio-take-hold.service")
    said = machine.succeed("journalctl -b -u mx-brio-take-hold -o cat --no-pager")
    assert "No MX Brio is plugged in" in said, f"the take-hold didn't say the Brio was gone: {said}"

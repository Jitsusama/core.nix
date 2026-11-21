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
# How long logind may take to hand the seat its cameras once someone is there, and systemd to
# settle what a replay set going: moments on a quiet host, the bound for a busy one.
SEATING = timedelta(seconds=30)
# How long journald may take to file what a run said after it's ended.
FILING = timedelta(seconds=10)


def the_stand_in(machine: BaseMachine, card: str) -> str:
    """Give the stand-in Brio's node, once udev is done with it."""
    # grep says which name files hold the card's name, and fails only when none does.
    status, names = machine.execute(f"grep -lx '{card}' /sys/class/video4linux/*/name")
    assert status == 0, f"no loopback is named {card}"
    node = names.split()[0].split("/")[-2]
    machine.succeed(f"udevadm wait --timeout {SETTLING.seconds} /dev/{node}")
    return f"/dev/{node}"


def the_old_watch(node: str) -> str:
    """Name the seat's camera watch on the node, which a relayed camera doesn't keep."""
    return f"usb-camera-holders@{node.removeprefix('/dev/')}.service"


# What the take-hold's last run said, read from that run alone.
RUN = "$(systemctl show -p InvocationID --value mx-brio-take-hold.service)"
SAID = f"journalctl _SYSTEMD_INVOCATION_ID={RUN} -o cat --no-pager"
# The line a run that replayed the Brio's arrival ends with.
REPLAYED = "Replayed the MX Brio's arrival, so its relay has it."


def the_run_said(machine: BaseMachine, words: str) -> None:
    """Wait for the take-hold's last run to have said `words`, read from that run alone."""
    try:
        machine.wait_until_succeeds(f"{SAID} | grep -qF '{words}'", timeout=FILING)
    except RequestedAssertionFailed:
        text = machine.execute(SAID)[1]
        pytest.fail(f"the take-hold's last run didn't say {words!r}: {text}")


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
        machine.wait_until_succeeds(f"getfacl -p {camera} | grep -q user:someone:", timeout=SEATING)
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
    machine.wait_for_unit("mx-brio.service", timeout=STARTING)


def test_the_switch_takes_the_brio_back_from_the_seat(machine: BaseMachine, switched: str) -> None:
    """Nobody at the seat keeps what logind handed them of the Brio before the switch."""
    assert not the_seat_can_open(machine, switched), (
        f"someone at the seat can still open the Brio: {machine.succeed(f'getfacl -p {switched}')}"
    )


def test_the_seat_loses_the_brio_before_its_arrival_is_replayed(
    machine: BaseMachine, switched: str
) -> None:
    """The seat's access goes before the replay tells the seat's programs the Brio has come."""
    the_run_said(machine, REPLAYED)
    lines = machine.succeed(SAID).splitlines()
    took = f"Took back the seat's access to {switched}."
    assert took in lines, f"the run never took the seat's access back first: {lines}"
    assert lines.index(took) < lines.index(REPLAYED), (
        f"the seat's access was taken back only after the replay: {lines}"
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
    """Udev keeps no uaccess tag on the Brio, which logind would hand the seat again.

    logind and the rule that hands a node to the seat read the current tags. TAGS keeps every
    tag an event added, the one 70-uaccess adds before mx-brio's rule takes it off among them.
    """
    tags = machine.succeed(
        f"udevadm info --query=property --property=CURRENT_TAGS --value {switched}"
    )
    assert ":uaccess:" not in tags, f"udev still tags the Brio for the seat: {tags.strip()}"


def test_the_seat_s_next_session_isnt_handed_the_brio(
    machine: BaseMachine, switched: str, values: dict[str, Any]
) -> None:
    """A new session at the seat is handed its other cameras by logind, but not the Brio."""
    sessions = machine.succeed("loginctl show-user someone -p Sessions --value").split()
    machine.succeed("loginctl terminate-user someone")
    old = " ".join(f"-e {session}" for session in sessions)
    # Autologin brings someone back in a session of its own, which logind hands the cameras.
    machine.wait_until_succeeds(
        "loginctl show-user someone -p State --value | grep -qx active && "
        f"! loginctl show-user someone -p Sessions --value | grep -qw {old}",
        timeout=SEATING,
    )
    other = the_stand_in(machine, values["other"])
    machine.wait_until_succeeds(f"getfacl -p {other} | grep -q user:someone:", timeout=SEATING)
    assert not the_seat_can_open(machine, switched), (
        f"the new session was handed the Brio: {machine.succeed(f'getfacl -p {switched}')}"
    )


def test_the_switch_ends_the_brios_old_watch(machine: BaseMachine, switched: str) -> None:
    """The watch started as the seat's camera ends, leaving the loopback's watch to say who."""
    old = the_old_watch(switched)
    # is-active fails for every state but active, and the driver's shell stops at a failure.
    try:
        machine.wait_until_succeeds(
            f"state=$(systemctl is-active {old} || true); "
            '[ "$state" = inactive ] || [ "$state" = failed ]',
            timeout=SEATING,
        )
    except RequestedAssertionFailed:
        state = machine.execute(f"systemctl is-active {old}")[1].strip()
        pytest.fail(f"the Brio's old watch is still {state}")
    relayed = old.replace("usb-camera-holders@", "usb-camera-holders-relayed@")
    machine.wait_for_unit(relayed, timeout=STARTING)


def test_a_later_run_leaves_the_relay_running(machine: BaseMachine, switched: str) -> None:
    """Run again over a Brio already held, as at boot, the take-hold leaves its relay be.

    The relay is bound to the Brio's device, so the device never going away means the relay was
    never stopped; read from the device, it holds however the relay fares on a stand-in.
    """
    relay = "systemctl show -p InactiveEnterTimestampMonotonic --value 'dev-mx\\x2dbrio.device'"
    before = machine.succeed(relay).strip()
    machine.succeed("systemctl restart mx-brio-take-hold.service")
    # A removal would have queued the relay's stop, so it's read once systemd has no job left.
    machine.succeed("udevadm settle")
    machine.wait_until_succeeds('[ -z "$(systemctl list-jobs --no-legend)" ]', timeout=SEATING)
    the_run_said(machine, "already held")
    assert machine.succeed(relay).strip() == before, (
        "the take-hold took the Brio away from its relay"
    )
    assert not the_seat_can_open(machine, switched), "the seat got the Brio back"


def test_with_no_brio_the_take_hold_does_nothing(machine: BaseMachine, switched: str) -> None:
    """With the Brio gone, as at a boot without it, the take-hold succeeds and says so."""
    machine.succeed("systemctl stop mx-brio.service")
    machine.succeed(f"v4l2loopback-ctl delete {switched}")
    machine.succeed("systemctl restart mx-brio-take-hold.service")
    the_run_said(machine, "No MX Brio is plugged in")

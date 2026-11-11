"""A virtual machine left idle, on the charger and on battery.

idle-sleep.nix, beside this, runs this through pytest inside the driver (tests/lib/vm_pytest.py),
and its opening comment says what the test proves. Each test asks for the machine booted with its
desktop's session in use, on the charger and awake, and leaves it so. The stand-in charger, the
session's account and the stand-in sleep are all named in the machine idle-sleep.nix builds, so the
tests need no value from Nix.
"""

from __future__ import annotations

import datetime as dt
import re
from typing import TYPE_CHECKING

import pytest

if TYPE_CHECKING:
    from test_driver.machine import BaseMachine

LOGIND = "org.freedesktop.login1"
MANAGER = f"{LOGIND} /org/freedesktop/login1 {LOGIND}.Manager"
# The sleep unit's state is read rather than asked with is-active, which calls a oneshot
# still running "activating" and so can't tell a sleep under way from one that's over.
SETTLED = (
    f"busctl get-property {MANAGER} PreparingForSleep | grep -qx 'b false'"
    " && ! systemctl list-jobs --no-legend | grep -q suspend"
    " && systemctl show -P ActiveState systemd-suspend.service | grep -qx inactive"
)
# The stand-in sleep leaves this file, under systemd's word for it.
ASLEEP = "test -e /run/suspended"
# A program holding logind's lock against sleep, as mycelium does while it holds work. logind
# counts anyone's lock alike, so root's stands in for the account's. systemd-run finds the program
# it's given itself, but the unit runs with systemd's own PATH, which on NixOS holds no sleep, so
# the program the lock is held around is named in full.
HOLD = (
    "systemd-run --unit=hold systemd-inhibit --what=sleep --who=hold --why=Working"
    ' --mode=block "$(command -v sleep)" infinity'
)
HELD = "systemd-inhibit --list --no-legend --no-pager | grep -q Working"
# How many times logind has locked its sessions since boot, as it says each time it does.
LOCKS = "journalctl -b -u systemd-logind -o cat | grep -c '^Locking sessions' || true"
# How long logind may take to sleep once nothing keeps it awake: its five seconds of idling,
# as long again for its timer to come round, and the charger's word reaching it.
FALLING_ASLEEP = dt.timedelta(minutes=1)
# How long logind is given to sleep when it mustn't: its five seconds three times over.
LOOKING = dt.timedelta(seconds=15)
# The unit udev restarts on every power supply event, which reads the charger.
FOLLOWER = "idle-sleep-follows-the-charger.service"
# How long the charger coming or going may take to reach logind's settings.
FOLLOWING = dt.timedelta(seconds=30)


class Laptop:
    """The machine's stand-in charger and its desktop's session, and what logind does."""

    def __init__(self, machine: BaseMachine, session: str) -> None:
        """Act on the stand-in charger, and on the session at that object path."""
        self.machine = machine
        self.session = session

    def charger(self, *, plugged: bool) -> None:
        """Plug the charger in or unplug it, and wait for logind to have heard."""
        state = "on" if plugged else "off"
        action = "lock" if plugged else "suspend"
        self.machine.succeed(f"echo {state} > /sys/module/test_power/parameters/ac_online")
        self.machine.wait_until_succeeds(
            f"busctl get-property {MANAGER} IdleAction | grep -qx 's \"{action}\"'",
            timeout=FOLLOWING,
        )

    def idle(self, *, idle: bool) -> None:
        """Say the session is idle, as swayidle does once the screens go off, or in use."""
        hint = "true" if idle else "false"
        self.machine.succeed(
            f"busctl call {LOGIND} {self.session} {LOGIND}.Session SetIdleHint b {hint}"
        )

    def machine_idle(self) -> bool:
        """Whether logind counts the machine idle, every session idle and nothing holding it."""
        return self.machine.succeed(f"busctl get-property {MANAGER} IdleHint").strip() == "b true"

    def settle(self) -> None:
        """Put the session back in use and the charger in, once any sleep under way is over."""
        self.idle(idle=False)
        self.charger(plugged=True)
        self.machine.wait_until_succeeds(SETTLED, timeout=FALLING_ASLEEP)
        self.machine.succeed("rm -f /run/suspended")


@pytest.fixture(scope="module")
def laptop(machine: BaseMachine) -> Laptop:
    """Boot the machine on the charger and open its desktop's session, of Wayland's type."""
    machine.wait_for_unit("multi-user.target")
    machine.succeed("grep -qx 1 /sys/class/power_supply/test_ac/online")
    machine.succeed(
        "systemd-run --unit=desktop -p PAMName=login -p User=away"
        " -E XDG_SESSION_TYPE=wayland sleep infinity"
    )
    found = machine.wait_until_succeeds(
        f"busctl call {MANAGER} GetSessionByPID u $(systemctl show -P MainPID desktop.service)",
        timeout=FOLLOWING,
    )
    match = re.fullmatch(r'o "(\S+)"', found.strip())
    assert match, found
    session = match.group(1)
    kind = machine.succeed(f"busctl get-property {LOGIND} {session} {LOGIND}.Session Type")
    assert kind.strip() == 's "wayland"', kind
    # Idle with no session at all from its boot, it never slept on the charger.
    machine.fail(ASLEEP)
    return Laptop(machine, session)


def test_an_idle_machine_on_the_charger_stays_awake(laptop: Laptop) -> None:
    """An idle machine on the charger stays awake."""
    laptop.idle(idle=True)
    assert laptop.machine_idle()
    laptop.machine.sleep(LOOKING)
    laptop.machine.fail(ASLEEP)
    laptop.settle()


def test_unplugging_the_charger_from_an_idle_machine_sleeps_it(laptop: Laptop) -> None:
    """Unplugging the charger from an idle machine puts it to sleep."""
    laptop.idle(idle=True)
    laptop.charger(plugged=False)
    laptop.machine.wait_until_succeeds(ASLEEP, timeout=FALLING_ASLEEP)
    laptop.settle()


def test_a_burst_of_power_events_leaves_it_following_the_charger(laptop: Laptop) -> None:
    """An unplug ending a burst of power supply events, as a supply settling sends, still counts."""
    laptop.machine.succeed(
        "for _ in $(seq 20); do"
        " udevadm trigger --action=change --subsystem-match=power_supply; done"
    )
    laptop.charger(plugged=False)
    laptop.machine.wait_until_succeeds(
        f"systemctl show -P ActiveState {FOLLOWER} | grep -qx inactive", timeout=FOLLOWING
    )
    laptop.settle()


def test_a_session_in_use_keeps_it_awake_on_battery(laptop: Laptop) -> None:
    """A session still in use, as a call in Chrome keeps it, keeps the machine awake on battery."""
    laptop.charger(plugged=False)
    assert not laptop.machine_idle()
    laptop.machine.sleep(LOOKING)
    laptop.machine.fail(ASLEEP)
    laptop.settle()


def test_a_hold_against_sleep_still_locks_and_keeps_it_awake_until_let_go(laptop: Laptop) -> None:
    """Held, an idle machine goes idle and locks, stays awake on battery, and sleeps once let go."""
    locked = int(laptop.machine.succeed(LOCKS))
    laptop.machine.succeed(HOLD)
    laptop.machine.wait_until_succeeds(HELD, timeout=FOLLOWING)
    laptop.idle(idle=True)
    # swayidle drops its lock and its screens going off while logind's blocks name idling.
    blocked = laptop.machine.succeed(f"busctl get-property {MANAGER} BlockInhibited")
    assert "idle" not in blocked, f"the hold stops idling, so nothing would lock: {blocked}"
    assert laptop.machine_idle(), "the hold kept the machine from counting as idle"
    laptop.machine.wait_until_succeeds(f"test $({LOCKS}) -gt {locked}", timeout=FALLING_ASLEEP)
    laptop.charger(plugged=False)
    laptop.machine.sleep(LOOKING)
    laptop.machine.fail(ASLEEP)
    laptop.machine.succeed("systemctl stop hold.service")
    laptop.machine.wait_until_succeeds(ASLEEP, timeout=FALLING_ASLEEP)
    laptop.settle()

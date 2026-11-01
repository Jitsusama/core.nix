"""The greeter signing an account in and out, in a virtual machine that doesn't sign in by itself.

tests/greeter.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), and its
opening comment says what the test proves. Each test asks for the state it starts from as a
fixture: the account's setup held, then let go, the greeter asking, the account signed in,
and the greeter asking again after the session ends. A fixture's work is done once a run,
however many tests ask for it, so any test runs alone with `-k`. The account's name and
password are greeter.nix's, given as the `values` fixture.
"""

from __future__ import annotations

import shlex
from dataclasses import dataclass
from datetime import timedelta
from typing import TYPE_CHECKING, Any, cast

import pytest

if TYPE_CHECKING:
    from test_driver.machine import BaseMachine, QemuMachine

# How long the boot may take to reach the greeter, and a step within it. The
# boot takes 90 s on a quiet builder and 165 s on a busy one, and the wait ends
# as soon as it's there, so the bound is well past either.
BOOTING = timedelta(seconds=600)
STEP = timedelta(seconds=60)
# How many mounts the burst makes, past systemd's limit on changes to the mount table.
BURST = 12


@dataclass(frozen=True)
class Account:
    """The account the greeter signs in, and the password it takes."""

    name: str
    secret: str


def mark(machine: QemuMachine) -> str:
    """Give a point in the greeter's journal, so only what's said after it counts."""
    said = machine.succeed("journalctl --identifier=greeter --lines=1 --show-cursor --output=cat")
    return said.rsplit("-- cursor: ", 1)[1].strip()


def heard(machine: QemuMachine, what: str, since: str, timeout: timedelta = STEP) -> None:
    """Wait until the greeter says exactly this after the point.

    It follows the journal as it's written, and prints what the greeter did say if it never
    says this.
    """
    point = f"--after-cursor={shlex.quote(since)}" if since else "--boot"
    journal = f"journalctl --identifier=greeter --output=cat --no-pager {point}"
    wanted = shlex.quote(f"report[{what}]")
    follow = shlex.quote(f"{journal} --follow | grep -m1 -aqF {wanted}")
    seconds = int(timeout.total_seconds())
    status, _ = machine.execute(
        f"timeout {seconds} sh -c {follow}",
        timeout=timeout + timedelta(seconds=30),
    )
    if status != 0:
        print(machine.succeed(journal))
        machine.screenshot("unheard")
    assert status == 0, f"the greeter never said {what!r}"


def active_state(machine: QemuMachine, unit: str) -> str:
    """Give the unit's state as systemd says it."""
    return machine.succeed(f"systemctl show --property=ActiveState --value {unit}").strip()


def monotonic(machine: QemuMachine, unit: str, when: str) -> int:
    """Give when the unit last made this change, in microseconds since boot."""
    return int(machine.succeed(f"systemctl show --property={when}Monotonic --value {unit}"))


@pytest.fixture(scope="module")
def account(values: dict[str, Any]) -> Account:
    """Give the account and its password, as greeter.nix declares them."""
    return Account(values["account"], values["secret"])


@pytest.fixture(scope="module")
def held(machine: BaseMachine) -> QemuMachine:
    """Give the machine with the greeter running and the account's setup held back."""
    # The test's one machine is QEMU's, which takes keys and screenshots.
    machine = cast("QemuMachine", machine)
    machine.wait_for_file("/run/home-hold/release", timeout=BOOTING)
    machine.wait_until_succeeds("systemctl is-active --quiet greetd.service", timeout=BOOTING)
    return machine


@pytest.fixture(scope="module")
def manager(held: QemuMachine, account: Account) -> str:
    """Give the account's service manager, once its setup is let go and it has started."""
    unit = f"user@$(id -u {account.name}).service"
    held.succeed("echo go > /run/home-hold/release")
    held.wait_until_succeeds(f"systemctl is-active --quiet {unit}", timeout=STEP)
    return unit


@pytest.fixture(scope="module")
def asking(held: QemuMachine, manager: str) -> QemuMachine:  # noqa: ARG001
    """Give the machine with the greeter asking who's there, after the account's setup."""
    heard(held, "keys greeter", "", timeout=BOOTING)
    return held


@pytest.fixture(scope="module")
def signed_in(asking: QemuMachine, account: Account) -> str:
    """Give a point in the greeter's journal once it has signed the account in."""
    since = mark(asking)
    asking.send_chars(f"{account.secret}\n")
    heard(asking, f"signs in {account.name}", since)
    # The file is the stand-in session's, on the machine, not a temporary file of the test's.
    asking.wait_for_file("/tmp/signed-in", timeout=STEP)  # noqa: S108
    return mark(asking)


@pytest.fixture(scope="module")
def asking_again(asking: QemuMachine, signed_in: str) -> QemuMachine:
    """Give the machine with the greeter asking again, once the session has ended."""
    heard(asking, "keys greeter", signed_in)
    return asking


def test_shows_while_the_account_is_set_up(held: QemuMachine, account: Account) -> None:
    """The greeter shows while the account is set up."""
    setup = active_state(held, f"home-manager-{account.name}.service")
    assert setup == "activating"


def test_the_service_manager_waits_for_its_setup(
    held: QemuMachine, account: Account, request: pytest.FixtureRequest
) -> None:
    """The account's service manager waits for its setup."""
    unit = f"user@$(id -u {account.name}).service"
    assert active_state(held, unit) == "inactive"
    # Only now is the setup let go, by the fixture every later state starts from.
    request.getfixturevalue("manager")
    setup = f"home-manager-{account.name}.service"
    finished = monotonic(held, setup, "ActiveEnterTimestamp")
    started = monotonic(held, unit, "InactiveExitTimestamp")
    assert started >= finished, "the manager started before the setup finished"


def test_asks_whos_there(asking: QemuMachine) -> None:
    """The greeter asks who's there."""
    asking.screenshot("greeter")


def test_refuses_a_wrong_password(asking: QemuMachine) -> None:
    """The greeter refuses a wrong password."""
    since = mark(asking)
    asking.send_chars("wrong-horse\n")
    heard(asking, "refused greeter", since)
    asking.succeed("test ! -e /tmp/signed-in")
    # Refused once, saying "try again" rather than that it couldn't check.
    _, said = asking.execute(
        f"journalctl --identifier=greeter --output=cat --after-cursor={shlex.quote(since)}"
        " | grep -c 'report.refused greeter'"
    )
    assert said.strip() == "1"
    asking.screenshot("greeter-refused")


def test_signs_in_with_the_right_one(
    asking: QemuMachine,
    account: Account,
    signed_in: str,  # noqa: ARG001
) -> None:
    """The greeter signs in with the right one."""
    assert asking.succeed("cat /tmp/signed-in").strip() == f"{account.name} user"


def test_asks_again_after_a_logout(asking_again: QemuMachine) -> None:
    """The greeter asks again after a logout."""
    asking_again.succeed("! pgrep -u greeter -x tuigreet")


def test_tuigreet_asks_when_the_greeter_falls_over(asking_again: QemuMachine) -> None:
    """Tuigreet asks when the greeter falls over."""
    # The brackets keep the test's own shell from matching.
    asking_again.succeed("pkill -u greeter -f '[q]uickshell[^ ]* -p '")
    asking_again.wait_until_succeeds("pgrep -u greeter -x tuigreet", timeout=STEP)
    asking_again.succeed("! pgrep -u greeter -f '[b]in/niri -c'")


def test_a_burst_of_mounts_doesnt_wait_on_systemds_limit(machine: BaseMachine) -> None:
    """A burst of mounts doesn't wait on systemd's limit.

    systemd starts no mount while it's over its limit on changes to the mount table, which
    held the mounts the greeter waits for at boot. Each systemd-mount waits for its mount, so
    each is a change of its own, and systemd says at debug level when the limit is reached.
    Debug goes only to the journal, since the serial console slows systemd enough to spread
    the burst out, and journald keeps it off the console too, so it keeps up and the last
    mount's line in the journal means all that came before it is there too.
    """
    target = machine.succeed("systemd-analyze log-target").strip()
    machine.succeed("systemd-analyze log-target journal")
    machine.succeed("systemd-analyze log-level debug")
    machine.succeed(
        f"for n in $(seq {BURST}); do"
        " timeout 30 systemd-mount --quiet --type=tmpfs tmpfs /mnt/burst$n || exit 1;"
        " done"
    )
    machine.succeed("systemd-analyze log-level info")
    machine.succeed(f"systemd-analyze log-target {shlex.quote(target)}")
    machine.wait_until_succeeds(
        f"journalctl --boot _PID=1 --output=cat | grep -qxF 'Mounted /mnt/burst{BURST}.'",
        timeout=timedelta(seconds=30),
    )
    machine.fail(
        "journalctl --boot _PID=1 --output=cat"
        " | grep -F 'mount-monitor-dispatch) entered rate limit state'"
    )

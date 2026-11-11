"""Root's Nix is given only the lender's lines that name a builder nix-offload's lines trust.

offload-lines.nix, beside this, runs this through pytest inside the driver (tests/lib/vm_pytest.py),
and its opening comment says what stands in for the cloud. The lender here is the test, writing
as the lender's account what the lender would: its lines, the instance it lent and a token.
"""

from __future__ import annotations

import base64
import shlex
from datetime import timedelta
from typing import TYPE_CHECKING, Any

import pytest

if TYPE_CHECKING:
    from test_driver.driver import Driver
    from test_driver.machine import BaseMachine

LENDER = "/run/nix-offload"
ROOTS = "/run/nix-offload-lines"
# How long the service may take to copy what the lender wrote: it reads every two seconds,
# and asks the cloud once for an instance it hasn't seen.
COPYING = timedelta(minutes=1)


def carried(key: str) -> str:
    """Give a host key as a line carries it, base64 of its type and key."""
    return base64.b64encode(f"ssh-ed25519 {key}".encode()).decode()


def as_lender(machine: BaseMachine, command: str) -> None:
    """Run `command` as the lender's account, as the lender would."""
    machine.succeed(f"runuser -u someone -- sh -c {shlex.quote(command)}")


def write(machine: BaseMachine, path: str, text: str) -> None:
    """Write `text` to `path` as the lender's account."""
    as_lender(machine, f"printf %s {shlex.quote(text)} > {path}")


@pytest.fixture(scope="module")
def lent(values: dict[str, Any]) -> str:
    """Give the line naming the lent builder with the host key the cloud gives for it."""
    return f"{values['lentUri']} x86_64-linux /etc/nix/offload_ed25519 8 1 big-parallel - " + (
        carried(values["lentKey"])
    )


@pytest.fixture(scope="module")
def standing(values: dict[str, Any]) -> str:
    """Give the line naming the standing builder as the system's settings trust it."""
    key: str = values["standingKey"]
    return f"{values['standingUri']} x86_64-linux /etc/nix/builder_ed25519 4 1 - - {key}"


@pytest.fixture(scope="module")
def lending(
    driver: Driver, machines: dict[str, BaseMachine], lent: str, standing: str
) -> BaseMachine:
    """Give the machine once the lender has lent a builder and named one it didn't."""
    driver.start_all()
    machine = machines["machine"]
    machine.wait_for_unit("nix-offload-lines.service")
    token = f"{LENDER}/cloud-token"
    write(machine, token, "ya29.a-token")
    # As the lender leaves it: its own, and read by the service's account through an ACL, since
    # the lender's account is in none of the service's groups.
    as_lender(machine, f"chmod 0600 {token} && setfacl -m u:nix-offload-lines:r {token}")
    write(machine, f"{LENDER}/instance", "a-builder-1 a-zone\n")
    forged = lent.rsplit(" ", 1)[0] + " " + carried("AAAAforged")
    write(machine, f"{LENDER}/machines", f"{lent}\n{forged}\n")
    write(machine, f"{LENDER}/machines-with-standing", f"{standing}\n")
    return machine


def test_root_is_given_only_the_lines_it_trusts(
    lending: BaseMachine, lent: str, standing: str
) -> None:
    """The lent builder's line and the standing one's reach root; a forged host key doesn't."""
    lending.wait_until_succeeds(f"grep -qxF {shlex.quote(lent)} {ROOTS}/machines", timeout=COPYING)
    assert lending.succeed(f"cat {ROOTS}/machines") == f"{lent}\n"
    lending.wait_until_succeeds(
        f"grep -qxF {shlex.quote(standing)} {ROOTS}/machines-with-standing", timeout=COPYING
    )
    assert lending.succeed(f"cat {ROOTS}/machines-with-standing") == f"{standing}\n"


def test_the_lenders_account_is_in_none_of_the_services_groups(lending: BaseMachine) -> None:
    """The lender's account hands the token over by an ACL, so needs no group of the service's."""
    groups = lending.succeed("id -nG someone").split()
    assert "nix-offload-lines" not in groups, groups


def test_the_lenders_account_cannot_write_roots_lines(lending: BaseMachine) -> None:
    """Root's lines are the service's account's, which the lender's can't change or add to."""
    owners = lending.succeed(f"stat -c %U {ROOTS} {ROOTS}/machines").split()
    assert owners == ["nix-offload-lines", "nix-offload-lines"], owners
    lending.fail(f"runuser -u someone -- sh -c ': >> {ROOTS}/machines'")
    lending.fail(f"runuser -u someone -- touch {ROOTS}/machines.draft")


def test_nothing_lent_takes_the_lent_line_away(lending: BaseMachine, standing: str) -> None:
    """With no instance named, the lent builder's line no longer reaches root, the standing does."""
    write(lending, f"{LENDER}/instance", "")
    lending.wait_until_succeeds(f"test ! -s {ROOTS}/machines", timeout=COPYING)
    assert lending.succeed(f"cat {ROOTS}/machines-with-standing") == f"{standing}\n"

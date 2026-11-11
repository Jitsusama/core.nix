"""The machine's administrators switch it to a system it has run, and nobody else does.

switch-system.nix, beside this, runs this through pytest inside the driver (tests/lib/vm_pytest.py).
The machine's system has a specialisation that adds /etc/switched, which root puts in the
system profile first, as update-to-main would, so switching to it and back is a switch to
another system the machine has run and to the first again. A second, which adds /etc/unrun,
is never put there. An account in wheel switches between the first two unasked; a switch to
the third asks for authentication, which an agent outside any session can't give, so it's
refused. An account outside wheel, and anything that isn't a NixOS system in the store, is
refused too, and the machine is left as it was. A test that switches switches back as it ends,
so any runs alone. someone has a YubiKey named to answer polkit, never plugged in, so the
authentication a switch asks for still takes the password.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import TYPE_CHECKING

import pytest

if TYPE_CHECKING:
    from collections.abc import Callable, Iterator

    from test_driver.machine import BaseMachine

PROFILE = "/nix/var/nix/profiles/system"


@dataclass(frozen=True)
class Systems:
    """The machine, the two systems it has run, and the one it never has."""

    machine: BaseMachine
    first: str
    switched: str
    unrun: str

    def switch(self, account: str, target: str) -> tuple[int, str]:
        """Switch the machine as an account, outside any session as an agent would.

        It gives switch-system's status and all it said.
        """
        return self.machine.execute(f"runuser -u {account} -- switch-system {target} 2>&1")

    def running(self) -> str:
        """Give the system the machine is running."""
        return self.machine.succeed("readlink -f /run/current-system").strip()

    def booting(self) -> str:
        """Give the system the machine boots next."""
        return self.machine.succeed(f"readlink -f {PROFILE}").strip()


@pytest.fixture(scope="module")
def systems(machine: BaseMachine) -> Systems:
    """Give the machine up on its first system, with the second in its system profile.

    Root puts the second there and the first back after it, as update-to-main switching on and
    back would, so both are systems the machine has run and it still boots the first.
    """
    machine.wait_for_unit("multi-user.target")
    first = machine.succeed("readlink -f /run/current-system").strip()
    switched = machine.succeed(f"readlink -f {first}/specialisation/switched").strip()
    unrun = machine.succeed(f"readlink -f {first}/specialisation/unrun").strip()
    for system in (first, switched, first):
        machine.succeed(f"nix-env --profile {PROFILE} --set {system}")
    return Systems(machine, first, switched, unrun)


@pytest.fixture
def switched(systems: Systems) -> Iterator[Systems]:
    """Give the machine switched to the other system by an administrator; it switches back."""
    status, said = systems.switch("someone", systems.switched)
    assert status == 0, said
    yield systems
    status, said = systems.switch("someone", systems.first)
    assert status == 0, said


def test_an_administrator_switches_to_a_system_it_has_run_with_no_password(
    switched: Systems,
) -> None:
    """An administrator switches to another system it has run, unasked, and boots it next."""
    switched.machine.succeed("test -e /etc/switched")
    assert switched.running() == switched.switched
    assert switched.booting() == switched.switched


def test_an_administrator_switches_back(switched: Systems) -> None:
    """An administrator switches back, and boots the first system next."""
    status, said = switched.switch("someone", switched.first)
    assert status == 0, said
    switched.machine.fail("test -e /etc/switched")
    assert switched.running() == switched.first
    assert switched.booting() == switched.first


def test_a_system_the_machine_never_ran_asks_an_administrator(systems: Systems) -> None:
    """An administrator's switch to a system it never ran asks, and nothing changes unasked."""
    status, said = systems.switch("someone", systems.unrun)
    assert status != 0, said
    assert "requires interactive authentication" in said
    systems.machine.fail("test -e /etc/unrun")
    assert systems.running() == systems.first
    assert systems.booting() == systems.first


@pytest.mark.parametrize(
    ("typed", "taken"),
    [
        pytest.param("typed", True, id="the password"),
        pytest.param("guessed", False, id="another"),
    ],
)
def test_polkit_takes_the_password_with_no_key_there(
    systems: Systems, typed: str, *, taken: bool
) -> None:
    """With a key named and none there, polkit's sign-in takes the password, and only it."""
    status, said = systems.machine.execute(
        f"printf '%s\\n' {typed} | pamtester polkit-1 someone authenticate 2>&1"
    )
    assert (status == 0) == taken, said


def test_an_account_outside_wheel_is_refused(systems: Systems) -> None:
    """An account outside wheel is refused by polkit, and the machine stays as it was."""
    status, said = systems.switch("other", systems.switched)
    assert status != 0, said
    assert "requires interactive authentication" in said
    assert systems.running() == systems.first


@pytest.mark.parametrize(
    ("target", "refusal"),
    [
        pytest.param(
            lambda bash: bash.removesuffix("/bin/bash"), "not a NixOS system", id="a package"
        ),
        pytest.param(lambda bash: bash, "not a path in the Nix store", id="a program in one"),
        pytest.param(lambda _: "/etc", "not a path in the Nix store", id="a folder outside"),
    ],
)
def test_what_isnt_a_nixos_system_in_the_store_is_refused(
    systems: Systems, target: Callable[[str], str], refusal: str
) -> None:
    """What isn't a NixOS system in the store is refused, and nothing changes.

    polkit asks for any of these, since none is a system the machine has run, so they're
    switched as root, whom polkit never asks, to reach the switch's own refusal. Each target is
    found from bash's path in the store, which the machine's system holds.
    """
    bash = systems.machine.succeed("readlink -f /run/current-system/sw/bin/bash").strip()
    status, said = systems.switch("root", target(bash))
    assert status != 0, said
    assert refusal in said
    assert systems.running() == systems.first
    assert systems.booting() == systems.first

"""The machine's administrators switch it with no password, and nobody else does.

tests/switch-system.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py).
The machine's system has a specialisation that adds /etc/switched, so switching to it and
back is a switch to another system and to the first again. An account in wheel switches; an
account outside it, and anything that isn't a NixOS system in the store, is refused, and the
machine is left as it was. A test that switches switches back as it ends, so any runs alone.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import TYPE_CHECKING

import pytest

if TYPE_CHECKING:
    from collections.abc import Callable, Iterator

    from test_driver.machine import BaseMachine


@dataclass(frozen=True)
class Systems:
    """The machine, and the two systems a test switches it between."""

    machine: BaseMachine
    first: str
    switched: str

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
        return self.machine.succeed("readlink -f /nix/var/nix/profiles/system").strip()


@pytest.fixture(scope="module")
def systems(machine: BaseMachine) -> Systems:
    """Give the machine up on its first system, and the specialisation beside it."""
    machine.wait_for_unit("multi-user.target")
    first = machine.succeed("readlink -f /run/current-system").strip()
    switched = machine.succeed(f"readlink -f {first}/specialisation/switched").strip()
    return Systems(machine, first, switched)


@pytest.fixture
def switched(systems: Systems) -> Iterator[Systems]:
    """Give the machine switched to the other system by an administrator; it switches back."""
    status, said = systems.switch("someone", systems.switched)
    assert status == 0, said
    yield systems
    status, said = systems.switch("someone", systems.first)
    assert status == 0, said


def test_an_administrator_switches_to_another_system_with_no_password(switched: Systems) -> None:
    """An administrator switches to another system, with no password, and boots it next."""
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

    Each target is found from bash's path in the store, which the machine's system holds.
    """
    bash = systems.machine.succeed("readlink -f /run/current-system/sw/bin/bash").strip()
    status, said = systems.switch("someone", target(bash))
    assert status != 0, said
    assert refusal in said
    assert systems.running() == systems.first
    assert systems.booting() == systems.first

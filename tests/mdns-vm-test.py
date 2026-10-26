"""A machine on a trusted network finds another by its .local name, and nobody finds a stranger.

tests/mdns.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), and its
opening comment says which machine trusts the network. Each name is asked of mDNS alone, so
the test network's /etc/hosts entries, which name every machine, answer nothing.
"""

from __future__ import annotations

from datetime import timedelta
from typing import TYPE_CHECKING

import pytest

if TYPE_CHECKING:
    from test_driver.driver import Driver
    from test_driver.machine import BaseMachine

# How long resolved may take to start answering for its name once the link is up.
ANNOUNCING = timedelta(minutes=1)


def asked(name: str) -> str:
    """Give the command asking mDNS alone for a name's IPv4 address."""
    return f"resolvectl query -4 --protocol=mdns --synthesize=no --cache=no {name}.local"


def address(machine: BaseMachine) -> str:
    """Give the machine's address on the test network."""
    return machine.succeed("ip -4 -o addr show dev eth1 | awk '{print $4}' | cut -d/ -f1").strip()


@pytest.fixture(scope="module")
def started(driver: Driver, machines: dict[str, BaseMachine]) -> dict[str, BaseMachine]:
    """Give every machine once resolved runs and its link to the others is configured."""
    driver.start_all()
    for machine in machines.values():
        machine.wait_for_unit("systemd-resolved.service")
        machine.wait_until_succeeds(
            "networkctl status eth1 | grep -q 'State: routable (configured)'"
        )
    return machines


def test_the_firewall_is_on(started: dict[str, BaseMachine]) -> None:
    """The firewall filters what reaches each machine, so it's what the port has to get past."""
    for machine in started.values():
        machine.succeed("nft list chain inet nixos-fw input-allow")


def test_a_trusting_machine_finds_another_by_name(started: dict[str, BaseMachine]) -> None:
    """The neighbour's .local name gives its address, through both firewalls."""
    home, neighbour = started["home"], started["neighbour"]
    expected = address(neighbour)
    home.wait_until_succeeds(f"{asked('neighbour')} | grep -qw {expected}", timeout=ANNOUNCING)


def test_nobody_finds_the_stranger(started: dict[str, BaseMachine]) -> None:
    """The stranger's network has mDNS off, so it answers nobody, though the port is open."""
    home = started["home"]
    home.wait_until_succeeds(asked("neighbour"), timeout=ANNOUNCING)
    home.fail(asked("stranger"))

"""A machine's clock sets from its own time server past the network's, which never answers.

tests/clock.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), and its
opening comment says why timesyncd waits to be started. The silent server the network names
is a Nix value, given as the `values` fixture, so networkd's and timesyncd's answers are
checked against the same address the machine was given.
"""

from __future__ import annotations

from datetime import timedelta
from typing import TYPE_CHECKING, Any

import pytest

if TYPE_CHECKING:
    from test_driver.driver import Driver
    from test_driver.machine import BaseMachine

# How long networkd may take to hear of the network's server once the link is up.
HEARING = timedelta(minutes=1)
# How long timesyncd may take to give up on the silent server and set the clock from its own.
SYNCING = timedelta(minutes=2)


@pytest.fixture(scope="module")
def silent(values: dict[str, Any]) -> str:
    """Give the address of the time server the network names, which never answers."""
    address: str = values["silent"]
    return address


@pytest.fixture(scope="module")
def syncing(driver: Driver, machines: dict[str, BaseMachine], silent: str) -> BaseMachine:
    """Give the machine with timesyncd started once networkd knows the network's server.

    Started sooner, timesyncd would find no server named and fall back to the machine's own,
    which proves nothing.
    """
    driver.start_all()
    machines["server"].wait_for_unit("chronyd.service")
    machine = machines["machine"]
    machine.wait_until_succeeds(
        f"networkctl status eth1 | grep -q 'NTP: {silent}'", timeout=HEARING
    )
    machine.succeed("systemctl start systemd-timesyncd.service")
    return machine


def test_timesyncd_is_told_the_networks_server(syncing: BaseMachine, silent: str) -> None:
    """Timesyncd is told the network's server, so it's the one it has to get past."""
    servers = syncing.succeed("timedatectl show-timesync -p LinkNTPServers --value").split()
    assert servers == [silent]


def test_the_clock_sets_from_the_machines_own_server(syncing: BaseMachine) -> None:
    """The clock sets from the machine's own server, past the one that never answers."""
    syncing.wait_until_succeeds(
        "timedatectl show -p NTPSynchronized --value | grep -qx yes", timeout=SYNCING
    )
    assert syncing.succeed("timedatectl show-timesync -p ServerName --value").strip() == "server"

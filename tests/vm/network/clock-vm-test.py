"""A machine's clock sets from its own time server past the network's, which never answers.

clock.nix, beside this, runs this through pytest inside the driver (tests/lib/vm_pytest.py), and its
opening comment says how chrony hears of the network's server. The silent server the network
names is a Nix value, given as the `values` fixture, so networkd's and chrony's answers are
checked against the same address the machine was given.
"""

from __future__ import annotations

from datetime import timedelta
from typing import TYPE_CHECKING, Any

import pytest

if TYPE_CHECKING:
    from test_driver.driver import Driver
    from test_driver.machine import BaseMachine

# How long networkd may take to hear of the network's server once the link is up, and
# chrony to be told it.
HEARING = timedelta(minutes=1)
# How long chrony may take to set the clock from the machine's own server.
SYNCING = timedelta(minutes=2)
# A link's state written this many times, a moment apart, as networkd wrote them on a laptop
# as it started: six in under three seconds, past systemd's default of five starts in ten.
BURST = 10


@pytest.fixture(scope="module")
def silent(values: dict[str, Any]) -> str:
    """Give the address of the time server the network names, which never answers."""
    address: str = values["silent"]
    return address


@pytest.fixture(scope="module")
def syncing(driver: Driver, machines: dict[str, BaseMachine], silent: str) -> BaseMachine:
    """Give the machine once networkd knows the network's server."""
    driver.start_all()
    machines["server"].wait_for_unit("chronyd.service")
    machine = machines["machine"]
    machine.wait_for_unit("chronyd.service")
    machine.wait_until_succeeds(
        f"networkctl status eth1 | grep -q 'NTP: {silent}'", timeout=HEARING
    )
    return machine


def test_chrony_is_told_the_networks_server(syncing: BaseMachine, silent: str) -> None:
    """Chrony is told the network's server, as it's named, beside the machine's own."""
    syncing.wait_until_succeeds(
        f"chronyc -n -c sources | cut -d, -f3 | grep -qx {silent}", timeout=HEARING
    )


def test_a_burst_of_link_states_leaves_it_listening(syncing: BaseMachine) -> None:
    """Link states written in a burst are each heard, and it's still listening after."""
    state = "/run/systemd/netif/links/burst"
    try:
        for _ in range(BURST):
            # Each write closes the file, which is what the path waits for; a moment
            # apart, each one starts a run of its own.
            syncing.succeed(f": > {state} && sleep 0.3")
    finally:
        syncing.succeed(f"rm -f {state}")
    # Done once it's no longer running: inactive, or failed if it stopped hearing.
    syncing.wait_until_succeeds(
        "systemctl show -p ActiveState --value chrony-network-servers.service"
        " | grep -qvx activating",
        timeout=60,
    )
    for unit in ("chrony-network-servers.path", "chrony-network-servers.service"):
        result = syncing.succeed(f"systemctl show -p Result --value {unit}").strip()
        assert result == "success", f"{unit}: {result}"
    syncing.succeed("systemctl is-active chrony-network-servers.path")


def test_the_clock_sets_from_the_machines_own_server(syncing: BaseMachine, silent: str) -> None:
    """The clock sets from the machine's own server, past the one that never answers."""
    syncing.wait_until_succeeds("chronyc waitsync 1", timeout=SYNCING)
    syncing.wait_until_succeeds(
        "timedatectl show -p NTPSynchronized --value | grep -qx yes", timeout=SYNCING
    )
    # The server chosen is the machine's own, by the name it was given, which chronyc says
    # only when asked to; the network's is out of reach.
    sources = syncing.succeed("chronyc -c -N sources").splitlines()
    chosen = [line.split(",")[2] for line in sources if line.split(",")[1] == "*"]
    assert chosen == ["server"], sources
    unheard = [line for line in sources if line.split(",")[2] == silent]
    assert unheard, sources
    assert unheard[0].split(",")[1] == "?", sources

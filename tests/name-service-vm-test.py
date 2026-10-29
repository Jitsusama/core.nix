"""A name whose IPv6 answer comes first still gives its IPv4 address through the name service.

tests/name-service.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), and
its opening comment says how the asker holds IPv4 back. Each name is a .local one, which only
mDNS answers, so the test network's /etc/hosts entries, which name every machine bare, answer
nothing.
"""

from __future__ import annotations

import json
from datetime import timedelta
from typing import TYPE_CHECKING

import pytest

if TYPE_CHECKING:
    from test_driver.driver import Driver
    from test_driver.machine import BaseMachine

# How long resolved may take to start answering for its name once the link is up.
ANNOUNCING = timedelta(minutes=1)

# The asker's IPv4 mDNS queries wait this long in a queue of their own before they leave,
# far past the few milliseconds the answer over IPv6 takes.
HELD_BACK = "1s"

# The address families as resolved names them.
IPV4, IPV6 = 2, 10

RESOLVE = "/run/systemd/resolve/io.systemd.Resolve"


def address(machine: BaseMachine) -> str:
    """Give the machine's IPv4 address on the test network."""
    return machine.succeed("ip -4 -o addr show dev eth1 | awk '{print $4}' | cut -d/ -f1").strip()


def hold_back_ipv4_mdns(machine: BaseMachine) -> None:
    """Have every IPv4 mDNS packet the machine sends wait in a delaying queue first."""
    machine.succeed(
        "tc qdisc add dev eth1 root handle 1: prio",
        f"tc qdisc add dev eth1 parent 1:3 handle 30: netem delay {HELD_BACK}",
        "tc filter add dev eth1 parent 1: protocol ip prio 1 u32"
        " match ip dport 5353 0xffff flowid 1:3",
    )


@pytest.fixture(scope="module")
def started(driver: Driver, machines: dict[str, BaseMachine]) -> dict[str, BaseMachine]:
    """Give both machines once each answers mDNS for its name and the asker holds IPv4 back."""
    driver.start_all()
    for machine in machines.values():
        machine.wait_for_unit("systemd-resolved.service")
        machine.wait_until_succeeds(
            "networkctl status eth1 | grep -q 'State: routable (configured)'"
        )
    asker = machines["asker"]
    asker.wait_for_unit("nscd.service")
    asker.wait_until_succeeds(
        "resolvectl query -4 --protocol=mdns --cache=no answerer.local", timeout=ANNOUNCING
    )
    hold_back_ipv4_mdns(asker)
    return machines


def test_resolved_answers_from_ipv6_alone(started: dict[str, BaseMachine]) -> None:
    """Asked for any family, resolved gives only what came first: the IPv6 address.

    This is what made the name service lose IPv4. If it fails, resolved now joins both
    families' answers, and the patch may no longer be needed.
    """
    asker = started["asker"]
    asker.succeed("resolvectl flush-caches")
    found = json.loads(
        asker.succeed(
            f"varlinkctl call {RESOLVE} io.systemd.Resolve.ResolveHostname"
            ' \'{"name":"answerer.local"}\''
        )
    )
    families = {entry["family"] for entry in found["addresses"]}
    assert families == {IPV6}, f"resolved answered with families {families}, not IPv6 alone"


def test_the_name_service_gives_the_ipv4_address(started: dict[str, BaseMachine]) -> None:
    """An IPv4 lookup through nsncd, as getaddrinfo makes it, gives the answerer's address."""
    asker, answerer = started["asker"], started["answerer"]
    expected = address(answerer)
    asker.succeed("resolvectl flush-caches")
    found = asker.succeed("getent ahostsv4 answerer.local")
    assert expected in found.split(), f"getent ahostsv4 gave {found!r}, not {expected}"


def test_the_name_service_gives_both_families(started: dict[str, BaseMachine]) -> None:
    """A lookup of any family through nsncd gives the IPv4 address beside the IPv6."""
    asker, answerer = started["asker"], started["answerer"]
    expected = address(answerer)
    asker.succeed("resolvectl flush-caches")
    found = asker.succeed("getent ahosts answerer.local").split()
    assert expected in found, f"getent ahosts gave {found}, without {expected}"
    assert any(":" in word for word in found), f"getent ahosts gave {found}, without IPv6"

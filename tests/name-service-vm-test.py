"""A name whose IPv6 answer comes first still gives its IPv4 address through the name service.

tests/name-service.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), and
its opening comment says how the asker holds IPv4 back. Each name is a .local one, which only
mDNS answers, so the test network's /etc/hosts entries, which name every machine bare, answer
nothing. The neighbour with no IPv6 is timed first, before IPv4 is held back, so its lookups
take what resolved takes and nothing the test adds.
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

# How much longer than resolved's own IPv4 answer a lookup through the name service may take,
# for a neighbour with no IPv6 address.
SLOWER_BY_AT_MOST = timedelta(milliseconds=500)


def resolve(name: str, family: int | None = None) -> str:
    """Give the command asking resolved itself for a name, in one family or any."""
    asked = {"name": name} | ({} if family is None else {"family": family})
    return f"varlinkctl call {RESOLVE} io.systemd.Resolve.ResolveHostname '{json.dumps(asked)}'"


def timed(machine: BaseMachine, command: str) -> tuple[timedelta, str]:
    """Run a command with resolved's cache emptied first, giving its time and its output.

    A command that fails is timed all the same, its output empty.
    """
    machine.succeed("resolvectl flush-caches")
    output = machine.succeed(
        f"start=$(date +%s%N); {command} > /tmp/timed || : > /tmp/timed; end=$(date +%s%N);"
        " echo $(( (end - start) / 1000 )); cat /tmp/timed"
    )
    took, _, rest = output.partition("\n")
    elapsed = timedelta(microseconds=int(took))
    machine.log(f"{command} took {elapsed}")
    return elapsed, rest


def address(machine: BaseMachine) -> str:
    """Give the machine's IPv4 address on the test network."""
    return machine.succeed("ip -4 -o addr show dev eth1 | awk '{print $4}' | cut -d/ -f1").strip()


@pytest.fixture(scope="module")
def started(driver: Driver, machines: dict[str, BaseMachine]) -> dict[str, BaseMachine]:
    """Give the machines once the asker finds each of the others by mDNS."""
    driver.start_all()
    for machine in machines.values():
        machine.wait_for_unit("systemd-resolved.service")
        machine.wait_until_succeeds(
            "networkctl status eth1 | grep -q 'State: routable (configured)'"
        )
    asker = machines["asker"]
    asker.wait_for_unit("nscd.service")
    for name in ("answerer.local", "plain.local"):
        asker.wait_until_succeeds(
            f"resolvectl query -4 --protocol=mdns --cache=no {name}", timeout=ANNOUNCING
        )
    return machines


@pytest.fixture(scope="module")
def held_back(started: dict[str, BaseMachine]) -> dict[str, BaseMachine]:
    """Give the machines once every IPv4 mDNS packet the asker sends waits in a delaying queue."""
    started["asker"].succeed(
        "tc qdisc add dev eth1 root handle 1: prio",
        f"tc qdisc add dev eth1 parent 1:3 handle 30: netem delay {HELD_BACK}",
        "tc filter add dev eth1 parent 1: protocol ip prio 1 u32"
        " match ip dport 5353 0xffff flowid 1:3",
    )
    return started


@pytest.mark.parametrize("lookup", ["ahostsv4", "ahosts"])
def test_a_neighbour_without_ipv6_answers_as_soon_as_resolved_does(
    started: dict[str, BaseMachine], lookup: str
) -> None:
    """A neighbour with no IPv6 address gives its IPv4 one through nsncd about as fast as resolved.

    nsncd asks for IPv6 beside IPv4, and nothing answers that, so its lookup mustn't wait for
    the IPv6 one to give up. The IPv6 lookup's own time is logged beside, for what's at stake.
    """
    asker, plain = started["asker"], started["plain"]
    expected = address(plain)
    timed(asker, resolve("plain.local", IPV6))
    direct, _ = timed(asker, resolve("plain.local", IPV4))
    through, found = timed(asker, f"getent {lookup} plain.local")
    assert expected in found.split(), f"getent {lookup} gave {found!r}, not {expected}"
    assert through - direct < SLOWER_BY_AT_MOST, (
        f"getent {lookup} took {through}, against resolved's {direct}"
    )


def test_resolved_answers_from_ipv6_alone(held_back: dict[str, BaseMachine]) -> None:
    """Asked for any family, resolved gives only what came first: the IPv6 address.

    This is what made the name service lose IPv4. If it fails, resolved now joins both
    families' answers, and the patch may no longer be needed.
    """
    asker = held_back["asker"]
    asker.succeed("resolvectl flush-caches")
    found = json.loads(asker.succeed(resolve("answerer.local")))
    families = {entry["family"] for entry in found["addresses"]}
    assert families == {IPV6}, f"resolved answered with families {families}, not IPv6 alone"


def test_the_name_service_gives_the_ipv4_address(held_back: dict[str, BaseMachine]) -> None:
    """An IPv4 lookup through nsncd, as getaddrinfo makes it, gives the answerer's address."""
    asker, answerer = held_back["asker"], held_back["answerer"]
    expected = address(answerer)
    asker.succeed("resolvectl flush-caches")
    found = asker.succeed("getent ahostsv4 answerer.local")
    assert expected in found.split(), f"getent ahostsv4 gave {found!r}, not {expected}"


def test_the_name_service_gives_both_families(held_back: dict[str, BaseMachine]) -> None:
    """A lookup of any family through nsncd gives the IPv4 address beside the IPv6."""
    asker, answerer = held_back["asker"], held_back["answerer"]
    expected = address(answerer)
    asker.succeed("resolvectl flush-caches")
    found = asker.succeed("getent ahosts answerer.local").split()
    assert expected in found, f"getent ahosts gave {found}, without {expected}"
    assert any(":" in word for word in found), f"getent ahosts gave {found}, without IPv6"

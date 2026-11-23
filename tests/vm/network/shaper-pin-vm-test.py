"""Whether the shaper pins the VPN's next socket to source ports that go the quick way.

shaper-pin.nix, beside this, runs this through pytest inside the driver (tests/lib/vm_pytest.py),
and its opening comment says what the test proves. The port a flow leaves from is read where it
lands, at the VPN's far end, which learns its peer's address and port from what reaches it.
"""

from __future__ import annotations

import datetime as dt
from dataclasses import dataclass
from typing import TYPE_CHECKING

import pytest

if TYPE_CHECKING:
    from test_driver.machine import BaseMachine

# The two ports the router sends the near way, the last two a round probes.
NEAR = (30030, 30031)
CLIENT = "192.168.1.1"
END = "192.168.9.2"
# The firewall mark the tunnel's own packets carry, and the table it routes the rest by,
# as wg-quick chooses them.
TUNNEL_MARK = "0xca6c"
TUNNEL_TABLE = 51820
# How long the client may take to boot to its first queues.
COMING_UP = dt.timedelta(seconds=30)
# How long a pin may take: a round, about 45 s, since the server answers expired probes six at
# once and then one a second, as Linux does, so most of the hundred wait out their half second;
# then the 35 s the round's probes hold their ports in conntrack, and the shaper's look every
# five seconds. The first run took 73 s.
PINNING = dt.timedelta(seconds=150)
# How long the far end may take to hear the VPN from its new port: the next keepalive.
MOVING = dt.timedelta(seconds=15)
# How long a pin no flow gets through may take to let go: the shaper's look every five
# seconds, twice, and then the next keepalive.
LETTING_GO = dt.timedelta(seconds=40)
# How many times the shaper takes its pin back in the restart subtest: at the restart and at
# the start after a stop.
TAKEN_BACK = 2
# A port the VPN listens on that no pin names.
ITS_OWN = 41002


@dataclass
class Rig:
    """The client, and the server holding the VPN's far end in its namespace."""

    client: BaseMachine
    server: BaseMachine

    def port(self) -> int:
        """Give the port the far end last heard the client's VPN from, or 0 before it has."""
        out = self.server.succeed("ip netns exec end wg show wg0 endpoints")
        for line in out.splitlines():
            _, _, endpoint = line.partition("\t")
            if endpoint.startswith(f"{CLIENT}:"):
                return int(endpoint.rpartition(":")[2])
        return 0

    def reopen(self, listen: int, timeout: dt.timedelta = MOVING) -> int:
        """Have the VPN open a new socket, from `listen`, and give the port it lands on."""
        was = self.port()
        self.client.succeed(f"wg set wg0 listen-port {listen}")
        self.server.wait_until_succeeds(
            f"ip netns exec end wg show wg0 endpoints | grep -v ':{was}$' | grep -q '{CLIENT}:'",
            timeout=timeout,
        )
        return self.port()


@pytest.fixture(scope="module")
def tunnel(machines: dict[str, BaseMachine]) -> Rig:
    """Give the machines once the client's VPN reaches its far end through the router."""
    client, router, server = machines["client"], machines["router"], machines["server"]
    for node in machines.values():
        node.start()
    router.wait_for_unit("ways.service")
    server.wait_for_unit("end.service")
    client.wait_until_succeeds("nft list set inet shaper uplinks | grep -q eth1", timeout=COMING_UP)
    keys = {}
    for name, node, run in (("client", client, ""), ("end", server, "ip netns exec end ")):
        node.succeed(f"{run}wg genkey > /root/{name}.key")
        keys[name] = node.succeed(f"{run}wg pubkey < /root/{name}.key").strip()
    # The client's runs in userspace, in a system service, as a VPN's daemon does.
    client.succeed("systemd-run --unit=tunnel --setenv=WG_PROCESS_FOREGROUND=1 wireguard-go -f wg0")
    client.wait_until_succeeds("ip link show wg0", timeout=COMING_UP)
    client.succeed(
        f"wg set wg0 private-key /root/client.key listen-port 51820 fwmark {TUNNEL_MARK}"
        f" peer {keys['end']} endpoint {END}:51820 allowed-ips 10.9.0.0/24"
        " persistent-keepalive 1",
        "ip addr add 10.9.0.1/24 dev wg0",
        "ip link set wg0 up",
        f"ip route add 10.9.0.0/24 dev wg0 table {TUNNEL_TABLE}",
        f"ip rule add not fwmark {TUNNEL_MARK} table {TUNNEL_TABLE}",
    )
    server.succeed(
        "ip -n end link add wg0 type wireguard",
        f"ip netns exec end wg set wg0 private-key /root/end.key listen-port 51820"
        f" peer {keys['client']} allowed-ips 10.9.0.1/32",
        "ip -n end addr add 10.9.0.2/24 dev wg0",
        "ip -n end link set wg0 up",
    )
    client.wait_until_succeeds("ping -c1 -W1 10.9.0.2", timeout=COMING_UP)
    return Rig(client=client, server=server)


@pytest.fixture(scope="module")
def pinned(tunnel: Rig) -> Rig:
    """Give the rig once the shaper has pinned the VPN's next socket, and says so."""
    tunnel.client.wait_until_succeeds(
        "journalctl -u shaper -o cat --no-pager"
        f" | grep -q 'leaves from port {NEAR[0]} or {NEAR[1]}'",
        timeout=PINNING,
    )
    return tunnel


@dataclass
class Moved:
    """The rig once the VPN opened a socket after the pin, with the ports before and after."""

    rig: Rig
    before: int
    after: int


@pytest.fixture(scope="module")
def moved(pinned: Rig) -> Moved:
    """Give the rig once the VPN has opened a new socket since the pin."""
    before = pinned.port()
    return Moved(rig=pinned, before=before, after=pinned.reopen(41000))


def test_the_vpns_next_socket_goes_the_quick_way(moved: Moved) -> None:
    """The socket the VPN opened before the pin keeps its port; the next lands on the pin."""
    assert moved.before not in NEAR, f"the VPN's first socket landed on {moved.before} unpinned"
    assert moved.after in NEAR, (
        f"the VPN's next socket landed on {moved.after}:\n"
        + moved.rig.client.succeed("nft list chain inet shaper pin")
    )


def test_a_held_port_yields_the_other(moved: Moved) -> None:
    """While the last socket's flow holds its pinned port, the next takes the other."""
    after = moved.rig.reopen(41001)
    assert {moved.after, after} == set(NEAR), (
        f"the VPN went from {moved.after} to {after}, not across the pair"
    )


def test_a_pin_is_kept_through_a_restart_and_a_stop(moved: Moved) -> None:
    """A restart keeps the pin in force; a stop lets it go, and the start puts it back.

    A switch restarts the shaper, or stops it and starts the new one, while the VPN's live
    flows keep the pairs NAT gave them, so the new shaper takes back what the old one kept.
    Which ports a round leaves out after it is the unit tests' to show.
    """
    rig = moved.rig
    pinned = f"nft list chain inet shaper pin | grep -q 'masquerade.*{NEAR[0]}-{NEAR[1]}'"
    kept = f"pin to ports {NEAR[0]} and {NEAR[1]} is kept through the restart"
    rig.client.succeed("systemctl restart shaper")
    rig.client.wait_until_succeeds(
        f"journalctl -u shaper -o cat --no-pager | grep -c '{kept}' | grep -qx 1",
        timeout=COMING_UP,
    )
    rig.client.succeed(pinned)
    rig.client.succeed("systemctl stop shaper")
    assert "masquerade" not in rig.client.succeed("nft list chain inet shaper pin"), (
        "a stopped shaper left the VPN pinned"
    )
    rig.client.succeed("systemctl start shaper")
    rig.client.wait_until_succeeds(pinned, timeout=COMING_UP)
    journal = rig.client.succeed("journalctl -u shaper -o cat --no-pager")
    assert journal.count(kept) == TAKEN_BACK, "the shaper put its pin back without saying so"


def test_a_pin_no_flow_gets_through_lets_go(moved: Moved) -> None:
    """With both pinned ports held toward the VPN's end, the pin lets go of its next socket.

    The VPN's own flow holds the port it's on, and a flow from the other port to the same
    end, refreshed each second, holds that one, so NAT finds neither free. In a full run the
    previous subtest's socket already holds the other port, so NAT moves the socat flow off
    it; either way both are held.
    """
    rig = moved.rig
    for port in sorted(set(NEAR) - {rig.port()}):
        # A datagram a second, inside conntrack's 30 s for unanswered UDP, keeps its entry.
        rig.client.succeed(
            f"systemd-run --unit=holds-{port} /run/current-system/sw/bin/sh -c 'while true;"
            f" do echo held | /run/current-system/sw/bin/socat -u -"
            f" UDP4-SENDTO:{END}:51820,sourceport={port};"
            " /run/current-system/sw/bin/sleep 1; done'"
        )
    try:
        after = rig.reopen(ITS_OWN, timeout=LETTING_GO)
    finally:
        rig.client.succeed("systemctl stop 'holds-*' || true")
    journal = rig.client.succeed("journalctl -u shaper -o cat --no-pager")
    assert after == ITS_OWN, (
        f"the VPN's next socket landed on {after}, not its own {ITS_OWN}:\n"
        + rig.client.succeed(
            "nft list chain inet shaper pin; nft list counter inet shaper pinretried"
        )
    )
    assert "came back unsent" in journal, "the shaper didn't say why its pin let go"
    # A round may pin again, but only ten minutes after the last, which a restart keeps,
    # so the rule is gone now.
    assert rig.client.succeed("nft list chain inet shaper pin").count("masquerade") == 0, (
        "the pin let go but its rule stayed"
    )

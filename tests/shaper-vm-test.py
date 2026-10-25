"""Which of CAKE's tins each kind of traffic reaches, both ways, read from CAKE's own counts.

tests/shaper.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), and its
opening comment says what the test proves. The counts are what CAKE says it sent, so a class
that never reached its packets, a set left empty or a rule that matched the wrong depth, shows
as bytes in the wrong tin. They are bytes, not packets: what arrives reaches the IFB merged by
GRO, so a download's packet count says more about the merging than about the traffic.

Each test asks for the state it starts from as a fixture: the line up with its slices held,
a WireGuard tunnel to the server, the default route moved to the client's other link, that
link unplugged and plugged back in. A fixture's work is done once a run, however many tests
ask for it, so any test runs alone with `-k`, and all of them run in the order they're
written, each after what it needs. The tests need no value from Nix: they ask the machines
for their addresses.
"""

from __future__ import annotations

import datetime as dt
import json
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any

import pytest
from test_driver.machine import retry

if TYPE_CHECKING:
    from collections.abc import Callable

    from test_driver.machine import BaseMachine

# A JSON object as tc gives it, whose fields the test reads as it finds them.
type Json = dict[str, Any]

# CAKE's diffserv4 tins, in the order its counts list them.
TINS = ["bulk", "best effort", "video", "voice"]
VOICE = TINS.index("voice")
# The machines' own programs, for units systemd-run makes: a system unit's
# PATH has no coreutils, where an account's does.
BIN = "/run/current-system/sw/bin"
# What a tin has to carry for a transfer to count as reaching it: most of a
# 1 MB upload, or most of 300 small datagrams.
LEAST_TRANSFER = 200_000
LEAST_DATAGRAMS = 5_000
# A tin the traffic shouldn't reach may carry under this share of what the
# right tin carried: a stray acknowledgement or two, not the transfer.
STRAY_SHARE = 4
# How many of the shaper's own pings show it's still pinging.
PINGS = 5
# Traffic to the router that may pass through CAKE, against the 1000
# datagrams sent, and how many of them the local class has to have carried.
LOCAL_STRAYS = 100
LOCAL_LEAST = 500
# How long the shaper may take to set a link's queues and sets up, or to
# notice a slice, a route or a link come and go.
SHAPING = dt.timedelta(seconds=15)
# How long a set may take to hold a slice held open for it.
HOLDING = dt.timedelta(seconds=10)
# How long the client may take to boot to its first queues, or a tunnel or a
# plugged-in link to come up.
COMING_UP = dt.timedelta(seconds=30)
# How long a device's queue may take to go still after a transfer.
STILLING = dt.timedelta(seconds=30)


def address(node_ip: str) -> str:
    """Give the IPv4 address an `ip -o address` line holds."""
    return node_ip.split()[3].split("/", maxsplit=1)[0]


@dataclass
class Line:
    """The client, its line through the router to the server, and what CAKE counts on it."""

    client: BaseMachine
    server: BaseMachine
    far: str
    near: str

    @property
    def upload(self) -> str:
        """Give a script sending 1 MB to the server."""
        return f"{BIN}/head -c 1000000 /dev/zero > /dev/tcp/{self.far}/5201"

    @property
    def download(self) -> str:
        """Give a script taking the server's 10 MB."""
        return f"exec 3<>/dev/tcp/{self.far}/5202; {BIN}/cat <&3 > /dev/null"

    def datagrams(self, port: int, n: int = 300, to: str | None = None) -> str:
        """Give a script sending small datagrams to a port, at the server unless told."""
        return f"for i in {{1..{n}}}; do echo x > /dev/udp/{to or self.far}/{port}; done"

    def qdiscs(self, dev: str) -> list[Json]:
        """Give the client's queues on a device, with their counts."""
        queues: list[Json] = json.loads(self.client.succeed(f"tc -s -j qdisc show dev {dev}"))
        return queues

    def counts(self, dev: str, what: str = "sent_bytes") -> list[int]:
        """Give what CAKE on a device sent from each tin."""
        cake = next(q for q in self.qdiscs(dev) if q["kind"] == "cake")
        return [int(tin[what]) for tin in cake["tins"]]

    def ifb(self, link: str) -> str:
        """Give the name of the IFB that carries a link's downloads."""
        index = int(self.client.succeed(f"cat /sys/class/net/{link}/ifindex"))
        return f"shaper{index:x}"

    def shaped(self, link: str) -> bool:
        """Say whether a link carries the shaper's HTB with CAKE beneath it."""
        kinds = {(q["kind"], q.get("parent", "root")) for q in self.qdiscs(link)}
        return ("htb", "root") in kinds and ("cake", "1:2") in kinds

    def hold(self, slice_name: str, *, system: bool = False) -> None:
        """Keep a slice open, as something always runs in one on a machine in use."""
        where = "" if system else "-M alice@ --user"
        unit = "hold-" + slice_name.removesuffix(".slice")
        self.client.succeed(
            f"systemctl {where} is-active {unit}.service"
            f" || systemd-run {where} --slice={slice_name} --unit={unit} {BIN}/sleep infinity"
        )

    def run(self, where: str, script: str) -> None:
        """Run a bash script in a slice: an account's, the system's, or a named unit's."""
        if where.startswith("system:"):
            slice_name = where.removeprefix("system:")
            self.client.succeed(
                f"systemd-run --wait --collect --slice={slice_name} {BIN}/bash -c '{script}'"
            )
        else:
            unit = f"--unit={where.removeprefix('unit:')}" if where.startswith("unit:") else ""
            slice_name = "app.slice" if unit else where
            self.client.succeed(
                f"systemd-run -M alice@ --user --wait --collect --slice={slice_name} {unit}"
                f" {BIN}/bash -c '{script}'"
            )

    def still(self, dev: str) -> None:
        """Wait until a device's queue goes still.

        A transfer's script ends once its bytes are in the socket, and the rest leaves after,
        so the next count would take its tail. Voice is left out, since the shaper's pings
        never stop.
        """
        last: list[list[int]] = []

        def unchanged(_last_try: bool) -> bool:
            now = self.counts(dev)[:VOICE]
            was = last[-1] if last else None
            last.append(now)
            return now == was

        retry(unchanged, timeout=STILLING)

    def moved_by(self, where: str, send: Callable[[], None], dev: str) -> dict[str, int]:
        """Send once a device's queue is still, and give the bytes each tin sent meanwhile."""
        self.still(dev)
        before = self.counts(dev)
        send()
        self.still(dev)
        after = self.counts(dev)
        moved = dict(zip(TINS, (a - b for a, b in zip(after, before, strict=True)), strict=True))
        print(f"{where} on {dev}: {moved}")
        return moved

    def reaches(self, tin: str, where: str, send: Callable[[], None], dev: str, least: int) -> None:
        """Check that what a sender sends reaches one tin, and no other."""
        moved = self.moved_by(where, send, dev)
        assert moved[tin] >= least, f"{where} on {dev}: {moved}"
        for other in TINS:
            # The shaper's own pings go on into voice meanwhile.
            if other not in {tin, "voice"}:
                assert moved[other] < moved[tin] // STRAY_SHARE, f"{where} on {dev}: {moved}"

    def lands(self, tin: str, where: str, script: str, dev: str = "eth1") -> None:
        """Check that what a script sends from a slice reaches one tin, and no other."""
        least = LEAST_DATAGRAMS if "/dev/udp" in script else LEAST_TRANSFER
        self.reaches(tin, where, lambda: self.run(where, script), dev, least)


@pytest.fixture(scope="module")
def line(machines: dict[str, BaseMachine]) -> Line:
    """Give the line once the shaper shapes it, every slice held open and in its set."""
    client, server = machines["client"], machines["server"]
    for node in machines.values():
        node.start()
    server.wait_for_unit("sink.service")
    server.wait_for_unit("source.service")
    server.wait_for_open_port(5201)
    server.wait_for_open_port(5202)
    client.wait_for_unit("user@1000.service")
    client.wait_for_unit("shaper.service")
    client.wait_until_succeeds("tc -j qdisc show dev eth1 | grep -q cake", timeout=COMING_UP)
    client.wait_until_succeeds("nft list set inet shaper uplinks | grep -q eth1", timeout=COMING_UP)
    held = Line(
        client=client,
        server=server,
        far=address(server.succeed("ip -4 -o address show dev eth1")),
        near=address(client.succeed("ip -4 -o address show dev eth1")),
    )
    # Each slice is held open; a slice made afresh is a test further on.
    for name in ["jobs.slice", "jobs-awaited.slice", "background.slice", "app.slice"]:
        held.hold(name)
    held.hold("build.slice", system=True)
    for set_name, name in [
        ("bulk4", "background.slice"),
        ("awaited5", "jobs-awaited.slice"),
        ("bulk1", "build.slice"),
    ]:
        client.wait_until_succeeds(
            f"nft list set inet shaper {set_name} | grep -q {name}", timeout=HOLDING
        )
    return held


@pytest.fixture(scope="module")
def tunnel(line: Line) -> Line:
    """Give the line with a WireGuard tunnel to the server, shaped as a VPN's is."""
    keys = {}
    for node in (line.client, line.server):
        node.succeed("wg genkey > /root/key")
        keys[node.name] = node.succeed("wg pubkey < /root/key").strip()
    for node, me, peer, there in (
        (line.client, "10.9.0.1", line.server, line.far),
        (line.server, "10.9.0.2", line.client, line.near),
    ):
        node.succeed(
            "ip link add wg0 type wireguard",
            f"wg set wg0 private-key /root/key listen-port 51820 peer {keys[peer.name]}"
            f" endpoint {there}:51820 allowed-ips 10.9.0.0/24",
            f"ip addr add {me}/24 dev wg0",
            "ip link set wg0 up",
        )
    line.client.wait_until_succeeds("ping -c1 -W1 10.9.0.2", timeout=COMING_UP)
    line.client.wait_until_succeeds("tc -j qdisc show dev wg0 | grep -q cake", timeout=SHAPING)
    line.client.wait_until_succeeds(
        "nft list set inet shaper uplinks | grep -q wg0", timeout=SHAPING
    )
    return line


@pytest.fixture(scope="module")
def on_eth2(line: Line) -> Line:
    """Give the line with the default route moved to the client's other link, eth2."""
    line.client.succeed("ip route replace default via 192.168.3.2 dev eth2")
    line.client.wait_until_succeeds("tc -j qdisc show dev eth2 | grep -q cake", timeout=SHAPING)
    line.client.wait_until_succeeds("! tc -j qdisc show dev eth1 | grep -q cake", timeout=SHAPING)
    line.client.wait_until_succeeds(
        "nft list set inet shaper uplinks | grep -q eth2", timeout=SHAPING
    )
    return line


@dataclass(frozen=True)
class Replugged:
    """The line after eth2 was unplugged and plugged back in, and its IFBs either side."""

    line: Line
    unplugged_ifb: str
    plugged_ifb: str


@pytest.fixture(scope="module")
def replugged(on_eth2: Line) -> Replugged:
    """Give the line after eth2 is unplugged and plugged back in, as a dock's adapter is.

    A dock's network adapter is a USB device: pulling the dock takes the link away, and
    plugging it back makes it anew under a new index, as unbinding a virtio NIC from its
    driver and binding it again does here. Its address comes back by itself, and the
    default route is put back as the network's DHCP would.
    """
    client = on_eth2.client
    device = client.succeed("basename $(readlink /sys/class/net/eth2/device)").strip()
    unplugged_ifb = on_eth2.ifb("eth2")
    client.succeed(f"echo {device} > /sys/bus/virtio/drivers/virtio_net/unbind")
    client.wait_until_succeeds("! ip link show eth2", timeout=SHAPING)
    client.wait_until_succeeds(
        "journalctl -u shaper | grep -q 'eth2 no longer routes'", timeout=SHAPING
    )
    client.succeed(f"! ip link show {unplugged_ifb}")
    client.succeed(f"echo {device} > /sys/bus/virtio/drivers/virtio_net/bind")
    client.wait_until_succeeds(
        "ip -4 -o address show dev eth2 | grep -q 192.168.3.", timeout=COMING_UP
    )
    client.succeed("ip route replace default via 192.168.3.2 dev eth2")
    client.wait_until_succeeds("tc -j qdisc show dev eth2 | grep -q cake", timeout=SHAPING)
    return Replugged(on_eth2, unplugged_ifb, on_eth2.ifb("eth2"))


def test_jobs(line: Line) -> None:
    """The account's work goes in bulk, both ways."""
    line.lands("bulk", "jobs.slice", line.upload)
    line.lands("bulk", "jobs.slice", line.download, dev=line.ifb("eth1"))
    line.lands("bulk", "background.slice", line.datagrams(9000))


def test_awaited(line: Line) -> None:
    """What Joel waits on at a terminal goes as an app's would, both ways."""
    line.lands("best effort", "jobs-awaited.slice", line.upload)
    line.lands("best effort", "jobs-awaited.slice", line.download, dev=line.ifb("eth1"))


def test_builds(line: Line) -> None:
    """The system's builds go in bulk."""
    line.lands("bulk", "system:build.slice", line.upload)


def test_guest(line: Line) -> None:
    """What it forwards for a machine or a container goes in bulk, both ways.

    A guest as libvirt's NAT or a container's bridge makes one: its own network namespace
    behind a veth, sent out eth1 under the client's address. Nothing it sends is a socket on
    the client, so only the forward chain can class it.
    """
    client = line.client
    client.succeed(
        "ip netns add guest",
        "ip link add vethguest type veth peer name eth0 netns guest",
        "ip address add 10.99.0.1/24 dev vethguest",
        "ip link set vethguest up",
        "ip -n guest address add 10.99.0.2/24 dev eth0",
        "ip -n guest link set eth0 up",
        "ip -n guest route add default via 10.99.0.1",
        "sysctl -w net.ipv4.ip_forward=1",
        "nft add table ip guest",
        "nft 'add chain ip guest out { type nat hook postrouting priority srcnat; }'",
        "nft add rule ip guest out ip saddr 10.99.0.0/24 oifname eth1 masquerade",
    )

    def guest(script: str) -> Callable[[], None]:
        """Give a sender that runs a script inside the guest."""

        def send() -> None:
            """Run the script inside the guest."""
            client.succeed(f"ip netns exec guest {BIN}/bash -c '{script}'")

        return send

    line.reaches("bulk", "a guest", guest(line.upload), "eth1", LEAST_TRANSFER)
    line.reaches("bulk", "a guest", guest(line.download), line.ifb("eth1"), LEAST_TRANSFER)
    client.succeed("nft delete table ip guest", "ip link delete vethguest", "ip netns delete guest")


def test_apps(line: Line) -> None:
    """An app's pages are best effort, and its calls video."""
    line.lands("best effort", "app.slice", line.upload)
    line.lands("best effort", "app.slice", line.download, dev=line.ifb("eth1"))
    line.lands("best effort", "app.slice", line.datagrams(443))
    line.lands("video", "app.slice", line.datagrams(9000))


def test_real_time(line: Line) -> None:
    """A call's packets make the shaper keep the line's queue here.

    Datagrams an app sends past 443 are a call's, in the video tin, and a few hundred in a
    second are as many as a call's audio sends; the shaper looks once a second.
    """
    line.run("app.slice", line.datagrams(9000, n=600))
    line.client.wait_until_succeeds(
        "shaper status | grep -q 'real-time traffic flows'", timeout=SHAPING
    )
    line.client.succeed("journalctl -u shaper | grep -q 'real-time traffic flows'")


def test_tuple(line: Line) -> None:
    """Everything Tuple's daemon sends is video.

    The daemon runs well before a call, so its unit's set knows it by the time it sends: the
    stand-in waits to send until the set holds it.
    """
    client = line.client
    client.succeed(
        "systemd-run -M alice@ --user --slice=app.slice --unit=tuple"
        f" {BIN}/bash -c 'until [ -e /tmp/send ]; do {BIN}/sleep 0.1; done; {line.upload}'"
    )
    client.wait_until_succeeds(
        "nft list set inet shaper video5 | grep -q tuple.service", timeout=HOLDING
    )

    def send() -> None:
        """Let the stand-in send, and wait until it's done."""
        client.succeed("touch /tmp/send")
        client.wait_until_fails("systemctl -M alice@ --user is-active tuple.service")

    line.reaches("video", "unit:tuple", send, "eth1", LEAST_TRANSFER)


def test_names(line: Line) -> None:
    """Names and time go first, from anything."""
    line.lands("voice", "jobs.slice", line.datagrams(53))


def test_pings(line: Line) -> None:
    """The shaper's pings go first, so they measure the line."""
    first = line.counts("eth1", "sent_packets")[VOICE]

    def pinged(_last_try: bool) -> bool:
        """Say whether the shaper has pinged a few times since the test began."""
        return line.counts("eth1", "sent_packets")[VOICE] - first > PINGS

    retry(pinged, timeout=HOLDING)


def test_slice_made_again(line: Line) -> None:
    """A slice made again keeps its class.

    The set holds a slice by its cgroup, which a new slice doesn't share, so it counts once
    the shaper has found the new one.
    """
    account = "/sys/fs/cgroup/user.slice/user-1000.slice/user@1000.service"
    line.client.succeed("systemctl -M alice@ --user stop background.slice")
    line.client.wait_until_fails(f"test -d {account}/background.slice")
    line.hold("background.slice")

    def found(_last_try: bool) -> bool:
        """Say whether most of a few datagrams from the new slice went in bulk."""
        script = line.datagrams(9000, n=30)
        moved = line.moved_by(
            "background.slice", lambda: line.run("background.slice", script), "eth1"
        )
        return moved["bulk"] > moved["best effort"]

    retry(found, timeout=SHAPING)
    line.lands("bulk", "background.slice", line.datagrams(9000))


def test_local(line: Line) -> None:
    """Traffic to the client's own network isn't slowed."""
    router_near = line.near.rsplit(".", 1)[0] + ".2"
    before = line.counts("eth1", "sent_packets")
    line.run("jobs.slice", line.datagrams(9000, n=1000, to=router_near))
    assert sum(line.counts("eth1", "sent_packets")) - sum(before) < LOCAL_STRAYS
    local = next(q for q in line.qdiscs("eth1") if q.get("parent") == "1:1")
    assert local["packets"] > LOCAL_LEAST, local


def test_tunnel(tunnel: Line) -> None:
    """Inside a tunnel, the same classes reach its own queue."""
    inner = f"{BIN}/head -c 1000000 /dev/zero > /dev/tcp/10.9.0.2/5201"
    tunnel.lands("bulk", "jobs.slice", inner, dev="wg0")
    tunnel.lands("best effort", "app.slice", inner, dev="wg0")


def test_route_moves(on_eth2: Line) -> None:
    """The queues follow the default route to another link."""
    assert not on_eth2.shaped("eth1")
    on_eth2.client.succeed(f"! ip link show {on_eth2.ifb('eth1')}")
    on_eth2.lands("bulk", "jobs.slice", on_eth2.upload, dev="eth2")


def test_status(on_eth2: Line) -> None:
    """Its status says what it holds the link to."""
    status = on_eth2.client.succeed("shaper status")
    assert "eth2" in status, status
    assert "held under load" in status, status


def test_ifbs_left_alone(on_eth2: Line) -> None:
    """Udev leaves the shaper's IFBs as it made them.

    Each IFB came and went with a link above, so udev has met several.
    """
    on_eth2.client.fail("journalctl -b --no-pager | grep -E 'shaper[0-9a-f]+: Could not set'")


def test_replug(replugged: Replugged) -> None:
    """A link unplugged and plugged back in is shaped afresh, both ways."""
    line = replugged.line
    assert replugged.plugged_ifb != replugged.unplugged_ifb
    line.client.wait_until_succeeds(
        f"tc -j qdisc show dev {replugged.plugged_ifb} | grep -q cake", timeout=SHAPING
    )
    assert line.shaped("eth2")
    line.client.wait_until_succeeds(
        "nft list set inet shaper uplinks | grep -q eth2", timeout=SHAPING
    )
    line.lands("bulk", "jobs.slice", line.upload, dev="eth2")
    line.lands("bulk", "jobs.slice", line.download, dev=replugged.plugged_ifb)


@pytest.mark.usefixtures("tunnel")
def test_stopping(on_eth2: Line) -> None:
    """Stopping it leaves the links as they were."""
    client = on_eth2.client
    eth2_ifb = on_eth2.ifb("eth2")
    client.succeed("systemctl stop shaper.service")
    assert not on_eth2.shaped("eth2")
    client.succeed(f"! ip link show {eth2_ifb}")
    client.succeed("! tc -j qdisc show dev eth2 | grep -q ingress")
    client.succeed("! tc -j qdisc show dev wg0 | grep -q cake")

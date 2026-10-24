"""Which of CAKE's tins each kind of traffic reaches, both ways, read from CAKE's own counts.

tests/shaper.nix runs this, and its opening comment says what the test proves. The counts are
what CAKE says it sent, so a class that never reached its packets, a set left empty or a rule
that matched the wrong depth, shows as bytes in the wrong tin. They are bytes, not packets:
what arrives reaches the IFB merged by GRO, so a download's packet count says more about the
merging than about the traffic. The script needs no value from Nix: it asks the machines for
their addresses.
"""

import json
from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from collections.abc import Callable

    from driver_names import client, retry, server, start_all, subtest, t

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


def address(node_ip: str) -> str:
    """Return the IPv4 address an `ip -o address` line holds."""
    return node_ip.split()[3].split("/", maxsplit=1)[0]


def qdiscs(dev: str) -> list[Json]:
    """Return the client's queues on a device, with their counts."""
    queues: list[Json] = json.loads(client.succeed(f"tc -s -j qdisc show dev {dev}"))
    return queues


def counts(dev: str, what: str = "sent_bytes") -> list[int]:
    """Return what CAKE on a device sent from each tin."""
    cake = next(q for q in qdiscs(dev) if q["kind"] == "cake")
    return [int(tin[what]) for tin in cake["tins"]]


def ifb(link: str) -> str:
    """Return the name of the IFB that carries a link's downloads."""
    index = int(client.succeed(f"cat /sys/class/net/{link}/ifindex"))
    return f"shaper{index:x}"


def shaped(link: str) -> bool:
    """Say whether a link carries the shaper's HTB with CAKE beneath it."""
    kinds = {(q["kind"], q.get("parent", "root")) for q in qdiscs(link)}
    return ("htb", "root") in kinds and ("cake", "1:2") in kinds


def hold(slice_name: str, *, system: bool = False) -> None:
    """Keep a slice open, as something always runs in one on a machine in use."""
    where = "" if system else "-M alice@ --user"
    unit = "hold-" + slice_name.removesuffix(".slice")
    client.succeed(
        f"systemctl {where} is-active {unit}.service"
        f" || systemd-run {where} --slice={slice_name} --unit={unit} {BIN}/sleep infinity"
    )


def run(where: str, script: str) -> None:
    """Run a bash script in a slice: an account's, the system's, or a named unit's."""
    if where.startswith("system:"):
        slice_name = where.removeprefix("system:")
        client.succeed(
            f"systemd-run --wait --collect --slice={slice_name} {BIN}/bash -c '{script}'"
        )
    else:
        unit = f"--unit={where.removeprefix('unit:')}" if where.startswith("unit:") else ""
        slice_name = "app.slice" if unit else where
        client.succeed(
            f"systemd-run -M alice@ --user --wait --collect --slice={slice_name} {unit}"
            f" {BIN}/bash -c '{script}'"
        )


def still(dev: str) -> None:
    """Wait until a device's queue goes still.

    A transfer's script ends once its bytes are in the socket, and the rest leaves after, so
    the next count would take its tail. Voice is left out, since the shaper's pings never stop.
    """
    last: list[list[int]] = []

    def unchanged(_last_try: bool) -> bool:
        now = counts(dev)[:VOICE]
        was = last[-1] if last else None
        last.append(now)
        return now == was

    retry(unchanged, timeout_seconds=30)


def moved_by(where: str, send: Callable[[], None], dev: str) -> dict[str, int]:
    """Send once a device's queue is still, and return the bytes each tin sent meanwhile."""
    still(dev)
    before = counts(dev)
    send()
    still(dev)
    after = counts(dev)
    moved = dict(zip(TINS, (a - b for a, b in zip(after, before, strict=True)), strict=True))
    print(f"{where} on {dev}: {moved}")
    return moved


def reaches(tin: str, where: str, send: Callable[[], None], dev: str, least: int) -> None:
    """Check that what a sender sends reaches one tin, and no other."""
    moved = moved_by(where, send, dev)
    t.assertGreaterEqual(moved[tin], least, f"{where} on {dev}: {moved}")
    for other in TINS:
        # The shaper's own pings go on into voice meanwhile.
        if other not in {tin, "voice"}:
            t.assertLess(moved[other], moved[tin] // STRAY_SHARE, f"{where} on {dev}: {moved}")


def lands(tin: str, where: str, script: str, dev: str = "eth1") -> None:
    """Check that what a script sends from a slice reaches one tin, and no other."""
    least = LEAST_DATAGRAMS if "/dev/udp" in script else LEAST_TRANSFER
    reaches(tin, where, lambda: run(where, script), dev, least)


start_all()
server.wait_for_unit("sink.service")
server.wait_for_unit("source.service")
server.wait_for_open_port(5201)
server.wait_for_open_port(5202)
client.wait_for_unit("user@1000.service")
client.wait_for_unit("shaper.service")
client.wait_until_succeeds("tc -j qdisc show dev eth1 | grep -q cake", timeout=30)
client.wait_until_succeeds("nft list set inet shaper uplinks | grep -q eth1", timeout=30)

far = address(server.succeed("ip -4 -o address show dev eth1"))
near = address(client.succeed("ip -4 -o address show dev eth1"))
upload = f"{BIN}/head -c 1000000 /dev/zero > /dev/tcp/{far}/5201"
download = f"exec 3<>/dev/tcp/{far}/5202; {BIN}/cat <&3 > /dev/null"


def datagrams(port: int, n: int = 300, to: str = far) -> str:
    """Return a script sending small datagrams to a port."""
    return f"for i in {{1..{n}}}; do echo x > /dev/udp/{to}/{port}; done"


# Each slice is held open; a slice made afresh is the subtest further on.
for name in ["jobs.slice", "jobs-awaited.slice", "background.slice", "app.slice"]:
    hold(name)
hold("build.slice", system=True)
for set_name, name in [
    ("bulk4", "background.slice"),
    ("awaited5", "jobs-awaited.slice"),
    ("bulk1", "build.slice"),
]:
    client.wait_until_succeeds(f"nft list set inet shaper {set_name} | grep -q {name}", timeout=10)

with subtest("the account's work goes in bulk, both ways"):
    lands("bulk", "jobs.slice", upload)
    lands("bulk", "jobs.slice", download, dev=ifb("eth1"))
    lands("bulk", "background.slice", datagrams(9000))

with subtest("what Joel waits on at a terminal goes as an app's would, both ways"):
    lands("best effort", "jobs-awaited.slice", upload)
    lands("best effort", "jobs-awaited.slice", download, dev=ifb("eth1"))

with subtest("the system's builds go in bulk"):
    lands("bulk", "system:build.slice", upload)

with subtest("what it forwards for a machine or a container goes in bulk, both ways"):
    # A guest as libvirt's NAT or a container's bridge makes one: its own network
    # namespace behind a veth, sent out eth1 under the client's address. Nothing it
    # sends is a socket on the client, so only the forward chain can class it.
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
        """Return a sender that runs a script inside the guest."""

        def send() -> None:
            """Run the script inside the guest."""
            client.succeed(f"ip netns exec guest {BIN}/bash -c '{script}'")

        return send

    reaches("bulk", "a guest", guest(upload), "eth1", LEAST_TRANSFER)
    reaches("bulk", "a guest", guest(download), ifb("eth1"), LEAST_TRANSFER)
    client.succeed("nft delete table ip guest", "ip link delete vethguest", "ip netns delete guest")

with subtest("an app's pages are best effort, and its calls video"):
    lands("best effort", "app.slice", upload)
    lands("best effort", "app.slice", download, dev=ifb("eth1"))
    lands("best effort", "app.slice", datagrams(443))
    lands("video", "app.slice", datagrams(9000))

with subtest("everything Tuple's daemon sends is video"):
    # The daemon runs well before a call, so its unit's set knows it by the
    # time it sends: the stand-in waits to send until the set holds it.
    client.succeed(
        "systemd-run -M alice@ --user --slice=app.slice --unit=tuple"
        f" {BIN}/bash -c 'until [ -e /tmp/send ]; do {BIN}/sleep 0.1; done; {upload}'"
    )
    client.wait_until_succeeds(
        "nft list set inet shaper video5 | grep -q tuple.service", timeout=10
    )

    def send() -> None:
        """Let the stand-in send, and wait until it's done."""
        client.succeed("touch /tmp/send")
        client.wait_until_fails("systemctl -M alice@ --user is-active tuple.service")

    reaches("video", "unit:tuple", send, "eth1", LEAST_TRANSFER)

with subtest("names and time go first, from anything"):
    lands("voice", "jobs.slice", datagrams(53))

with subtest("the shaper's pings go first, so they measure the line"):
    first = counts("eth1", "sent_packets")[VOICE]

    def pinged(_last_try: bool) -> bool:
        """Say whether the shaper has pinged a few times since the subtest began."""
        return counts("eth1", "sent_packets")[VOICE] - first > PINGS

    retry(pinged, timeout_seconds=10)

with subtest("a slice made again keeps its class"):
    account = "/sys/fs/cgroup/user.slice/user-1000.slice/user@1000.service"
    client.succeed("systemctl -M alice@ --user stop background.slice")
    client.wait_until_fails(f"test -d {account}/background.slice")
    hold("background.slice")

    # The set holds a slice by its cgroup, which a new slice doesn't share,
    # so it counts once the shaper has found the new one.
    def found(_last_try: bool) -> bool:
        """Say whether most of a few datagrams from the new slice went in bulk."""
        script = datagrams(9000, n=30)
        moved = moved_by("background.slice", lambda: run("background.slice", script), "eth1")
        return moved["bulk"] > moved["best effort"]

    retry(found, timeout_seconds=15)
    lands("bulk", "background.slice", datagrams(9000))

with subtest("traffic to the client's own network isn't slowed"):
    router_near = near.rsplit(".", 1)[0] + ".2"
    before = counts("eth1", "sent_packets")
    run("jobs.slice", datagrams(9000, n=1000, to=router_near))
    t.assertLess(sum(counts("eth1", "sent_packets")) - sum(before), LOCAL_STRAYS)
    local = next(q for q in qdiscs("eth1") if q.get("parent") == "1:1")
    t.assertGreater(local["packets"], LOCAL_LEAST)

with subtest("inside a tunnel, the same classes reach its own queue"):
    keys = {}
    for node in (client, server):
        node.succeed("wg genkey > /root/key")
        keys[node.name] = node.succeed("wg pubkey < /root/key").strip()
    for node, me, peer, there in (
        (client, "10.9.0.1", server, far),
        (server, "10.9.0.2", client, near),
    ):
        node.succeed(
            "ip link add wg0 type wireguard",
            f"wg set wg0 private-key /root/key listen-port 51820 peer {keys[peer.name]}"
            f" endpoint {there}:51820 allowed-ips 10.9.0.0/24",
            f"ip addr add {me}/24 dev wg0",
            "ip link set wg0 up",
        )
    client.wait_until_succeeds("ping -c1 -W1 10.9.0.2", timeout=30)
    client.wait_until_succeeds("tc -j qdisc show dev wg0 | grep -q cake", timeout=15)
    client.wait_until_succeeds("nft list set inet shaper uplinks | grep -q wg0", timeout=15)
    inner = f"{BIN}/head -c 1000000 /dev/zero > /dev/tcp/10.9.0.2/5201"
    lands("bulk", "jobs.slice", inner, dev="wg0")
    lands("best effort", "app.slice", inner, dev="wg0")

with subtest("the queues follow the default route to another link"):
    client.succeed("ip route replace default via 192.168.3.2 dev eth2")
    client.wait_until_succeeds("tc -j qdisc show dev eth2 | grep -q cake", timeout=15)
    client.wait_until_succeeds("! tc -j qdisc show dev eth1 | grep -q cake", timeout=15)
    t.assertFalse(shaped("eth1"))
    client.succeed(f"! ip link show {ifb('eth1')}")
    client.wait_until_succeeds("nft list set inet shaper uplinks | grep -q eth2", timeout=15)
    lands("bulk", "jobs.slice", upload, dev="eth2")

with subtest("its status says what it holds the link to"):
    status = client.succeed("shaper status")
    t.assertIn("eth2", status)
    t.assertIn("held under load", status)

with subtest("stopping it leaves the links as they were"):
    eth2_ifb = ifb("eth2")
    client.succeed("systemctl stop shaper.service")
    t.assertFalse(shaped("eth2"))
    client.succeed(f"! ip link show {eth2_ifb}")
    client.succeed("! tc -j qdisc show dev eth2 | grep -q ingress")
    client.succeed("! tc -j qdisc show dev wg0 | grep -q cake")

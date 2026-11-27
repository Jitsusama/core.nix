"""Whether the shaper takes back an upload's queue that a router sharing it flow by flow hides.

shaper-owned.nix, beside this, runs this through pytest inside the driver (tests/lib/vm_pytest.py),
and its opening comment says what the test proves. The router is the connection: HTB at 8 Mbit/s
with fq_codel under it, whose queue delays no probe, as no flow but the one that fills it waits in
it. The server counts what reaches each port, the jobs' and the app's, so a share is read where it
lands rather than where it left.
"""

from __future__ import annotations

import datetime as dt
import json
import time
from dataclasses import dataclass, field
from typing import TYPE_CHECKING, Any

import pytest
from test_driver.machine import retry

if TYPE_CHECKING:
    from test_driver.machine import BaseMachine

# A JSON object as nft or the shaper gives it, whose fields the test reads as it finds them.
type Json = dict[str, Any]

BIN = "/run/current-system/sw/bin"
FAR = "203.0.113.3"
# The jobs' uploads beside the app's one.
JOBS = 32
JOBS_PORT = 5201
APP_PORT = 5202
# What the server takes while a share is read: four seconds of the line,
# so a share is many round trips' worth, in bytes.
WINDOW = 4_000_000
# What the app sends before its share counts, so its slow start, while the
# jobs still hold the connection, isn't counted against it.
APP_STARTS = 500_000
# The app's least share with the shaper. Agents keep a quarter of the rate
# while any wait, and Joel's lane, which the app's upload rides, grows each
# second from what it carried, settling near two thirds of the rate here, so
# the share is read once that lane is past half. Were the router's queue
# still hiding there, or work put first left among the agents', the upload
# would carry one flow's share of 33, its lane would never pass half, and the
# wait for it fails first. Its lane runs a fifth and a megabit ahead of what
# it carried, so passing the wait doesn't settle the bar; the share read
# after it does. Without the shaper the router gives it that one flow's
# share, and a fifth is well clear of it.
SHAPED_SHARE = 0.5
UNSHAPED_SHARE = 0.2
# How long the client may take to boot to its first queues, and the shaper
# to take the queue back from the router once the jobs fill it.
COMING_UP = dt.timedelta(seconds=30)
TAKING = dt.timedelta(seconds=90)
# How long the router must shed nothing, while the shaper holds the queue
# as its own, for the queue to count as taken back. A router still holding
# it under 32 uploads sheds each time codel's 100 ms interval passes with
# its queue over the 5 ms target, and a stray drop as the controller climbs
# only starts the count again.
HELD = dt.timedelta(seconds=5)
SENDING = dt.timedelta(seconds=60)
# The router's rate on eth2, in kbit/s, as shaper-owned.nix sets its HTB.
CONNECTION_KBIT = 8_000
# Under this share of the rate it should send at, a machine fell behind: the client
# with data waiting in eth1's queues, or the router while its fq_codel shed, as the
# rate test reads its client (shaper-rate-vm-test.py). A slow shaper never does.
BEHIND = 0.9


@dataclass
class Rig:
    """The client, the router that is its connection, and the server that counts what reaches it."""

    client: BaseMachine
    router: BaseMachine
    server: BaseMachine

    def link(self) -> Json:
        """Give what the shaper says of eth1, or nothing before it says; fails once it stops."""
        status: list[Json] = json.loads(self.client.succeed("cat /run/shaper/status.json"))
        return next((link for link in status if link["name"] == "eth1"), {})

    def up(self) -> int:
        """Give the rate the shaper sends at on eth1, in kbit/s, or 0 before it says."""
        return int(self.link().get("up") or 0)

    def joel_share(self, link: Json | None = None) -> float:
        """Give Joel's lane's share of the rate in `link`, eth1's status read now unless given."""
        link = self.link() if link is None else link
        up = int(link.get("up") or 0)
        joel = int((link.get("lanes") or {}).get("joel") or 0)
        return joel / up if up else 0.0

    def trace(self) -> str:
        """Give what the recorder saw each second, and what the shaper said, for a failure.

        Each second's upload, in kbit/s, from the link's counters, and the median of its
        own delays up, in ms, as the controller reads them before taking each
        reflector's least off.
        """
        text = self.client.succeed("cat /var/lib/shaper-record/*.jsonl")
        seconds: dict[int, tuple[list[int], list[float]]] = {}
        for line in text.splitlines():
            sample: Json = json.loads(line)
            if "tx" not in sample:
                continue
            sent, delays = seconds.setdefault(sample["at"] // 1000, ([], []))
            sent.append(sample["tx"])
            if "out" in sample:
                delays.append(sample["out"])
        rows = []
        last = None
        for second, (sent, delays) in sorted(seconds.items()):
            kbit = (max(sent) - last) * 8 // 1000 if last is not None else 0
            last = max(sent)
            median = sorted(delays)[len(delays) // 2] if delays else None
            rows.append(f"{second % 1000:03d} {kbit:6d} kbit/s up, out {median}")
        said = self.client.succeed("journalctl -u shaper -o cat --no-pager")
        status = self.client.succeed("cat /run/shaper/status.json")
        return "\n".join([*rows, said, status])

    def router_read(self) -> tuple[float, int, int]:
        """Give the router's clock, the bytes its HTB passed and what its fq_codel shed.

        What fq_codel dropped or marked is shed, as only a queue there sheds: marks count
        beside drops, since a flow that asks for ECN is marked where another would be
        dropped. Read in one command, so what HTB passed between two readings is timed by
        the router.
        """
        now, _, text = self.router.succeed("date +%s.%N; tc -s -j qdisc show dev eth2").partition(
            "\n"
        )
        qdiscs: list[Json] = json.loads(text)
        codels = [q for q in qdiscs if q.get("kind") == "fq_codel"]
        assert codels, f"no fq_codel on the router's eth2: {qdiscs}"
        htb = next((q for q in qdiscs if q.get("kind") == "htb"), None)
        assert htb is not None, f"no HTB on the router's eth2: {qdiscs}"
        shed = sum(int(q.get("drops") or 0) + int(q.get("ecn_mark") or 0) for q in codels)
        return float(now), int(htb["bytes"]), shed

    def client_read(self) -> tuple[float, int, int]:
        """Give the client's clock, the bytes eth1 has sent, and what its queues hold waiting.

        The link's own count says what left the machine, every lane's; the root qdisc, which
        tc names without a parent, holds every lane's waiting. Read in one command, so what
        it sent between two readings is timed by the client.
        """
        now, sent, text = self.client.succeed(
            "date +%s.%N; cat /sys/class/net/eth1/statistics/tx_bytes; tc -s -j qdisc show dev eth1"
        ).split("\n", 2)
        queues: list[Json] = json.loads(text)
        root = next(q for q in queues if "parent" not in q)
        return float(now), int(sent), int(root["backlog"])

    def htb_behind(self) -> str:
        """Give what either machine's kernel said of HTB falling behind its events.

        A starved machine's HTB says it had too many events, as the rate test's client's
        did on one core (shaper-rate.nix).
        """
        said = [
            f"{name}: {line}"
            for name, machine in (("client", self.client), ("router", self.router))
            for line in machine.succeed("dmesg | grep -F 'too many events' || true").splitlines()
        ]
        return "\n".join(said) or "neither kernel said HTB had too many events"

    def taken(self) -> dict[int, int]:
        """Give the bytes the server has taken on each port."""
        rules: Json = json.loads(self.server.succeed("nft -j list chain inet seen input"))
        counted: dict[int, int] = {}
        for item in rules["nftables"]:
            expr = item.get("rule", {}).get("expr", [])
            port = next((e["match"]["right"] for e in expr if "match" in e), None)
            count = next((e["counter"]["bytes"] for e in expr if "counter" in e), None)
            if port is not None and count is not None:
                counted[int(port)] = int(count)
        return counted

    def uploads(self, unit: str, slice_name: str, flows: int, port: int) -> None:
        """Start a slice's uploads to a port, until they're stopped."""
        self.client.succeed(
            f"systemd-run -M alice@ --user --slice={slice_name} --unit={unit}"
            f" {BIN}/bash -c 'for i in $({BIN}/seq {flows}); do (while true; do"
            f" {BIN}/cat /dev/zero > /dev/tcp/{FAR}/{port}; done) & done; wait'"
        )

    def app_share(self, slice_name: str = "app.slice", *, shaped: bool = True) -> float:
        """Give the app's share of what the server takes, once its upload is under way.

        The app's one upload runs in `slice_name`, an app's own unless a test says otherwise.
        While `shaped`, the share is read once the shaper gives Joel's lane, which the upload
        rides, more than SHAPED_SHARE of its rate: on 19 November one read soon after the
        upload began gave 0.47, where reads once the lane had grown gave 0.71 and over.
        The lane outlasts the upload before, so the upload starts only once it has fallen back
        under SHAPED_SHARE: a put-first read begun 0.3 s after another's stop passed the wait
        on that one's lane, and read 0.39 as the lane fell back to what it carried.
        The upload is stopped however this ends, so a failure leaves the next test the line,
        and a stop that fails after a good read fails here rather than in the next test.
        """

        def grown(last_try: bool) -> bool:
            link = self.link()
            share = self.joel_share(link)
            if last_try and share <= SHAPED_SHARE:
                print(
                    f"Joel's lane never passed {SHAPED_SHARE} of the rate, at {share:.2f}"
                    f" with the lanes {link.get('lanes')}:\n{self.trace()}"
                )
            return share > SHAPED_SHARE

        def fallen(last_try: bool) -> bool:
            link = self.link()
            share = self.joel_share(link)
            if last_try and share > SHAPED_SHARE:
                print(
                    f"Joel's lane never fell back under {SHAPED_SHARE} of the rate, at {share:.2f}"
                    f" with the lanes {link.get('lanes')}:\n{self.trace()}"
                )
            return share <= SHAPED_SHARE

        if shaped:
            retry(fallen, timeout=SENDING)
        stop = "systemctl -M alice@ --user stop app-upload.service"
        self.uploads("app-upload", slice_name, 1, APP_PORT)
        try:
            started = self.taken().get(APP_PORT, 0)
            retry(
                lambda _last_try: self.taken().get(APP_PORT, 0) - started >= APP_STARTS,
                timeout=SENDING,
            )
            if shaped:
                retry(grown, timeout=SENDING)
            before = self.taken()
            retry(
                lambda _last_try: sum(self.taken().values()) - sum(before.values()) >= WINDOW,
                timeout=SENDING,
            )
            after = self.taken()
            # The lanes as the share was read, since the trace reads them after the stop; with
            # the shaper stopped there is no status to read.
            lanes_said = f", with the lanes {self.link().get('lanes')}" if shaped else ", unshaped"
        except BaseException:
            self.client.execute(stop)
            raise
        self.client.succeed(stop)
        moved = {port: after[port] - before.get(port, 0) for port in after}
        print(f"what the server took while the app uploaded: {moved}{lanes_said}")
        return moved.get(APP_PORT, 0) / sum(moved.values())


@dataclass
class Readings:
    """What the wait for the queue read of both machines' sending, try by try, for its print.

    A run that passes says it too, so the net's runs show whether a passing rig was starved
    as well as a failing one: the count alone can't tell a rig that stopped starving from one
    on a quiet builder.
    """

    client: list[int] = field(default_factory=list)
    router: list[int] = field(default_factory=list)
    client_behind: int = 0
    router_behind: int = 0
    tries: int = 0
    restarts: int = 0

    def read(self, sent: int, up: int, waiting: int, passed: int, *, shedding: bool) -> None:
        """Keep one try's rates, in kbit/s, and whether either machine fell behind in it."""
        self.client.append(sent)
        self.router.append(passed)
        self.client_behind += int(waiting > 0 and sent < up * BEHIND)
        self.router_behind += int(shedding and passed < CONNECTION_KBIT * BEHIND)

    def said(self, seconds: float) -> str:
        """Say what the tries read, over the `seconds` the wait took."""
        began = f"after {seconds:.0f} s and {self.tries} tries, the count starting over"
        if not self.client:
            return f"{began} on {self.restarts}, with no two tries to read sending between"
        return (
            f"{began} on {self.restarts}: eth1 sent {min(self.client)} to {max(self.client)}"
            f" kbit/s a try, under {BEHIND} of the shaper's rate with data waiting on"
            f" {self.client_behind} of {len(self.client)}; the router's HTB passed"
            f" {min(self.router)} to {max(self.router)} kbit/s of its {CONNECTION_KBIT},"
            f" under {BEHIND} of it while shedding on {self.router_behind}"
        )


@pytest.fixture(scope="module")
def rig(machines: dict[str, BaseMachine]) -> Rig:
    """Give the machines once the shaper shapes the client's connection and the server counts."""
    client, router, server = machines["client"], machines["router"], machines["server"]
    for node in machines.values():
        node.start()
    server.wait_for_open_port(JOBS_PORT)
    server.wait_for_open_port(APP_PORT)
    server.succeed(
        "nft add table inet seen",
        "nft add chain inet seen input '{ type filter hook input priority 0; }'",
        f"nft add rule inet seen input tcp dport {JOBS_PORT} counter",
        f"nft add rule inet seen input tcp dport {APP_PORT} counter",
    )
    router.wait_for_unit("connection.service")
    client.wait_for_unit("user@1000.service")
    client.wait_until_succeeds("tc -j qdisc show dev eth1 | grep -q cake", timeout=COMING_UP)
    return Rig(client=client, router=router, server=server)


@pytest.fixture(scope="module")
def loaded(rig: Rig) -> Rig:
    """Give the rig with the jobs' uploads filling the router, the shaper holding under it.

    The shaper starts above the line, so the router's queue is the one that fills, and
    sheds what it can't hold. Taken back, the queue stands in the shaper's CAKE, which
    says so, and the router sheds nothing; jobs that stopped sending would leave both
    empty. The router's shedding is read, not the shaper's rate against 8 Mbit/s: HTB let
    about 8.5 through as the client counts, and on 19 November a shaper holding at
    8099 kbit/s, with the queue its own, failed that bar.

    Each try also reads what the client's queues sent and the router's HTB passed since
    the try before, so a wait that times out says whether a machine fell behind, a
    starved rig, or neither did, a shaper slow to take the queue back.
    """
    rig.uploads("uploads", "jobs.slice", JOBS, JOBS_PORT)
    # Since when the router has shed nothing more while the shaper held the queue, what it
    # had shed by then, and why the count last started over.
    began = time.monotonic()
    since, shed_then, why = began, -1, ""
    readings = Readings()
    # The last try's clocks and counts, the client's then the router's.
    last: list[tuple[float, int, float, int]] = []

    def taken_back(last_try: bool) -> bool:
        nonlocal since, shed_then, why
        router_at, passed, shed = rig.router_read()
        client_at, sent, waiting = rig.client_read()
        link = rig.link()
        up = int(link.get("up") or 0)
        heard = link.get("heard")
        ours = bool(((heard or {}).get("ours") or {}).get("queued"))
        shed_more = shed != shed_then
        if last:
            was_client_at, was_sent, was_router_at, was_passed = last[0]
            readings.read(
                round((sent - was_sent) * 8 / 1000 / (client_at - was_client_at)),
                up,
                waiting,
                round((passed - was_passed) * 8 / 1000 / (router_at - was_router_at)),
                shedding=shed_more,
            )
        last[:] = [(client_at, sent, router_at, passed)]
        readings.tries += 1
        if shed_more or not ours:
            readings.restarts += int(shed_then >= 0)
            since, shed_then = time.monotonic(), shed
            if shed == 0:
                why = (
                    "the router shed nothing, so the jobs never filled it, or the shaper"
                    " started under it"
                )
            elif shed_more:
                why = f"the router kept shedding, at {shed}, with the shaper at {up} kbit/s"
            elif not (heard or {}).get("ours"):
                why = f"the shaper said nothing of its queue, at {up} kbit/s"
            else:
                why = f"the shaper held no queue of its own, at {up} kbit/s"
        held = shed > 0 and time.monotonic() - since >= HELD.total_seconds()
        read = readings.said(time.monotonic() - began)
        if last_try and not held:
            print(
                f"{why}, the last time the count started over; {read}\n{rig.htb_behind()}"
                f"\n{rig.trace()}"
            )
        if held:
            print(
                f"the router let its queue go, at {shed} shed, with the shaper at {up} kbit/s;"
                f" {read}\n{rig.htb_behind()}"
            )
        return held

    retry(taken_back, timeout=TAKING)
    return rig


def test_an_app_outruns_the_jobs(loaded: Rig) -> None:
    """With the shaper, an app's one upload gets most of the connection beside the 32 jobs'."""
    share = loaded.app_share()
    assert share > SHAPED_SHARE, (
        f"the app got {share:.2f} of the line at {loaded.up()} kbit/s:\n{loaded.trace()}"
    )


@pytest.mark.parametrize(
    ("slice_name", "name"),
    [
        # A unit directly in jobs, at depth 5, as the deploy's scope was.
        ("jobs.slice", "app-upload.service"),
        # A quest's cell, at depth 7, with the upload a unit in it.
        ("jobs-work-delta-qest20261116abcdef.slice", "QEST-20261116-ABCDEF"),
    ],
)
def test_work_put_first_outruns_the_jobs(loaded: Rig, slice_name: str, name: str) -> None:
    """An upload in jobs that its account has put first gets most of the connection.

    Without it, the one upload is one of 33 in jobs, as a production deploy was on 16 November
    beside offload's copies and agents' turns.
    """
    folder = "/run/user/1000/shaper/first"
    loaded.client.succeed(f"su alice -c 'mkdir -p {folder} && touch {folder}/{name}'")
    try:
        share = loaded.app_share(slice_name)
    finally:
        loaded.client.succeed(f"rm -rf {folder}")
    assert share > SHAPED_SHARE, (
        f"work put first got {share:.2f} of the line at {loaded.up()} kbit/s:\n{loaded.trace()}"
    )
    # The journal names the cgroup found: the unit itself, or the quest's cell.
    found = name if name.endswith(".service") else slice_name
    said = loaded.client.succeed("journalctl -u shaper -o cat --no-pager")
    assert f"putting first, ahead of offload and agents: {found}\n" in said, (
        f"{found} not said to be first:\n{said}"
    )


def test_unshaped(loaded: Rig) -> None:
    """Without it, the router gives the app one flow's share."""
    loaded.client.succeed("systemctl stop shaper.service")
    share = loaded.app_share(shaped=False)
    assert share < UNSHAPED_SHARE, f"the app got {share:.2f} of the line without the shaper"

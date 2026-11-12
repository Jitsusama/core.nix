"""How much of a call is lost while the jobs and a page fill the connection both ways.

shaper-call.nix, beside this, runs this through pytest inside the driver (tests/lib/vm_pytest.py),
and its opening comment says what the test proves. The router is the connection, 90 Mbit/s down
and 15 up, each shared flow by flow under fq_codel. iperf3 stands in for the calls: each sends
datagrams of a video call's size and rate both ways and counts every one that never arrived,
which is the loss a call hears, wherever it was lost. CAKE's own drop counts say whether the
shaper's queues were the ones that lost them.
"""

from __future__ import annotations

import datetime as dt
import json
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any

import pytest
from test_driver.machine import retry

if TYPE_CHECKING:
    from test_driver.machine import BaseMachine

# A JSON object as tc, iperf3 or the shaper gives it, whose fields the test reads as it finds them.
type Json = dict[str, Any]

BIN = "/run/current-system/sw/bin"
FAR = "203.0.113.3"
SINK_PORT = 5201
SOURCE_PORT = 5202
# The two calls: one to a media server's own port, which goes in the video tin, and one to
# 443, which the shaper takes for a page's QUIC and sends best effort.
CALLS = {"media": 5301, "page": 443}
# CAKE's diffserv4 tins, in the order its counts list them.
TINS = ["bulk", "best effort", "video", "voice"]
# What fills the connection: the jobs' downloads and uploads, as agents' clones and pushes do,
# and an app's downloads, as a page's do.
JOBS_DOWNLOADS = 8
JOBS_UPLOADS = 4
APP_DOWNLOADS = 2
# A call's video each way: datagrams of 1,200 bytes at 2.5 Mbit/s, as a Meet call sends at
# its sharpest, for long enough that a drop every few seconds would show.
CALL_RATE = "2500K"
CALL_LENGTH = 1200
CALL_SECONDS = 20
# The least of a direction's datagrams that must have been sent for a call to count: most of
# what the rate sends in its seconds.
CALL_LEAST = 2_500_000 * CALL_SECONDS // (CALL_LENGTH * 8) * 9 // 10
# The most a shaped call may lose coming down, of that least: the router alone loses none,
# and a CAKE that limits the download drops a few as it takes the queue home. Going up is
# only printed: the router VM dropped from 94 to 1,296 of an unshaped call's datagrams from
# run to run, and dropped a shaped one's while the client sent 10 Mbit/s into its 15, which
# is the builder's CPU and no queue of the shaper's.
DOWN_MOST = CALL_LEAST // 100
# What the client takes in before the calls start: a few seconds of the line, so every
# flow is past its slow start.
FULL_BYTES = 50_000_000
# How long the client may take to boot to its first queues, and the loads to take that in.
COMING_UP = dt.timedelta(seconds=30)
FILLING = dt.timedelta(seconds=60)
# How long a call alone runs to show which tin it goes in, and how long the shaper may take to
# hear that a call started, which it asks every 2 s.
ALONE_SECONDS = 5
HEARING = dt.timedelta(seconds=10)
# The file the client's stand-in for a-call-is-on reads: a call is on while it's there.
CALL_FLAG = "/run/call-is-on"


@dataclass
class Rig:
    """The client, the router that is its connection, and the server past it."""

    client: BaseMachine
    router: BaseMachine

    def cake(self, dev: str) -> Json:
        """Give the CAKE on one of the client's devices, with its counts."""
        queues: list[Json] = json.loads(self.client.succeed(f"tc -s -j qdisc show dev {dev}"))
        return next(q for q in queues if q["kind"] == "cake")

    def counts(self, dev: str, what: str) -> list[int]:
        """Give one of CAKE's counts on a device, a value per tin."""
        return [int(tin[what]) for tin in self.cake(dev)["tins"]]

    def ifb(self) -> str:
        """Give the name of the IFB that carries eth1's downloads."""
        index = int(self.client.succeed("cat /sys/class/net/eth1/ifindex"))
        return f"shaper{index:x}"

    def holding(self) -> bool:
        """Give whether the controller holds eth1 under a queue it took back."""
        links: list[Json] = json.loads(self.client.succeed("cat /run/shaper/status.json"))
        return any(link["name"] == "eth1" and link["state"] == "holding" for link in links)

    def start(self, unit: str, slice_name: str, flows: int, script: str) -> None:
        """Start flows of a script in a slice, each again as it ends, until they're stopped."""
        self.client.succeed(
            f"systemd-run -M alice@ --user --slice={slice_name} --unit={unit}"
            f" {BIN}/bash -c 'for i in $({BIN}/seq {flows}); do (while true; do {script};"
            " done) & done; wait'"
        )

    def stop(self, *units: str) -> None:
        """Stop units the loads run in."""
        for unit in units:
            self.client.succeed(f"systemctl -M alice@ --user stop {unit}.service")

    def calls(self, names: list[str] | None = None, seconds: int = CALL_SECONDS) -> dict[str, Json]:
        """Hold calls at once from an app, both unless named, and give what iperf3 made of each."""
        held = names or list(CALLS)
        each = " ".join(
            f"{BIN}/iperf3 --client {FAR} --port {CALLS[name]} --udp --bidir --bitrate {CALL_RATE}"
            f" --length {CALL_LENGTH} --time {seconds} --json > /tmp/call-{name}.json &"
            for name in held
        )
        self.client.succeed(
            "systemd-run -M alice@ --user --wait --collect --slice=app.slice"
            f" {BIN}/bash -c '{each} wait'"
        )
        return {
            name: json.loads(self.client.succeed(f"cat /tmp/call-{name}.json")) for name in held
        }

    def seconds(self) -> str:
        """Give what the recorder saw each second, a line each.

        Each second's upload and download, in kbit/s, from the link's counters, and the
        medians of the delays out and back, in ms, as the controller reads them before
        taking the reflector's least off.
        """
        text = self.client.succeed("cat /var/lib/shaper-record/*.jsonl")
        seconds: dict[int, list[Json]] = {}
        for line in text.splitlines():
            sample: Json = json.loads(line)
            if "tx" in sample:
                seconds.setdefault(sample["at"] // 1000, []).append(sample)
        rows = []
        last: tuple[int, int] | None = None
        for second, samples in sorted(seconds.items()):
            sent = max(s["tx"] for s in samples)
            got = max(s["rx"] for s in samples)
            was = last or (sent, got)
            up, down = (sent - was[0]) * 8 // 1000, (got - was[1]) * 8 // 1000
            last = (sent, got)
            out = sorted(s["out"] for s in samples if "out" in s)
            back = sorted(s["back"] for s in samples if "back" in s)
            rows.append(
                f"{second % 1000:03d} up {up:6d} down {down:6d} kbit/s,"
                f" out {out[len(out) // 2] if out else None}"
                f" back {back[len(back) // 2] if back else None}"
            )
        return "\n".join(rows)

    def trace(self) -> str:
        """Give what the shaper said, held and heard, and every queue's counts, for a failure."""
        return "\n".join(
            [
                self.seconds(),
                self.client.succeed("journalctl -u shaper -o cat --no-pager"),
                self.client.succeed("cat /run/shaper/status.json"),
                self.client.succeed(
                    f"tc -s qdisc show dev eth1; tc -s qdisc show dev {self.ifb()}"
                ),
                self.client.succeed("nstat -az Udp* Ip*Discards; cat /proc/net/softnet_stat"),
                self.router.succeed("tc -s qdisc show dev eth1; tc -s qdisc show dev eth2"),
            ]
        )


@pytest.fixture(scope="module")
def rig(machines: dict[str, BaseMachine]) -> Rig:
    """Give the machines once the shaper shapes the client's connection both ways."""
    client, router, server = machines["client"], machines["router"], machines["server"]
    for node in machines.values():
        node.start()
    for port in [SINK_PORT, SOURCE_PORT, *CALLS.values()]:
        server.wait_for_open_port(port)
    router.wait_for_unit("connection.service")
    client.wait_for_unit("user@1000.service")
    client.wait_until_succeeds("tc -j qdisc show dev eth1 | grep -q cake", timeout=COMING_UP)
    held = Rig(client=client, router=router)
    client.wait_until_succeeds(
        f"tc -j qdisc show dev {held.ifb()} | grep -q cake", timeout=COMING_UP
    )
    return held


def received(result: Json, way: str) -> Json:
    """Give what a call's receiving end counted one way: `up` to the server, `down` back."""
    end: Json = result.get("end", {})
    summary: Json = end.get("sum_received" if way == "up" else "sum_received_bidir_reverse", {})
    return summary


def under_load(rig: Rig, *, shaped: bool) -> dict[str, dict[str, int]]:
    """Hold both calls while the jobs and an app fill the connection, and give what each lost.

    The loads run until the client has taken in a few seconds of the line, so the calls cross
    a connection that's full, as a busy afternoon's is, rather than one with room to spare.
    Shaped, they run on until the controller holds the link too, as it has all day on a
    connection the jobs keep full: a call that starts in the seconds before it does crosses
    the router's queue as an unshaped one would.
    """
    download = f"{BIN}/cat < /dev/tcp/{FAR}/{SOURCE_PORT} > /dev/null"
    upload = f"{BIN}/cat /dev/zero > /dev/tcp/{FAR}/{SINK_PORT}"
    rx = "cat /sys/class/net/eth1/statistics/rx_bytes"
    first = int(rig.client.succeed(rx))
    rig.start("jobs-downloads", "jobs.slice", JOBS_DOWNLOADS, download)
    rig.start("jobs-uploads", "jobs.slice", JOBS_UPLOADS, upload)
    rig.start("app-downloads", "app.slice", APP_DOWNLOADS, download)
    retry(lambda _: int(rig.client.succeed(rx)) - first >= FULL_BYTES, timeout=FILLING)
    if shaped:
        retry(lambda _: rig.holding(), timeout=FILLING)
    results = rig.calls()
    rig.stop("jobs-downloads", "jobs-uploads", "app-downloads")
    lost: dict[str, dict[str, int]] = {}
    for name, result in results.items():
        assert "error" not in result, f"the {name} call failed: {result['error']}"
        lost[name] = {}
        for way in ["up", "down"]:
            summary = received(result, way)
            assert summary.get("packets", 0) >= CALL_LEAST, (
                f"the {name} call took {summary.get('packets')} datagrams {way}, under {CALL_LEAST}"
            )
            lost[name][way] = int(summary["lost_packets"])
    print(f"lost by call and way: {lost}")
    return lost


def test_a_call_to_443_goes_as_video_while_one_is_on(rig: Rig) -> None:
    """While a call is on, the call an app sends to 443 goes in the video tin, seen as a call.

    Meet sends calls to Google's front ends on 443 too, where only a call being on tells one
    from a page's QUIC: on 12 November one went as best effort while the jobs filled the
    upload, and its picture went blocky.
    """
    rig.client.succeed(f"touch {CALL_FLAG}")
    rig.client.wait_until_succeeds(
        "journalctl -u shaper -o cat | grep -q 'a call is on'", timeout=HEARING
    )
    before = rig.counts("eth1", "sent_packets")
    result = rig.calls(["page"], seconds=ALONE_SECONDS)["page"]
    after = rig.counts("eth1", "sent_packets")
    rig.client.succeed(f"rm {CALL_FLAG}")
    sent = dict(zip(TINS, (a - b for a, b in zip(after, before, strict=True)), strict=True))
    call = int(received(result, "up").get("packets", 0))
    assert call > 0, f"the call sent nothing up: {result}"
    assert sent["video"] >= call, (
        f"the call sent {call} datagrams up, but the video tin only {sent['video']}: {sent}"
    )
    journal = rig.client.succeed("journalctl -u shaper -o cat --no-pager")
    assert "real-time traffic flows" in journal, f"the call wasn't seen as one:\n{journal}"
    rig.client.wait_until_succeeds(
        "journalctl -u shaper -o cat | grep -q 'no call is on'", timeout=HEARING
    )


def test_a_call_loses_little(rig: Rig) -> None:
    """Under a full connection both ways, each call loses at most a hundredth coming down.

    While the controller sets no download limit, downloads go straight in, past the IFB's
    CAKE: at the link's ceiling it dropped four in five of the best-effort call's datagrams.
    Once it limits the download, CAKE drops a few of the calls' datagrams as it brings the
    queue home.
    """
    ifb = rig.ifb()
    before = {dev: rig.counts(dev, "drops") for dev in ["eth1", ifb]}
    lost = under_load(rig, shaped=True)
    after = {dev: rig.counts(dev, "drops") for dev in ["eth1", ifb]}
    dropped = {
        dev: dict(
            zip(TINS, (a - b for a, b in zip(after[dev], before[dev], strict=True)), strict=True)
        )
        for dev in after
    }
    print(f"dropped by tin meanwhile: {dropped}")
    if any(ways["down"] > DOWN_MOST for ways in lost.values()):
        trace = rig.trace()
        for name, ways in lost.items():
            assert ways["down"] <= DOWN_MOST, (
                f"the {name} call lost {ways['down']} datagrams down, over {DOWN_MOST};"
                f" dropped by tin: {dropped}\n{trace}"
            )


def test_unshaped(rig: Rig) -> None:
    """Without the shaper, the same calls under the same loads lose nothing coming down.

    So the router alone keeps a call's download whole, and what a shaped call loses coming
    down is the shaper's.
    """
    rig.client.succeed("systemctl stop shaper.service")
    lost = under_load(rig, shaped=False)
    for name, ways in lost.items():
        assert ways["down"] == 0, f"the {name} call lost {ways['down']} datagrams down unshaped"

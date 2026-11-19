"""How much of a call is lost while the jobs and a page fill the connection both ways.

shaper-call.nix, beside this, runs this through pytest inside the driver (tests/lib/vm_pytest.py),
and its opening comment says what the test proves. The router is the connection, 90 Mbit/s down
and 15 up, each shared flow by flow under fq_codel. iperf3 stands in for the calls: each sends
datagrams of a video call's size and rate both ways and counts every one that never arrived,
which is the loss a call hears, wherever it was lost. CAKE's own drop counts say whether the
shaper's queues were the ones that lost them.
"""

from __future__ import annotations

import bisect
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
# The most a call may lose coming down, of that least, shaped or not: a CAKE that limits the
# download drops a few as it takes the queue home, and on 13 November the router alone lost
# 4, then 6, on a busy builder, and none on a rerun. Going up is only printed: the router VM
# dropped from 94 to 1,296 of an unshaped call's datagrams from run to run, and dropped a
# shaped one's while the client sent 10 Mbit/s into its 15, which is the builder's CPU and
# no queue of the shaper's.
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
# How many seconds the shaper's last traced ask stands over seconds its trace missed, as a
# replay takes it (STANDS_S in src/trace.rs): a step that drains two seconds writes one line.
STANDS = 5
# The file the client's stand-in for a-call-is-on reads: a call is on while it's there.
CALL_FLAG = "/run/call-is-on"
# What systemctl exits with when it's asked to stop a unit that isn't loaded.
NOT_LOADED = 5


@dataclass
class Rig:
    """The client, the router that is its connection, and the server past it."""

    client: BaseMachine
    router: BaseMachine
    server: BaseMachine
    # Every machine's counts as the loads that are on started, for a failure's trace.
    since: str | None = None

    def cake(self, dev: str) -> Json:
        """Give the CAKE on one of the client's devices, with its counts: on a link, Joel's."""
        queues: list[Json] = json.loads(self.client.succeed(f"tc -s -j qdisc show dev {dev}"))
        return next(q for q in queues if q["kind"] == "cake" and q["handle"] == "2:")

    def counts(self, dev: str, what: str) -> list[int]:
        """Give one of CAKE's counts on a device, a value per tin."""
        return [int(tin[what]) for tin in self.cake(dev)["tins"]]

    def ifb(self) -> str:
        """Give the name of the IFB that carries eth1's downloads."""
        index = int(self.client.succeed("cat /sys/class/net/eth1/ifindex"))
        return f"shaper{index:x}"

    def holding(self) -> bool:
        """Give whether the controller holds eth1 under a queue it took back.

        A shaper with no status, gone or starting again, holds nothing.
        """
        status, text = self.client.execute("cat /run/shaper/status.json")
        if status:
            return False
        links: list[Json] = json.loads(text)
        return any(link["name"] == "eth1" and link["state"] == "holding" for link in links)

    def start(self, unit: str, slice_name: str, flows: int, script: str) -> None:
        """Start flows of a script in a slice, each again as it ends, until they're stopped."""
        self.client.succeed(
            f"systemd-run -M alice@ --user --slice={slice_name} --unit={unit}"
            f" {BIN}/bash -c 'for i in $({BIN}/seq {flows}); do (while true; do {script};"
            " done) & done; wait'"
        )

    def stop(self, *units: str) -> list[str]:
        """Stop units the loads run in, each tried, and give those that wouldn't stop.

        It fails nothing itself, so a stop after a failure never takes its place. A unit that
        isn't loaded, never started or already gone, is stopped as far as the next start goes.
        """
        stuck = []
        for unit in units:
            status, reply = self.client.execute(
                f"systemctl -M alice@ --user stop {unit}.service 2>&1"
            )
            if status not in (0, NOT_LOADED):
                print(f"the loads in {unit} wouldn't stop: {reply.strip()}")
                stuck.append(unit)
            # A unit that failed stays loaded, stopped or not, and takes its name with it.
            self.client.execute(f"systemctl -M alice@ --user reset-failed {unit}.service")
        return stuck

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

    def asked(self) -> dict[int, str]:
        """Give what the controller asked each way of eth1 be held to, by the second it stepped on.

        From the shaper's own trace, before its floor and the link's most; a way it asked
        no limit of reads as no limit. A line still writing is passed over.
        """
        _, text = self.client.execute("cat /var/lib/shaper/heard/*.jsonl 2>/dev/null")
        asked: dict[int, str] = {}
        for line in text.splitlines():
            try:
                step: Json = json.loads(line)
            except json.JSONDecodeError:
                continue
            if isinstance(step, dict) and step.get("link") == "eth1" and "at" in step:
                up, down = step.get("up", "no limit"), step.get("down", "no limit")
                asked[step["at"]] = f", asked up {up}, down {down}"
        return asked

    @staticmethod
    def held(asked: dict[int, str], seconds: list[int], running: bool) -> dict[int, str]:
        """Give the ask that held each recorded second: the one made after the second before.

        The shaper steps on a second once it's recorded, so what it asked then holds from the
        next, as a replay scores it. A second whose one before has no line takes the last line's
        ask for up to STANDS seconds, marked as standing, and reads as not traced past that, or
        past the trace's last line once the shaper isn't `running`, since nothing held it then.
        The last line is the trace's newest before that second, whether or not the recorder kept
        the second after it.
        """
        held: dict[int, str] = {}
        traced = sorted(asked)
        ended = None if running else traced[-1]
        for second in seconds:
            before = bisect.bisect_right(traced, second - 1)
            last = traced[before - 1] if before else None
            if last == second - 1:
                held[second] = asked[last]
            elif (
                last is not None
                and second - 1 - last <= STANDS
                and (ended is None or second - 1 < ended)
            ):
                held[second] = f"{asked[last]} (standing)"
            else:
                held[second] = ", not traced"
        return held

    def seconds(self) -> str:
        """Give what the recorder saw each second, a line each.

        Each second's upload and download, in kbit/s, from the link's counters, and the
        medians of the delays out and back, in ms, as the controller reads them before
        taking the reflector's least off; then the ask that held it (`held`). A trace with no
        line for eth1, or none that held a recorded second, says so. The recorder is read
        first, so the trace has stepped on all but its newest second or two.
        """
        status, text = self.client.execute("cat /var/lib/shaper-record/*.jsonl 2>&1")
        if status:
            return f"the recorder's samples couldn't be read: {text.strip()}"
        asked = self.asked()
        seconds: dict[int, list[Json]] = {}
        for line in text.splitlines():
            try:
                sample: Json = json.loads(line)
            except json.JSONDecodeError:
                # The recorder keeps writing while a trace reads, so its last line may be half.
                continue
            if "tx" in sample:
                seconds.setdefault(sample["at"] // 1000, []).append(sample)
        running = self.client.execute("systemctl is-active --quiet shaper.service")[0] == 0
        held = self.held(asked, sorted(seconds), running) if asked else {}
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
                f"{held.get(second, '')}"
            )
        if not asked:
            rows.append("no shaper trace for eth1 under /var/lib/shaper/heard")
        elif not any(second - 1 in asked for second in seconds):
            rows.append(f"none of the shaper's {len(asked)} traced seconds held a recorded one")
        return "\n".join(rows)

    def counted(self) -> str:
        """Give every machine's counts of what it queued, dropped and lost, since it booted.

        They count every test's traffic, so a failure that needs this one's alone takes them
        before as well as after. Each machine's links give what it sent and took in, so what
        one sent and the next never took in is the host's, between them; nstat gives only
        what a machine itself sent or took, never what the router forwards. Read while the
        loads run, as a wait's last try reads them, a gap is also what's still in flight or
        queued, so it counts as loss only once the loads have stopped.
        """
        gaps = "ip -s link; nstat -az Udp* Ip*Discards; cat /proc/net/softnet_stat"
        return "\n".join(
            [
                said(self.client, "tc -s qdisc show dev eth1"),
                said(self.client, f"tc -s qdisc show dev {self.ifb()}"),
                said(self.client, gaps),
                said(self.router, "tc -s qdisc show dev eth1; tc -s qdisc show dev eth2"),
                said(self.router, gaps),
                said(self.server, gaps),
            ]
        )

    def trace(self) -> str:
        """Give what the shaper said, held and heard, and every count, for a failure.

        Each part that can't be read says so rather than raising, since a failure's trace is
        taken when things are wrong: with the shaper stopped, its status and IFB are gone.
        The counts from when the loads started come beside those now, so they tell this
        test's losses from every earlier test's.
        """
        parts = [
            self.seconds(),
            said(self.client, "journalctl -u shaper -o cat --no-pager"),
            said(self.client, "cat /run/shaper/status.json"),
        ]
        if self.since is not None:
            parts.append(f"counted as the loads started:\n{self.since}\ncounted now:")
        parts.append(self.counted())
        return "\n".join(parts)


def said(machine: BaseMachine, command: str) -> str:
    """Give what a command printed, or that it failed and what it said, for a trace.

    Each opens with the machine's name, since every machine's links print alike.
    """
    status, output = machine.execute(f"( {command} ) 2>&1")
    said = output if status == 0 else f"failed ({status}): {output.strip()}"
    return f"{machine.name}: {command}\n{said}"


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
    held = Rig(client=client, router=router, server=server)
    client.wait_until_succeeds(
        f"tc -j qdisc show dev {held.ifb()} | grep -q cake", timeout=COMING_UP
    )
    return held


def received(result: Json, way: str) -> Json:
    """Give what a call's receiving end counted one way: `up` to the server, `down` back."""
    end: Json = result.get("end", {})
    summary: Json = end.get("sum_received" if way == "up" else "sum_received_bidir_reverse", {})
    return summary


def met(rig: Rig, *, now: bool, last_try: bool, otherwise: str) -> bool:
    """Give whether a wait is met, saying otherwise with the trace on its last try."""
    if last_try and not now:
        print(f"{otherwise}:\n{rig.trace()}")
    return now


def under_load(rig: Rig, *, shaped: bool) -> dict[str, dict[str, int]]:
    """Hold both calls while the jobs and an app fill the connection, and give what each lost.

    The loads run until the client has taken in a few seconds of the line, so the calls cross
    a connection that's full, as a busy afternoon's is, rather than one with room to spare.
    Shaped, they run on until the controller holds the link too, as it has all day on a
    connection the jobs keep full: a call that starts in the seconds before it does crosses
    the router's queue as an unshaped one would. The loads stop however the calls end, so a
    test that fails here leaves the next its own loads to start.
    """
    download = f"{BIN}/cat < /dev/tcp/{FAR}/{SOURCE_PORT} > /dev/null"
    upload = f"{BIN}/cat /dev/zero > /dev/tcp/{FAR}/{SINK_PORT}"
    rx = "cat /sys/class/net/eth1/statistics/rx_bytes"
    first = int(rig.client.succeed(rx))
    rig.since = rig.counted()
    loads = [
        ("jobs-downloads", "jobs.slice", JOBS_DOWNLOADS, download),
        ("jobs-uploads", "jobs.slice", JOBS_UPLOADS, upload),
        ("app-downloads", "app.slice", APP_DOWNLOADS, download),
    ]
    try:
        for unit, slice_name, flows, script in loads:
            rig.start(unit, slice_name, flows, script)
        retry(
            lambda last_try: met(
                rig,
                now=int(rig.client.succeed(rx)) - first >= FULL_BYTES,
                last_try=last_try,
                otherwise="the loads never filled the line",
            ),
            timeout=FILLING,
        )
        if shaped:
            retry(
                lambda last_try: met(
                    rig,
                    now=rig.holding(),
                    last_try=last_try,
                    otherwise="the controller never held the link",
                ),
                timeout=FILLING,
            )
        results = rig.calls()
    finally:
        # Every load, started or not: a start can fail after making its unit.
        stuck = rig.stop(*(unit for unit, _, _, _ in loads))
    lost: dict[str, dict[str, int]] = {}
    for name, result in results.items():
        # The messages are made only on a failure, so the trace is taken only then.
        assert "error" not in result, f"the {name} call failed: {result['error']}\n{rig.trace()}"
        lost[name] = {}
        for way in ["up", "down"]:
            summary = received(result, way)
            assert summary.get("packets", 0) >= CALL_LEAST, (
                f"the {name} call took {summary.get('packets')} datagrams {way},"
                f" under {CALL_LEAST}\n{rig.trace()}"
            )
            lost[name][way] = int(summary["lost_packets"])
    print(f"lost by call and way: {lost}")
    # Last, so a load that wouldn't stop never takes the place of what the calls met.
    assert not stuck, f"the loads in {stuck} wouldn't stop"
    return lost


def test_a_call_to_443_goes_as_video_while_one_is_on(rig: Rig) -> None:
    """While a call is on, the call an app sends to 443 goes in the video tin, seen as a call.

    Meet sends calls to Google's front ends on 443 too, where only a call being on tells one
    from a page's QUIC: on 12 November one went as best effort while the jobs filled the
    upload, and its picture went blocky.
    """
    try:
        rig.client.succeed(f"touch {CALL_FLAG}")
        rig.client.wait_until_succeeds(
            "journalctl -u shaper -o cat | grep -q 'a call is on'", timeout=HEARING
        )
        before = rig.counts("eth1", "sent_packets")
        result = rig.calls(["page"], seconds=ALONE_SECONDS)["page"]
        after = rig.counts("eth1", "sent_packets")
    finally:
        # A call left on would put the next tests' calls in the video tin by the flag alone.
        rig.client.succeed(f"rm -f {CALL_FLAG}")
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
    """Without the shaper, the same calls under the same loads meet the same bar coming down.

    So the bar is one the router alone meets, and what a shaped call loses past it is the
    shaper's.
    """
    rig.client.succeed("systemctl stop shaper.service")
    lost = under_load(rig, shaped=False)
    if any(ways["down"] > DOWN_MOST for ways in lost.values()):
        # Unshaped, only the router's queues and the machines' own gaps are left to lose
        # them, which their counts as the loads started and now tell apart.
        trace = rig.trace()
        for name, ways in lost.items():
            assert ways["down"] <= DOWN_MOST, (
                f"the {name} call lost {ways['down']} datagrams down unshaped,"
                f" over {DOWN_MOST}\n{trace}"
            )

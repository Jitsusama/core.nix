"""How much of a call is lost while the jobs and a page fill the connection both ways.

shaper-call.nix, beside this, runs this through pytest inside the driver (tests/lib/vm_pytest.py),
and its opening comment says what the test proves. The router is the connection, 90 Mbit/s down
and 15 up, each shared flow by flow under fq_codel. iperf3 stands in for the calls: each sends
datagrams of a video call's size and rate both ways. Every machine on a call's path counts its
datagrams as it meets them (shaper-call-path.nft), so what a call lost is placed: between the
machines, in the router's queues or the shaper's, or in the receiving socket, which a reader
starved of the builder's CPU overflows. CAKE's own drop counts say whether the shaper's queues
were the ones that lost them.
"""

from __future__ import annotations

import bisect
import datetime as dt
import json
import time
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
# An app's own uploads beside its call, a file's over TCP, which go best effort in Joel's lane
# rather than the agents'.
APP_UPLOADS = 2
# The upload a call goes ahead of its app's uploads under, held there by a trial's ceiling:
# with agents' quarter taken it leaves Joel's lane 4.5 Mbit/s, and a third of that, what
# turns by weight give video, is under the call's 2.5.
HELD_KBIT = 6_000
TRIAL = "/run/shaper-trial/upload.json"
# A call's video each way: datagrams of 1,200 bytes at 2.5 Mbit/s, as a Meet call sends at
# its sharpest, for long enough that a drop every few seconds would show.
CALL_RATE = "2500K"
CALL_LENGTH = 1200
CALL_SECONDS = 20
# The room each call's sockets ask for, both ends, about seven seconds of its datagrams, within
# what shaper-call.nix lets a socket have: a call app keeps up with its datagrams, where iperf3
# on a busy builder may not.
WINDOW = "2M"
# The machines on a call's path each way, in the order its datagrams meet them.
WAYS = {"up": ["client", "router", "server"], "down": ["server", "router", "client"]}
# The least of a direction's datagrams that must have been sent for a call to count: most of
# what the rate sends in its seconds.
CALL_LEAST = 2_500_000 * CALL_SECONDS // (CALL_LENGTH * 8) * 9 // 10
# The most a call may lose coming down before it reaches the client's socket, of that least,
# shaped or not: a CAKE that limits the download drops a few as it takes the queue home, and
# on 13 November the router alone lost 4, then 6, on a busy builder, and none on a rerun.
# What overflowed the client's socket is the builder's and only printed, as is what a call
# loses going up:
# the router VM dropped from 94 to 1,296 of an unshaped call's datagrams from run to run,
# and dropped a shaped one's while the client sent 10 Mbit/s into its 15, which is the
# builder's CPU and no queue of the shaper's.
DOWN_MOST = CALL_LEAST // 100
# The most of a call the shaper's own CAKE may drop going up, the same hundredth: unlike the
# router VM's, those drops are the shaper's queue alone.
UP_DROPPED_MOST = DOWN_MOST
# How many times the calls are made in all when a call's handshake goes unanswered. iperf3
# sends a stream's connect datagram once and gives up after 30 s with no reply, where a call
# asks again, so one datagram lost fails the whole call: on 19 November the page call did,
# after 31 s, with nothing limited down. A connect the server never heard was lost going up,
# where the router VM drops some of a call's datagrams and the shaper's upload queue may too.
# Only test_a_call_goes_ahead_of_its_own_uploads judges the shaper's, in the video tin, and its
# counts span every time the calls are made, so a connect the shaper dropped still counts
# there. Any other may have been lost in the shaper's IFB, so it's excused only when the IFB
# dropped nothing meanwhile.
HANDSHAKES = 2
# What iperf3 says when a stream's handshake went unanswered, before the call's first second.
# It sends a stream's connect once and takes the first datagram back for the reply
# (iperf_udp_connect in 3.21's iperf_udp.c). With none in 30 s the error ends "Resource
# temporarily unavailable". But the server starts the call once it hears every connect, so a
# reply lost coming down leaves the call's own data to come first, and the error ends with
# whatever errno an earlier call left: "No such file or directory" on 22 November, the server
# having heard both connects. Either is a handshake lost, which the counters read as lost
# going up when the server heard no connect, and coming down otherwise.
UNANSWERED = "unable to read from stream socket: "
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

    def upload_rates(self) -> tuple[int, int | None]:
        """Give the rate the upload's classes are held to, and Joel's CAKE's, in kbit/s.

        tc gives both in bytes a second, and a CAKE with no rate of its own as unlimited,
        which reads here as none.
        """
        classes = json.loads(self.client.succeed("tc -j class show dev eth1"))
        qdiscs = json.loads(self.client.succeed("tc -j qdisc show dev eth1"))
        held = next(c["rate"] for c in classes if c["handle"] == "1:2")
        joel = next(q["options"]["bandwidth"] for q in qdiscs if q["handle"] == "2:")
        kbit = int(joel) * 8 // 1000 if isinstance(joel, int) else None
        return int(held) * 8 // 1000, kbit

    def least_up(self, since: int) -> int | None:
        """Give the least the controller asked eth1's upload be held to from a second on.

        From its trace, in kbit/s, so a cut that climbed back before it's read shows too; none
        if it asked no limit then.
        """
        _, text = self.client.execute("cat /var/lib/shaper/heard/*.jsonl 2>/dev/null")
        asks: list[int] = []
        for line in text.splitlines():
            try:
                step: Json = json.loads(line)
            except json.JSONDecodeError:
                continue
            if (
                isinstance(step, dict)
                and step.get("link") == "eth1"
                and step.get("at", 0) >= since
                and isinstance(step.get("up"), int)
            ):
                asks.append(step["up"])
        return min(asks, default=None)

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
        """Hold calls at once from an app, both unless named, and give what iperf3 made of each.

        When a call's handshake goes unanswered and its loss is excused (lost going up, or
        anywhere while the IFB dropped nothing), and no call failed any other way, every call
        is made again, so the calls that count ran together, up to HANDSHAKES times in all. A
        call whose handshake is never answered still fails, saying how many times it was made
        and what was heard.
        """
        held = names or list(CALLS)
        for made in range(1, HANDSHAKES + 1):
            before = self.downs()
            connects, replies = self.handshakes(self.server), self.handshakes(self.client)
            walked = self.path()
            results = self.hold(held, seconds)
            now = self.path()
            for name in held:
                results[name]["path"] = {
                    way: [a - b for a, b in zip(now[name][way], walked[name][way], strict=True)]
                    for way in WAYS
                }
            dropped = sum(self.downs()) - sum(before)
            heard = {name: n - connects[name] for name, n in self.handshakes(self.server).items()}
            came = {name: n - replies[name] for name, n in self.handshakes(self.client).items()}
            lost = [name for name in held if unanswered(results[name])]
            failed = [name for name in held if "error" in results[name] and name not in lost]
            # iperf3 connects a call's streams one after another and stops at the first left
            # unanswered, so a reply came for every connect heard only when the failing one
            # never reached the server: lost going up, judged only where a subtest counts the
            # shaper's upload drops over every try.
            down = [name for name in lost if heard[name] > came[name]]
            # A call that ran in a try made again would go unjudged, so one past the bar keeps it.
            over = over_the_bar(results, [name for name in held if name not in lost])
            if not lost or failed or over or (down and dropped) or made == HANDSHAKES:
                break
            print(
                f"the {', '.join(lost)} call's handshake went unanswered, its server hearing"
                f" {[heard[name] for name in lost]} connects and the client getting"
                f" {[came[name] for name in lost]} replies, with the IFB dropping {dropped},"
                " so the calls are made again"
            )
        for name in lost:
            results[name]["error"] += (
                f" (made {made} of {HANDSHAKES} times, its server hearing {heard[name]} connects"
                f" and the client getting {came[name]} replies, with the IFB dropping {dropped},"
                f" {stopped(made, failed, over, down)})"
            )
        return results

    def handshakes(self, machine: BaseMachine) -> dict[str, int]:
        """Give how many call handshake datagrams a machine has received, by call."""
        return {name: counted(machine, "handshakes", name) for name in CALLS}

    def path(self) -> dict[str, dict[str, list[int]]]:
        """Give how many of each call's datagrams each machine on its path has met, each way.

        In the order they meet them (WAYS): up, what the client sent, the router passed and the
        server heard; down, what the server sent, the router passed and the client heard.
        """
        machines = {"client": self.client, "router": self.router, "server": self.server}
        return {
            name: {
                way: [counted(machines[m], "path", f"{name}_{way}") for m in order]
                for way, order in WAYS.items()
            }
            for name in CALLS
        }

    def downs(self) -> list[int]:
        """Give the IFB's CAKE drops by tin, or none while it has no CAKE, as with no shaper."""
        status, text = self.client.execute(f"tc -s -j qdisc show dev {self.ifb()}")
        queues: list[Json] = json.loads(text) if status == 0 and text.strip() else []
        cake = next((q for q in queues if q["kind"] == "cake" and q["handle"] == "2:"), None)
        return [int(tin["drops"]) for tin in cake["tins"]] if cake else []

    def hold(self, held: list[str], seconds: int) -> dict[str, Json]:
        """Hold the named calls at once from an app, and give what iperf3 made of each."""
        each = " ".join(
            f"{BIN}/iperf3 --client {FAR} --port {CALLS[name]} --udp --bidir --bitrate {CALL_RATE}"
            f" --length {CALL_LENGTH} --window {WINDOW} --time {seconds} --json"
            f" > /tmp/call-{name}.json &"
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


def counted(machine: BaseMachine, table: str, name: str) -> int:
    """Give how many packets one of a machine's nft counters has counted."""
    listed: Json = json.loads(machine.succeed(f"nft -j list counter inet {table} {name}"))
    return next(int(item["counter"]["packets"]) for item in listed["nftables"] if "counter" in item)


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


def under_load(
    rig: Rig, *, shaped: bool, app_uploads: int = 0, names: list[str] | None = None
) -> dict[str, dict[str, int]]:
    """Hold the calls named, both unless named, while the jobs and an app fill the connection.

    What each call lost each way before its receiving socket is given, as the machines on its
    path counted it, and where each was met is printed. With app_uploads, the app uploads too,
    filling Joel's lane rather than the agents'.

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
    if app_uploads:
        loads.append(("app-uploads", "app.slice", app_uploads, upload))
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
        results = rig.calls(names)
    finally:
        # Every load, started or not: a start can fail after making its unit.
        stuck = rig.stop(*(unit for unit, _, _, _ in loads))
    lost: dict[str, dict[str, int]] = {}
    for name, result in results.items():
        # The messages are made only on a failure, so the trace is taken only then.
        assert "error" not in result, f"the {name} call failed: {result['error']}\n{rig.trace()}"
        lost[name] = {}
        for way, order in WAYS.items():
            summary = received(result, way)
            assert summary.get("packets", 0) >= CALL_LEAST, (
                f"the {name} call took {summary.get('packets')} datagrams {way},"
                f" under {CALL_LEAST}\n{rig.trace()}"
            )
            seen = result["path"][way]
            read = int(summary["packets"]) - int(summary["lost_packets"])
            steps = ", ".join(f"the {m} {n}" for m, n in zip(order, seen, strict=True))
            print(f"the {name} call's datagrams {way}, as met: {steps}, iperf3 {read}")
            # Every datagram iperf3 read passed each machine, so a count under it is a
            # counter that missed the call, which would read as nothing lost.
            assert min(seen) >= read, (
                f"a machine counted fewer of the {name} call's datagrams {way} than iperf3"
                f" read, so its counter missed them: {steps}, iperf3 {read}"
            )
            lost[name][way] = seen[0] - seen[-1]
    print(f"lost before the receiving socket, by call and way: {lost}")
    # Last, so a load that wouldn't stop never takes the place of what the calls met.
    assert not stuck, f"the loads in {stuck} wouldn't stop"
    return lost


def unanswered(result: Json) -> bool:
    """Give whether iperf3 gave up on a call's handshake, before it sent a second of it."""
    return not result.get("intervals") and str(result.get("error", "")).startswith(UNANSWERED)


def over_the_bar(results: dict[str, Json], ran: list[str]) -> dict[str, int]:
    """Give what each call that ran lost coming down, of those that lost more than the bar.

    What was lost before the client's socket, as the bar counts it: the server sent it and
    the client never heard it.
    """
    lost = {
        name: results[name]["path"]["down"][0] - results[name]["path"]["down"][-1]
        for name in ran
        if "error" not in results[name]
    }
    return {name: n for name, n in lost.items() if n > DOWN_MOST}


def stopped(made: int, failed: list[str], over: dict[str, int], down: list[str]) -> str:
    """Give why a lost handshake's calls stopped, so a count short of the most reads right."""
    if made == HANDSHAKES:
        return "on the last"
    if failed:
        return f"not made again, as the {', '.join(failed)} call failed otherwise"
    if over:
        losses = ", ".join(f"the {name} call's {n}" for name, n in over.items())
        return (
            f"not made again, so that {losses} datagrams lost coming down, over {DOWN_MOST}, count"
        )
    return (
        f"not made again, as the {', '.join(down)} call's reply was lost coming down"
        " while the IFB dropped some of what came down"
    )


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
    """Under a full connection both ways, each call loses a hundredth at most coming down.

    The bar counts what was lost before the client's socket; what overflows the socket is the
    builder's, and only printed.

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
                f"the {name} call lost {ways['down']} datagrams down before the client's socket,"
                f" over {DOWN_MOST}; dropped by tin: {dropped}\n{trace}"
            )


def test_a_call_goes_ahead_of_its_own_uploads(rig: Rig) -> None:
    """While an app uploads beside its call, the shaper's upload drops at most a hundredth of it.

    The app's uploads, over TCP, go best effort in Joel's own CAKE, beside the call's video. A
    CAKE with no rate of its own takes turns by weight, best effort's twice video's, so with the
    lane full it dropped about a third of the call going up. A trial holds the upload where the
    lane is full whatever the controller finds. Only the media call is made, since the page call
    goes best effort, where the app's uploads would crowd its handshake out too. The bar is on the
    shaper's own drops in the video tin, which only the media call fills, since what the
    router VM drops going up is the builder's.
    """
    try:
        rig.client.succeed(f"echo '{{\"most_kbit\": {HELD_KBIT}}}' > {TRIAL}")
        before = {what: rig.counts("eth1", what) for what in ("drops", "sent_packets")}
        since = int(rig.client.succeed("date +%s"))
        under_load(rig, shaped=True, app_uploads=APP_UPLOADS, names=["media"])
        least = rig.least_up(since)
        after = {what: rig.counts("eth1", what) for what in ("drops", "sent_packets")}
        # The step that moves the classes' rate tells Joel's CAKE a moment after, so a read
        # between the two is taken again; one that never agrees is a CAKE left behind.
        for _ in range(5):
            held, joel = rig.upload_rates()
            if joel == held:
                break
            time.sleep(1)
    finally:
        # A ceiling left on would hold the next tests' upload under it for ten minutes, and
        # the rate climbs back gently even once it's gone: test_unshaped, next, stops the
        # shaper, so no shaped test is to follow this one.
        rig.client.succeed(f"rm -f {TRIAL}")
    meanwhile = {
        what: dict(
            zip(TINS, (a - b for a, b in zip(after[what], before[what], strict=True)), strict=True)
        )
        for what in after
    }
    dropped, sent = meanwhile["drops"], meanwhile["sent_packets"]
    print(f"the upload dropped by tin meanwhile: {dropped}, and sent {sent}")
    print(
        f"the upload's classes held to {held} kbit/s, and Joel's CAKE to {joel},"
        f" the controller asking at least {least}"
    )
    # Without the hold, with the call in another tin, or with the app's uploads elsewhere
    # than beside it, the video tin would drop nothing whatever CAKE did; and counts that
    # started again would go back.
    assert held <= HELD_KBIT, f"the trial never held the upload: its classes at {held} kbit/s"
    # Held much under it at any moment, the call would outgrow the half that CAKE serves video
    # first, so the least it asked through the loads and the call is what's judged.
    assert least is not None, "the controller asked the upload no limit under the trial"
    assert least >= HELD_KBIT * 9 // 10, f"the controller cut under the trial, to {least} kbit/s"
    assert joel is not None, f"Joel's CAKE has no rate, with the classes at {held} kbit/s"
    assert joel == held, f"Joel's CAKE at {joel} kbit/s doesn't follow the classes' {held}"
    assert sent["video"] >= CALL_LEAST, f"the call didn't go in the video tin: {sent}"
    assert min(dropped.values()) >= 0, f"Joel's CAKE started its counts again: {dropped}"
    assert dropped["best effort"] > 0, f"the app's uploads never filled Joel's lane: {dropped}"
    assert dropped["video"] <= UP_DROPPED_MOST, (
        f"the shaper's upload dropped {dropped['video']} of the video call's datagrams, over"
        f" {UP_DROPPED_MOST}; by tin: {dropped}\n{rig.trace()}"
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
                f"the {name} call lost {ways['down']} datagrams down unshaped, before the"
                f" client's socket, over {DOWN_MOST}\n{trace}"
            )

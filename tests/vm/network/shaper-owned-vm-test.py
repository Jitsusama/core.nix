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
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any

import pytest
from test_driver.machine import retry

if TYPE_CHECKING:
    from test_driver.machine import BaseMachine

# A JSON object as nft or the shaper gives it, whose fields the test reads as it finds them.
type Json = dict[str, Any]

BIN = "/run/current-system/sw/bin"
FAR = "203.0.113.3"
# The connection, in kbit/s, and the jobs' uploads beside the app's one.
LINE = 8000
JOBS = 32
JOBS_PORT = 5201
APP_PORT = 5202
# What the server takes while a share is read: four seconds of the line,
# so a share is many round trips' worth, in bytes.
WINDOW = 4_000_000
# What the app sends before its share counts, so its slow start, while the
# jobs still hold the connection, isn't counted against it.
APP_STARTS = 500_000
# The app's least share with the shaper: CAKE gives bulk a sixteenth while
# anything else wants the connection, so it's near fifteen sixteenths, and
# half is well clear of what a busy host's gaps take from it. Without the
# shaper the router gives it one flow's share of 33, and a fifth is well
# clear of that.
SHAPED_SHARE = 0.5
UNSHAPED_SHARE = 0.2
# How long the client may take to boot to its first queues, and the shaper
# to take the queue back from the router once the jobs fill it.
COMING_UP = dt.timedelta(seconds=30)
TAKING = dt.timedelta(seconds=90)
# How long the shaper's rate must stay under the line to count as held
# there: a moment under it, as a cut for delay makes, holds nothing.
HELD = dt.timedelta(seconds=3)
SENDING = dt.timedelta(seconds=60)


@dataclass
class Rig:
    """The client, and the server past the router that counts what reaches it."""

    client: BaseMachine
    server: BaseMachine

    def up(self) -> int:
        """Give the rate the shaper sends at on eth1, in kbit/s, or 0 before it says."""
        status: list[Json] = json.loads(self.client.succeed("cat /run/shaper/status.json"))
        return next((int(link.get("up") or 0) for link in status if link["name"] == "eth1"), 0)

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

    def app_share(self) -> float:
        """Give the app's share of what the server takes, once its upload is under way."""
        self.uploads("app-upload", "app.slice", 1, APP_PORT)
        started = self.taken().get(APP_PORT, 0)
        retry(
            lambda _last_try: self.taken().get(APP_PORT, 0) - started >= APP_STARTS,
            timeout=SENDING,
        )
        before = self.taken()
        retry(
            lambda _last_try: sum(self.taken().values()) - sum(before.values()) >= WINDOW,
            timeout=SENDING,
        )
        after = self.taken()
        self.client.succeed("systemctl -M alice@ --user stop app-upload.service")
        moved = {port: after[port] - before.get(port, 0) for port in after}
        print(f"what the server took while the app uploaded: {moved}")
        return moved.get(APP_PORT, 0) / sum(moved.values())


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
    return Rig(client=client, server=server)


@pytest.fixture(scope="module")
def loaded(rig: Rig) -> Rig:
    """Give the rig with the jobs' uploads filling the router, the shaper holding under it.

    The shaper starts above the line, so the router's queue is the one that fills;
    only taking it back brings the shaper's rate under the line, and keeps it there.
    """
    rig.uploads("uploads", "jobs.slice", JOBS, JOBS_PORT)
    under_since: list[float] = []

    def taken_back(last_try: bool) -> bool:
        up = rig.up()
        if not 0 < up < LINE:
            under_since.clear()
        elif not under_since:
            under_since.append(time.monotonic())
        held = bool(under_since) and time.monotonic() - under_since[0] >= HELD.total_seconds()
        if last_try and not held:
            print(f"never held under the line, at {up} kbit/s:\n{rig.trace()}")
        return held

    retry(taken_back, timeout=TAKING)
    return rig


def test_an_app_outruns_the_jobs(loaded: Rig) -> None:
    """With the shaper, an app's one upload gets most of the connection beside the 32 jobs'."""
    share = loaded.app_share()
    assert share > SHAPED_SHARE, (
        f"the app got {share:.2f} of the line at {loaded.up()} kbit/s:\n{loaded.trace()}"
    )


def test_unshaped(loaded: Rig) -> None:
    """Without it, the router gives the app one flow's share."""
    loaded.client.succeed("systemctl stop shaper.service")
    share = loaded.app_share()
    assert share < UNSHAPED_SHARE, f"the app got {share:.2f} of the line without the shaper"

"""Whether the shaper finds a modem's connection and keeps an app's pings near it, under load.

tests/shaper-rate.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), and
its opening comment says what the test proves. The router is the connection: a netem delay,
then a tbf whose rate the test changes and whose backlog is the modem's queue. The test waits
on that queue rather than on the clock, as it's what the pings would wait in.

Each test asks for the state it starts from as a fixture: the connection found under the jobs'
uploads, then halved, then back at its full rate. A fixture's work is done once a run, however
many tests ask for it, so any test runs alone with `-k`, and all of them run in the order
they're written, each after what it needs. The tests need no value from Nix: they ask the
server for its address.
"""

from __future__ import annotations

import datetime as dt
import json
import statistics
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any

import pytest
from test_driver.machine import retry

if TYPE_CHECKING:
    from test_driver.machine import BaseMachine

# A JSON object as tc or the shaper gives it, whose fields the test reads as it finds them.
type Json = dict[str, Any]

BIN = "/run/current-system/sw/bin"
# The connection's queue at 8 Mbit/s, for a burst of 32 kbit and a second of buffer.
CONNECTION = "tbf rate {} burst 32kbit limit 1000000"
# The connection at its full rate and halved, in kbit/s.
FULL = 8000
HALVED = 4000
# The shaper probes a little past the connection and cuts at the queue that
# builds, so any moment finds it within a tenth of the connection; at a
# quarter under, it's still draining a cut.
PROBED = 8800
SETTLED = 6000
# The connection's queue, in bytes, when it's drained: 40 ms at 8 Mbit/s, or 80 ms
# at 4; and full, half a second of it at 8.
DRAINED = 40_000
FILLED = 500_000
# An app's median round trip with the shaper at the full connection and
# the halved one, and the least it takes without the shaper, in ms.
SHAPED = 80
DROPPED = 100
UNSHAPED = 300
# How many of an app's 40 pings have to come back for a median.
ANSWERS = 20
# CAKE's diffserv4 tins, in the order its counts list them.
TINS = ["bulk", "best effort", "video", "voice"]
# What an app's uploads send while the jobs' uploads go on, and the most of
# that the jobs may send meanwhile: CAKE leaves bulk a sixteenth while
# anything else wants the connection, where six flows shared evenly would leave
# the jobs twice the app's. The app sends two, since one alone, short of
# CPU in a VM on a host at a load of 40, left gaps in its queue that bulk
# rightly filled, to over a fifth of the connection.
APP_FLOWS = 2
APP_SENDS = 2_000_000
JOBS_SHARE = 4
# The most a link that carries no uploads sends between two looks a second
# apart: the shaper's pings and the test's own commands.
QUIET = 20_000
# How many looks running a link must be that quiet, past the shaper's
# window of 1.2 s.
QUIET_LOOKS = 2
# What the app's upload sends before the count starts, so its first round
# trips, while the jobs still have the connection, aren't counted against it.
APP_STARTS = 500_000
# How long the client may take to boot to its first queues, and the
# server's link to take its address.
COMING_UP = dt.timedelta(seconds=30)
ADDRESSING = dt.timedelta(seconds=60)
# How long the shaper may take to find the connection from where it starts, to
# follow it down or a second link, and to climb back to it.
FINDING = dt.timedelta(seconds=90)
FOLLOWING = dt.timedelta(seconds=60)
CLIMBING = dt.timedelta(seconds=120)
# How long an upload may take to send what's counted, and the connection's
# queue to fill without the shaper.
SENDING = dt.timedelta(seconds=60)
FILLING = dt.timedelta(seconds=30)


@dataclass
class Rig:
    """The client, the router that is its connection, the server past it, and what each counts."""

    client: BaseMachine
    router: BaseMachine
    far: str

    def up(self) -> int:
        """Give the rate the shaper sends at on eth1, in kbit/s, or 0 before it says.

        The status the shaper wrote before it began on eth1 doesn't name it yet,
        which a wait read as soon as the shaper had started.
        """
        status: list[Json] = json.loads(self.client.succeed("cat /run/shaper/status.json"))
        return next((int(link["up"]) for link in status if link["name"] == "eth1"), 0)

    def backlog(self) -> int:
        """Give the bytes waiting in the connection's queue at the router."""
        queues: list[Json] = json.loads(self.router.succeed("tc -s -j qdisc show dev eth2"))
        return int(next(q for q in queues if q["kind"] == "tbf")["backlog"])

    def sent(self) -> dict[str, int]:
        """Give the bytes CAKE on eth1 has sent from each tin."""
        queues: list[Json] = json.loads(self.client.succeed("tc -s -j qdisc show dev eth1"))
        cake = next(q for q in queues if q["kind"] == "cake")
        return dict(zip(TINS, (int(tin["sent_bytes"]) for tin in cake["tins"]), strict=True))

    def sent_by(self, link: str) -> int:
        """Give the bytes the client has sent on a link."""
        return int(self.client.succeed(f"cat /sys/class/net/{link}/statistics/tx_bytes"))

    def connection(self, kbit: int) -> None:
        """Set the connection's rate."""
        self.router.succeed(
            f"tc qdisc change dev eth2 parent 1: handle 2: {CONNECTION.format(f'{kbit}kbit')}"
        )

    def uploads(self) -> None:
        """Start four uploads from the account's jobs, until they're stopped."""
        self.client.succeed(
            "systemd-run -M alice@ --user --slice=jobs.slice --unit=uploads"
            f" {BIN}/bash -c 'for i in 1 2 3 4; do"
            f" (while true; do {BIN}/cat /dev/zero > /dev/tcp/{self.far}/5201; done) & done;"
            " wait'"
        )

    def ping(self, name: str) -> float:
        """Give the median round trip of an app's pings to the server, in ms."""
        self.client.succeed(
            "systemd-run -M alice@ --user --wait --collect --slice=app.slice"
            f" -p StandardOutput=file:/tmp/{name}.txt {BIN}/ping -n -c 40 -i 0.25 {self.far}"
        )
        text = self.client.succeed(f"cat /tmp/{name}.txt")
        words = text.split()
        times = [float(word.removeprefix("time=")) for word in words if word.startswith("time=")]
        assert len(times) > ANSWERS, text
        return statistics.median(times)

    def holds_under(self, kbit: int) -> bool:
        """Say whether the shaper holds under that rate with the connection's queue drained."""
        return self.up() < kbit and self.backlog() < DRAINED


@pytest.fixture(scope="module")
def rig(machines: dict[str, BaseMachine]) -> Rig:
    """Give the machines once the shaper shapes the client's connection and the server listens."""
    client, router, server = machines["client"], machines["router"], machines["server"]
    for node in machines.values():
        node.start()
    server.wait_for_open_port(5201)
    router.wait_for_unit("connection.service")
    client.wait_for_unit("user@1000.service")
    client.wait_until_succeeds("tc -j qdisc show dev eth1 | grep -q cake", timeout=COMING_UP)
    # The sink listens on every address, so its port can open before the
    # server's link has its own; wait for the address itself.
    shown = server.wait_until_succeeds(
        "ip -4 -o address show dev eth1 | grep inet", timeout=ADDRESSING
    )
    return Rig(client=client, router=router, far=shown.split()[3].split("/")[0])


@pytest.fixture(scope="module")
def loaded(rig: Rig) -> Rig:
    """Give the rig with the jobs' uploads filling the connection, the shaper holding under it."""
    rig.uploads()

    def found(_last_try: bool) -> bool:
        """Say whether the shaper holds under the connection with its queue drained."""
        return SETTLED < rig.up() < FULL and rig.backlog() < DRAINED

    retry(found, timeout=FINDING)
    return rig


@pytest.fixture(scope="module")
def dropped(loaded: Rig) -> Rig:
    """Give the loaded rig once its connection is halved and the shaper has followed it down."""
    loaded.connection(HALVED)
    retry(lambda _last_try: loaded.holds_under(HALVED), timeout=FOLLOWING)
    return loaded


@pytest.fixture(scope="module")
def returned(dropped: Rig) -> Rig:
    """Give the rig once its connection is back at full rate and the shaper has climbed to it."""
    dropped.connection(FULL)
    retry(lambda _last_try: dropped.up() > SETTLED, timeout=CLIMBING)
    return dropped


def test_finds_the_connection(loaded: Rig) -> None:
    """The shaper finds the connection and an app's pings stay near it."""
    shaped = loaded.ping("shaped")
    rate = loaded.up()
    assert SETTLED < rate < PROBED, rate
    assert shaped < SHAPED, f"{shaped} ms shaped at {rate} kbit/s"


def test_app_upload(loaded: Rig) -> None:
    """An app's upload takes the connection from the jobs' while it runs."""
    started = loaded.sent()["best effort"]
    loaded.client.succeed(
        "systemd-run -M alice@ --user --slice=app.slice --unit=app-upload"
        f" {BIN}/bash -c 'for i in $({BIN}/seq {APP_FLOWS}); do (while true; do"
        f" {BIN}/cat /dev/zero > /dev/tcp/{loaded.far}/5201; done) & done; wait'"
    )
    retry(lambda _last_try: loaded.sent()["best effort"] - started >= APP_STARTS, timeout=SENDING)
    before = loaded.sent()
    retry(
        lambda _last_try: loaded.sent()["best effort"] - before["best effort"] >= APP_SENDS,
        timeout=SENDING,
    )
    after = loaded.sent()
    loaded.client.succeed("systemctl -M alice@ --user stop app-upload.service")
    moved = {tin: after[tin] - before[tin] for tin in TINS}
    print(f"while an app uploads: {moved}")
    assert moved["bulk"] < moved["best effort"] // JOBS_SHARE, moved


def test_follows_down(dropped: Rig) -> None:
    """When the connection drops, the shaper follows it down."""
    assert dropped.ping("dropped") < DROPPED


def test_climbs_back(returned: Rig) -> None:
    """And back up when it returns."""
    assert returned.up() > SETTLED


def test_second_link(returned: Rig) -> None:
    """Uploads over a second link to the same router count against the connection."""
    rig = returned
    rig.client.succeed("systemctl -M alice@ --user stop uploads.service")

    def follows(_last_try: bool) -> bool:
        """Say whether eth2 takes eth1's rate, as both reach the one router."""
        status: list[Json] = json.loads(rig.client.succeed("cat /run/shaper/status.json"))
        return any(link["name"] == "eth2" and link.get("follows") == "eth1" for link in status)

    retry(follows, timeout=FOLLOWING)
    # eth1's controller judges by its last second or so of traffic, so it
    # must have none of its own left before only eth2 carries the uploads:
    # then it sees the connection fill only if it counts what eth2 sends.
    last = {"sent": rig.sent_by("eth1"), "quiet": 0}

    def quiet(_last_try: bool) -> bool:
        """Say whether eth1 has sent next to nothing for QUIET_LOOKS looks running."""
        now = rig.sent_by("eth1")
        last["quiet"] = last["quiet"] + 1 if now - last["sent"] < QUIET else 0
        last["sent"] = now
        return last["quiet"] >= QUIET_LOOKS

    retry(quiet, timeout=SENDING)
    rig.client.succeed(
        "systemd-run --unit=second-link-uploads"
        f" {BIN}/bash -c 'for i in 1 2 3 4; do (while true; do"
        f" {BIN}/socat -u /dev/zero TCP:{rig.far}:5201,so-bindtodevice=eth2; done) & done; wait'"
    )
    started = rig.sent_by("eth2")
    retry(lambda _last_try: rig.sent_by("eth2") - started >= APP_SENDS, timeout=SENDING)
    rig.connection(HALVED)
    retry(lambda _last_try: rig.holds_under(HALVED), timeout=FOLLOWING)
    rig.client.succeed("systemctl stop second-link-uploads.service")
    rig.connection(FULL)
    rig.uploads()


def test_unshaped(loaded: Rig) -> None:
    """Without the shaper, the connection's buffer fills."""
    loaded.client.succeed("systemctl stop shaper.service")
    retry(lambda _last_try: loaded.backlog() > FILLED, timeout=FILLING)
    unshaped = loaded.ping("unshaped")
    assert unshaped > UNSHAPED, unshaped
    print(f"an app's median round trip without the shaper: {unshaped} ms")

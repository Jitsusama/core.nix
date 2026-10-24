"""Whether the shaper finds a modem's line and keeps an app's pings near it, under load.

tests/shaper-rate.nix runs this, and its opening comment says what the test proves. The router
is the line: a netem delay, then a tbf whose rate the test changes and whose backlog is the
modem's queue. The test waits on that queue rather than on the clock, as it's what the pings
would wait in. The script needs no value from Nix: it asks the server for its address.
"""

import json
import statistics
from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from driver_names import client, retry, router, server, start_all, subtest, t

# A JSON object as tc or the shaper gives it, whose fields the test reads as it finds them.
type Json = dict[str, Any]

BIN = "/run/current-system/sw/bin"
# The line's queue at 8 Mbit/s, for a burst of 32 kbit and a second of buffer.
LINE = "tbf rate {} burst 32kbit limit 1000000"
# The line at its full rate and halved, in kbit/s.
FULL = 8000
HALVED = 4000
# The shaper probes a little past the line and cuts at the queue that
# builds, so any moment finds it within a tenth of the line; at a quarter
# under, it's still draining a cut.
PROBED = 8800
SETTLED = 6000
# The line's queue, in bytes, when it's drained: 40 ms at 8 Mbit/s, or 80 ms
# at 4; and full, half a second of it at 8.
DRAINED = 40_000
FILLED = 500_000
# An app's median round trip with the shaper at the full line and the
# halved one, and the least it takes without the shaper, in ms.
SHAPED = 80
DROPPED = 100
UNSHAPED = 300
# How many of an app's 40 pings have to come back for a median.
ANSWERS = 20
# CAKE's diffserv4 tins, in the order its counts list them.
TINS = ["bulk", "best effort", "video", "voice"]
# What an app's upload sends while the jobs' uploads go on, and the most of
# that the jobs may send meanwhile: CAKE leaves bulk a sixteenth while
# anything else wants the line, where five flows shared evenly would leave
# the jobs four times the app's.
APP_SENDS = 2_000_000
JOBS_SHARE = 4


def up() -> int:
    """Return the rate the shaper sends at on eth1, in kbit/s."""
    status: list[Json] = json.loads(client.succeed("cat /run/shaper/status.json"))
    return int(next(link for link in status if link["name"] == "eth1")["up"])


def backlog() -> int:
    """Return the bytes waiting in the line's queue at the router."""
    queues: list[Json] = json.loads(router.succeed("tc -s -j qdisc show dev eth2"))
    return int(next(q for q in queues if q["kind"] == "tbf")["backlog"])


def sent() -> dict[str, int]:
    """Return the bytes CAKE on eth1 has sent from each tin."""
    queues: list[Json] = json.loads(client.succeed("tc -s -j qdisc show dev eth1"))
    cake = next(q for q in queues if q["kind"] == "cake")
    return dict(zip(TINS, (int(tin["sent_bytes"]) for tin in cake["tins"]), strict=True))


def line(kbit: int) -> None:
    """Set the line's rate."""
    router.succeed(f"tc qdisc change dev eth2 parent 1: handle 2: {LINE.format(f'{kbit}kbit')}")


def ping(name: str) -> float:
    """Return the median round trip of an app's pings to the server, in ms."""
    client.succeed(
        "systemd-run -M alice@ --user --wait --collect --slice=app.slice"
        f" -p StandardOutput=file:/tmp/{name}.txt {BIN}/ping -n -c 40 -i 0.25 {far}"
    )
    text = client.succeed(f"cat /tmp/{name}.txt")
    times = [float(word.removeprefix("time=")) for word in text.split() if word.startswith("time=")]
    t.assertGreater(len(times), ANSWERS, text)
    return statistics.median(times)


start_all()
server.wait_for_open_port(5201)
router.wait_for_unit("line.service")
client.wait_for_unit("user@1000.service")
client.wait_until_succeeds("tc -j qdisc show dev eth1 | grep -q cake", timeout=30)
far = server.succeed("ip -4 -o address show dev eth1").split()[3].split("/")[0]

# Four uploads from the account's jobs, for as long as the test runs.
client.succeed(
    "systemd-run -M alice@ --user --slice=jobs.slice --unit=uploads"
    f" {BIN}/bash -c 'for i in 1 2 3 4; do"
    f" (while true; do {BIN}/cat /dev/zero > /dev/tcp/{far}/5201; done) & done; wait'"
)

with subtest("the shaper finds the line and an app's pings stay near it"):

    def found(_last_try: bool) -> bool:
        """Say whether the shaper holds under the line with its queue drained."""
        return SETTLED < up() < FULL and backlog() < DRAINED

    retry(found, timeout_seconds=90)
    shaped = ping("shaped")
    rate = up()
    t.assertGreater(rate, SETTLED)
    t.assertLess(rate, PROBED)
    t.assertLess(shaped, SHAPED, f"shaped at {rate} kbit/s")

with subtest("an app's upload takes the line from the jobs' while it runs"):
    before = sent()
    client.succeed(
        "systemd-run -M alice@ --user --slice=app.slice --unit=app-upload"
        f" {BIN}/bash -c 'while true; do {BIN}/cat /dev/zero > /dev/tcp/{far}/5201; done'"
    )

    def uploaded(_last_try: bool) -> bool:
        """Say whether the app's upload has sent its share."""
        return sent()["best effort"] - before["best effort"] >= APP_SENDS

    retry(uploaded, timeout_seconds=60)
    after = sent()
    client.succeed("systemctl -M alice@ --user stop app-upload.service")
    moved = {tin: after[tin] - before[tin] for tin in TINS}
    print(f"while an app uploads: {moved}")
    t.assertLess(moved["bulk"], moved["best effort"] // JOBS_SHARE, f"{moved}")

with subtest("when the line drops, the shaper follows it down"):
    line(HALVED)

    def followed(_last_try: bool) -> bool:
        """Say whether the shaper holds under the halved line with its queue drained."""
        return up() < HALVED and backlog() < DRAINED

    retry(followed, timeout_seconds=60)
    t.assertLess(ping("dropped"), DROPPED)

with subtest("and back up when it returns"):
    line(FULL)

    def climbed(_last_try: bool) -> bool:
        """Say whether the shaper has climbed back near the full line."""
        return up() > SETTLED

    retry(climbed, timeout_seconds=120)

with subtest("without the shaper, the line's buffer fills"):
    client.succeed("systemctl stop shaper.service")

    def filled(_last_try: bool) -> bool:
        """Say whether the line's queue holds half a second."""
        return backlog() > FILLED

    retry(filled, timeout_seconds=30)
    unshaped = ping("unshaped")
    t.assertGreater(unshaped, UNSHAPED)
    print(f"an app's median round trip: {shaped} ms shaped, {unshaped} ms not")

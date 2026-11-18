"""The network page, as the sites' server serves it, reads net's daemon its module runs.

network-page.nix, beside this, runs this through pytest inside the driver (tests/lib/vm_pytest.py),
on a machine whose account lingers with the local sites' and net's home modules as they're rendered
for it. Every ask is one a browser on the machine makes: the page fetched by the server's own name,
and its feed's WebSocket opened from the page's origin, saying hello and watching the kinds the page
reads as its kit does. The feed must reach net's daemon over the socket systemd holds for it, with
eero's source saying it hasn't written yet before anything is there, the house's devices once eero's
record is at the path eero's daemon writes and the running daemon's next round has read it, say
broken once net's socket and daemon are stopped, and live again once the socket is back. The tests
need no value from Nix.
"""

from __future__ import annotations

import json
from typing import TYPE_CHECKING, Any

import pytest
from sites_session import LISTENING_S, SITES, as_the_account, listened

if TYPE_CHECKING:
    from collections.abc import Iterator

    from test_driver.machine import BaseMachine

# A line the feed says, as JSON reads it.
type Said = dict[str, Any]

FEED = "ws://127.0.0.1:8484/feed/net"
# The kinds the network page watches, as home.nix declares its feed.
KINDS = ["device", "link", "source", "internet", "concern"]
# The page's kit's asks of a feed: hello, then a watch of each kind it reads.
ASKS: list[dict[str, object]] = [
    {"id": 1, "ask": "hello"},
    *({"id": at, "ask": "watch", "kind": kind} for at, kind in enumerate(KINDS, start=2)),
]
# How long the feed has to say what changed: past the bridge's first few tries at the daemon, and
# past net's next round, a minute after its last, which once eero's record is in also waits two
# seconds on each receiver, printer and Apple TV it lists, none of which answers in a VM.
SAYING_S = 150
# net's socket and its unit, as its home module names them, which sites_session's daemon_back starts
# again after a test that stops them.
DAEMON = ["net.socket", "net.service"]
# Where eero's daemon writes what it read (modules/eero/src/daemon.rs), which net reads each round
# (modules/net/src/daemon.rs). The test writes it, since eero's daemon needs its cloud.
EERO_WRITES = "/run/user/1000/eero/home-network.json"
# What net says of eero's source while nothing is at that path.
NOT_WRITTEN = "eero hasn't written what it read"


def told(heard: list[Said]) -> dict[str, Said]:
    """Give every record the feed told, by its key, the last word on each kept."""
    return {
        record["key"]: record
        for line in heard
        for record in line.get("records", []) + line.get("changed", [])
        if "key" in record
    }


def stands(machine: BaseMachine, state: str, holding: str = "") -> list[Said]:
    """Listen until the feed says it stands at `state`, holding a key starting `holding`."""
    words = ""
    for _ in range(SAYING_S // LISTENING_S):
        words = listened(machine, FEED, ASKS)
        heard = [json.loads(line) for line in words.splitlines() if line.startswith("{")]
        # A broken feed tells no records, so only a state that holds them is asked for one.
        if any(line.get("state") == state for line in heard) and (
            not holding or any(key.startswith(holding) for key in told(heard))
        ):
            return heard
    pytest.fail(f"the feed never said it was {state} with {holding!r}; last it said:\n{words}")


@pytest.fixture(scope="module")
def served(machine: BaseMachine) -> BaseMachine:
    """Boot the machine with the account's manager running the sites' server and net's daemon."""
    machine.wait_for_unit("multi-user.target")
    machine.wait_for_unit("user@1000.service")
    machine.wait_until_succeeds(as_the_account("systemctl --user is-active local-sites.service"))
    machine.wait_until_succeeds(as_the_account("systemctl --user is-active net.socket"))
    machine.wait_until_succeeds(f"curl -sf {SITES}/network/")
    return machine


@pytest.fixture
def eero_written(served: BaseMachine) -> Iterator[BaseMachine]:
    """Put eero's record where its daemon writes it, and take it away again after the test."""
    served.succeed(as_the_account(f"install -D -m 0600 /etc/eero-heard.json {EERO_WRITES}"))
    yield served
    # Restarted, so the next test's net hasn't read the record it no longer has.
    served.succeed(as_the_account(f"rm -f {EERO_WRITES} && systemctl --user restart net.service"))


def test_the_server_serves_the_network_page(served: BaseMachine) -> None:
    """The page and its script are served by name, with the strip its build writes."""
    page = served.succeed(f"curl -sf {SITES}/network/")
    assert '<nav class="sites-strip"' in page, (
        "the page was served without the strip its build writes"
    )
    served.succeed(f"curl -sf {SITES}/network/site.js")


def test_the_feed_says_how_each_source_stands_before_eero_has(served: BaseMachine) -> None:
    """With nothing from eero yet, the feed says live, and that eero hasn't written."""
    records = told(stands(served, "live", "source:eero"))
    eero = records["source:eero"]
    assert eero.get("state") == "unreachable", f"eero answered with nothing written: {eero}"
    assert NOT_WRITTEN in str(eero.get("reason", "")), f"eero's source didn't say why: {eero}"


def test_the_house_eero_wrote_reaches_the_feed(eero_written: BaseMachine) -> None:
    """Once eero's record is where its daemon writes it, the running net's next round tells it.

    Away from any home network, the modem and the switch say nothing, so every device the feed
    tells is one net read from eero's record.
    """
    records = told(stands(eero_written, "live", "device:"))
    assert records["source:eero"].get("state") == "answering", (
        f"eero's record was read but its source didn't answer: {records['source:eero']}"
    )


def test_a_daemon_stopped_is_said_broken_then_live(daemon_back: BaseMachine) -> None:
    """The feed says broken once net and its socket stop, and live once the socket's back."""
    stands(daemon_back, "live", "source:")
    units = " ".join(DAEMON)
    daemon_back.succeed(as_the_account(f"systemctl --user stop {units}"))
    stands(daemon_back, "broken")
    daemon_back.succeed(as_the_account("systemctl --user start net.socket"))
    stands(daemon_back, "live", "source:")

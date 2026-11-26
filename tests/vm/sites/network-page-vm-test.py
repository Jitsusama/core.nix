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

from typing import TYPE_CHECKING

import pytest
from sites_session import SITES, Feed, Said, as_the_account, stands, told

if TYPE_CHECKING:
    from collections.abc import Iterator

    from test_driver.machine import BaseMachine

# The kinds the network page watches, as home.nix declares its feed.
KINDS = ["device", "link", "source", "internet", "concern"]
# The feed, asked hello and then a watch of each kind the page reads, as its kit asks. It has past
# the bridge's first few tries at the daemon, and past net's next round, to say what changed. That
# round comes a minute after the last, and once eero's record is in it waits up to about 14 s on
# what the record lists, none of which answers here: 2 s on the modem's first page, 2 s on the
# receiver's description, 2 s on each of the printer's four pages and 2 s on the Apple TV.
FEED = Feed(
    address="ws://127.0.0.1:8484/feed/net",
    asks=[
        {"id": 1, "ask": "hello"},
        *({"id": at, "ask": "watch", "kind": kind} for at, kind in enumerate(KINDS, start=2)),
    ],
    saying_s=150,
)
# net's socket and its unit, as its home module names them, which sites_session's daemon_back starts
# again after a test that stops them.
DAEMON = ["net.socket", "net.service"]
# Where eero's daemon writes what it read (modules/eero/src/daemon.rs), which net reads each round
# (modules/net/src/daemon.rs). The test writes it, since eero's daemon needs its cloud.
EERO_WRITES = "/run/user/1000/eero/home-network.json"
# What net says of eero's source while nothing is at that path.
NOT_WRITTEN = "eero hasn't written what it read"


def eero_said(records: dict[str, Said]) -> bool:
    """Whether the feed has told eero's source yet."""
    return "source:eero" in records


def eero_answers(records: dict[str, Said]) -> bool:
    """Whether eero's source answers, as it does once net has read eero's record."""
    return records.get("source:eero", {}).get("state") == "answering"


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
    """Put eero's record where its daemon writes it, and take it away again after the test.

    It's stamped as read now, as the daemon stamps each answered read, since net takes a record
    whose read is long past as one its daemon stopped writing.
    """
    stamped = 's/"read": "[^"]*"/"read": "\'"$now"\'"/'
    served.succeed(
        as_the_account(
            f'mkdir -p "$(dirname {EERO_WRITES})" && umask 077 && '
            "now=$(date -u +%Y-%m-%dT%H:%M:%SZ) && "
            f"sed '{stamped}' /etc/eero-heard.json > {EERO_WRITES}.new && "
            f"mv {EERO_WRITES}.new {EERO_WRITES}"
        )
    )
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
    records = told(stands(served, FEED, "live", eero_said))
    eero = records["source:eero"]
    assert eero.get("state") == "unreachable", f"eero answered with nothing written: {eero}"
    assert NOT_WRITTEN in str(eero.get("reason", "")), f"eero's source didn't say why: {eero}"


def test_the_house_eero_wrote_reaches_the_feed(eero_written: BaseMachine) -> None:
    """Once eero's record is where its daemon writes it, the running net's next round tells it.

    Away from any home network, the modem and the switch say nothing, so every device the feed
    tells is one net read from eero's record, told over its own watch apart from eero's source.
    """
    records = told(stands(eero_written, FEED, "live", eero_answers))
    assert any(key.startswith("device:") for key in records), (
        f"eero's source answered but no device of its record was told: {sorted(records)}"
    )


def test_a_daemon_stopped_is_said_broken_then_live(daemon_back: BaseMachine) -> None:
    """The feed says broken once net and its socket stop, and live once the socket's back."""
    stands(daemon_back, FEED, "live", eero_said)
    units = " ".join(DAEMON)
    daemon_back.succeed(as_the_account(f"systemctl --user stop {units}"))
    stands(daemon_back, FEED, "broken")
    daemon_back.succeed(as_the_account("systemctl --user start net.socket"))
    stands(daemon_back, FEED, "live", eero_said)

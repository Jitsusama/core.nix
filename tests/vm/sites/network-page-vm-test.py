"""The network page, as the sites' server serves it, reads net's daemon its module runs.

network-page.nix, beside this, runs this through pytest inside the driver (tests/lib/vm_pytest.py),
on a machine whose account lingers with the local sites' and net's home modules as they're rendered
for it. Every ask is one a browser on the machine makes: the page fetched by the server's own name,
and its feed's WebSocket opened from the page's origin, saying hello and watching the kinds the page
reads as its kit does. The feed must reach net's daemon over the socket systemd holds for it, with
how each source stands even before eero has written anything, the house's devices once eero's
record is where its daemon writes it, say broken once net's socket and daemon are stopped, and live
again once the socket is back. The tests need no value from Nix.
"""

from __future__ import annotations

import json
import shlex
from typing import TYPE_CHECKING, Any

import pytest

if TYPE_CHECKING:
    from collections.abc import Iterator

    from test_driver.machine import BaseMachine

# A line the feed says, as JSON reads it.
type Said = dict[str, Any]

# Where the server listens, by the name a browser on the machine sends.
SITES = "http://127.0.0.1:8484"
FEED = "ws://127.0.0.1:8484/feed/net"
# The kinds the network page watches, as home.nix declares its feed.
KINDS = ["device", "link", "source", "internet", "concern"]
# The page's kit's asks of a feed: hello, then a watch of each kind it reads.
ASKS = [
    {"id": 1, "ask": "hello"},
    *({"id": at, "ask": "watch", "kind": kind} for at, kind in enumerate(KINDS, start=2)),
]
# How long one listen keeps the feed open; the daemon's first answer comes well inside it.
LISTENING_S = 8
# How long the feed has to say what changed: past the bridge's first few tries at the daemon, and
# past net's first round, which waits two seconds on each modem page that doesn't answer.
SAYING_S = 120
# net's socket and its unit, as its home module names them.
DAEMON = ["net.socket", "net.service"]
# Where eero's daemon writes what it read, which net reads each round.
EERO_WRITES = "/run/user/1000/eero/home-network.json"


def as_the_account(command: str) -> str:
    """Give a command run as the account in its own session, its manager reachable."""
    session = "export XDG_RUNTIME_DIR=/run/user/1000; "
    return f"runuser -u alice -- bash -c {shlex.quote(session + command)}"


def listened(machine: BaseMachine) -> str:
    """Open the feed as the page does, ask what its kit asks, and give all it heard."""
    asks = " ".join(shlex.quote(json.dumps(ask)) for ask in ASKS)
    # websocat's -H takes every word after it as a header, the URL included, so the page's
    # origin goes by its own option.
    _, heard = machine.execute(
        f"{{ printf '%s\\n' {asks}; sleep {LISTENING_S}; }} "
        f"| websocat --text --origin {shlex.quote(SITES)} {FEED} 2>&1"
    )
    return heard


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
        words = listened(machine)
        heard = [json.loads(line) for line in words.splitlines() if line.startswith("{")]
        if any(line.get("state") == state for line in heard) and any(
            key.startswith(holding) for key in told(heard)
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
def daemon_back(served: BaseMachine) -> Iterator[BaseMachine]:
    """Leave net's socket and unit running again after a test that stops them."""
    yield served
    units = " ".join(DAEMON)
    served.succeed(as_the_account(f"systemctl --user start {units}"))


def test_the_server_serves_the_network_page(served: BaseMachine) -> None:
    """The page and its script are served by name, with the strip its build writes."""
    page = served.succeed(f"curl -sf {SITES}/network/")
    assert '<nav class="sites-strip"' in page, (
        "the page was served without the strip its build writes"
    )
    served.succeed(f"curl -sf {SITES}/network/site.js")


def test_the_feed_says_how_each_source_stands_before_eero_has(served: BaseMachine) -> None:
    """With nothing from eero yet, the feed says live, and that eero isn't answering and why."""
    records = told(stands(served, "live", "source:eero"))
    eero = records["source:eero"]
    assert eero.get("state") != "answering", f"eero was said to answer with nothing written: {eero}"
    assert "eero" in str(eero.get("reason", "")), f"eero's source didn't say why: {eero}"


def test_the_house_eero_wrote_reaches_the_feed(served: BaseMachine) -> None:
    """Once eero's record is where its daemon writes it, net's next round tells the devices."""
    served.succeed(
        as_the_account(
            f"install -D -m 0600 /etc/eero-heard.json {EERO_WRITES}"
            " && systemctl --user restart net.service"
        )
    )
    records = told(stands(served, "live", "device:"))
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

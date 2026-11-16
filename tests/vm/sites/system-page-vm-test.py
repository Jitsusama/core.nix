"""The system page, as the sites' server serves it, reads the system daemon its module runs.

system-page.nix, beside this, runs this through pytest inside the driver (tests/lib/vm_pytest.py),
on a machine whose account lingers with the local sites' and the system's home modules as they're
rendered for it. Every ask is one a browser on the machine makes: the page and the kit it imports
fetched by the server's own name, and its feed's WebSocket opened from the page's origin, saying
hello and watching the summary as the page's kit does. The feed must reach the daemon over the
socket systemd holds for it, say broken once that socket and the daemon are stopped, and live again
once the socket is back. The tests need no value from Nix.
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
FEED = "ws://127.0.0.1:8484/feed/system"
# The page's kit's first two asks of a feed: hello, then a watch of the one kind it reads first.
ASKS = [{"id": 1, "ask": "hello"}, {"id": 2, "ask": "watch", "kind": "summary"}]
# How long one listen keeps the feed open; the daemon's first answer comes well inside it.
LISTENING_S = 8
# How long the feed has to say what changed: past the bridge's first few tries at the daemon.
SAYING_S = 90
# The daemon's socket and its unit, as its home module names them.
DAEMON = ["system.socket", "system.service"]


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


def stands(machine: BaseMachine, state: str) -> list[Said]:
    """Listen until the feed says it stands at `state`, giving what was heard then."""
    words = ""
    for _ in range(SAYING_S // LISTENING_S):
        words = listened(machine)
        heard = [json.loads(line) for line in words.splitlines() if line.startswith("{")]
        if any(line.get("state") == state for line in heard):
            return heard
    pytest.fail(f"the feed never said it was {state}; last it said:\n{words}")


@pytest.fixture(scope="module")
def served(machine: BaseMachine) -> BaseMachine:
    """Boot the machine with the account's manager running the sites' server and the daemon."""
    machine.wait_for_unit("multi-user.target")
    machine.wait_for_unit("user@1000.service")
    machine.wait_until_succeeds(as_the_account("systemctl --user is-active local-sites.service"))
    machine.wait_until_succeeds(as_the_account("systemctl --user is-active system.socket"))
    machine.wait_until_succeeds(f"curl -sf {SITES}/system/")
    return machine


@pytest.fixture
def daemon_back(served: BaseMachine) -> Iterator[BaseMachine]:
    """Leave the daemon's socket and unit running again after a test that stops them."""
    yield served
    units = " ".join(DAEMON)
    served.succeed(as_the_account(f"systemctl --user start {units}"))


def test_the_server_serves_the_system_page_and_its_kit(served: BaseMachine) -> None:
    """The page, its script and the kit it imports are served by the server's own name."""
    page = served.succeed(f"curl -sf {SITES}/system/")
    assert '<nav class="sites-strip"' in page, (
        "the page was served without the strip its build writes"
    )
    served.succeed(f"curl -sf {SITES}/system/site.js")
    served.succeed(f"curl -sf {SITES}/local-sites/kit/feed.js")


def test_the_feed_reaches_the_daemon_over_its_socket(served: BaseMachine) -> None:
    """The page's feed says live and hands over the daemon's summary, read over its socket."""
    heard = stands(served, "live")
    told = [record for line in heard for record in line.get("records", [])]
    assert any(record.get("key") == "summary" for record in told), f"no summary in {heard}"


def test_a_daemon_stopped_is_said_broken_then_live(daemon_back: BaseMachine) -> None:
    """The feed says broken once the daemon and its socket stop, and live once the socket's back."""
    stands(daemon_back, "live")
    units = " ".join(DAEMON)
    daemon_back.succeed(as_the_account(f"systemctl --user stop {units}"))
    stands(daemon_back, "broken")
    daemon_back.succeed(as_the_account("systemctl --user start system.socket"))
    stands(daemon_back, "live")

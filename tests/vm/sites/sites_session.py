"""What the sites' VM tests share: asking as the account, and a feed opened as a page opens it.

A module of the driver's Python, which each sites' VM test gives its driver as a pytest plugin
(system-page.nix, network-page.nix), so its fixture reaches the tests without their importing it,
and its helpers are imported by name. Each test's own file keeps its machine's readiness, its feed
and the units its daemon runs as, named DAEMON.
"""

from __future__ import annotations

import json
import shlex
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any

import pytest

if TYPE_CHECKING:
    from collections.abc import Callable, Iterator

    from test_driver.machine import BaseMachine

# A line the feed says, as JSON reads it.
type Said = dict[str, Any]

# Where the server listens, by the name a browser on the machine sends.
SITES = "http://127.0.0.1:8484"
# How long one listen keeps the feed open; a daemon's first answer comes well inside it.
LISTENING_S = 8
# The longest line websocat takes whole. Its own default is 64 KiB, which a feed's first answer of
# a whole house, each device carrying all its source said of it, could pass as the house grows.
LONGEST_LINE = 4 * 1024 * 1024


@dataclass(frozen=True)
class Feed:
    """A page's feed: where it is, what the page's kit asks of it, and how long it has to say."""

    address: str
    asks: list[dict[str, object]]
    saying_s: int


def as_the_account(command: str) -> str:
    """Give a command run as the account in its own session, its manager reachable."""
    session = "export XDG_RUNTIME_DIR=/run/user/1000; "
    return f"runuser -u alice -- bash -c {shlex.quote(session + command)}"


def listened(machine: BaseMachine, feed: Feed) -> str:
    """Open `feed` as the page does, ask what its kit asks, and give all it heard."""
    said = " ".join(shlex.quote(json.dumps(ask)) for ask in feed.asks)
    # websocat's -H takes every word after it as a header, the URL included, so the page's
    # origin goes by its own option.
    _, heard = machine.execute(
        f"{{ printf '%s\\n' {said}; sleep {LISTENING_S}; }} "
        f"| websocat --text --buffer-size {LONGEST_LINE} --origin {shlex.quote(SITES)} "
        f"{shlex.quote(feed.address)} 2>&1"
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


def stands(
    machine: BaseMachine,
    feed: Feed,
    state: str,
    holds: Callable[[dict[str, Said]], bool] | None = None,
) -> list[Said]:
    """Listen until the feed says it stands at `state` with records that `holds`, if it's given.

    A broken feed tells no records, so only a state that holds them is asked for one. Any ask the
    server refused fails at once, since a page asking it would read nothing of that kind.
    """
    words = ""
    for _ in range(feed.saying_s // LISTENING_S):
        words = listened(machine, feed)
        heard = [json.loads(line) for line in words.splitlines() if line.startswith("{")]
        refused = [line for line in heard if "error" in line]
        if refused:
            pytest.fail(f"the server refused what the page asks: {refused}")
        if any(line.get("state") == state for line in heard) and (
            holds is None or holds(told(heard))
        ):
            return heard
    pytest.fail(f"the feed never said it was {state} as asked; last it said:\n{words}")


@pytest.fixture
def daemon_back(request: pytest.FixtureRequest, served: BaseMachine) -> Iterator[BaseMachine]:
    """Leave the test file's DAEMON units running again after a test that stops them.

    They're read before the test runs, so a file that doesn't name them fails before it stops any.
    """
    daemon = getattr(request.module, "DAEMON", None)
    if not daemon:
        pytest.fail(f"{request.module.__name__} names no DAEMON units to start again")
    units = " ".join(daemon)
    yield served
    served.succeed(as_the_account(f"systemctl --user start {units}"))

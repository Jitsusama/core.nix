"""What the sites' VM tests share: asking as the account, and a feed opened as a page opens it.

A module of the driver's Python, which each sites' VM test gives its driver as a pytest plugin
(system-page.nix, network-page.nix), so its fixture reaches the tests without their importing it,
and its helpers are imported by name. Each test's own file keeps its machine's readiness, its feed
and the units its daemon runs as, named DAEMON.
"""

from __future__ import annotations

import json
import shlex
from typing import TYPE_CHECKING

import pytest

if TYPE_CHECKING:
    from collections.abc import Iterator

    from test_driver.machine import BaseMachine

# Where the server listens, by the name a browser on the machine sends.
SITES = "http://127.0.0.1:8484"
# How long one listen keeps the feed open; a daemon's first answer comes well inside it.
LISTENING_S = 8


def as_the_account(command: str) -> str:
    """Give a command run as the account in its own session, its manager reachable."""
    session = "export XDG_RUNTIME_DIR=/run/user/1000; "
    return f"runuser -u alice -- bash -c {shlex.quote(session + command)}"


def listened(machine: BaseMachine, feed: str, asks: list[dict[str, object]]) -> str:
    """Open `feed` as the page does, ask what its kit asks, and give all it heard."""
    said = " ".join(shlex.quote(json.dumps(ask)) for ask in asks)
    # websocat's -H takes every word after it as a header, the URL included, so the page's
    # origin goes by its own option.
    _, heard = machine.execute(
        f"{{ printf '%s\\n' {said}; sleep {LISTENING_S}; }} "
        f"| websocat --text --origin {shlex.quote(SITES)} {feed} 2>&1"
    )
    return heard


@pytest.fixture
def daemon_back(request: pytest.FixtureRequest, served: BaseMachine) -> Iterator[BaseMachine]:
    """Leave the test file's DAEMON units running again after a test that stops them."""
    yield served
    units = " ".join(request.module.DAEMON)
    served.succeed(as_the_account(f"systemctl --user start {units}"))

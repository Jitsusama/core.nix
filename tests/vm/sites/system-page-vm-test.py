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

from typing import TYPE_CHECKING

import pytest
from sites_session import SITES, Feed, as_the_account, stands, told

if TYPE_CHECKING:
    from test_driver.machine import BaseMachine

# The feed, asked what the page's kit asks first: hello, then a watch of the one kind it reads
# first. It has past the bridge's first few tries at the daemon to say what changed.
FEED = Feed(
    address="ws://127.0.0.1:8484/feed/system",
    asks=[{"id": 1, "ask": "hello"}, {"id": 2, "ask": "watch", "kind": "summary"}],
    saying_s=90,
)
# The daemon's socket and its unit, as its home module names them, which sites_session's
# daemon_back starts again after a test that stops them.
DAEMON = ["system.socket", "system.service"]


@pytest.fixture(scope="module")
def served(machine: BaseMachine) -> BaseMachine:
    """Boot the machine with the account's manager running the sites' server and the daemon."""
    machine.wait_for_unit("multi-user.target")
    machine.wait_for_unit("user@1000.service")
    machine.wait_until_succeeds(as_the_account("systemctl --user is-active local-sites.service"))
    machine.wait_until_succeeds(as_the_account("systemctl --user is-active system.socket"))
    machine.wait_until_succeeds(f"curl -sf {SITES}/system/")
    return machine


def test_the_server_serves_the_system_page_and_its_kit(served: BaseMachine) -> None:
    """The page, its script, the kit it imports and the theme it reads are served by name.

    The theme's web kit is mounted once for every site, so it is asked for where
    the page reads it, beside the kit, as the machine mounts it.
    """
    page = served.succeed(f"curl -sf {SITES}/system/")
    assert '<nav class="sites-strip"' in page, (
        "the page was served without the strip its build writes"
    )
    served.succeed(f"curl -sf {SITES}/system/site.js")
    served.succeed(f"curl -sf {SITES}/local-sites/kit/feed.js")
    served.succeed(f"curl -sf {SITES}/local-sites/web/theme.css")
    served.succeed(f"curl -sf {SITES}/local-sites/web/fonts/fraunces.woff2 -o /dev/null")


def test_the_home_is_built_where_the_server_and_the_daemon_read_it(served: BaseMachine) -> None:
    """The home's unit builds into the login's runtime folder, which the server serves at its root.

    The unit's `--into=%t/...`, the server's home and the daemon's LOCAL_SITES_HOME each name
    that folder on their own, so only a machine running all three shows they agree.
    """
    served.wait_until_succeeds(
        as_the_account("test -f $XDG_RUNTIME_DIR/local-sites/home/rows.json")
    )
    page = served.succeed(f"curl -sf {SITES}/")
    assert "<h1" in page, "the home wasn't served at the server's root"
    rows = served.succeed(f"curl -sf {SITES}/rows.json")
    assert rows.lstrip().startswith(("[", "{")), f"the home's rows weren't JSON: {rows[:80]!r}"
    environment = served.succeed(as_the_account("systemctl --user show -p Environment system"))
    assert "LOCAL_SITES_HOME=/run/user/1000/local-sites/home" in environment, (
        f"the daemon reads its home elsewhere: {environment}"
    )


def test_the_feed_reaches_the_daemon_over_its_socket(served: BaseMachine) -> None:
    """The page's feed says live and hands over the daemon's summary, read over its socket."""
    heard = stands(served, FEED, "live")
    assert "summary" in told(heard), f"no summary in {heard}"


def test_a_daemon_stopped_is_said_broken_then_live(daemon_back: BaseMachine) -> None:
    """The feed says broken once the daemon and its socket stop, and live once the socket's back."""
    stands(daemon_back, FEED, "live")
    units = " ".join(DAEMON)
    daemon_back.succeed(as_the_account(f"systemctl --user stop {units}"))
    stands(daemon_back, FEED, "broken")
    daemon_back.succeed(as_the_account("systemctl --user start system.socket"))
    stands(daemon_back, FEED, "live")

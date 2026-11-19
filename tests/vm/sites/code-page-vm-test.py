"""The code page, as the sites' server serves it, reads the repos daemon its module runs.

code-page.nix, beside this, runs this through pytest inside the driver (tests/lib/vm_pytest.py),
on a machine whose account lingers with the local sites' and repos's home modules as they're
rendered for it, and one repository on its disk under a root of its own given to repos. Every ask
is one a browser on the machine makes: the page and its script fetched by the server's own name,
its feed's WebSocket opened from the page's origin, saying hello and watching a kind as the page's
kit does, and the code site's homes beside it. The feed must start the daemon through the
socket systemd holds for it and tell commits git makes while one watch stays open, and the homes
the daemon writes as it reads must say where the repository is home. The tests need no value from
Nix.
"""

from __future__ import annotations

import json
import shlex
from datetime import timedelta
from typing import TYPE_CHECKING, Any

import pytest
from vm_helpers import as_account

if TYPE_CHECKING:
    from test_driver.machine import BaseMachine

# A line the feed says, as JSON reads it.
type Said = dict[str, Any]

# Where the server listens, by the name a browser on the machine sends.
SITES = "http://127.0.0.1:8484"
FEED = "ws://127.0.0.1:8484/feed/repos"
# How long one listen keeps the feed open; the daemon's first answer comes well inside it.
LISTENING_S = 8
# How long the feed has to say live: past the bridge's first few tries at the daemon.
SAYING_S = 90
# How long a watch waits before a commit, so the whole records are told first, and after each,
# so the daemon's read within two seconds of git writing is told. One tick of its once-a-minute
# pace tells only the head it finds, so of commits this far apart at most one is told by it.
SETTLING_S = 4
# The commits made while a watch is open: three, so two must be told because git wrote, the
# second showing the daemon watches again after a reading.
WHILE_WATCHED = ["second", "third", "fourth"]
# Where what the commits made during a watch said goes, since the feed's own text is read.
MEANWHILE = "/root/meanwhile.log"
# How long the daemon has to write its homes: its first reading writes them, and its pace reads
# again a minute on.
HOMES = timedelta(minutes=1)
# The repository made on the disk, under the root code-page.nix gives repos instead of its own.
REPOSITORY = "/home/alice/code/kestrel"
# A tree cut from it once the daemon runs, and where Code is home to it, as jq's @uri writes it.
TREE = "/home/alice/trees/wing"
TREE_HOME = "code/#tree=%2Fhome%2Falice%2Ftrees%2Fwing"
# A commit's author, given each time, since the account has no git config of its own.
AUTHOR = "-c user.name=Ada -c user.email=ada@example.invalid"


def commit(message: str) -> str:
    """Give the command that commits nothing to the repository as the account, so git writes."""
    words = f"{AUTHOR} commit -q --allow-empty -m {shlex.quote(message)}"
    return f"runuser -u alice -- git -C {REPOSITORY} {words}"


def listening(machine: BaseMachine, kind: str, meanwhile: str = "") -> tuple[list[Said], str]:
    """Open the feed as the page does, watch `kind`, run `meanwhile`, and give what it heard.

    What it heard comes back as lines JSON reads and as websocat's own text, which says why
    when the feed never answered.
    """
    asks = [{"id": 1, "ask": "hello"}, {"id": 2, "ask": "watch", "kind": kind}]
    said = " ".join(shlex.quote(json.dumps(ask)) for ask in asks)
    during = f"{{ {meanwhile}; }} >>{MEANWHILE} 2>&1; " if meanwhile else ""
    # websocat's -H takes every word after it as a header, the URL included, so the page's
    # origin goes by its own option.
    _, heard = machine.execute(
        f"{{ printf '%s\\n' {said}; {during}sleep {LISTENING_S}; }} "
        f"| websocat --text --origin {shlex.quote(SITES)} {FEED} 2>&1"
    )
    lines = [json.loads(line) for line in heard.splitlines() if line.startswith("{")]
    return lines, heard


def told(heard: list[Said], part: str = "") -> list[Said]:
    """Every record the feed told in what was heard, whole or changed, or in `part` alone."""
    parts = [part] if part else ["records", "changed"]
    return [record for line in heard for name in parts for record in line.get(name, [])]


def stands(machine: BaseMachine, state: str, kind: str = "repository") -> list[Said]:
    """Listen until the feed says it stands at `state`, giving what was heard then."""
    text = ""
    for _ in range(SAYING_S // LISTENING_S):
        heard, text = listening(machine, kind)
        if any(line.get("state") == state for line in heard):
            return heard
    pytest.fail(f"the feed never said it was {state}; last it said:\n{text}")


@pytest.fixture(scope="module")
def served(machine: BaseMachine) -> BaseMachine:
    """Boot the machine with a repository on the disk and both units running for it."""
    machine.wait_for_unit("multi-user.target")
    machine.wait_for_unit("user@1000.service")
    as_account(machine, f"git init -q -b main {REPOSITORY}", account="alice")
    machine.succeed(commit("first"))
    # Started again once the repository is there, so its first reading holds it.
    as_account(machine, "systemctl --user restart repos.service", account="alice")
    machine.wait_for_unit("local-sites.service", "alice")
    machine.wait_for_unit("repos.socket", "alice")
    machine.wait_until_succeeds(f"curl -sf {SITES}/code/")
    return machine


def test_the_server_serves_the_code_page(served: BaseMachine) -> None:
    """The page and its script are served by the server's own name, the page with its strip."""
    page = served.succeed(f"curl -sf {SITES}/code/")
    assert '<nav class="sites-strip"' in page, (
        "the page was served without the strip its build writes"
    )
    served.succeed(f"curl -sf {SITES}/code/site.js")


def test_the_feed_starts_the_daemon_through_the_socket_systemd_holds(served: BaseMachine) -> None:
    """With the daemon stopped and its socket left, the feed starts it and says the repository.

    The daemon binds a socket of its own when it isn't handed one, so only a daemon stopped
    shows the feed reaching the one systemd holds.
    """
    as_account(served, "systemctl --user stop repos.service", account="alice")
    as_account(served, "systemctl --user is-active repos.socket", account="alice")
    # The record names every repository and tree, so the socket and its
    # folder are its account's alone, whatever the folders above allow.
    modes = served.succeed(
        "stat -c '%a %U' /run/user/1000/daemons /run/user/1000/daemons/repos.sock"
    )
    assert modes.split() == ["700", "alice", "600", "alice"], f"the socket is open: {modes}"
    heard = stands(served, "live")
    names = [record.get("name") for record in told(heard)]
    assert "kestrel" in names, f"the repository on the disk wasn't told: {heard}"
    as_account(served, "systemctl --user is-active repos.service", account="alice")


def test_commits_reach_an_open_watch_as_git_writes(served: BaseMachine) -> None:
    """Commits made while one watch is open are each told on it, unasked, as changed."""
    stands(served, "live", "checkout")
    pause = f"sleep {SETTLING_S}"
    meanwhile = "; ".join([pause, *(f"{commit(each)}; {pause}" for each in WHILE_WATCHED)])
    heard, text = listening(served, "checkout", meanwhile)
    made = served.succeed(f"cat {MEANWHILE}")
    log = f"git -C {REPOSITORY} log --format=%s -{len(WHILE_WATCHED)}"
    subjects = as_account(served, log, account="alice").split()
    assert subjects == WHILE_WATCHED[::-1], f"git made {subjects}, saying:\n{made}"
    listed = f"git -C {REPOSITORY} rev-list -{len(WHILE_WATCHED)} HEAD"
    heads = as_account(served, listed, account="alice").split()
    changed = {
        (record.get("head") or {}).get("commit")
        for record in told(heard, "changed")
        if record.get("path") == REPOSITORY
    }
    missed = [head for head in heads if head not in changed]
    assert not missed, f"the open watch never told the checkout at {missed}; it heard:\n{text}"


def test_the_code_sites_homes_say_where_the_repository_is_home(served: BaseMachine) -> None:
    """The daemon writes Code's homes as it reads, which the server answers beside the page."""
    # The fixture's restart read the repository, and each reading writes homes that changed.
    try:
        served.wait_until_succeeds(
            f"curl -sf {SITES}/code/homes.json | grep -qF kestrel", timeout=HOMES
        )
    except Exception:  # noqa: BLE001 # The driver raises a bare Exception when time is up.
        journal = "journalctl --user -u repos.service --no-pager"
        said = as_account(served, journal, account="alice")
        pytest.fail(f"no homes came; the daemon said:\n{said}")
    homes = json.loads(served.succeed(f"curl -sf {SITES}/code/homes.json"))
    addresses = {thing.get("name"): thing.get("address") for thing in homes.get("things", [])}
    assert addresses.get("kestrel") == "code/#repo=kestrel", f"no home for kestrel in {homes}"
    # A tree cut after the first reading comes only from a later one, so the homes move with the
    # disk rather than staying as the daemon first found them.
    as_account(served, f"git -C {REPOSITORY} worktree add -q {TREE}", account="alice")
    try:
        served.wait_until_succeeds(
            f"curl -sf {SITES}/code/homes.json | grep -qF {shlex.quote(TREE_HOME)}", timeout=HOMES
        )
    except Exception:  # noqa: BLE001 # The driver raises a bare Exception when time is up.
        now = served.succeed(f"curl -sf {SITES}/code/homes.json")
        pytest.fail(f"the tree cut never came home; the homes say:\n{now}")

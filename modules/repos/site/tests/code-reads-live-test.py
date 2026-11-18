"""Code reads live from the repos daemon and the node, through the sites' server, in a browser.

The page is the one Nix builds, served by the real local-sites server beside a
stand-in repos daemon and a stand-in node (modules/local-sites/tests/
stand_in_daemon.py), holding two repositories, their trees and a change under
test names (repos.json) and one cell at work in a tree (node.json), with the
sites' record, Code's homes and the map's homes among the documents. It must
say it's live with every repository drawn, grow a repository whose tree is
written now, drop a repository that goes, draw the cell at work and link it to
its home on the map, say each repository's weight and due upkeep in its row and
drawer, say broken when the daemon stops and live again, its repositories
drawn, when it's back, say stale when the server goes, move its ages on while
nothing is written, make today's growth yesterday's once the day turns, and pass
the web-design skill's page checks and its homes check at a phone's width and
a desk's in both looks. Anything the page logs fails it, but for the browser's
own lines once the test takes the server away. The server, the browser and the
tab are modules/local-sites/tests/live_page.py's.
"""

import json
import os
from datetime import UTC, datetime
from pathlib import Path
from typing import TYPE_CHECKING

import pytest
from live_page import LOOKS, SIZES, Opened, Served, serving
from playwright.sync_api import Browser, BrowserContext, Locator, Page, expect
from stand_in_daemon import StandIn, standing_in

if TYPE_CHECKING:
    from collections.abc import Iterator

HERE = Path(__file__).parent
# Code draws once a frame after a record moves, so a change must show well inside this.
PACE_MS = 6000
# The server watches a gone daemon again every 2 seconds, so its return shows inside this.
BACK_MS = 15000
# The browser's own lines for a feed that couldn't connect and a fetch that
# failed, once the server is gone: cut off in flight, or refused when the page's
# second read of its homes starts after.
UNREACHED = ["WebSocket connection to", "Failed to load resource: net::ERR_"]
# The kinds the repos daemon serves, each told whole, as home.nix declares them.
KINDS = ["reading", "repository", "checkout", "tree", "change"]
# The sites' record as the home keeps it, each row named for people. The site
# home to cells sits at a path of its own, and the row named and served as the
# map claims nothing, so a cell links home only by the row that claims cells.
CELLS_SITE = "cells/"
SITES = {
    "sites": [
        {"path": "code/", "name": "Code", "homes": ["repository", "tree"]},
        {"path": "mycelium/", "name": "mycelium"},
        {"path": CELLS_SITE, "name": "Your cells, now", "homes": ["cell"]},
    ]
}
# Where the cells' site says the cell at work is home.
CELL_HOME = f"{CELLS_SITE}#cell=E2ZIAQ"
# Where Code is home to each repository and present tree in repos.json, named as
# code-site.sh names them.
CODE_HOMES = [
    {
        "kind": "repository",
        "key": "github:owl/core",
        "name": "core",
        "names": ["core"],
        "address": "code/#repo=owl%2Fcore",
    },
    {
        "kind": "repository",
        "key": "local:~/src/notes",
        "name": "notes",
        "names": ["notes"],
        "address": "code/#repo=notes",
    },
    {
        "kind": "tree",
        "key": "/home/ada/trees/grove",
        "name": "/home/ada/trees/grove",
        "names": ["/home/ada/trees/grove"],
        "address": "code/#tree=%2Fhome%2Fada%2Ftrees%2Fgrove",
    },
    {
        "kind": "tree",
        "key": "/home/ada/trees/old",
        "name": "/home/ada/trees/old",
        "names": ["/home/ada/trees/old"],
        "address": "code/#tree=%2Fhome%2Fada%2Ftrees%2Fold",
    },
]


@pytest.fixture
def repos_daemon(tmp_path: Path) -> Iterator[StandIn]:
    yield from standing_in(tmp_path / "repos.sock", "repos", HERE / "repos.json")


@pytest.fixture
def mycelium_node(tmp_path: Path) -> Iterator[StandIn]:
    yield from standing_in(tmp_path / "node.sock", "node", HERE / "node.json")


@pytest.fixture
def code_served(tmp_path: Path, repos_daemon: StandIn, mycelium_node: StandIn) -> Iterator[Served]:
    feeds = {
        "repos": {"socket": str(repos_daemon.path), "kinds": {kind: {} for kind in KINDS}},
        # home.nix lets cells through whole, and Code asks only the fields it draws.
        "node": {"socket": str(mycelium_node.path), "kinds": {"cell": {}}},
    }
    with serving(tmp_path, {"code": os.environ["CODE_SITE"]}, feeds) as served:
        documents = tmp_path / "documents"
        (documents / "sites.json").write_text(json.dumps(SITES))
        (documents / CELLS_SITE).mkdir()
        thing = {"kind": "cell", "key": "E2ZIAQ", "name": "E2ZIAQ", "address": CELL_HOME}
        (documents / CELLS_SITE / "homes.json").write_text(json.dumps({"things": [thing]}))
        # Code's own homes, as code-site.sh writes them among the documents, which the
        # server answers for the mount: the repositories and the present trees.
        (documents / "code").mkdir()
        (documents / "code" / "homes.json").write_text(json.dumps({"things": CODE_HOMES}))
        yield served


@pytest.fixture
def code_opened(browser: Browser, code_served: Served) -> Iterator[Opened]:
    root, _ = code_served
    opened = Opened(browser, f"{root}code/", SIZES[-1], "dark")
    yield opened
    opened.close()


@pytest.fixture
def code_page(code_opened: Opened) -> Page:
    return code_opened.page


def says_it_stands(page: Page, state: str, timeout: int = PACE_MS) -> None:
    expect(page.locator("#asof"), f"Code never said it was {state}").to_have_attribute(
        "data-state", state, timeout=timeout
    )


def now() -> str:
    return datetime.now(UTC).isoformat().replace("+00:00", "Z")


def row(page: Page, key: str) -> Locator:
    return page.locator(f'#list .row[data-key="{key}"]')


def grove_written_now(repos_daemon: StandIn, at: str | None = None) -> None:
    """Write the grove tree now, or `at`, as a cell at work does, so the drawer shows it open."""
    written = now() if at is None else at
    repos_daemon.tell(
        "tree", changed=[{"key": "tree:/home/ada/trees/grove", "written_at": written}]
    )


def clocked(browser: Browser, root: str, at: str) -> tuple[BrowserContext, Page, list[str]]:
    """Open Code on a clock of its own, set to `at` in UTC.

    The clock is set before the page loads so its timers run on it, and every line the page logs
    is kept, with what it throws.
    """
    context = browser.new_context(timezone_id="UTC")
    logged: list[str] = []
    page = context.new_page()
    page.on("console", lambda message: logged.append(f"{message.type}: {message.text}"))
    page.on("pageerror", lambda error: logged.append(f"thrown: {error}"))
    page.clock.install(time=at)
    page.goto(f"{root}code/")
    says_it_stands(page, "live")
    return context, page, logged


def noise(logged: list[str]) -> list[str]:
    return [line for line in logged if line.startswith(("warning:", "error:", "thrown:"))]


def test_code_draws_every_repository_live(code_page: Page) -> None:
    says_it_stands(code_page, "live")
    for key in ["github:owl/core", "local:~/src/notes"]:
        expect(row(code_page, key), f"Code drew no row for {key}").to_be_visible()
    expect(code_page.locator("#asof"), "Code didn't say where it read").to_contain_text("kestrel")
    expect(code_page.locator("#n-every"), "Code miscounted the repositories").to_have_text("2")
    expect(code_page.locator("#n-growing"), "a resting grove was said growing").to_have_text("0")


def test_a_key_tapped_at_the_head_does_what_pressing_it_does(code_page: Page) -> None:
    says_it_stands(code_page, "live")
    chosen = code_page.locator('#list .row[aria-selected="true"]')
    expect(chosen, "Code chose no row to start on").to_have_count(1)
    first = chosen.get_attribute("data-key")
    head = code_page.locator(".head .keys")
    head.get_by_text("j", exact=True).click()
    expect(chosen, "a tapped j didn't step to the next row").not_to_have_attribute(
        "data-key", first or ""
    )
    expect(code_page.locator("#list"), "the list didn't take the keys back").to_be_focused()
    head.get_by_text("k", exact=True).click()
    expect(chosen, "a tapped k didn't step back").to_have_attribute("data-key", first or "")
    head.get_by_text("l", exact=True).click()
    expect(chosen, "a tapped l didn't walk the grove").not_to_have_attribute(
        "data-key", first or ""
    )
    expect(code_page.locator("#grove"), "a tapped l left the grove without keys").to_be_focused()


def test_return_on_a_key_at_the_head_presses_only_that_key(
    code_page: Page, repos_daemon: StandIn
) -> None:
    says_it_stands(code_page, "live")
    core = "github:owl/core"
    # Core sorts first, ahead of notes, so k at the head keeps the cursor on it.
    expect(code_page.locator("#list .row").first).to_have_attribute("data-key", core)
    # A tree written now is drawn open in core's drawer, for Return to step into.
    grove_written_now(repos_daemon)
    trees = code_page.locator("#open .tree")
    expect(trees.first, "core's drawer drew no tree to step into").to_be_visible(timeout=PACE_MS)
    head = code_page.locator(".head .keys")
    stepped_in = code_page.locator('#open .tree[aria-current="true"]')
    head.get_by_text("k", exact=True).focus()
    code_page.keyboard.press("Enter")
    expect(row(code_page, core), "Return on k moved off core").to_have_attribute(
        "aria-selected", "true"
    )
    expect(stepped_in, "Return on k stepped in too").to_have_count(0)
    head.get_by_text("Return", exact=True).focus()
    code_page.keyboard.press("Enter")
    expect(stepped_in, "Return on the head's Return didn't step in").to_have_count(1)
    expect(stepped_in, "the tree stepped into lost the keys to the list").to_be_focused()


def test_a_tree_written_now_grows_its_repository(code_page: Page, repos_daemon: StandIn) -> None:
    says_it_stands(code_page, "live")
    written: dict[str, object] = {"key": "tree:/home/ada/trees/old", "written_at": now()}
    repos_daemon.tell("tree", changed=[written])
    expect(code_page.locator("#n-growing"), "a tree written now grew nothing").to_have_text(
        "1", timeout=PACE_MS
    )


def test_a_repository_that_goes_leaves_the_list(code_page: Page, repos_daemon: StandIn) -> None:
    says_it_stands(code_page, "live")
    notes = row(code_page, "local:~/src/notes")
    expect(notes, "Code drew no row for notes").to_be_visible()
    repos_daemon.tell("repository", gone=["repository:local:~/src/notes"])
    expect(notes, "the gone repository was still drawn").to_have_count(0, timeout=PACE_MS)
    expect(row(code_page, "github:owl/core"), "the one left went too").to_be_visible()


def test_a_repositorys_weight_and_due_upkeep_are_said(code_page: Page) -> None:
    says_it_stands(code_page, "live")
    # Only core's checkout weighs anything, reckoned from one folder, and git
    # would gather it now.
    expect(row(code_page, "github:owl/core"), "core's weight wasn't said").to_contain_text(
        "about 2.5 GB, upkeep due"
    )
    expect(code_page.locator("#open"), "the drawer didn't say why").to_contain_text(
        "about 2.5 GB in git: 7,168 loose, 11 packs; git's upkeep is due"
    )


def test_a_cell_at_work_is_drawn_in_its_repository(code_page: Page) -> None:
    says_it_stands(code_page, "live")
    expect(row(code_page, "github:owl/core"), "the cell at work wasn't drawn").to_contain_text(
        "1 cell", timeout=PACE_MS
    )


def test_a_cell_at_work_links_to_its_home_on_the_map(
    code_page: Page, repos_daemon: StandIn
) -> None:
    says_it_stands(code_page, "live")
    # A tree written today is drawn open in the drawer, where its cells are named; an older
    # one folds shut.
    grove_written_now(repos_daemon)
    home = code_page.locator(f'#open a[href="../{CELL_HOME}"]')
    expect(home, "the cell at work wasn't linked to its home").to_contain_text(
        "E2ZIAQ", timeout=PACE_MS
    )


def test_a_repos_daemon_that_stops_is_said_broken_then_live(
    code_page: Page, repos_daemon: StandIn
) -> None:
    says_it_stands(code_page, "live")
    repos_daemon.leave()
    says_it_stands(code_page, "broken")
    # What the daemon holds moves while it's away, so only reading it again shows it.
    repos_daemon.tell("repository", gone=["repository:local:~/src/notes"])
    repos_daemon.start()
    says_it_stands(code_page, "live", BACK_MS)
    expect(row(code_page, "local:~/src/notes"), "Code didn't read the daemon again").to_have_count(
        0, timeout=PACE_MS
    )
    expect(row(code_page, "github:owl/core"), "core wasn't drawn again").to_be_visible()
    expect(code_page.locator("#n-every"), "the count didn't follow").to_have_text("1")


def test_a_sites_server_that_goes_is_said_stale(code_opened: Opened, code_served: Served) -> None:
    code_page = code_opened.page
    says_it_stands(code_page, "live")
    # Once the server's gone, the browser logs the feed's next try, and any of
    # the page's reads of its homes not yet answered, as lines of its own.
    for words in UNREACHED:
        code_opened.expecting(words)
    _, server = code_served
    server.kill()
    says_it_stands(code_page, "stale")


def test_ages_move_on_while_nothing_is_written(
    browser: Browser, code_served: Served, repos_daemon: StandIn
) -> None:
    root, _ = code_served
    # Two hours on from mid-morning stay within the day, so only the ages may change.
    at = "2026-11-17T10:00:00Z"
    context, page, logged = clocked(browser, root, at)
    try:
        repos_daemon.tell(
            "repository", changed=[{"key": "repository:github:owl/core", "written_at": at}]
        )
        grove_written_now(repos_daemon, at)
        when = row(page, "github:owl/core").locator(".when")
        expect(when, "a repository written now wasn't said so").to_have_text("1m", timeout=PACE_MS)
        trees = page.locator("#open .tree")
        expect(trees.first, "core's drawer drew no tree to step into").to_be_visible(
            timeout=PACE_MS
        )
        # A tree stepped into holds focus. Marking the element itself shows the minute told its
        # age in place: a redraw would focus a new element, the same tree drawn again.
        stepped_in = page.locator('#open .tree[aria-current="true"]')
        page.locator(".head .keys").get_by_text("Return", exact=True).focus()
        page.keyboard.press("Enter")
        expect(stepped_in, "Return didn't step into the tree").to_be_focused()
        stepped_in.evaluate("tree => { tree.kept = true; }")
        page.clock.fast_forward("02:00:00")
        expect(when, "an age stood still while nothing was written").to_have_text(
            "2h", timeout=PACE_MS
        )
        expect(stepped_in.locator(".when"), "a tree's age stood still").to_have_text("2h")
        expect(stepped_in, "telling the ages again took focus away").to_be_focused()
        kept = page.evaluate("() => document.activeElement?.kept === true")
        assert kept, "the minute drew the focused tree again rather than telling its age"
        assert noise(logged) == [], f"Code logged while its ages moved on: {noise(logged)}"
    finally:
        context.close()


def test_the_day_turning_makes_todays_growth_yesterdays(
    browser: Browser, code_served: Served, repos_daemon: StandIn
) -> None:
    root, _ = code_served
    at = "2026-11-17T23:30:00Z"
    context, page, logged = clocked(browser, root, at)
    try:
        grove_written_now(repos_daemon, at)
        growing = page.locator("#n-growing")
        expect(growing, "a tree written now grew nothing").to_have_text("1", timeout=PACE_MS)
        page.clock.fast_forward("01:00:00")
        expect(growing, "yesterday's tree still grew today").to_have_text("0", timeout=PACE_MS)
        assert noise(logged) == [], f"Code logged as the day turned: {noise(logged)}"
    finally:
        context.close()


@pytest.mark.parametrize("look", LOOKS)
@pytest.mark.parametrize("size", SIZES)
def test_code_passes_the_skills_page_checks(
    browser: Browser,
    code_served: Served,
    repos_daemon: StandIn,
    size: tuple[int, int],
    look: str,
) -> None:
    root, _ = code_served
    # Checked with a tree open in the drawer and its cell named, not every tree folded.
    grove_written_now(repos_daemon)
    opened = Opened(browser, f"{root}code/", size, look)
    try:
        says_it_stands(opened.page, "live")
        linked = opened.page.locator(f'#open a[href="../{CELL_HOME}"]')
        expect(linked, "the checks would read no cell").to_have_count(1, timeout=PACE_MS)
        opened.passes_the_skills_checks()
        # The skill's link check, which the shared checks leave out since only a page with
        # homes can pass it: every repository, tree and cell Code names links home.
        unlinked = opened.page.evaluate((Path(os.environ["WEB_SKILL"]) / "homes.js").read_text())
        assert unlinked == [], f"homes.js at {size} in the {look} look found {unlinked}"
    finally:
        # Closed whatever failed, so what the page logged is said too.
        opened.close()

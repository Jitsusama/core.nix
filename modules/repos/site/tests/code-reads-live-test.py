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
nothing is written, make today's growth yesterday's once the day turns, move no
row or tree as the cursor steps, keep focus where it was through a redraw, let
Tab reach the head's keys once, not shift as its feeds arrive on a reload, nor
hold room for what a reload forgets or at another width, draw a
second's burst of records a few times rather than each, draw a thread for
each cell joining two repositories on the map under the work and walk it by
keys, and pass the web-design skill's page checks, its homes check and its
walked map's check at a phone's width and a desk's in both looks. Anything
the page logs fails it, but for the browser's own lines once the test takes
the server away. The server, the browser and the
tab are modules/local-sites/tests/live_page.py's.

Long because each case is one thing the page does, on the same server, stand-ins
and browser, and the fixtures that start them live here alone.
"""

import json
import math
import os
import re
import time
from datetime import UTC, datetime
from pathlib import Path
from typing import TYPE_CHECKING

import pytest
from live_page import LOOKS, SIZES, Budget, Opened, Served, serving
from playwright.sync_api import Browser, BrowserContext, Locator, Page, expect
from stand_in_daemon import StandIn, standing_in

if TYPE_CHECKING:
    from collections.abc import Iterator

HERE = Path(__file__).parent
# Code draws on the frame after a record moves, then at most twice a second while records keep
# coming, so a change must show well inside this.
PACE_MS = 6000
# Long enough for Code to ask three times again for a cell with no home, 14 s, with room.
ASKED_AGAIN_MS = 20_000
# How far the page may shift unasked: a tenth of the web's "good" bound, which a font
# swapping in stays under, and a row or a sentence arriving doesn't.
STILL = 0.01
# How long page.ts holds the last visit's room before letting it go anyway.
ROOM_HELD_MS = 3000
# A phone's window short enough that an open drawer runs past its foot.
SHORT_PHONE = 360
# Core's resting tree, in a season that stays folded until opened.
OLD = "/home/ada/trees/old"
# Code draws a burst of records at most twice a second, so each second of one may draw this
# often, with two draws over for its first record and its last.
DRAWS_A_SECOND = 2
# The server watches a gone daemon again every 2 seconds, so its return shows inside this.
BACK_MS = 15000
# The kinds the repos daemon serves, each told whole, as home.nix declares them.
KINDS = ["reading", "repository", "checkout", "tree", "change", "using"]
# What a first visit to Code may cost, drawn from the stand-in's repositories: two of the files
# are the map of ties, its view and its logic, one says what works in each repository, and the
# kilobytes and elements count its section, its keys' lines in the list of keys and the tying
# cell's link too, two of the kilobytes lead each tree's cells on to the other repositories
# they work across, and two say what works unread where the daemon's reading stopped. One
# element is each drawer's way to its row in the launcher, where its acts are. Three of the files
# and four of the kilobytes are what keeps a draw light: its pace, the trees a drawer lists and
# the foot's reasons. One kilobyte is the theme's 12 px floor on each step of its type, which
# every page reads. One file, two of the kilobytes and four elements are the kit's head, which
# every live page shares: head.css less the page's own head rules, and the head itself, its
# title's emphasis, the eyebrow's mark, the standing line's glyph and the quiet row, less the
# foot the standing line came from. One file more is the kit's early.js, the script the page
# once wrote into itself, which the sites' policy now runs only from a file.
BUDGET = Budget(files=39, kilobytes=538, elements=280)
CELLS_SITE = "cells/"


def sites_homing_cells(claims: list[object]) -> dict[str, list[dict[str, object]]]:
    """Give the sites' rows, each named for people, with `claims` as the cells site's homes.

    The site home to cells sits at a path of its own, and the row named and served as the map
    claims nothing, so a cell links home only by the row that claims cells.
    """
    return {
        "sites": [
            {"path": "code/", "name": "Code", "homes": ["repository", "tree"]},
            {"path": "mycelium/", "name": "mycelium"},
            {"path": CELLS_SITE, "name": "Your cells, now", "homes": claims},
        ]
    }


# The rows as the home lists them, its declared rows as declared.
SITES = sites_homing_cells(["cell"])
# The sites' record as a session last wrote it, before the home's declared rows took the place
# of its live ones: its cells row claims nothing, so a reader of it links no cell home.
WRITTEN = sites_homing_cells([])
# Where the cells' site says the cell at work is home.
CELL_HOME = f"{CELLS_SITE}#cell=E2ZIAQ"
# Where Code is home to each repository and present tree in repos.json, named as
# the repos daemon names them (modules/repos/src/homes.rs).
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
        (documents / "sites.json").write_text(json.dumps(WRITTEN))
        # The rows the home lists, the declared ones as declared, which it writes beside the
        # record for the readers of a row's homes.
        (documents / "rows.json").write_text(json.dumps(SITES))
        (documents / CELLS_SITE).mkdir()
        thing = {"kind": "cell", "key": "E2ZIAQ", "name": "E2ZIAQ", "address": CELL_HOME}
        (documents / CELLS_SITE / "homes.json").write_text(json.dumps({"things": [thing]}))
        # Code's own homes, as the repos daemon writes them among the documents, which the
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


# The rows of the map of how the repositories join.
TIE_ROWS = "#ties-map .tie-row"


def spanning(mycelium_node: StandIn) -> None:
    """Start a cell at work in core whose networks name notes too, so the map ties the two."""
    cell: dict[str, object] = {
        "key": "cell:W9TIES",
        "name": "W9TIES",
        "cwd": "/home/ada/trees/grove",
        "title": "Join the notes to the grove",
        "standing": "moving",
        "networks": ["repo.core", "repo.notes"],
    }
    mycelium_node.tell("cell", changed=[cell])


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


def test_a_seasons_button_opens_its_trees_and_keeps_focus_in_the_drawer(code_page: Page) -> None:
    says_it_stands(code_page, "live")
    row(code_page, "github:owl/core").click()
    trees = code_page.locator("#open .tree")
    expect(trees, "core's folded trees were drawn before they were asked for").to_have_count(0)
    code_page.evaluate(
        """() => {
          globalThis.listFocused = false;
          document.querySelector("#list").addEventListener("focus", () => {
            globalThis.listFocused = true;
          });
        }"""
    )
    code_page.get_by_role("button", name="Open them").first.click()
    expect(trees, "the button didn't open the folded trees").to_have_count(2)
    expect(
        code_page.locator(".drawer:focus, .drawer :focus"), "focus left the drawer"
    ).to_have_count(1)
    assert not code_page.evaluate("() => globalThis.listFocused"), "focus went by the list"


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


def test_what_works_in_a_repository_is_said(code_page: Page) -> None:
    says_it_stands(code_page, "live")
    # Three of Ada's processes work in core, one in its checkout and two in the grove, whose
    # season folds it shut, and nothing works in notes, whose row stays quiet.
    expect(row(code_page, "github:owl/core"), "what works in core wasn't said").to_contain_text(
        "40% of a core, 1.3\u00a0GB of memory"
    )
    expect(row(code_page, "local:~/src/notes"), "notes said a use").not_to_contain_text("memory")
    expect(code_page.locator("#open"), "the checkout's use wasn't said").to_contain_text(
        "1\u00a0process: 10% of a core, 300\u00a0MB of memory"
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


def test_a_home_without_its_rows_has_cells_linked_by_its_record(
    code_opened: Opened, repos_daemon: StandIn, tmp_path: Path
) -> None:
    # A home from before it listed its rows apart keeps every row in its record.
    documents = tmp_path / "documents"
    (documents / "rows.json").unlink()
    (documents / "sites.json").write_text(json.dumps(SITES))
    code_opened.expecting("the server responded with a status of 404")
    code_page = code_opened.page
    code_page.reload()
    says_it_stands(code_page, "live")
    grove_written_now(repos_daemon)
    home = code_page.locator(f'#open a[href="../{CELL_HOME}"]')
    expect(home, "a record's cells row didn't link the cell home").to_contain_text(
        "E2ZIAQ", timeout=PACE_MS
    )


def test_a_cell_is_linked_home_by_its_sites_pattern(
    code_page: Page, repos_daemon: StandIn, tmp_path: Path
) -> None:
    # The page first opened reads the cells' record, so it's let finish before the record goes,
    # or its read is answered not found and the page logs it.
    says_it_stands(code_page, "live")
    grove_written_now(repos_daemon)
    home = code_page.locator(f'#open a[href="../{CELL_HOME}"]')
    expect(home, "the record didn't link the cell home").to_contain_text("E2ZIAQ", timeout=PACE_MS)
    # A site whose every cell has its page there says so as a pattern, keeping no record.
    documents = tmp_path / "documents"
    (documents / CELLS_SITE / "homes.json").unlink()
    pattern = sites_homing_cells([{"kind": "cell", "address": "#cell={name}"}])
    (documents / "rows.json").write_text(json.dumps(pattern))
    code_page.reload()
    says_it_stands(code_page, "live")
    grove_written_now(repos_daemon)
    expect(home, "the pattern didn't link the cell home").to_contain_text("E2ZIAQ", timeout=PACE_MS)


def test_a_cell_that_starts_once_code_is_open_links_to_its_home(
    code_page: Page, repos_daemon: StandIn, mycelium_node: StandIn, tmp_path: Path
) -> None:
    says_it_stands(code_page, "live")
    grove_written_now(repos_daemon)
    expect(code_page.locator(f'#open a[href="../{CELL_HOME}"]')).to_contain_text(
        "E2ZIAQ", timeout=PACE_MS
    )
    started: dict[str, object] = {
        "key": "cell:Q3NEWS",
        "name": "Q3NEWS",
        "cwd": "/home/ada/trees/grove/modules",
        "title": "Weigh the grove",
        "standing": "moving",
        "networks": ["repo.core"],
    }
    mycelium_node.tell("cell", changed=[started])
    # The cell is drawn before its site lists it, so the ask its start brings finds no home.
    expect(code_page.locator("#open"), "the cell that started wasn't drawn").to_contain_text(
        "Q3NEWS", timeout=PACE_MS
    )
    things = [
        {"kind": "cell", "key": key, "name": key, "address": f"{CELLS_SITE}#cell={key}"}
        for key in ("E2ZIAQ", "Q3NEWS")
    ]
    (tmp_path / "documents" / CELLS_SITE / "homes.json").write_text(json.dumps({"things": things}))
    home = code_page.locator(f'#open a[href="../{CELLS_SITE}#cell=Q3NEWS"]')
    # Code asks again 2 s after, then 4 s and 8 s, so the link lands within the third wait.
    expect(home, "a cell that started later wasn't linked home").to_contain_text(
        "Q3NEWS", timeout=ASKED_AGAIN_MS
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
    _, server = code_served
    code_opened.loses(server)
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


def test_stepping_the_cursor_moves_no_row(code_page: Page) -> None:
    says_it_stands(code_page, "live")
    rows = code_page.locator("#list .row")
    expect(rows, "Code drew no rows to step through").to_have_count(2)
    rows.first.click()
    # Where each row stands and how tall it is: the cursor moving must change neither.
    boxes = (
        "() => [...document.querySelectorAll('#list .row')]"
        ".map(each => [each.offsetTop, each.offsetHeight])"
    )
    before = code_page.evaluate(boxes)
    code_page.keyboard.press("j")
    expect(rows.nth(1), "j didn't step to the next row").to_have_attribute("aria-selected", "true")
    expect(rows.nth(1).locator(".row-keys"), "the cursor's keys weren't shown").to_be_visible()
    assert code_page.evaluate(boxes) == before, "stepping the cursor moved the rows"


def test_stepping_through_the_trees_moves_no_tree(code_page: Page, repos_daemon: StandIn) -> None:
    says_it_stands(code_page, "live")
    for path in ["grove", "old"]:
        repos_daemon.tell(
            "tree", changed=[{"key": f"tree:/home/ada/trees/{path}", "written_at": now()}]
        )
    trees = code_page.locator("#open .tree")
    expect(trees, "core's drawer didn't draw both trees open").to_have_count(2, timeout=PACE_MS)
    row(code_page, "github:owl/core").click()
    boxes = (
        "() => [...document.querySelectorAll('#open .tree')]"
        ".map(each => [each.offsetTop, each.offsetHeight])"
    )
    before = code_page.evaluate(boxes)
    code_page.keyboard.press("Enter")
    stepped_in = code_page.locator('#open .tree[aria-current="true"]')
    expect(stepped_in, "Return didn't step into the first tree").to_be_focused()
    expect(code_page.locator(".drawer-keys"), "the trees' keys weren't shown").to_be_visible()
    assert code_page.evaluate(boxes) == before, "stepping in moved the trees"
    code_page.keyboard.press("j")
    expect(trees.nth(1), "j didn't step to the next tree").to_have_attribute("aria-current", "true")
    assert code_page.evaluate(boxes) == before, "stepping through the trees moved them"


def test_a_phone_shows_the_trees_keys_at_the_foot_of_the_window(
    code_page: Page, repos_daemon: StandIn
) -> None:
    says_it_stands(code_page, "live")
    code_page.set_viewport_size({"width": 390, "height": SHORT_PHONE})
    for path in ["grove", "old"]:
        repos_daemon.tell(
            "tree", changed=[{"key": f"tree:/home/ada/trees/{path}", "written_at": now()}]
        )
    expect(code_page.locator("#open .tree"), "core's trees weren't drawn open").to_have_count(
        2, timeout=PACE_MS
    )
    row(code_page, "github:owl/core").click()
    code_page.keyboard.press("Enter")
    stepped_in = code_page.locator('#open .tree[aria-current="true"]')
    expect(stepped_in, "Return didn't step into the first tree").to_be_focused()
    # The drawer's head at the window's top, so its foot lies below the window.
    drawer = code_page.locator(".drawer")
    code_page.evaluate(
        "() => scrollTo(0, document.querySelector('.drawer').getBoundingClientRect().top + scrollY)"
    )
    box = drawer.bounding_box()
    assert box is not None, "the drawer wasn't drawn"
    assert box["height"] > SHORT_PHONE, "the drawer fit the window, proving nothing"
    line = code_page.locator(".drawer-keys").bounding_box()
    assert line is not None, "the trees' keys weren't drawn"
    foot = line["y"] + line["height"]
    assert foot <= SHORT_PHONE + 1, f"the trees' keys sat below the window, at {line}"
    assert foot >= SHORT_PHONE - 1, f"the trees' keys sat above the window's foot, at {line}"


def test_a_redraw_keeps_focus_where_it_was(code_page: Page, repos_daemon: StandIn) -> None:
    says_it_stands(code_page, "live")
    every = code_page.locator('#lede a[data-place="every"]')
    every.focus()
    # A tree written now makes core grow, which the lede says again around the link.
    grove_written_now(repos_daemon)
    expect(code_page.locator("#lede"), "the lede wasn't said again").to_contain_text(
        "1 growing", timeout=PACE_MS
    )
    expect(every, "the lede said again lost the link focused in it").to_be_focused()


def redrawn_drawer(code_page: Page, repos_daemon: StandIn) -> None:
    """Write the grove tree again, and wait for the drawer to be drawn anew for it."""
    code_page.evaluate("() => { document.querySelector('#open .tree').dataset.before = ''; }")
    grove_written_now(repos_daemon)
    code_page.wait_for_function(
        "() => document.querySelector('#open [data-before]') === null", timeout=PACE_MS
    )


def test_a_redraw_keeps_focus_on_a_link_in_a_tree(code_page: Page, repos_daemon: StandIn) -> None:
    says_it_stands(code_page, "live")
    grove_written_now(repos_daemon)
    home = code_page.locator(f'#open a[href="../{CELL_HOME}"]').first
    expect(home, "the cell's home wasn't linked").to_contain_text("E2ZIAQ", timeout=PACE_MS)
    row(code_page, "github:owl/core").click()
    code_page.keyboard.press("Enter")
    expect(code_page.locator('#open .tree[aria-current="true"]')).to_be_focused()
    home.focus()
    redrawn_drawer(code_page, repos_daemon)
    expect(home, "a redraw took focus off the link to the tree").to_be_focused()


def test_focus_stays_in_the_drawer_when_the_tree_holding_it_goes(
    code_page: Page, repos_daemon: StandIn
) -> None:
    says_it_stands(code_page, "live")
    grove_written_now(repos_daemon)
    home = code_page.locator(f'#open a[href="../{CELL_HOME}"]').first
    expect(home, "the cell's home wasn't linked").to_contain_text("E2ZIAQ", timeout=PACE_MS)
    row(code_page, "github:owl/core").click()
    code_page.keyboard.press("Enter")
    expect(code_page, "the address didn't name the tree stepped into").to_have_url(
        re.compile("#tree=")
    )
    home.focus()
    repos_daemon.tell("tree", gone=["tree:/home/ada/trees/grove"])
    expect(home, "the tree stayed in the drawer").to_have_count(0, timeout=PACE_MS)
    held = code_page.evaluate("() => document.activeElement.closest('.drawer') !== null")
    assert held, "focus fell out of the drawer with the tree that held it"
    expect(code_page, "the address still named the tree gone").not_to_have_url(re.compile("#tree="))


def test_tab_reaches_the_keys_at_the_head_once(code_page: Page) -> None:
    says_it_stands(code_page, "live")
    keys = code_page.locator(".head .keys [role=button]")
    expect(keys, "the head's keys weren't made pressable").to_have_count(6)
    # The list's cursor stands on its first row for End and on its last for Home, so a key that
    # let either through to the list would move it.
    rows = code_page.locator("#list .row")
    expect(rows, "the list didn't draw both repositories").to_have_count(2)
    chosen = code_page.locator('#list .row[aria-selected="true"]')
    expect(chosen, "the list's cursor didn't stand on its first row").to_have_attribute(
        "data-key", rows.first.get_attribute("data-key") or ""
    )
    keys.first.focus()
    code_page.keyboard.press("ArrowRight")
    expect(keys.nth(1), "the right arrow didn't move to the next key").to_be_focused()
    code_page.keyboard.press("End")
    expect(keys.last, "End didn't move to the last key").to_be_focused()
    first = rows.first.get_attribute("data-key") or ""
    expect(chosen, "End on a key moved the list's cursor too").to_have_attribute("data-key", first)
    code_page.locator("#list").focus()
    code_page.keyboard.press("j")
    last = rows.last.get_attribute("data-key") or ""
    expect(chosen, "j didn't step the list's cursor on").to_have_attribute("data-key", last)
    keys.nth(2).focus()
    code_page.keyboard.press("Home")
    expect(keys.first, "Home didn't move to the first key").to_be_focused()
    expect(chosen, "Home on a key moved the list's cursor too").to_have_attribute("data-key", last)
    # A chord is the browser's or the reader's, never a step between the keys, nor the page's.
    code_page.keyboard.press("Alt+ArrowRight")
    expect(keys.first, "a key took Alt with the arrow as its own step").to_be_focused()
    code_page.keyboard.press("Shift+ArrowRight")
    expect(keys.first, "Shift with an arrow on a key went to the page").to_be_focused()
    code_page.keyboard.press("ArrowRight")
    code_page.keyboard.press("Tab")
    left = code_page.evaluate("() => document.activeElement.closest('.head .keys') === null")
    assert left, "Tab stopped on a second key at the head"
    code_page.keyboard.press("Shift+Tab")
    expect(keys.nth(1), "Tab back didn't come to the key last left").to_be_focused()


def test_a_reload_holds_the_page_still(code_page: Page) -> None:
    says_it_stands(code_page, "live")
    expect(row(code_page, "github:owl/core"), "Code drew no row for core").to_be_visible()
    # Every shift of the page the reader didn't cause, from the reload's first paint on.
    code_page.add_init_script(
        """globalThis.shifted = 0;
        new PerformanceObserver((list) => {
          for (const each of list.getEntries()) {
            globalThis.shifted += each.hadRecentInput ? 0 : each.value;
          }
        }).observe({ type: "layout-shift", buffered: true });
        document.addEventListener("DOMContentLoaded", () => {
          globalThis.held = document.documentElement.style.getPropertyValue("--room-list");
        });"""
    )
    code_page.reload()
    says_it_stands(code_page, "live")
    expect(row(code_page, "github:owl/core"), "the reload drew no row").to_be_visible()
    code_page.wait_for_timeout(1000)
    shifted = code_page.evaluate("() => globalThis.shifted")
    assert shifted < STILL, f"the page shifted {shifted} as its feeds arrived on a reload"
    assert code_page.evaluate("() => globalThis.held") != "", "the reload held no room"
    room = code_page.evaluate(
        "() => document.documentElement.style.getPropertyValue('--room-list')"
    )
    assert room == "", f"the list's room, {room}, was held after every feed had drawn"


# What a page holds at DOMContentLoaded of each part's room, before any feed can draw.
HELD_ROOM = """document.addEventListener("DOMContentLoaded", () => {
  const style = document.documentElement.style;
  globalThis.held = Object.fromEntries(
    ["lede", "list", "open"].map((id) => [id, style.getPropertyValue(`--room-${id}`)]),
  );
  globalThis.loadedAt = performance.now();
  addEventListener("resize", () => { globalThis.resizedAt ??= performance.now(); });
  // When the list's room goes, read as it goes rather than whenever the test looks.
  new MutationObserver(() => {
    if (style.getPropertyValue("--room-list") === "") {
      globalThis.roomWentAt ??= performance.now();
    }
  }).observe(document.documentElement, { attributes: true, attributeFilter: ["style"] });
});"""


def test_a_reload_holds_no_room_for_a_name_it_forgets(code_page: Page) -> None:
    says_it_stands(code_page, "live")
    code_page.locator("#find").fill("notes")
    expect(code_page.locator("#list .row"), "finding didn't narrow the list").to_have_count(1)
    code_page.add_init_script(HELD_ROOM)
    code_page.reload()
    says_it_stands(code_page, "live")
    held = code_page.evaluate("() => globalThis.held")
    assert held["lede"] != "", "the reload held no room at all"
    # The reload forgets what was found, so the list it draws is every repository.
    assert held["list"] == "", f"the reload held {held['list']} for a list it won't draw"


def test_a_reload_holds_no_room_for_a_place_the_address_left(
    code_page: Page, repos_daemon: StandIn
) -> None:
    says_it_stands(code_page, "live")
    grove_written_now(repos_daemon)
    code_page.locator('#lede a[data-place="growing"]').click()
    expect(code_page.locator("#list .row"), "Growing didn't narrow the list").to_have_count(
        1, timeout=PACE_MS
    )
    # Choosing core puts it in the address, which then says nothing of Growing.
    row(code_page, "github:owl/core").click()
    code_page.add_init_script(HELD_ROOM)
    code_page.reload()
    says_it_stands(code_page, "live")
    held = code_page.evaluate("() => globalThis.held")
    assert held["lede"] != "", "the reload held no room at all"
    assert held["list"] == "", f"the reload held {held['list']} for a Growing it won't draw"


def test_a_reload_holds_room_for_the_season_its_address_opens(code_page: Page) -> None:
    says_it_stands(code_page, "live")
    code_page.evaluate("(path) => { location.hash = `#tree=${encodeURIComponent(path)}`; }", OLD)
    expect(
        code_page.locator('#open .tree[aria-current="true"]'), "the address didn't open the tree"
    ).to_have_count(1, timeout=PACE_MS)
    code_page.add_init_script(HELD_ROOM)
    code_page.reload()
    says_it_stands(code_page, "live")
    held = code_page.evaluate("() => globalThis.held")
    assert held["open"] != "", "the reload held no room for a drawer its address opens again"


def test_a_reload_holds_no_room_for_a_season_only_z_opened(code_page: Page) -> None:
    says_it_stands(code_page, "live")
    row(code_page, "github:owl/core").click()
    code_page.keyboard.press("z")
    expect(code_page.locator("#open .tree"), "z didn't open the resting trees").to_have_count(2)
    code_page.add_init_script(HELD_ROOM)
    code_page.reload()
    says_it_stands(code_page, "live")
    held = code_page.evaluate("() => globalThis.held")
    assert held["lede"] != "", "the reload held no room at all"
    assert held["open"] == "", f"the reload held {held['open']} for trees it folds again"


def test_a_resize_before_the_feeds_arrive_lets_the_room_go(code_page: Page) -> None:
    says_it_stands(code_page, "live")
    # The list is empty until its rows draw, and an empty list keeps no room, so the reload
    # waits for a row as the others do; live can be said before the paced list has drawn.
    expect(row(code_page, "github:owl/core"), "Code drew no row for core").to_be_visible()
    # Every feed answered by a socket that never speaks, so nothing draws and the room stays
    # held for its while, as on a slow start; the server answers from what it holds otherwise.
    code_page.route_web_socket("**", lambda _socket: None)
    code_page.add_init_script(HELD_ROOM)
    code_page.reload(wait_until="domcontentloaded")
    assert code_page.evaluate("() => globalThis.held.list") != "", "the reload held no room"
    size = code_page.viewport_size or {"width": 1280, "height": 800}
    code_page.set_viewport_size({"width": size["width"] - 200, "height": size["height"]})
    # Well inside the 3 seconds the room is held for anyway, so only the resize can let it go.
    code_page.wait_for_function("() => globalThis.roomWentAt !== undefined", timeout=1000)
    # The timer starts after the page's clock does, so room gone before it can't be the timer's.
    went, resized = code_page.evaluate("() => [globalThis.roomWentAt, globalThis.resizedAt]")
    assert resized < ROOM_HELD_MS, f"the builder resized only at {resized} ms, proving nothing"
    assert went < ROOM_HELD_MS, f"the room went on its timer, at {went} ms, not the resize"


def test_a_burst_of_records_draws_the_list_a_few_times(
    code_page: Page, repos_daemon: StandIn
) -> None:
    says_it_stands(code_page, "live")
    expect(row(code_page, "github:owl/core"), "Code drew no row for core").to_be_visible()
    code_page.evaluate(
        """() => {
          globalThis.writes = 0;
          new MutationObserver(() => { globalThis.writes += 1; })
            .observe(document.querySelector("#list"), { childList: true });
        }"""
    )
    # A second of a tree written over and over, as an agent's build does, each record moving
    # the list, then a last record no other says: a repository going.
    started = time.monotonic()
    for each in range(20):
        written = now() if each % 2 == 0 else "2026-01-01T00:00:00Z"
        repos_daemon.tell(
            "tree", changed=[{"key": "tree:/home/ada/trees/old", "written_at": written}]
        )
        time.sleep(0.05)
    repos_daemon.tell("repository", gone=["repository:local:~/src/notes"])
    lasted = time.monotonic() - started
    notes = row(code_page, "local:~/src/notes")
    expect(notes, "the burst's last record wasn't drawn").to_have_count(0, timeout=PACE_MS)
    writes = code_page.evaluate("() => globalThis.writes")
    allowed = math.ceil(lasted * DRAWS_A_SECOND) + 2
    assert writes <= allowed, f"21 records in {lasted:.1f} s drew the list {writes} times"


@pytest.fixture
def spanned(tmp_path: Path, code_served: Served, mycelium_node: StandIn) -> None:
    """Tie core and notes, with the tying cell home on the cells' site as the first is."""
    _ = code_served
    things = [
        {"kind": "cell", "key": key, "name": key, "address": f"{CELLS_SITE}#cell={key}"}
        for key in ("E2ZIAQ", "W9TIES")
    ]
    homes = tmp_path / "documents" / CELLS_SITE / "homes.json"
    homes.write_text(json.dumps({"things": things}))
    spanning(mycelium_node)


# The tying cell is home, since Code asks the sites again for a cell that has none, which is
# a later visit's cost and not a first's.
@pytest.mark.usefixtures("spanned")
def test_code_keeps_to_its_budget(code_opened: Opened) -> None:
    says_it_stands(code_opened.page, "live")
    expect(code_opened.page.locator(TIE_ROWS), "the map wasn't drawn").to_have_count(
        2, timeout=PACE_MS
    )
    # Counted once every part the feeds draw is there, so the count is the page's
    # whole and not how far it got.
    for key in ["github:owl/core", "local:~/src/notes"]:
        expect(row(code_opened.page, key), f"Code drew no row for {key}").to_be_visible()
    # The cell tying the map works in core's grove beside the one already there.
    expect(
        row(code_opened.page, "github:owl/core"), "the cells at work weren't drawn"
    ).to_contain_text("2 cells", timeout=PACE_MS)
    code_opened.keeps_to(BUDGET)
    says_it_stands(code_opened.page, "live")


@pytest.fixture
def code_whole(repos_daemon: StandIn, mycelium_node: StandIn) -> None:
    """Open a tree in the drawer with its cell named, not every tree folded, and draw the map."""
    grove_written_now(repos_daemon)
    spanning(mycelium_node)


@pytest.mark.usefixtures("code_whole")
@pytest.mark.parametrize("look", LOOKS)
@pytest.mark.parametrize("size", SIZES)
def test_code_passes_the_skills_page_checks(
    browser: Browser,
    code_served: Served,
    size: tuple[int, int],
    look: str,
) -> None:
    root, _ = code_served
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
        # The skill's check for a walked map, last since it presses keys on it: every name
        # clear of the others and of every thread, and the keys walking as a map's do.
        expect(opened.page.locator(TIE_ROWS), "the map wasn't drawn").to_have_count(
            2, timeout=PACE_MS
        )
        unwalked = opened.page.evaluate((Path(os.environ["WEB_SKILL"]) / "map.js").read_text())
        assert unwalked == [], f"map.js at {size} in the {look} look found {unwalked}"
    finally:
        # Closed whatever failed, so what the page logged is said too.
        opened.close()


def test_the_map_draws_a_thread_for_each_cell_joining_two_repositories(
    code_page: Page, mycelium_node: StandIn
) -> None:
    says_it_stands(code_page, "live")
    # The one cell the stand-in holds names only its own repository, which joins nothing; it's
    # heard first, so the map hidden is the mycelium's word and not its silence.
    expect(row(code_page, "github:owl/core"), "the cell at work wasn't drawn").to_contain_text(
        "1 cell", timeout=PACE_MS
    )
    expect(code_page.locator("#ties"), "a map with nothing joined was shown").to_be_hidden()
    spanning(mycelium_node)
    rows = code_page.locator(TIE_ROWS)
    expect(rows, "the map drew no row for each joined repository").to_have_count(2, timeout=PACE_MS)
    expect(code_page.locator("#ties"), "the map wasn't shown").to_be_visible()
    expect(rows.first, "core wasn't the first row, in the grove's order").to_have_attribute(
        "data-key", "github:owl/core"
    )
    threads = code_page.locator("#ties-map [data-line]")
    expect(threads, "the map drew a thread other than the cell's one").to_have_count(1)
    lit = code_page.locator("#ties-map .tie-thread.lit")
    expect(lit, "the threads of the row the keys rest on weren't lit").to_have_count(1)
    here = code_page.locator("#ties-here")
    expect(here, "the map didn't say what core joins").to_contain_text("Joined to notes by 1")
    expect(here.locator("a", has_text="notes"), "notes wasn't linked home").to_have_attribute(
        "href", "#repo=notes"
    )
    # The keys on the map when its last tie goes are handed to the list, not dropped, and said
    # to be, so the next j isn't heard moving something else.
    code_page.locator("#ties-map").focus()
    mycelium_node.tell("cell", gone=["cell:W9TIES"])
    expect(code_page.locator("#ties"), "the map stayed once nothing joined").to_be_hidden(
        timeout=PACE_MS
    )
    expect(code_page.locator("#list"), "the keys fell out of the page").to_be_focused()
    # Said outside the hidden map, where a screen reader still hears it.
    ties_said = code_page.locator("#ties-said")
    expect(ties_said, "the keys left the map unsaid").to_have_text(
        "No cell at work joins two repositories now, so the keys are on the list."
    )
    expect(ties_said, "the map's last words hid with it").to_be_visible()


def test_the_maps_keys_walk_its_rows(code_page: Page, mycelium_node: StandIn) -> None:
    says_it_stands(code_page, "live")
    spanning(mycelium_node)
    rows = code_page.locator(TIE_ROWS)
    expect(rows, "the map wasn't drawn").to_have_count(2, timeout=PACE_MS)
    chosen = code_page.locator("#list .row[aria-selected='true']")
    listed = chosen.get_attribute("data-key")
    tied = code_page.locator("#ties-map")
    tied.focus()
    code_page.keyboard.press("j")
    expect(rows.last, "j didn't step down the map").to_have_attribute("aria-selected", "true")
    expect(rows.last, "the row the keys rest on wasn't marked").to_have_attribute("data-here", "")
    expect(tied, "the reader wasn't told the row").to_have_attribute(
        "aria-activedescendant", "tie-1"
    )
    expect(chosen, "the map's j stepped the list too").to_have_attribute("data-key", listed or "")
    code_page.keyboard.press("k")
    expect(rows.first, "k didn't step back up").to_have_attribute("aria-selected", "true")
    # Sideways lies nothing, since the rows stand in one column: the keys stay, and say so in
    # other words when asked again, so a repeat is heard too.
    said = code_page.locator("#ties-said")
    code_page.keyboard.press("ArrowLeft")
    expect(said, "the left arrow said nothing").to_have_text(
        "Nothing lies to the left of core on the map."
    )
    code_page.keyboard.press("ArrowLeft")
    expect(said, "a second left arrow said the same").to_have_text(
        "Still nothing to the left of core."
    )
    expect(rows.first, "the left arrow moved the keys").to_have_attribute("aria-selected", "true")
    # A step clears the words, so the same words said after it are heard as new.
    code_page.keyboard.press("j")
    expect(said, "a step left the last words standing").to_have_text("")
    code_page.keyboard.press("k")
    # Home on the first row has nowhere to go, as the arrows past an end don't.
    code_page.keyboard.press("Home")
    expect(said, "Home on the first row said nothing").to_have_text(
        "Nothing lies before core on the map."
    )
    # The list's own keys act on the list's cursor, which the map doesn't hold, so they say
    # where they act rather than act on another repository than the one marked, and in other
    # words when pressed again, so a repeat is heard too.
    code_page.keyboard.press("z")
    expect(said, "z on the map didn't say where it acts").to_have_text(
        "z acts on the list: Return takes the keys to core there."
    )
    code_page.keyboard.press("z")
    expect(said, "a second z said the same").to_have_text(
        "z still acts on the list: Return takes the keys to core there."
    )
    # Escape on the map holds on to a find the list holds.
    find = code_page.locator("#find")
    find.fill("o")
    tied.focus()
    code_page.keyboard.press("Escape")
    expect(find, "Escape on the map let go of the find").to_have_value("o")
    expect(rows.first, "Escape moved the map").to_have_attribute("aria-selected", "true")
    code_page.keyboard.press("Tab")
    left = code_page.evaluate("() => document.activeElement.id !== 'ties-map'")
    assert left, "Tab didn't leave the map"


def test_return_on_the_map_opens_a_repository_as_the_list_does(
    code_page: Page, mycelium_node: StandIn
) -> None:
    says_it_stands(code_page, "live")
    spanning(mycelium_node)
    rows = code_page.locator(TIE_ROWS)
    expect(rows, "the map wasn't drawn").to_have_count(2, timeout=PACE_MS)
    chosen = code_page.locator("#list .row[aria-selected='true']")
    find = code_page.locator("#find")
    tied = code_page.locator("#ties-map")
    tied.focus()
    # Opening a repository moves the list's cursor, which takes the place of the address it
    # had, as the list's own moves do, rather than adding a step to Back.
    steps = code_page.evaluate("() => history.length")
    code_page.keyboard.press("End")
    code_page.keyboard.press("Enter")
    expect(code_page, "Return didn't open notes").to_have_url(re.compile(r"#repo=notes$"))
    assert code_page.evaluate("() => history.length") == steps, "Return added a step to Back"
    expect(chosen, "notes wasn't the repository opened").to_have_attribute(
        "data-key", "local:~/src/notes"
    )
    lister = code_page.locator("#list")
    expect(lister, "Return didn't take the keys to the list").to_be_focused()
    # The repository already open is opened as a link to it would be: the keys go to it, shown
    # in the list even where a find had hidden it.
    find.fill("core")
    tied.focus()
    code_page.keyboard.press("Enter")
    expect(lister, "Return on the repository open did nothing").to_be_focused()
    expect(find, "the find hid the repository opened").to_have_value("")
    expect(chosen, "the list didn't show notes as opened").to_have_attribute(
        "data-key", "local:~/src/notes"
    )
    rows.first.click()
    expect(code_page, "a click didn't open core").to_have_url(re.compile(r"#repo=owl%2Fcore$"))
    expect(rows.first, "a click left the map's keys elsewhere").to_have_attribute(
        "aria-selected", "true"
    )

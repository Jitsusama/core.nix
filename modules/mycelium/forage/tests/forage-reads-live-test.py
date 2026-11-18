"""The forage page reads live from the node, through the sites' server, in a browser.

The page is the one Nix builds, served by the real local-sites server beside a
stand-in node (modules/local-sites/tests/stand_in_daemon.py) holding four cells'
forage on kestrel (node.json): one with a purpose, a plan and a thing it waits
on its person for, one it started, one with no purpose written, and one whose
record carries no lineage at all. Its feed is trimmed to the fields home.nix
declares for a machine in a realm (NODE_KINDS). It must say it's live with every
cell drawn, each started cell under the one that started it, say what waits on
its person in its lede and its panel, follow a purpose written, drawn as words
though it holds markup, and a cell going, say broken when the node stops and
live again, its cells read again, when it's back, say stale when the server
goes and keep what it drew, keep to its budget, and pass the web-design skill's
page checks at a phone's width and a desk's in both looks.
Anything the page logs fails it, but for the browser's own lines once the test
takes the server away. The server, the browser and the tab are
modules/local-sites/tests/live_page.py's.
"""

import json
import os
from pathlib import Path
from typing import TYPE_CHECKING

import pytest
from live_page import LOOKS, SIZES, Budget, Opened, Served, serving
from playwright.sync_api import Browser, Locator, Page, expect
from stand_in_daemon import StandIn, standing_in

if TYPE_CHECKING:
    from collections.abc import Iterator

HERE = Path(__file__).parent
# The page draws as soon as a record moves, so a change must show well inside this.
PACE_MS = 6000
# The server watches a gone daemon again every 2 seconds, so its return shows inside this.
BACK_MS = 15000
# The browser's own line for a feed that couldn't connect, once the server is gone.
UNREACHED = "WebSocket connection to"
# A page that dropped what it drew on a change shows it within a redraw, and one that lost its
# server tries again a second on (feed.ts); waiting this long lets both happen first.
REDRAWN_MS = 2500
# A purpose a cell's writer chose, with markup in it.
WRITTEN = "OSPREY keeps watch over the <i>harbour</i>."
# Every cell node.json holds, in the order the page lists them: those nothing here started by
# address, each followed by the cells it started.
CELLS = ["KESTRL", "SKIFFS", "OSPREY", "TERNAA"]
# What a first visit to the forage page may cost, drawn from the stand-in's cells.
BUDGET = Budget(files=22, kilobytes=545, elements=255)


@pytest.fixture
def mycelium_node(tmp_path: Path) -> Iterator[StandIn]:
    yield from standing_in(tmp_path / "node.sock", "node", HERE / "node.json")


@pytest.fixture
def forage_served(tmp_path: Path, mycelium_node: StandIn) -> Iterator[Served]:
    feed = {"socket": str(mycelium_node.path), "kinds": json.loads(os.environ["NODE_KINDS"])}
    with serving(tmp_path, {"forage": os.environ["FORAGE_SITE"]}, {"node": feed}) as served:
        yield served


@pytest.fixture
def forage_opened(browser: Browser, forage_served: Served) -> Iterator[Opened]:
    root, _ = forage_served
    opened = Opened(browser, f"{root}forage/", SIZES[-1], "dark")
    yield opened
    opened.close()


@pytest.fixture
def forage_page(forage_opened: Opened) -> Page:
    return forage_opened.page


def says_it_stands(page: Page, state: str, timeout: int = PACE_MS) -> None:
    expect(page.locator("#asof"), f"the forage page never said it was {state}").to_have_attribute(
        "data-state", state, timeout=timeout
    )


def cell(page: Page, name: str) -> Locator:
    return page.locator(f'#cells [role=option][data-cell="{name}"]')


def every_cell_drawn(page: Page) -> None:
    rows = page.locator("#cells [role=option]")
    for at, name in enumerate(CELLS):
        expect(rows.nth(at), f"the list's row {at} wasn't {name}").to_have_attribute(
            "data-cell", name, timeout=PACE_MS
        )
    expect(rows, "the page listed cells node.json doesn't hold").to_have_count(len(CELLS))


def forage_of(name: str, **more: object) -> dict[str, object]:
    """Give `name`'s forage as node.json holds it, with `more` over its fields."""
    records = json.loads((HERE / "node.json").read_text())["kinds"]["forage"]
    held: dict[str, object] = next(each for each in records if each["cell"] == name)
    return {**held, **more}


def test_the_forage_page_draws_every_cell_live(forage_page: Page) -> None:
    says_it_stands(forage_page, "live")
    every_cell_drawn(forage_page)
    expect(forage_page.locator("#headline"), "the headline didn't count the cells").to_have_text(
        "4 cells, 1 without purpose."
    )
    expect(forage_page.locator("#substrate"), "the page didn't say where they stand").to_have_text(
        "kestrel"
    )


def test_a_cell_started_by_another_stands_under_it(forage_page: Page) -> None:
    says_it_stands(forage_page, "live")
    every_cell_drawn(forage_page)
    expect(cell(forage_page, "SKIFFS"), "SKIFFS wasn't drawn under KESTRL").to_have_attribute(
        "style", "--depth:1"
    )
    # A record with no lineage at all, as some cells' are, still draws, at the top.
    expect(cell(forage_page, "TERNAA"), "TERNAA didn't stand at the top").to_have_attribute(
        "style", "--depth:0"
    )


def test_what_waits_on_its_person_is_said_first(forage_page: Page) -> None:
    says_it_stands(forage_page, "live")
    every_cell_drawn(forage_page)
    expect(forage_page.locator("#lede"), "the lede didn't say what waits").to_contain_text(
        "One thing waits on you, from KESTRL."
    )
    cell(forage_page, "KESTRL").click()
    panel = forage_page.locator("#panel-body")
    expect(panel, "KESTRL's panel didn't say what it waits on").to_contain_text(
        "Say which field to hover over"
    )
    expect(panel, "KESTRL's wait didn't say it's ready").to_contain_text("Ready now")
    cell(forage_page, "SKIFFS").click()
    expect(panel, "SKIFFS's panel drew KESTRL's wait").not_to_contain_text("What It Waits On")


def test_a_purpose_written_and_a_cell_gone_are_followed(
    forage_page: Page, mycelium_node: StandIn
) -> None:
    says_it_stands(forage_page, "live")
    every_cell_drawn(forage_page)
    headline = forage_page.locator("#headline")
    mycelium_node.tell("forage", changed=[forage_of("OSPREY", purpose=WRITTEN)])
    expect(headline, "a purpose written wasn't followed").to_have_text(
        "4 cells, each with its purpose.", timeout=PACE_MS
    )
    expect(cell(forage_page, "OSPREY"), "OSPREY's purpose wasn't drawn as words").to_contain_text(
        WRITTEN
    )
    expect(forage_page.locator("#cells i"), "OSPREY's purpose was read as markup").to_have_count(0)
    mycelium_node.tell("forage", gone=["forage:TERNAA"])
    expect(cell(forage_page, "TERNAA"), "the gone cell was still drawn").to_have_count(
        0, timeout=PACE_MS
    )
    expect(headline, "the count didn't follow the cell going").to_have_text(
        "3 cells, each with its purpose."
    )


def test_a_node_that_stops_is_said_broken_then_live(
    forage_page: Page, mycelium_node: StandIn
) -> None:
    says_it_stands(forage_page, "live")
    every_cell_drawn(forage_page)
    mycelium_node.leave()
    says_it_stands(forage_page, "broken")
    # What the node holds moves while it's away, so only reading it again shows it.
    mycelium_node.tell("forage", gone=["forage:TERNAA"])
    mycelium_node.start()
    says_it_stands(forage_page, "live", BACK_MS)
    expect(cell(forage_page, "TERNAA"), "the page didn't read the node again").to_have_count(
        0, timeout=PACE_MS
    )
    expect(cell(forage_page, "KESTRL"), "KESTRL went with TERNAA").to_have_count(1)


def test_a_sites_server_that_goes_is_said_stale(
    forage_opened: Opened, forage_served: Served
) -> None:
    forage_page = forage_opened.page
    says_it_stands(forage_page, "live")
    # Once the server's gone, the browser logs the feed's next try as a line of its own.
    forage_opened.expecting(UNREACHED)
    _, server = forage_served
    server.kill()
    says_it_stands(forage_page, "stale")
    forage_page.wait_for_timeout(REDRAWN_MS)
    every_cell_drawn(forage_page)
    says_it_stands(forage_page, "stale")


def test_the_forage_page_keeps_to_its_budget(forage_opened: Opened) -> None:
    says_it_stands(forage_opened.page, "live")
    # Counted once every cell the feed holds is drawn, so the count is the page's whole and not
    # how far it got.
    every_cell_drawn(forage_opened.page)
    forage_opened.keeps_to(BUDGET)
    says_it_stands(forage_opened.page, "live")


@pytest.mark.parametrize("look", LOOKS)
@pytest.mark.parametrize("size", SIZES)
def test_the_forage_page_passes_the_skills_page_checks(
    browser: Browser, forage_served: Served, size: tuple[int, int], look: str
) -> None:
    root, _ = forage_served
    opened = Opened(browser, f"{root}forage/", size, look)
    try:
        says_it_stands(opened.page, "live")
        every_cell_drawn(opened.page)
        opened.passes_the_skills_checks()
    finally:
        # Closed whatever failed, so what the page logged is said too.
        opened.close()

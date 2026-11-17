"""The mycelium map reads live from its node, through the sites' server, in a real browser.

The page is the one Nix builds, served by the real local-sites server beside a
stand-in node (modules/local-sites/tests/stand_in_daemon.py) holding a small
realm under test names, node.json, through the node feed exactly as home.nix
declares it for a machine in a realm (NODE_KINDS). It must say it's live with
the realm drawn, follow a change within its pace, say broken when the node
stops while keeping the realm drawn, and live again when it's back, say stale
when the server goes, draw a record's words as words and never as markup, and
pass the web-design skill's page checks and the map's own (site-checks.js) at
a phone's width and a desk's in both looks. Any error the page logs fails it,
since that is how the map froze on "reading" unseen. The server, the browser
and the tab are modules/local-sites/tests/live_page.py's.
"""

import json
import os
from pathlib import Path
from typing import TYPE_CHECKING

import pytest
from live_page import LOOKS, SIZES, Opened, Served, serving
from playwright.sync_api import Browser, Page, expect
from stand_in_daemon import StandIn

if TYPE_CHECKING:
    from collections.abc import Iterator

HERE = Path(__file__).parent
# The map redraws at most every 2 seconds, so a change must show well inside this.
PACE_MS = 6000
# The server watches a daemon that went again every 2 seconds (upstream.rs's
# AGAIN) while the page stays connected to it, so a node back is live well
# inside this.
BACK_MS = 12000
# A page hands the map a reading at most every 2 seconds (site.ts's FLOOR_MS),
# so waiting past one hand-over lets a page that dropped what it drew on a
# change show it, before the test looks for what should have stayed.
REDRAWN_MS = 2500
# What node.json's realm comes to on the map, every part of the count.
COUNTED = "8 cells, 2 moving, 1 stuck"
# The browser's own line for a feed that couldn't connect, once the server is gone.
UNREACHED = "WebSocket connection to"


@pytest.fixture
def mycelium_node(tmp_path: Path) -> Iterator[StandIn]:
    reading = json.loads((HERE / "node.json").read_text())
    stand_in = StandIn(tmp_path / "node.sock", "node", reading["kinds"]).start()
    yield stand_in
    stand_in.leave()


@pytest.fixture
def mycelium_served(tmp_path: Path, mycelium_node: StandIn) -> Iterator[Served]:
    node = {"socket": str(mycelium_node.path), "kinds": json.loads(os.environ["NODE_KINDS"])}
    with serving(tmp_path, {"mycelium": os.environ["MYCELIUM_SITE"]}, {"node": node}) as served:
        yield served


@pytest.fixture
def mycelium_opened(browser: Browser, mycelium_served: Served) -> Iterator[Opened]:
    root, _ = mycelium_served
    opened = Opened(browser, f"{root}mycelium/", SIZES[-1], "dark")
    yield opened
    opened.close()


def the_map_stands(page: Page, state: str, timeout: int = PACE_MS) -> None:
    expect(page.locator("#asof"), f"the map never said it was {state}").to_have_attribute(
        "data-state", state, timeout=timeout
    )


def the_map_counts(page: Page, counted: str, why: str, timeout: int = PACE_MS) -> None:
    expect(page.locator("#status-words"), why).to_have_text(counted, timeout=timeout)


def test_the_map_draws_the_realm_live(mycelium_opened: Opened) -> None:
    page = mycelium_opened.page
    the_map_stands(page, "live")
    expect(page.locator("#world .hub").first, "the map drew no network").to_be_visible()
    the_map_counts(page, COUNTED, "the map miscounted the realm")


def test_a_cell_that_moves_is_drawn_moving(mycelium_opened: Opened, mycelium_node: StandIn) -> None:
    page = mycelium_opened.page
    the_map_stands(page, "live")
    the_map_counts(page, COUNTED, "the map miscounted the realm")
    mycelium_node.tell("cell", changed=[{"key": "cell:TERN03", "standing": "moving"}])
    the_map_counts(page, "8 cells, 3 moving, 1 stuck", "the moved cell wasn't drawn moving")


def test_a_cell_that_goes_leaves_the_map(mycelium_opened: Opened, mycelium_node: StandIn) -> None:
    page = mycelium_opened.page
    the_map_stands(page, "live")
    the_map_counts(page, COUNTED, "the map miscounted the realm")
    mycelium_node.tell("cell", gone=["cell:HERON1"])
    the_map_counts(page, "7 cells, 2 moving", "the gone cell was still counted")


def test_a_node_that_stops_is_said_broken_with_the_realm_kept_then_live(
    mycelium_opened: Opened, mycelium_node: StandIn
) -> None:
    page = mycelium_opened.page
    the_map_stands(page, "live")
    the_map_counts(page, COUNTED, "the map miscounted the realm")
    mycelium_node.leave()
    the_map_stands(page, "broken")
    expect(page.locator("#asof"), "the map didn't say it wasn't live").to_contain_text(
        "Not live since"
    )
    page.wait_for_timeout(REDRAWN_MS)
    expect(page.locator("#world .hub").first, "the broken map dropped the realm").to_be_visible()
    the_map_counts(page, COUNTED, "the broken map lost count of the realm")
    mycelium_node.start()
    the_map_stands(page, "live", BACK_MS)
    # A change after it's back shows the watch delivers again, not just the old map.
    mycelium_node.tell("cell", changed=[{"key": "cell:TERN03", "standing": "moving"}])
    the_map_counts(page, "8 cells, 3 moving, 1 stuck", "the map back live missed a change")


def test_a_server_that_goes_is_said_stale(mycelium_opened: Opened, mycelium_served: Served) -> None:
    page = mycelium_opened.page
    the_map_stands(page, "live")
    # The feed tries the server again a second on, which the browser logs.
    mycelium_opened.expecting(UNREACHED)
    _, server = mycelium_served
    server.kill()
    the_map_stands(page, "stale")
    page.wait_for_timeout(REDRAWN_MS)
    expect(page.locator("#world .hub").first, "the stale map dropped the realm").to_be_visible()
    the_map_counts(page, COUNTED, "the stale map lost count of the realm")


def test_a_records_words_are_drawn_as_words(
    mycelium_opened: Opened, mycelium_node: StandIn
) -> None:
    page = mycelium_opened.page
    the_map_stands(page, "live")
    the_map_counts(page, COUNTED, "the map miscounted the realm")
    # Markup, and a quote that would break out of an attribute it's written into.
    hostile = "Paint the <b>Hull</b> & \"oars\" ' onmouseover='x' <img src=x>"
    mycelium_node.tell("cell", changed=[{"key": "cell:GULL01", "title": hostile}])
    page.evaluate("location.hash = '#cell=GULL01'")
    # The panel says a cell's title as what it's for, so the words show there.
    expect(page.locator("#panel-body"), "the cell's words never reached its panel").to_contain_text(
        "<b>Hull</b>", timeout=PACE_MS
    )
    # Past a hand-over, so every other place the title is drawn has drawn it too.
    page.wait_for_timeout(REDRAWN_MS)
    drawn = page.locator('b:has-text("Hull"), img[src="x"], [onmouseover]')
    expect(drawn, "a cell's title was drawn as markup somewhere on the map").to_have_count(0)


@pytest.mark.parametrize("look", LOOKS)
@pytest.mark.parametrize("size", SIZES)
def test_the_map_passes_the_skills_page_checks_and_its_own(
    browser: Browser, mycelium_served: Served, size: tuple[int, int], look: str
) -> None:
    root, _ = mycelium_served
    opened = Opened(browser, f"{root}mycelium/", size, look)
    try:
        the_map_stands(opened.page, "live")
        the_map_counts(opened.page, COUNTED, "the map didn't draw")
        opened.passes_the_skills_checks()
        # The map's own, after the skill's, since it opens things through the hash.
        found = opened.page.evaluate((HERE / "site-checks.js").read_text())
        assert found == [], f"site-checks.js at {size} in the {look} look found {found}"
    finally:
        opened.close()

"""The mycelium map reads live from its node, through the sites' server, in a real browser.

The page is the one Nix builds, served by the real local-sites server beside a
stand-in node (modules/local-sites/tests/stand_in_daemon.py) holding a small
realm under test names, node.json. It must say it's live with the realm drawn,
follow a change within its pace, say broken when the node stops and live again
when it's back, say stale when the server goes, and pass the web-design
skill's page checks and the map's own (site-checks.js) at a phone's width and
a desk's in both looks. Any error
the page logs fails it, since that is how the map froze on "reading" unseen.
The server, the browser and the tab are modules/local-sites/tests/live_page.py's.
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
# The feed waits up to 30 seconds between tries once the node has been gone a while.
RETRY_MS = 35000
# What node.json's realm comes to on the map.
COUNTED = "8 cells, 2 moving, 1 stuck"
# The fields of a machine and a message the map draws, as a machine's feed declares them.
MACHINE = ["name", "state", "os", "system", "cpu", "memory", "disk", "power", "measured_at"]
MESSAGE = ["at", "from", "intent", "groups"]


@pytest.fixture
def mycelium_node(tmp_path: Path) -> Iterator[StandIn]:
    reading = json.loads((HERE / "node.json").read_text())
    stand_in = StandIn(tmp_path / "node.sock", "node", reading["kinds"]).start()
    yield stand_in
    stand_in.leave()


@pytest.fixture
def mycelium_served(tmp_path: Path, mycelium_node: StandIn) -> Iterator[Served]:
    kinds = {
        "network": {},
        "cell": {},
        "machine": {"fields": MACHINE},
        "message": {"fields": MESSAGE},
    }
    node = {"socket": str(mycelium_node.path), "kinds": kinds}
    with serving(tmp_path, {"mycelium": os.environ["MYCELIUM_SITE"]}, {"node": node}) as served:
        yield served


@pytest.fixture
def mycelium_page(browser: Browser, mycelium_served: Served) -> Iterator[Page]:
    root, _ = mycelium_served
    opened = Opened(browser, f"{root}mycelium/", SIZES[-1], "dark")
    yield opened.page
    opened.close()


def the_map_stands(page: Page, state: str, timeout: int = PACE_MS) -> None:
    expect(page.locator("#asof"), f"the map never said it was {state}").to_have_attribute(
        "data-state", state, timeout=timeout
    )


def test_the_map_draws_the_realm_live(mycelium_page: Page) -> None:
    the_map_stands(mycelium_page, "live")
    expect(mycelium_page.locator("#world .hub").first, "the map drew no network").to_be_visible()
    expect(mycelium_page.locator("#status-words"), "the map miscounted the realm").to_have_text(
        COUNTED
    )


def test_a_cell_that_moves_is_drawn_moving(mycelium_page: Page, mycelium_node: StandIn) -> None:
    the_map_stands(mycelium_page, "live")
    words = mycelium_page.locator("#status-words")
    expect(words, "the map miscounted the realm").to_have_text(COUNTED)
    mycelium_node.tell("cell", changed=[{"key": "cell:TERN03", "standing": "moving"}])
    expect(words, "the moved cell wasn't drawn moving").to_contain_text(
        "8 cells, 3 moving", timeout=PACE_MS
    )


def test_a_cell_that_goes_leaves_the_map(mycelium_page: Page, mycelium_node: StandIn) -> None:
    the_map_stands(mycelium_page, "live")
    mycelium_node.tell("cell", gone=["cell:HERON1"])
    expect(mycelium_page.locator("#status-words"), "the gone cell was still counted").to_have_text(
        "7 cells, 2 moving", timeout=PACE_MS
    )


def test_a_node_that_stops_is_said_broken_and_then_live(
    mycelium_page: Page, mycelium_node: StandIn
) -> None:
    the_map_stands(mycelium_page, "live")
    mycelium_node.leave()
    the_map_stands(mycelium_page, "broken")
    expect(mycelium_page.locator("#asof"), "the map didn't say since when").to_contain_text("since")
    mycelium_node.start()
    the_map_stands(mycelium_page, "live", RETRY_MS)
    expect(mycelium_page.locator("#status-words"), "the map lost the realm").to_have_text(COUNTED)


def test_a_server_that_goes_is_said_stale(mycelium_page: Page, mycelium_served: Served) -> None:
    the_map_stands(mycelium_page, "live")
    _, server = mycelium_served
    server.kill()
    the_map_stands(mycelium_page, "stale")


@pytest.mark.parametrize("look", LOOKS)
@pytest.mark.parametrize("size", SIZES)
def test_the_map_passes_the_skills_page_checks_and_its_own(
    browser: Browser, mycelium_served: Served, size: tuple[int, int], look: str
) -> None:
    root, _ = mycelium_served
    opened = Opened(browser, f"{root}mycelium/", size, look)
    the_map_stands(opened.page, "live")
    expect(opened.page.locator("#status-words"), "the map didn't draw").to_have_text(COUNTED)
    opened.passes_the_skills_checks()
    # The map's own, after the skill's, since it opens things through the hash.
    found = opened.page.evaluate((HERE / "site-checks.js").read_text())
    assert found == [], f"site-checks.js at {size} in the {look} look found {found}"
    opened.close()

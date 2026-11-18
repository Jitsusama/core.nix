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

import copy
import json
import os
import re
from pathlib import Path
from typing import TYPE_CHECKING, Any

import pytest
from live_page import LOOKS, SIZES, Budget, Opened, Served, serving
from playwright.sync_api import Browser, Page, expect
from stand_in_daemon import StandIn, standing_in

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
CELLS = 8
# What a first visit to the map may cost, drawn from the stand-in's cells.
BUDGET = Budget(files=17, kilobytes=700, elements=590)
# The browser's own line for a feed that couldn't connect, once the server is gone.
UNREACHED = "WebSocket connection to"
# The twin's size, past which the map once misdrew, and its orchestrations.
TWIN_CELLS = 10_000
TWIN_NETWORKS = 200
TWIN_RECEPTORS = 4
# How long a twin of ten thousand cells may take to read and draw.
TWIN_MS = 60_000
# A crowd of networks, past the seven names a screen holds, around one cell
# that needs him, which must take a name before any of them.
CROWD_NETWORKS = 20
CROWD_ASKING = "cell:ASKS01"


@pytest.fixture
def mycelium_node(tmp_path: Path) -> Iterator[StandIn]:
    yield from standing_in(tmp_path / "node.sock", "node", HERE / "node.json")


def twin() -> dict[str, list[dict[str, object]]]:
    """Node.json's realm grown to TWIN_CELLS sessions in TWIN_NETWORKS orchestrations.

    Every tenth orchestration nests in the one before it, and every seventh
    has no leader, as a network does when its cells hear its group while
    none answers to it, titled by its quest as the node titles one. The
    shapes that once misdrew on a real realm are in it too: orchestrations
    run from both machines, every eleventh cell hearing the next
    orchestration's group as well, a cell both machines still list as it
    moves, and TWIN_RECEPTORS receptors at the realm's edge.
    """
    reading = json.loads((HERE / "node.json").read_text())["kinds"]
    session = next(cell for cell in reading["cell"] if cell["type"] == "session")
    receptor = next(cell for cell in reading["cell"] if cell["type"] == "receptor")
    network = next(each for each in reading["network"] if each["shape"] == "orchestration")
    networks, cells = [], []
    for n in range(TWIN_NETWORKS):
        address = f"skf{n:03d}"
        networks.append(
            {
                **copy.deepcopy(network),
                "key": f"network:orchestration.{address}",
                "name": f"orchestration.{address}",
                "short": f"skiff-{n}",
                "title": f"Rig Skiff {n} for the Season",
                "within": f"orchestration.skf{n - 1:03d}" if n % 10 else None,
                "cells": [],
            }
        )
    per = TWIN_CELLS // TWIN_NETWORKS
    for n in range(TWIN_CELLS):
        hub = networks[n // per]
        name = f"T{n:05d}"
        leads = [hub["name"]] if n % per == 0 and (n // per) % 7 else []
        heard = [hub["name"]]
        if n % 11 == 0:
            beside = networks[(n // per + 1) % TWIN_NETWORKS]
            heard.append(beside["name"])
            beside["cells"].append(name)
        hub["cells"].append(name)
        cells.append(
            {
                **copy.deepcopy(session),
                "key": f"cell:{name}",
                "id": f"01a1a877-0000-7000-8000-{n:012d}",
                "name": name,
                "machine": "osprey" if (n // per + n) % 3 == 0 else "kestrel",
                "quest": f"QEST-20261116-{name}",
                "leads": leads,
                "networks": heard,
            }
        )
    # Still listed by the machine it left, an hour older.
    cells.append({**copy.deepcopy(cells[1]), "machine": "osprey", "since": "2026-11-16T08:00:00Z"})
    for n in range(TWIN_RECEPTORS):
        name = f"radio{n}@kestrel"
        cells.append({**copy.deepcopy(receptor), "key": f"cell:{name}", "name": name})
    others = [each for each in reading["network"] if each["shape"] != "orchestration"]
    return {**reading, "network": others + networks, "cell": cells}


def crowd() -> dict[str, list[dict[str, object]]]:
    """Node.json's realm as CROWD_NETWORKS orchestrations of two quiet cells, and one that asks.

    More networks than the seven names a screen holds, so the cap is reached
    at the whole map, and the asking cell is the one the cap must not take.
    """
    reading = json.loads((HERE / "node.json").read_text())["kinds"]
    quiet = next(cell for cell in reading["cell"] if cell["standing"] == "quiet")
    asking = next(cell for cell in reading["cell"] if cell["standing"] == "needs-you")
    network = next(each for each in reading["network"] if each["shape"] == "orchestration")
    networks, cells = [], []
    for n in range(CROWD_NETWORKS):
        name = f"orchestration.crowd{n:02d}"
        members = [f"Q{n:02d}A", f"Q{n:02d}B"]
        for at, member in enumerate(members):
            cells.append(
                {
                    **copy.deepcopy(quiet),
                    "key": f"cell:{member}",
                    "id": f"01a1a877-0000-7000-8001-{n * 2 + at:012d}",
                    "name": member,
                    "leads": [],
                    "networks": [name],
                }
            )
        networks.append(
            {
                **copy.deepcopy(network),
                "key": f"network:{name}",
                "name": name,
                "short": f"crowd-{n}",
                "title": f"Crowd {n}",
                "within": None,
                "cells": members,
            }
        )
    address = CROWD_ASKING.removeprefix("cell:")
    cells.append(
        {
            **copy.deepcopy(asking),
            "key": CROWD_ASKING,
            "id": "01a1a877-0000-7000-8002-000000000001",
            "name": address,
            "leads": [],
            "networks": [networks[0]["name"]],
        }
    )
    networks[0]["cells"].append(address)
    others = [each for each in reading["network"] if each["shape"] != "orchestration"]
    return {**reading, "network": others + networks, "cell": cells}


def serve(folder: Path, node: StandIn) -> Iterator[Served]:
    """Serve the map through the real server beside `node`, its feed as home.nix declares."""
    feed = {"socket": str(node.path), "kinds": json.loads(os.environ["NODE_KINDS"])}
    with serving(folder, {"mycelium": os.environ["MYCELIUM_SITE"]}, {"node": feed}) as served:
        yield served


@pytest.fixture
def mycelium_served(tmp_path: Path, mycelium_node: StandIn) -> Iterator[Served]:
    yield from serve(tmp_path, mycelium_node)


@pytest.fixture
def twin_served(tmp_path: Path) -> Iterator[Served]:
    stand_in = StandIn(tmp_path / "twin.sock", "node", twin()).start()
    yield from serve(tmp_path, stand_in)
    stand_in.leave()


@pytest.fixture
def crowd_served(tmp_path: Path) -> Iterator[Served]:
    stand_in = StandIn(tmp_path / "crowd.sock", "node", crowd()).start()
    yield from serve(tmp_path, stand_in)
    stand_in.leave()


def reach(browser: Browser, root: str, timeout: int = PACE_MS) -> dict[str, Any]:
    """Say what the map opened with `?reach` says of whether it reaches every cell."""
    opened = Opened(browser, f"{root}mycelium/?reach", SIZES[-1], "dark")
    expect(opened.page.locator("html"), "the map never said its reach").to_have_attribute(
        "data-reach", re.compile("."), timeout=timeout
    )
    said: dict[str, Any] = json.loads(
        opened.page.locator("html").get_attribute("data-reach") or "{}"
    )
    opened.close()
    return said


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


def test_the_map_reaches_every_cell(browser: Browser, mycelium_served: Served) -> None:
    root, _ = mycelium_served
    said = reach(browser, root)
    assert said["cells"] == CELLS, f"the map judged {said['cells']} of node.json's {CELLS} cells"
    assert said["failures"] == [], f"the map misses {said['failures']}"


def test_the_map_reaches_every_cell_of_ten_thousand(browser: Browser, twin_served: Served) -> None:
    root, _ = twin_served
    said = reach(browser, root, TWIN_MS)
    judged = TWIN_CELLS + TWIN_RECEPTORS
    assert said["cells"] == judged, f"the map judged {said['cells']} of {judged} cells"
    assert said["failures"] == [], f"the map misses {said['failures'][:5]} of {said['count']}"


def test_a_cell_that_needs_you_takes_a_name_before_the_networks(
    browser: Browser, crowd_served: Served
) -> None:
    root, _ = crowd_served
    opened = Opened(browser, f"{root}mycelium/", SIZES[-1], "light")
    try:
        the_map_stands(opened.page, "live")
        # The map lists the names the cap left off; with more networks than
        # names, it leaves some off, and never the cell that needs him.
        labels = opened.page.locator("#labels")
        expect(labels, "the crowd never reached the cap").to_have_attribute(
            "data-capped", re.compile("."), timeout=PACE_MS
        )
        capped = (labels.get_attribute("data-capped") or "").split()
        asking = CROWD_ASKING.replace("cell:", "c:")
        assert asking not in capped, f"the cap left off the cell that needs him, among {capped}"
        expect(
            labels.locator(f'.tag[data-name="{asking}"]'), "the cell that needs him went unnamed"
        ).to_be_visible()
    finally:
        opened.close()


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


def test_the_map_keeps_to_its_budget(browser: Browser, mycelium_served: Served) -> None:
    root, _ = mycelium_served
    opened = Opened(browser, f"{root}mycelium/", SIZES[-1], "dark")
    try:
        the_map_stands(opened.page, "live")
        the_map_counts(opened.page, COUNTED, "the map didn't draw")
        opened.keeps_to(BUDGET)
        the_map_stands(opened.page, "live")
    finally:
        opened.close()


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

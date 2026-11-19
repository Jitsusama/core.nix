"""The mycelium map reads live from its node, through the sites' server, in a real browser.

The page is the one Nix builds, served by the real local-sites server beside a
stand-in node (modules/local-sites/tests/stand_in_daemon.py) holding a small
realm under test names, node.json, through the node feed exactly as home.nix
declares it for a machine in a realm (NODE_KINDS). It must say it's live with
the realm drawn, follow a change within its pace, say broken when the node
stops while keeping the realm drawn, and live again when it's back, say stale
when the server goes, draw a record's words as words and never as markup,
draw a receptor's feeding of a cell while the node says it and not after,
answer a key of each kind keys.ts decides (the keys dialog, find and its
field, stepping through what was found, Escape's order, the walk, the
machines, Now, the tree and the theme switch), and pass the web-design
skill's page checks and the map's own (site-checks.js) at a phone's width
and a desk's in both looks. Any error the page logs fails it, since that is
how the map froze on "reading" unseen, and how the drawing says a key
keys.ts can ask does nothing. The server, the browser and the tab are
modules/local-sites/tests/live_page.py's.
"""

import copy
import json
import os
import re
from datetime import UTC, datetime, timedelta
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
# Two machines lent from one basin, a cell moved onto each.
BASIN = "shoal"
BASIN_MARK = f"{BASIN} basin"
BASIN_MACHINES = ("pool-a", "pool-b")
BASIN_CELLS = ("HERON1", "HERON2")
# A cell find is asked for by name, and the title its drawer heads it with.
SOUGHT = "TERN03"
SOUGHT_TITLE = "Count the Cleats"
# A receptor of node.json's feeding one of its cells a call's summary, as the
# node says a feeding under its `fed` kind.
FED_RECEPTOR = "calls@kestrel"
FED_CELL = "SKIFF1"
FED_KEY = f"fed:{FED_RECEPTOR}~{FED_CELL}"
FED_SAID = "a call's summary"
# The names the two ends' drawers head them with: the receptor's title, and
# the cell's short name, from its window in node.json.
FED_RECEPTOR_TITLE = "Calls"
FED_CELL_TITLE = "Rig The Skiff"
# How long before its ten minutes end a second feeding is told, to see the
# map let it go on its own clock with the node saying nothing more: the six
# seconds a change may take to be drawn, and as many again for a busy builder,
# so it is drawn before it goes.
FED_ENDING_S = 14


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


def basins() -> dict[str, list[dict[str, object]]]:
    """Node.json's realm with the BASIN_MACHINES lent from one basin, a cell on each."""
    reading = json.loads((HERE / "node.json").read_text())["kinds"]
    lent = next(machine for machine in reading["machine"] if machine["name"] == "osprey")
    declared = {"reach": ["internet"], "leaves_at": None, "basin": BASIN}
    pool = [
        {**copy.deepcopy(lent), "key": f"machine:{name}", "name": name, "declared": declared}
        for name in BASIN_MACHINES
    ]
    cells = copy.deepcopy(reading["cell"])
    moved = (cell for cell in cells if cell["name"] in BASIN_CELLS)
    for cell, machine in zip(moved, BASIN_MACHINES, strict=True):
        cell["machine"] = machine
    return {**reading, "machine": reading["machine"] + pool, "cell": cells}


def feeding(cell: str, ago: timedelta = timedelta()) -> dict[str, object]:
    """FED_RECEPTOR feeding `cell` a call's summary, `ago` before now, as the node serves it."""
    said = datetime.now(UTC) - ago
    return {
        "key": f"fed:{FED_RECEPTOR}~{cell}",
        "kind": "fed",
        "heard": int(said.timestamp() * 1000),
        "receptor": FED_RECEPTOR,
        "cell": cell,
        "at": said.isoformat().replace("+00:00", "Z"),
        # The node moves a field named as the interface's own under source.
        "source": {"kind": "summary"},
    }


def fed() -> dict[str, list[dict[str, object]]]:
    """Node.json's realm with FED_RECEPTOR having just fed FED_CELL a call's summary."""
    reading = json.loads((HERE / "node.json").read_text())["kinds"]
    return {**reading, "fed": [feeding(FED_CELL)]}


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


@pytest.fixture
def basins_served(tmp_path: Path) -> Iterator[Served]:
    stand_in = StandIn(tmp_path / "basins.sock", "node", basins()).start()
    yield from serve(tmp_path, stand_in)
    stand_in.leave()


@pytest.fixture
def fed_node(tmp_path: Path) -> Iterator[StandIn]:
    stand_in = StandIn(tmp_path / "fed.sock", "node", fed()).start()
    yield stand_in
    stand_in.leave()


@pytest.fixture
def fed_served(tmp_path: Path, fed_node: StandIn) -> Iterator[Served]:
    yield from serve(tmp_path, fed_node)


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


def test_the_keys_dialog_and_find_answer_their_keys(mycelium_opened: Opened) -> None:
    page = mycelium_opened.page
    title, find_line = page.locator("#panel-title"), page.locator("#find-line")
    the_map_stands(page, "live")
    page.locator("#map-view").focus()
    page.keyboard.press("?")
    expect(page.locator("#keys"), "? didn't open the keys").to_be_visible()
    page.keyboard.press("?")
    expect(page.locator("#keys"), "? didn't close the keys").to_be_hidden()
    page.keyboard.press("/")
    expect(page.locator("#find"), "/ didn't open find").to_be_focused()
    # A letter typed into find is find's: m here shows no machines.
    page.keyboard.press("m")
    expect(page.locator("#find"), "m typed into find didn't stay there").to_have_value("m")
    expect(page.locator("#where"), "m typed into find acted").not_to_contain_text("machines")
    # Escape in find's own field closes the line, and the map takes focus.
    page.keyboard.press("Escape")
    expect(find_line, "Escape in find didn't close it").to_be_hidden()
    expect(page.locator("#map-view"), "closing find left focus nowhere").to_be_focused()
    page.keyboard.press("/")
    page.keyboard.type(SOUGHT)
    page.keyboard.press("Enter")
    expect(title, "Return didn't open what was found").to_have_text(SOUGHT_TITLE)
    # Several match the start of its name, and n and N step between them.
    page.keyboard.press("/")
    page.keyboard.type(SOUGHT[:-1])
    page.keyboard.press("Enter")
    expect(title, "Return found nothing").not_to_have_text("")
    first = title.inner_text()
    page.keyboard.press("n")
    expect(title, "n didn't go to the next match").not_to_have_text(first)
    page.keyboard.press("Shift+N")
    expect(title, "N didn't go back to the match before").to_have_text(first)
    # Escape closes the find line and only that, then lets go.
    page.keyboard.press("Escape")
    expect(find_line, "Escape didn't close find first").to_be_hidden()
    expect(title, "the first Escape let go too").to_have_text(first)
    page.keyboard.press("Escape")
    expect(page, "Escape didn't let go of the cell").to_have_url(re.compile(r"/mycelium/$"))


def test_the_walk_the_tree_and_the_switch_answer_their_keys(mycelium_opened: Opened) -> None:
    page = mycelium_opened.page
    where = page.locator("#where")
    the_map_stands(page, "live")
    page.locator("#map-view").focus()
    # Focus starts the walk somewhere, so Return opens what it reached, and a
    # step with j then opens something else.
    page.keyboard.press("Enter")
    expect(where, "Return didn't open what the walk reached").to_contain_text("Opened")
    opened = where.inner_text()
    page.keyboard.press("j")
    page.keyboard.press("Enter")
    expect(where, "j didn't walk anywhere new").not_to_have_text(opened)
    expect(where, "Return didn't open where j walked").to_contain_text("Opened")
    page.keyboard.press("m")
    expect(where, "m didn't show the machines").to_contain_text("machines are shown")
    page.keyboard.press(".")
    expect(page.locator("#panel-title"), ". didn't open Now").to_have_text("Now")
    items = page.locator("#tree [role=treeitem]")
    items.first.focus()
    page.keyboard.press("j")
    expect(items.nth(1), "j didn't step down the tree").to_be_focused()
    page.keyboard.press("Shift+G")
    expect(items.last, "G didn't go to the tree's last").to_be_focused()
    page.keyboard.press("g")
    expect(items.first, "g didn't go to the tree's first").to_be_focused()
    themes = page.locator(".switch button")
    themes.first.focus()
    page.keyboard.press("ArrowLeft")
    expect(themes.last, "the theme switch didn't wrap round").to_be_focused()


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
        labels = opened.page.locator("#labels")
        # With more networks than names, the cap leaves some off.
        expect(labels, "the crowd never reached the cap").to_have_attribute(
            "data-capped", re.compile("."), timeout=PACE_MS
        )
        # Names are laid out in the order the cap takes them: the cell that
        # needs him comes before every network, so the cap never leaves it.
        order = labels.evaluate(
            "(el) => [...el.querySelectorAll(':scope > .tag:not(.t-mark)')]"
            ".map((g) => g.dataset.name)"
        )
        asking = CROWD_ASKING.replace("cell:", "c:")
        assert order[0] == asking, f"the cell that needs him is named after {order[:3]}"
        # Drawn at the whole map, small as it is, it is named there.
        expect(
            labels.locator(f'.tag[data-name="{asking}"]'), "the cell that needs him went unnamed"
        ).to_be_visible()
    finally:
        opened.close()


def test_a_basins_machines_are_drawn_as_one(browser: Browser, basins_served: Served) -> None:
    root, _ = basins_served
    # Other sites link a machine by its own name, so a link to a basin's
    # machine opens the basin it is drawn as, and the address says so.
    opened = Opened(browser, f"{root}mycelium/#machine={BASIN_MACHINES[0]}", SIZES[-1], "light")
    try:
        page = opened.page
        the_map_stands(page, "live")
        the_map_counts(page, COUNTED, "the map miscounted the realm")
        # Two machines of their own and the basin: its machines are never
        # drawn apart, and its mark is named by the basin, open at the top.
        expect(page.locator("#world g.machine"), "the basin was drawn apart").to_have_count(3)
        expect(
            page.locator(f'#labels .tag.t-basin[data-mark="m:{BASIN_MARK}"]'),
            "the basin had no mark of its own",
        ).to_have_count(1)
        # Its drawer lists each of its machines, its cells counted on each.
        title = page.locator("#panel-title")
        expect(title, "the basin's machine didn't open the basin").to_have_text(BASIN_MARK)
        expect(page, "the address kept the machine's name").to_have_url(
            re.compile(f"#machine={re.escape(BASIN)}%20basin$")
        )
        body = page.locator("#panel-body")
        said = "A basin, drawn as one machine: 2 machines"
        expect(body, "the drawer didn't say it was a basin").to_contain_text(said)
        for name in BASIN_MACHINES:
            exactly = page.locator(".name", has_text=re.compile(f"^{re.escape(name)}$"))
            row = body.locator(".health .row", has=exactly)
            expect(row, f"the drawer left {name} out").to_contain_text(": 1 cell;")
        # A cell on one says the machine it runs on, and the basin it's in.
        page.evaluate(f"location.hash = '#cell={BASIN_CELLS[0]}'")
        expect(body, "the cell lost its own machine").to_contain_text(
            f"on {BASIN_MACHINES[0]}, in the {BASIN} basin", timeout=PACE_MS
        )
        # Now counts every machine, a basin's each its own, not each mark.
        page.locator("#now-control").click()
        expect(title, "Now never opened").to_have_text("Now")
        machines = len(BASIN_MACHINES) + 2
        expect(body, "Now counted the basin as one machine").to_contain_text(
            f"of {machines} machines."
        )
    finally:
        opened.close()


def test_a_basin_passes_the_maps_own_checks(browser: Browser, basins_served: Served) -> None:
    root, _ = basins_served
    opened = Opened(browser, f"{root}mycelium/", SIZES[-1], "light")
    try:
        the_map_stands(opened.page, "live")
        the_map_counts(opened.page, COUNTED, "the map didn't draw")
        # They open every machine's drawer, the basin's among them.
        found = opened.page.evaluate((HERE / "site-checks.js").read_text())
        assert found == [], f"site-checks.js on a realm with a basin found {found}"
    finally:
        opened.close()


def test_a_receptor_feeding_a_cell_something_is_drawn_between_the_two(
    browser: Browser, fed_served: Served, fed_node: StandIn
) -> None:
    root, _ = fed_served
    opened = Opened(browser, f"{root}mycelium/#cell={FED_RECEPTOR}", SIZES[-1], "light")
    try:
        page = opened.page
        the_map_stands(page, "live")
        the_map_counts(page, COUNTED, "a feeding changed the count")
        page.wait_for_function(
            "document.getElementById('map-ground').painted().fed === 1", timeout=PACE_MS
        )
        # Each end's drawer says what was fed, the cell's by whom.
        title = page.locator("#panel-title")
        body = page.locator("#panel-body")
        expect(title, "the receptor's drawer never opened").to_contain_text(FED_RECEPTOR_TITLE)
        receptor = title.inner_text()
        page.evaluate(f"location.hash = '#cell={FED_CELL}'")
        expect(title, "the cell's drawer never opened").to_contain_text(FED_CELL_TITLE)
        cell = title.inner_text()
        expect(body, "the cell's drawer left out what it was fed").to_contain_text(
            f"{FED_SAID}, by {receptor}", timeout=PACE_MS
        )
        page.evaluate(f"location.hash = '#cell={FED_RECEPTOR}'")
        expect(body, "the receptor's drawer left out what it fed to whom").to_contain_text(
            f"{cell}, {FED_SAID}", timeout=PACE_MS
        )
        found = page.evaluate((HERE / "site-checks.js").read_text())
        assert found == [], f"site-checks.js on a realm with a feeding found {found}"
        # A feeding near its ten minutes' end goes on the map's own clock,
        # with the node saying nothing more of it.
        ending = feeding("TERN03", timedelta(minutes=10) - timedelta(seconds=FED_ENDING_S))
        fed_node.tell("fed", changed=[ending])
        page.wait_for_function(
            "document.getElementById('map-ground').painted().fed === 2", timeout=PACE_MS
        )
        page.wait_for_function(
            "document.getElementById('map-ground').painted().fed === 1",
            timeout=PACE_MS + FED_ENDING_S * 1000,
        )
        # Once the node stops saying one, its line goes too.
        fed_node.tell("fed", gone=[FED_KEY])
        page.wait_for_function(
            "document.getElementById('map-ground').painted().fed === 0", timeout=PACE_MS
        )
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

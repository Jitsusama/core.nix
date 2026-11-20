"""The mycelium map reads live from its node, through the sites' server, in a real browser.

The page is the one Nix builds, served by the real local-sites server beside a
stand-in node (modules/local-sites/tests/stand_in_daemon.py) holding a small
realm under test names, node.json, through the node feed exactly as home.nix
declares it for a machine in a realm (NODE_KINDS). It must say it's live with
the realm drawn, follow a change within its pace, say broken when the node
stops while keeping the realm drawn, and live again when it's back, say stale
when the server goes, draw a record's words as words and never as markup,
list what each cell, machine, network and branch spent today, most first,
say what waits on a cell and what holds stand, as the node judges them,
draw every cell until one is too small to read, thin every line as the map
steps out past its own size, gather the talk between two networks into one
line until it or a cell it touches opens,
draw a receptor's feeding of a cell while the node says it and not after,
grow one network out of another while a forage says a cell of the one
started a cell of the other,
answer a key of each kind keys.ts decides (the keys dialog, find and its
field, stepping through what was found, Escape's order, the walk, the
machines, Now and Escape closing it, the view zoomed about its middle,
panned across and down and fitted, -
climbing from a cell to its network, the tree's steps, branches and
Return, and the theme switch), and pass the web-design
skill's page checks and the map's own (site-checks.js) at a phone's width
and a desk's in both looks. Any error the page logs fails it, since that is
how the map froze on "reading" unseen, and how the drawing says a key
keys.ts can ask does nothing. The server, the browser and the tab are
modules/local-sites/tests/live_page.py's.

Long because every case reads the one page against the one stand-in realm and
its fixtures, and splitting the cases by what they press or watch would give
each file a copy of that realm's serving.
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
from playwright.sync_api import TimeoutError as PlaywrightTimeoutError
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
# What a first visit to the map may cost, drawn from the stand-in's cells:
# 28 files since the balance, who started each cell, the tree beside the map,
# what each thing is in words, how a time is said, how each thing stands, what
# each is doing, what each spent, what waits, how wide each line is drawn, how
# a cell is named and what new work hands the Launcher were read in modules of
# their own; about 712 KB, with room above.
BUDGET = Budget(files=28, kilobytes=750, elements=600)
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
# map let it go on its own clock with the node saying nothing more. Its line
# may take FED_DRAWN_MS to be drawn, twice the map's pace for a busy builder,
# and two seconds more are left so it is drawn before it goes.
FED_DRAWN_MS = 2 * PACE_MS
FED_ENDING_S = FED_DRAWN_MS // 1000 + 2
# TERN03's question to HERON1, ten minutes old and unanswered, and GULL01's
# hold on the skiff's network and the harbour repository, five minutes old
# with most of an hour left; and the names Now and HERON1's drawer give each.
ASKER, WAITED, HOLDER = "TERN03", "HERON1", "GULL01"
ASKED_AGO, HELD_AGO, HELD_FOR = timedelta(minutes=10), timedelta(minutes=5), timedelta(hours=1)
HELD_ON = ["#orchestration/skiff1", "#repo/harbour"]
WAITS = ("asked", "held")
MINUTE = timedelta(minutes=1)
ASKER_SHORT, WAITED_SHORT, HOLDER_SHORT = "Count the Cleats", "Chart The Harbour", "Paint The Hull"
ASKED_WORDS = f"a question waiting; it came from {ASKER_SHORT} {{}} min ago, with no answer yet"
HELD_WORDS = "holding Rig the Skiff for the Season and the harbour repository"
# HERON1, which leads node.json's Chart the Harbour, said by its forage to
# have been started by SKIFF1, which leads Rig the Skiff, so the one network
# grows out of the other; and the names the two drawers head them with.
GROWN = "orchestration.heron1"
GROWN_FROM = "orchestration.skiff1"
GROWN_FROM_TITLE = "Rig the Skiff for the Season"
GROWN_CELL = "HERON1"
GROWN_BY = "SKIFF1"
GROWN_BY_TITLE = "Rig The Skiff"
# A cell's membrane in the map's units (view.js's R), the radius on screen it
# draws from (view.js's FOLD_PX), and the radius one step out from node.json's
# whole map stays under at a desk's width, 8 px across, where a hundred cells
# on a laptop's whole map once all folded.
CELL_R = 11
FOLD_PX = 2.5
STEPPED_PX = 4
# The least a line thins to as the map steps out (view.js's HAIRLINE), and a
# zoom far enough out that a line thins to well under what it carries.
HAIRLINE_PX = 1
THINNED_UNDER = 0.25
THICKER_BY = 0.05  # px a thinned line is told from a hairline or its own width by
# Two cells of Chart the Harbour who just told TERN03, of Mend the Sails,
# something, so their talk runs between two networks: HERON1 three times, as
# warm as view.js's second step (WARM_AT), and HERON2 once.
TALKERS = {"HERON1": 3, "HERON2": 1}
HEARD = "TERN03"
TALKED_WITH = "orchestration.heron1"


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


def unanswered(key: str, sender: str, ago: timedelta, **said: object) -> dict[str, object]:
    """Make what the node judges waits on the mesh, sent by `sender` `ago` before now."""
    since = datetime.now(UTC) - ago
    return {
        "key": f"unanswered:{key}",
        "kind": "unanswered",
        "heard": int(since.timestamp() * 1000),
        "id": key,
        "from": sender,
        "since": since.isoformat().replace("+00:00", "Z"),
        "text": "what was said",
        **said,
    }


def waiting() -> dict[str, list[dict[str, object]]]:
    """Node.json's realm with ASKER's question waiting on WAITED and HOLDER's hold standing."""
    reading = json.loads((HERE / "node.json").read_text())["kinds"]
    asked = unanswered("asked", ASKER, ASKED_AGO, intent="question", groups=[f"@{WAITED}"])
    held = unanswered("held", HOLDER, HELD_AGO, intent="hold", groups=HELD_ON)
    until = datetime.now(UTC) + HELD_FOR
    return {
        **reading,
        "unanswered": [
            {**asked, "waiting_on": [WAITED]},
            {**held, "waiting_on": [], "until": until.isoformat().replace("+00:00", "Z")},
        ],
    }


def foraging() -> dict[str, list[dict[str, object]]]:
    """Node.json's realm with forages served, none of them yet, as a node serves the kind."""
    reading = json.loads((HERE / "node.json").read_text())["kinds"]
    return {**reading, "forage": []}


def told(sender: str, nth: int) -> dict[str, object]:
    """Make `sender`'s `nth` finding for HEARD, a minute old, as node.json holds a message."""
    said = datetime.now(UTC) - MINUTE
    return {
        "key": f"message:{sender}:{nth}",
        "at": said.isoformat().replace("+00:00", "Z"),
        "from": sender,
        "intent": "finding",
        "groups": [f"peer.{HEARD.lower()}"],
    }


def talking() -> dict[str, list[dict[str, object]]]:
    """Node.json's realm with each of TALKERS having just told HEARD something."""
    reading = json.loads((HERE / "node.json").read_text())["kinds"]
    said = [told(sender, nth) for sender, times in TALKERS.items() for nth in range(times)]
    return {**reading, "message": said}


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


@pytest.fixture
def waiting_node(tmp_path: Path) -> Iterator[StandIn]:
    stand_in = StandIn(tmp_path / "waiting.sock", "node", waiting()).start()
    yield stand_in
    stand_in.leave()


@pytest.fixture
def waiting_served(tmp_path: Path, waiting_node: StandIn) -> Iterator[Served]:
    yield from serve(tmp_path, waiting_node)


@pytest.fixture
def forage_node(tmp_path: Path) -> Iterator[StandIn]:
    stand_in = StandIn(tmp_path / "forage.sock", "node", foraging()).start()
    yield stand_in
    stand_in.leave()


@pytest.fixture
def forage_served(tmp_path: Path, forage_node: StandIn) -> Iterator[Served]:
    yield from serve(tmp_path, forage_node)


@pytest.fixture
def talking_served(tmp_path: Path) -> Iterator[Served]:
    stand_in = StandIn(tmp_path / "talking.sock", "node", talking()).start()
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
    # h closes an open branch and l opens it again, then steps into it; h
    # steps back out, and Return opens the row the keys are on. The branch is
    # one whose first row is closed, since h on an open row closes it rather
    # than stepping out.
    key = page.evaluate(
        """[...document.querySelectorAll("#tree [role=treeitem][aria-expanded=true]")]
          .find((b) => b.querySelector("[role=treeitem]")
            ?.getAttribute("aria-expanded") !== "true")?.dataset.key ?? null""",
    )
    assert isinstance(key, str), f"no open branch whose first row is closed, but {key!r}"
    branch = page.locator(f'#tree [role=treeitem][data-key="{key}"]')
    branch.focus()
    page.keyboard.press("h")
    expect(branch, "h didn't close the open branch").to_have_attribute("aria-expanded", "false")
    page.keyboard.press("l")
    expect(branch, "l didn't open the closed branch").to_have_attribute("aria-expanded", "true")
    page.keyboard.press("l")
    under = branch.locator("[role=treeitem]").first
    expect(under, "l didn't step into the open branch").to_be_focused()
    page.keyboard.press("h")
    expect(branch, "h didn't step out to the branch above").to_be_focused()
    page.keyboard.press("Enter")
    expect(branch, "Return didn't open the row the keys are on").to_have_attribute(
        "aria-current",
        "true",
    )
    themes = page.locator(".switch button")
    themes.first.focus()
    page.keyboard.press("ArrowLeft")
    expect(themes.last, "the theme switch didn't wrap round").to_be_focused()


# What the view's transform says it shows: where its corner sits and how far in it is.
VIEWED = re.compile(r"translate\((-?[\d.]+),(-?[\d.]+)\) scale\(([\d.]+)\)")

# How far one Shift and a direction moves the map, in pixels, as view.js's PAN.
PANNED = 80


def viewed(page: Page) -> tuple[float, float, float]:
    """Where the map's view stands now, as its world's transform says."""
    transform = page.locator("#world").get_attribute("transform") or ""
    found = VIEWED.fullmatch(transform)
    assert found, f"the map's world is drawn at {transform!r}, no translate and scale"
    x, y, k = found.groups()
    return float(x), float(y), float(k)


# How long the view must read the same to stand still: three polls a fifth of a second apart,
# twice the fit's glide (view.js's GLIDE_MS, 320), so a glide whose first frame is late still
# moves between them.
STILL_POLLS = 3
STILL_POLL_MS = 200


def settled(page: Page, why: str) -> tuple[float, float, float]:
    """Where the view stands once it has stopped moving, as a fit glides there."""
    page.evaluate("window.viewedStill = 0; window.lastViewed = null")
    try:
        page.wait_for_function(
            """(polls) => {
              const now = document.getElementById("world").getAttribute("transform");
              const still = now !== null && now === window.lastViewed;
              window.viewedStill = still ? window.viewedStill + 1 : 0;
              window.lastViewed = now;
              return window.viewedStill >= polls;
            }""",
            arg=STILL_POLLS,
            polling=STILL_POLL_MS,
            timeout=PACE_MS,
        )
    except PlaywrightTimeoutError as moving:
        message = f"{why}: the view never stood still, last at {viewed(page)}"
        raise AssertionError(message) from moving
    return viewed(page)


def pivot(before: tuple[float, float, float], after: tuple[float, float, float]) -> float:
    """Read the height a zoom from `before` to `after` kept in place from how it moved."""
    by = after[2] / before[2]
    return (after[1] - before[1] * by) / (1 - by)


def test_the_view_and_now_answer_their_keys(mycelium_opened: Opened) -> None:
    page = mycelium_opened.page
    the_map_stands(page, "live")
    expect(page.locator("#world"), "the map was never drawn").to_have_attribute(
        "transform",
        VIEWED,
    )
    middle = page.evaluate(
        "document.getElementById('world').ownerSVGElement.getBoundingClientRect().height / 2",
    )
    page.locator("#map-view").focus()
    page.keyboard.press("0")
    whole = settled(page, "0 didn't fit the whole map")
    page.keyboard.press("i")
    zoomed = viewed(page)
    assert zoomed[2] > whole[2], f"i didn't zoom in from {whole}: {zoomed}"
    assert pivot(whole, zoomed) == pytest.approx(middle, abs=1), (
        f"i didn't zoom about the view's middle, {middle}: from {whole} to {zoomed}"
    )
    page.keyboard.press("o")
    assert viewed(page) == pytest.approx(whole, abs=0.05), (
        f"o didn't zoom back out to {whole}: {viewed(page)}"
    )
    page.keyboard.press("+")
    assert viewed(page)[2] > whole[2], f"+ didn't zoom in from {whole}: {viewed(page)}"
    page.keyboard.press("0")
    assert settled(page, "0 didn't fit after a zoom") == pytest.approx(whole, abs=0.05), (
        "0 didn't fit the whole map again"
    )
    page.keyboard.press("Shift+L")
    panned = (whole[0] - PANNED, whole[1], whole[2])
    assert viewed(page) == pytest.approx(panned, abs=0.05), (
        f"Shift and l didn't move the map one step from {whole}: {viewed(page)}"
    )
    page.keyboard.press("Shift+J")
    panned = (whole[0] - PANNED, whole[1] - PANNED, whole[2])
    assert viewed(page) == pytest.approx(panned, abs=0.05), (
        f"Shift and j didn't move the map one step down from {whole}: {viewed(page)}"
    )
    page.keyboard.press("0")
    assert settled(page, "0 didn't fit after a pan") == pytest.approx(whole, abs=0.05), (
        "0 didn't fit the map after a pan"
    )
    # Escape closes Now when nothing is open over it, and the drawer with it.
    now = page.locator("#now-control")
    page.keyboard.press(".")
    expect(now, ". didn't open Now").to_have_attribute("aria-expanded", "true")
    page.keyboard.press("Escape")
    expect(now, "Escape didn't close Now").to_have_attribute("aria-expanded", "false")
    expect(page.locator("#where"), "Escape didn't say Now closed").to_have_text("Now is closed.")
    # - climbs from a cell to the network it's in.
    page.keyboard.press("/")
    page.keyboard.type(SOUGHT)
    page.keyboard.press("Enter")
    expect(page.locator("#panel-title"), "find didn't open its cell").to_have_text(SOUGHT_TITLE)
    page.keyboard.press("-")
    expect(page, "- didn't climb to the cell's network").to_have_url(
        re.compile(r"/mycelium/#net=orchestration\.tern01$"),
    )


# Node.json's cells that need the reader, as Now lists them: the one asking
# something, then the one stuck. Node.json lists them in that order too, so
# waits.test.ts is what proves the order; this proves the page follows it.
NEEDING = ("TERN02", "HERON1")
# The machine the last of them runs on, in node.json, which its link names.
NEEDING_ON = "osprey"
NEEDING_SAID = ("One needs you:", "One is stuck:")
# The first line after Now's heading, and the next line over a list.
NOW_LEADS = """(() => {
  const first = document.getElementById("panel-title").nextElementSibling;
  const next = first.nextElementSibling.nextElementSibling;
  return [first.textContent, next.textContent];
})()"""
# Holds a click on a link to the Launcher, saying where it went, since a
# test browser has nothing to open it with.
TALK_HELD = """document.addEventListener("click", (event) => {
  const link = event.target.closest?.('a[href^="mycelium:"]');
  if (link) { event.preventDefault(); window.talkedTo = link.getAttribute("href"); }
}, true)"""


def test_now_leads_with_who_needs_you_and_a_and_t_reach_them(mycelium_opened: Opened) -> None:
    page = mycelium_opened.page
    the_map_stands(page, "live")
    page.locator("#map-view").focus()
    page.keyboard.press(".")
    body = page.locator("#panel-body")
    expect(body, "Now never said who needs you").to_contain_text(NEEDING_SAID[0])
    led = page.evaluate(NOW_LEADS)
    assert led == list(NEEDING_SAID), f"Now didn't lead with who needs you: {led}"
    for n, each in enumerate(NEEDING):
        rows = body.locator("ul.rows").nth(n).locator("[data-go]")
        expect(rows, f"Now's list under {NEEDING_SAID[n]} misread").to_have_count(1)
        expect(rows, f"Now's list under {NEEDING_SAID[n]} misread").to_have_attribute(
            "data-go", f"cell={each}"
        )
    # a steps through them in Now's order; t talks to the one open, which
    # by then is no longer the first, and a goes round to the first again.
    for each in NEEDING:
        page.keyboard.press("a")
        expect(page, f"a didn't open {each}").to_have_url(re.compile(rf"#cell={each}$"))
    page.evaluate(TALK_HELD)
    page.keyboard.press("t")
    talked = page.evaluate("window.talkedTo")
    assert talked == f"mycelium:cell/{NEEDING[-1]}@{NEEDING_ON}", (
        f"t talked to {talked}, not the open cell on its machine"
    )
    page.keyboard.press("a")
    expect(page, "a didn't go round to the first").to_have_url(re.compile(rf"#cell={NEEDING[0]}$"))
    # A receptor is a cell the Launcher can't talk to, and t says so.
    page.evaluate(f"location.hash = '#cell={FED_RECEPTOR}'")
    expect(page.locator("#panel-title"), "the receptor's drawer never opened").to_contain_text(
        FED_RECEPTOR_TITLE
    )
    page.locator("#map-view").focus()
    page.keyboard.press("t")
    expect(page.locator("#where"), "t on a receptor didn't say it can't talk").to_contain_text(
        "The Launcher can't talk to"
    )
    page.keyboard.press("Escape")
    expect(page, "Escape didn't let the cell go").not_to_have_url(re.compile("#cell="))
    page.keyboard.press("t")
    expect(page.locator("#where"), "t with no cell open didn't say so").to_have_text(
        "No cell is open to talk to; a opens the next that needs you."
    )


# A cell named the ways another page or a notice names it, each with what
# may follow its address: its whole link, its address on its machine, and
# its link as a page escapes it into the hash.
NAMED = (
    f"mycelium:cell/{NEEDING[-1]}@{NEEDING_ON}#notice:4",
    f"{NEEDING[-1]}@{NEEDING_ON}",
    f"mycelium%3Acell%2F{NEEDING[-1]}%40{NEEDING_ON}%2Fgo",
)


def test_a_cell_opens_by_its_whole_link_as_its_address(mycelium_opened: Opened) -> None:
    page = mycelium_opened.page
    the_map_stands(page, "live")
    talk = page.locator('#panel-body a[href^="mycelium:cell/"]')
    for named in NAMED:
        page.evaluate("location.hash = ''")
        expect(talk, "the last cell's drawer stayed open").to_have_count(0)
        page.evaluate(f"location.hash = '#cell={named}'")
        expect(talk, f"#cell={named} didn't open its cell's drawer").to_have_attribute(
            "href", f"mycelium:cell/{NEEDING[-1]}@{NEEDING_ON}"
        )
        expect(page, f"#cell={named} wasn't said as its address").to_have_url(
            re.compile(f"#cell={NEEDING[-1]}$")
        )
    # A stray % typed by hand names nothing, and the map still answers after it.
    page.evaluate("location.hash = '#cell=%E0%A4%A'")
    expect(talk, "a stray % opened a cell").to_have_count(0)
    page.evaluate(f"location.hash = '#cell={NAMED[1]}'")
    expect(talk, "the map stopped answering after a stray %").to_have_count(1)


# What new work's line hands the Launcher, said back once the line closes,
# and what it ends with.
HANDED = "Handed to the Launcher's Start New Work"
HANDED_END = "; nothing starts until you press Return there."


def test_s_hands_new_work_to_the_launcher_in_its_own_words(mycelium_opened: Opened) -> None:
    page = mycelium_opened.page
    the_map_stands(page, "live")
    page.evaluate(TALK_HELD)
    line = page.locator("#sprout-line")
    # With no cell open, s hands the words alone, and Return with none
    # hands nothing at all.
    page.locator("#map-view").focus()
    page.keyboard.press("s")
    expect(line, "s didn't open new work's line").to_be_visible()
    expect(page.locator("#trail"), "the trail stood beside the line").to_be_hidden()
    expect(page.locator("#sprout-from-line"), "a cell was offered with none open").to_be_hidden()
    page.keyboard.press("Enter")
    assert page.evaluate("window.talkedTo") is None, "Return with no words handed something on"
    page.keyboard.type("tidy the rows & more")
    page.keyboard.press("Enter")
    handed = page.evaluate("window.talkedTo")
    assert handed == "mycelium:sprout?words=tidy%20the%20rows%20%26%20more", f"s handed {handed}"
    expect(line, "the line stayed open once it handed the work on").to_be_hidden()
    expect(page.locator("#where"), "the hand-off wasn't said").to_have_text(HANDED + HANDED_END)
    # The control beside Now opens and closes the line, and says which.
    control = page.locator("#sprout-control")
    control.click()
    expect(line, "New Work didn't open the line").to_be_visible()
    expect(control, "New Work didn't say it's open").to_have_attribute("aria-expanded", "true")
    control.click()
    expect(line, "New Work didn't close the line").to_be_hidden()
    expect(control, "New Work didn't say it's shut").to_have_attribute("aria-expanded", "false")


def test_new_work_from_a_cell_takes_its_address_until_unticked(mycelium_opened: Opened) -> None:
    page = mycelium_opened.page
    the_map_stands(page, "live")
    page.evaluate(TALK_HELD)
    # From a cell's drawer, its address goes with the words, and a tick
    # lets it go.
    page.locator("#map-view").focus()
    page.keyboard.press("a")
    expect(page, "a didn't open a cell").to_have_url(re.compile(rf"#cell={NEEDING[0]}$"))
    drawer = page.locator("#panel-body").get_by_role("button", name="Start New Work From It")
    drawer.click()
    offered = page.locator("#sprout-from-words")
    expect(offered, "the open cell wasn't offered").to_be_visible()
    short = offered.inner_text().removeprefix("From ")
    page.keyboard.type("follow up")
    page.keyboard.press("Enter")
    handed = page.evaluate("window.talkedTo")
    want = f"mycelium:sprout?words=follow%20up&from={NEEDING[0]}"
    assert handed == want, f"the drawer handed {handed}, not {want}"
    expect(page.locator("#where"), "the hand-off didn't name its cell").to_have_text(
        f"{HANDED}, from {short}{HANDED_END}"
    )
    expect(drawer, "the keys didn't go back to the drawer's button").to_be_focused()
    # The words handed on come back chosen, so Return hands them again.
    drawer.click()
    expect(page.locator("#sprout"), "the handed words didn't come back").to_have_value("follow up")
    page.keyboard.press("Escape")
    expect(drawer, "Escape didn't give the keys back to the drawer").to_be_focused()
    page.locator("#map-view").focus()
    page.keyboard.press("s")
    expect(page.locator("#sprout-from-line"), "s didn't offer the open cell").to_be_visible()
    expect(page.locator("#sprout"), "Escape kept the words it let go of").to_have_value("")
    page.locator("#sprout-from").uncheck()
    # s again while the line is open only takes the keys back, the tick
    # still cleared.
    page.locator("#map-view").focus()
    page.keyboard.press("s")
    expect(page.locator("#sprout"), "s didn't take the keys back to the line").to_be_focused()
    expect(page.locator("#sprout-from"), "s again ticked the cell back").not_to_be_checked()
    page.locator("#sprout").fill("on its own")
    page.locator("#sprout").press("Enter")
    handed = page.evaluate("window.talkedTo")
    assert handed == "mycelium:sprout?words=on%20its%20own", f"the cleared tick handed {handed}"


def test_escape_closes_only_new_works_line(mycelium_opened: Opened) -> None:
    page = mycelium_opened.page
    the_map_stands(page, "live")
    page.evaluate(TALK_HELD)
    line = page.locator("#sprout-line")
    # Escape on the line's button, or on the map while it's open, closes
    # only the line: the cell stays open.
    page.locator("#map-view").focus()
    page.keyboard.press("a")
    expect(page, "a didn't open a cell").to_have_url(re.compile(rf"#cell={NEEDING[0]}$"))
    page.keyboard.press("s")
    page.locator(".sprout-go").focus()
    page.keyboard.press("Escape")
    expect(line, "Escape on the button didn't close the line").to_be_hidden()
    expect(page, "Escape on the button let the cell go").to_have_url(re.compile("#cell="))
    page.keyboard.press("s")
    page.locator("#map-view").focus()
    page.keyboard.press("Escape")
    expect(line, "Escape on the map didn't close the line").to_be_hidden()
    expect(page, "Escape closing the line let the cell go").to_have_url(re.compile("#cell="))
    # Escape closes the line and hands nothing; a receptor is never
    # offered as the cell work starts from.
    page.evaluate(f"location.hash = '#cell={FED_RECEPTOR}'")
    expect(page.locator("#panel-title"), "the receptor's drawer never opened").to_contain_text(
        FED_RECEPTOR_TITLE
    )
    page.locator("#map-view").focus()
    page.keyboard.press("s")
    expect(line, "s on a receptor didn't open the line").to_be_visible()
    expect(page.locator("#sprout-from-line"), "a receptor was offered").to_be_hidden()
    page.keyboard.type("not this")
    page.keyboard.press("Escape")
    expect(line, "Escape didn't close new work's line").to_be_hidden()
    expect(page.locator("#trail"), "the trail didn't come back").to_be_visible()
    assert page.evaluate("window.talkedTo") is None, "Escape handed the words on"


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
        # Its link to the Launcher names that machine too, never the basin.
        expect(
            body.locator('a[href^="mycelium:cell/"]'), "the cell's link lost its machine"
        ).to_have_attribute("href", f"mycelium:cell/{BASIN_CELLS[0]}@{BASIN_MACHINES[0]}")
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
            "document.getElementById('map-ground').painted().fed === 2", timeout=FED_DRAWN_MS
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


def test_a_network_whose_cell_another_started_grows_out_of_it(
    browser: Browser, forage_served: Served, forage_node: StandIn
) -> None:
    root, _ = forage_served
    opened = Opened(browser, f"{root}mycelium/#net={GROWN}", SIZES[-1], "light")
    try:
        page = opened.page
        the_map_stands(page, "live")
        hub = page.locator(f'#world [data-key="h:{GROWN}"]')
        expect(hub, "the network to grow was never drawn").to_have_count(1, timeout=PACE_MS)
        page.wait_for_timeout(REDRAWN_MS)
        expect(hub, "a network nobody else started was drawn as a branch").not_to_have_attribute(
            "data-branch-of", re.compile(".")
        )
        # Where it stood before it grew: a branch is laid out further out,
        # so a layout kept from before the forage would leave it there.
        stood = hub.get_attribute("transform")
        started = {"started_by": GROWN_BY, "split_from": None, "parent": None}
        key = f"forage:{GROWN_CELL}"
        heard = int(datetime.now(UTC).timestamp() * 1000)
        forage = {"key": key, "kind": "forage", "heard": heard, "cell": GROWN_CELL}
        forage_node.tell("forage", changed=[{**forage, "lineage": started}])
        expect(hub, "the network never grew out of the one that started it").to_have_attribute(
            "data-branch-of", GROWN_FROM, timeout=PACE_MS
        )
        expect(hub, "the branch was drawn where it stood before it grew").not_to_have_attribute(
            "transform", stood or ""
        )
        the_map_counts(page, COUNTED, "a branch grown changed the count")
        # Its drawer says what it grew out of and whose start grew it, and
        # the cell's says who started it.
        body = page.locator("#panel-body")
        expect(body, "the branch's drawer never said what it grew out of").to_contain_text(
            f"Grown out of {GROWN_FROM_TITLE}", timeout=PACE_MS
        )
        expect(body, "the branch's drawer never said whose start grew it").to_contain_text(
            f"Started by {GROWN_BY_TITLE}: 1 of its 2 cells"
        )
        found = page.evaluate((HERE / "site-checks.js").read_text())
        assert found == [], f"site-checks.js on a realm with a branch grown found {found}"
        page.evaluate(f"location.hash = '#cell={GROWN_CELL}'")
        expect(body, "the cell's drawer never said who started it").to_contain_text(
            f"by {GROWN_BY_TITLE}", timeout=PACE_MS
        )
        # A cell with a forage opens the forage page on it, and one without
        # doesn't, since the forage page would have nothing to show.
        foraged = body.locator(f'a[href="../forage/#cell={GROWN_CELL}"]')
        expect(foraged, "the cell's drawer never opened its forage").to_have_count(1)
        # Once its forage goes, so does the branch it grew.
        forage_node.tell("forage", gone=[key])
        expect(hub, "the branch outlived the forage that grew it").not_to_have_attribute(
            "data-branch-of", re.compile("."), timeout=PACE_MS
        )
        expect(foraged, "the cell's drawer kept a forage that went").to_have_count(0)
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
    _, server = mycelium_served
    mycelium_opened.loses(server)
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


# What node.json's cells spent today, as the Spent Today fold lists it under
# its sum, line by line: by machine and network in Now, by network on osprey
# with only the cells that run there, and by branch in the skiff's network.
SPENT_NOW = ["$8.10", "By Machine", "kestrel", "$5.60, 5 cells", "osprey", "$2.50, 3 cells"]
SPENT_NOW_NETWORKS = ["By Network", "Rig the Skiff for the Season", "$6.10, 5 cells"]
SPENT_NOW_CELLS = ["Chart the Harbour", "$2.00, 2 cells", "Its Cells", "Rig The Skiff"]
SPENT_ON_OSPREY = ["$2.50", "By Network", "Chart the Harbour", "$2.00, 2 cells"]
SPENT_ON_OSPREY_CELLS = ["Rig the Skiff for the Season", "$0.50, 1 cell", "Its Cells"]
SPENT_IN_SKIFF = ["$6.10", "By Branch", "Its own cells", "$4.75, 3 cells"]
SPENT_IN_SKIFF_BRANCHES = ["Mend the Sails", "$1.35, 2 cells", "Its Cells", "Rig The Skiff"]


def spent(page: Page) -> list[str]:
    """Open the drawer's Spent Today fold and read its lines after its heading and hint."""
    fold = page.locator("#panel details.spend")
    # A click landing while the drawer is redrawn can be lost, and a redraw
    # keeps an open fold open, so press only while it is shut and wait for it.
    if fold.get_attribute("open") is None:
        fold.locator("summary").click()
    expect(fold.locator("ul.rows").first, "the Spent Today fold never opened").to_be_visible()
    lines = [line for line in fold.inner_text().splitlines() if line.strip()]
    return lines[1:2] + lines[3:]


def test_what_each_spent_today_is_listed_most_first(mycelium_opened: Opened) -> None:
    page = mycelium_opened.page
    the_map_stands(page, "live")
    page.locator("#map-view").focus()
    page.keyboard.press(".")
    now = SPENT_NOW + SPENT_NOW_NETWORKS + SPENT_NOW_CELLS
    said = spent(page)
    assert said[: len(now)] == now, f"Now's fold misread what was spent: {said}"
    page.evaluate("location.hash = '#machine=osprey'")
    expect(page.locator("#panel-title"), "osprey's drawer never opened").to_have_text("osprey")
    osprey = SPENT_ON_OSPREY + SPENT_ON_OSPREY_CELLS
    said = spent(page)
    assert said[: len(osprey)] == osprey, (
        f"osprey's fold should count only the cells that run there: {said}"
    )
    page.evaluate("location.hash = '#net=orchestration.skiff1'")
    expect(page.locator("#panel-title"), "the skiff's drawer never opened").to_have_text(
        "Rig the Skiff for the Season"
    )
    skiff = SPENT_IN_SKIFF + SPENT_IN_SKIFF_BRANCHES
    said = spent(page)
    assert said[: len(skiff)] == skiff, (
        f"the skiff's fold should count its own cells beside its branch: {said}"
    )


def day_clock(said: object, now: datetime) -> str:
    """Say a time within a day of `now` as when.ts's dayClock does, in the browser's zone."""
    t = datetime.fromisoformat(str(said)).astimezone()
    days = (t.date() - now.astimezone().date()).days
    assert abs(days) <= 1, f"day_clock says only times within a day of now, not {said!r}"
    hm = f"{t:%H:%M}"
    return {0: hm, -1: f"yesterday at {hm}", 1: f"tomorrow at {hm}"}[days]


def test_what_waits_and_what_holds_are_said_as_the_node_judges(
    browser: Browser, waiting_served: Served, waiting_node: StandIn
) -> None:
    root, _ = waiting_served
    opened = Opened(browser, f"{root}mycelium/", SIZES[-1], "dark")
    try:
        page = opened.page
        the_map_stands(page, "live")
        page.locator("#map-view").focus()
        page.keyboard.press(".")
        body = page.locator("#panel-body")
        expect(body, "Now never listed the holds standing").to_contain_text("Holds Standing")
        lines = [line for line in body.inner_text().splitlines() if line.strip()]
        # The page says each against its reading's time, which falls between
        # the realm being built and now, so either end's words will do.
        read = datetime.now(UTC)
        asked, held = (waiting_node.records["unanswered"][f"unanswered:{k}"] for k in WAITS)
        since = datetime.fromisoformat(str(asked["since"]))
        ends = (since + ASKED_AGO, read)
        ages = range(round(ASKED_AGO / MINUTE), round((read - since) / MINUTE) + 1)
        reach = [f"{HELD_WORDS} since {day_clock(held['since'], at)}" for at in ends]
        until = [f"until {day_clock(held['until'], at)} at most" for at in ends]
        first = lines.index("Questions Waiting") if "Questions Waiting" in lines else len(lines)
        glance = next(n for n, line in enumerate(lines) if line.startswith("Your whole network"))
        assert first < glance, f"Now should say what holds the flow before the glance: {lines}"
        said = lines[first : first + 6]
        assert said[:2] == ["Questions Waiting", WAITED_SHORT], f"Now misread who waits: {said}"
        assert said[2:3] in [[ASKED_WORDS.format(age)] for age in ages], (
            f"Now misread the question waiting: {said}"
        )
        assert said[3:5] == ["Holds Standing", HOLDER_SHORT], f"Now misread who holds: {said}"
        held_words = [f"{one}, {other}" for one, other in zip(reach, until, strict=True)]
        assert said[5:] in [[words] for words in held_words], f"Now misread the hold: {said}"
        page.evaluate(f"location.hash = '#cell={WAITED}'")
        title = page.locator("#panel-title")
        expect(title, f"{WAITED}'s drawer never opened").to_have_text(WAITED_SHORT)
        waits = f"{ASKER_SHORT}'s question waiting since"
        owed = [f"{waits} {day_clock(asked['since'], at)}" for at in ends]
        expect(body, f"{WAITED}'s health left out the question waiting on it").to_contain_text(
            re.compile("|".join(map(re.escape, owed)))
        )
    finally:
        opened.close()


PAINTED = "document.getElementById('map-ground').painted()"


def test_every_cell_is_drawn_until_too_small_to_read(mycelium_opened: Opened) -> None:
    page = mycelium_opened.page
    the_map_stands(page, "live")
    page.locator("#map-view").focus()
    page.keyboard.press("0")
    settled(page, "0 didn't fit the whole map")
    page.keyboard.press("o")
    *_, k = settled(page, "o didn't step out from the whole map")
    assert FOLD_PX <= k * CELL_R < STEPPED_PX, (
        f"one step out drew a cell {k * CELL_R:.2f} px round, not the case this reads"
    )
    cells = page.locator("#world circle.cell")
    expect(cells, "a step out folded cells big enough to read").to_have_count(CELLS)
    assert page.evaluate(f"{PAINTED}.folded") == 0, "a step out folded a network"
    # Far enough out, a cell is too small to read as one and its network
    # folds, all but the one the walk points at.
    while k * CELL_R >= FOLD_PX:
        page.keyboard.press("o")
        *_, further = settled(page, "o didn't step further out")
        assert further < k, f"the map stopped stepping out at a cell {k * CELL_R:.2f} px round"
        k = further
    page.wait_for_function(f"{PAINTED}.folded > 0", timeout=PACE_MS)
    assert cells.count() < CELLS, "every cell was drawn too small to read"


# One cell of each kind node.json holds: a receptor, a moving session
# leading a network, a stuck one leading another, and a quiet one that
# leads none.
SENSOR = f"c:{FED_RECEPTOR}"
LEADER = f"c:{FED_CELL}"
STUCK_LEADER = f"c:{GROWN_CELL}"
WORKER = "c:TERN03"
MARKS = "document.getElementById('map-ground').marksOf({!r})"
LOOKED = "(look) => { document.documentElement.dataset.theme = look; }"

type Mark = dict[str, str | float | None]


def marks_of(page: Page, key: str) -> list[Mark]:
    """Read what one cell's marks are painted in, failing when none were drawn."""
    marks: list[Mark] = page.evaluate(MARKS.format(key))
    assert marks, f"{key} wasn't drawn"
    return marks


def paints(marks: list[Mark]) -> set[str | float | None]:
    """Every ink a cell's marks are painted in, fill or stroke."""
    return {ink for mark in marks for ink in (mark["fill"], mark["stroke"]) if ink}


def membrane_of(marks: list[Mark]) -> Mark:
    """Find a session's membrane among its marks."""
    membrane = next((mark for mark in marks if str(mark["cls"]).startswith("m-membrane")), None)
    assert membrane, f"a session was drawn with no membrane: {marks}"
    return membrane


# A cell keeps its strokes' widths as the map steps out while the cell
# shrinks, so what tells one kind from another a few pixels across is its
# inks and widths, which read the same at every zoom; these are read in
# each look, since each sets its own.
@pytest.mark.parametrize("look", ["dark", "light"])
def test_each_kind_of_cell_is_painted_apart(
    mycelium_opened: Opened, mycelium_node: StandIn, look: str
) -> None:
    page = mycelium_opened.page
    the_map_stands(page, "live")
    opened_in = next(m for m in marks_of(page, WORKER) if m["cls"] == "m-quiet")["stroke"]
    page.evaluate(LOOKED, look)
    worker = marks_of(page, WORKER)
    dot = next((mark for mark in worker if mark["cls"] == "m-quiet"), None)
    assert dot, f"the quiet worker {WORKER} was drawn with no quiet dot: {worker}"
    gold = dot["stroke"]
    assert gold, f"the worker's quiet dot had no ink: {worker}"
    # The page opens dark, so the light look must have changed the inks,
    # or every comparison below would only read the dark ones again.
    assert (gold == opened_in) == (look == "dark"), (
        f"the {look} look painted the dot {gold}, opened in {opened_in}"
    )
    # A session's dot is its gold, which is most of what shows of a cell
    # a few pixels across, so a receptor carries none of it, quiet or
    # reading.
    quiet = marks_of(page, SENSOR)
    assert any(mark["cls"] == "m-sensed" and mark["stroke"] for mark in quiet), (
        f"a quiet receptor was drawn with no dot: {quiet}"
    )
    assert gold not in paints(quiet), f"a quiet receptor was painted in a session's ink: {quiet}"
    mycelium_node.tell("cell", changed=[{"key": f"cell:{FED_RECEPTOR}", "state": "reading"}])
    # Waited on by the quiet dot going, whatever the reading one is drawn as.
    page.wait_for_function(
        f"!{MARKS.format(SENSOR)}.some((mark) => mark.cls === 'm-sensed')", timeout=PACE_MS
    )
    reading = marks_of(page, SENSOR)
    assert gold not in paints(reading), (
        f"a reading receptor was painted in a session's ink: {reading}"
    )
    # A leader's membrane is in that gold, and heavier than a worker's.
    led, worked = membrane_of(marks_of(page, LEADER)), membrane_of(worker)
    assert led["stroke"] == gold, f"the leader's membrane wasn't in its dot's ink: {led}"
    assert worked["stroke"] != gold, f"a worker's membrane was in a leader's ink: {worked}"
    assert float(led["width"] or 0) > float(worked["width"] or 0), (
        f"the leader's membrane was no heavier than a worker's: {led}, {worked}"
    )
    # A leader that is stuck keeps the ring that says so.
    stuck = membrane_of(marks_of(page, STUCK_LEADER))
    assert stuck["stroke"] not in {gold, worked["stroke"]}, (
        f"a stuck leader lost its stuck ring: {stuck}"
    )
    # The drawer's rows mark a leader as the map does.
    page.evaluate("location.hash = '#net=orchestration.skiff1'")
    expect(
        page.locator(f".row[data-go='cell={FED_CELL}'] .glyph .m-leads").first,
        "the drawer's row didn't mark the leader",
    ).to_be_attached()


def test_a_line_thins_as_the_map_steps_out_past_its_own_size(mycelium_opened: Opened) -> None:
    page = mycelium_opened.page
    the_map_stands(page, "live")
    page.locator("#map-view").focus()
    page.keyboard.press("0")
    *_, k = settled(page, "0 didn't fit the whole map")
    while k < 1:
        page.keyboard.press("i")
        *_, nearer = settled(page, "i didn't step in")
        assert nearer > k, f"the map stopped stepping in at {k:.3f}"
        k = nearer
    painted = page.evaluate(PAINTED)
    assert painted["widestCarried"] > HAIRLINE_PX, (
        f"at {k:.3f} the widest line carried {painted['widestCarried']} px, too few to tell"
    )
    # A canvas holds a line's width at single precision, so widths match to
    # a hundredth rather than exactly.
    assert painted["widest"] == pytest.approx(painted["widestCarried"], abs=0.01), (
        f"at its own size or nearer, {k:.3f}, a line was drawn {painted['widest']} px"
        f" though it carries {painted['widestCarried']} px"
    )
    while k >= THINNED_UNDER:
        page.keyboard.press("o")
        *_, further = settled(page, "o didn't step out")
        assert further < k, f"the map stopped stepping out at {k:.3f}"
        k = further
    painted = page.evaluate(PAINTED)
    carried, drawn = painted["widestCarried"], painted["widest"]
    assert carried > HAIRLINE_PX, f"the widest line carried {carried} px, too few to thin"
    # How far each line thins is the unit cases' (lines.test.ts); here, that
    # the drawing thins the heaviest under what it carries but over the
    # hairline the lighter ones are drawn at, so it still reads heavier.
    assert HAIRLINE_PX + THICKER_BY < drawn < carried - THICKER_BY, (
        f"stepped out to {k:.3f}, the widest line, {carried:.2f} px for what it carries,"
        f" was drawn {drawn:.2f} px, not thinned between a hairline and that"
    )
    cell, cell_carried = painted["cellWidest"], painted["cellWidestCarried"]
    assert cell == pytest.approx(cell_carried, abs=0.01), (
        f"stepped out to {k:.3f}, a cell's mark carrying {cell_carried:.2f} px was drawn"
        f" {cell:.2f} px, though a cell's marks shrink with it and never thin"
    )


def test_talk_between_two_networks_is_one_line_until_it_or_a_cell_it_touches_opens(
    browser: Browser, talking_served: Served
) -> None:
    root, _ = talking_served
    opened = Opened(browser, f"{root}mycelium/", SIZES[-1], "dark")
    try:
        page = opened.page
        the_map_stands(page, "live")
        page.wait_for_function(f"{PAINTED}.said === {len(TALKERS)}", timeout=PACE_MS)
        cells = page.locator("#world circle.cell")
        expect(cells, "the realm's cells folded").to_have_count(CELLS)
        painted = page.evaluate(PAINTED)
        assert painted["folded"] == 0, "a network folded, so one line proves no gathering"
        assert painted["talks"] == ["m-talk warm2 faint"], (
            f"the talk between two networks was drawn as {painted['talks']}, not one faint"
            " line as warm as its warmest pair"
        )
        ends = painted["talkEnds"]
        assert all(end.startswith("h:") for end in ends), f"gathered talk ran to {ends}, not hubs"
        # A network open brightens the line it touches, and keeps it one.
        page.evaluate(f"location.hash = '#net={TALKED_WITH}'")
        page.wait_for_function(
            f"!{PAINTED}.talks.some((t) => t.includes('faint'))", timeout=PACE_MS
        )
        drawn = page.evaluate(f"{PAINTED}.talks")
        assert drawn == ["m-talk warm2"], f"a network open drew its talk as {drawn}"
        # The line open spreads into the pairs it stands for, its own lit.
        pair = "encodeURIComponent('c:HERON2|c:TERN03')"
        page.evaluate(f"location.hash = '#talk=' + {pair}")
        page.wait_for_function(f"{PAINTED}.talk === {len(TALKERS)}", timeout=PACE_MS)
        drawn = sorted(page.evaluate(f"{PAINTED}.talks"))
        assert drawn == ["m-talk warm1 lit", "m-talk warm2"], f"a talk open drew {drawn}"
        ends = page.evaluate(f"{PAINTED}.talkEnds")
        assert all(end.startswith("c:") for end in ends), f"a talk open drew lines to {ends}"
        page.evaluate(f"location.hash = '#cell={HEARD}'")
        page.wait_for_function(
            f"{PAINTED}.talks.filter((t) => !t.includes('faint')).length === {len(TALKERS)}",
            timeout=PACE_MS,
        )
        found = page.evaluate((HERE / "site-checks.js").read_text())
        assert found == [], f"site-checks.js on a realm with talk found {found}"
    finally:
        opened.close()


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

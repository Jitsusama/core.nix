"""The mycelium map reads live from its node, through the sites' server, in a real browser.

The page is the one Nix builds, served by the real local-sites server beside a
stand-in node (modules/local-sites/tests/stand_in_daemon.py) holding a small
realm under test names, node.json, through the node feed exactly as home.nix
declares it for a machine in a realm (NODE_KINDS). It must say it's live with
the realm drawn, its networks and the lines joining them and their cells
in one window that opens a cell clicked, follow a change within its pace,
two cells sharing a title moving among them, say broken when the node
stops while keeping the realm drawn, and live again when it's back, say stale
when the server goes, draw a record's words as words and never as markup,
list what each cell, machine, network and branch spent today, most first,
say what waits on a cell and what holds stand, as the node judges them,
draw every cell until one is too small to read, thin every line as the map
steps out past its own size, gather the talk between two networks into one
line until it or a cell it touches opens,
draw a receptor's feeding of a cell while the node says it and not after,
grow one network out of another while a forage says a cell of the one
started a cell of the other, and across machines while the cell's own
record says so,
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
# 33 files since the balance, who started each cell, the tree beside the map,
# what each thing is in words, how a time is said, how each thing stands, what
# each is doing, what each spent, what waits, how wide each line is drawn, how
# a cell is named, how the map's motion is timed, where everything stands,
# what the ground layer's canvas touches, a reading's shape, what new work
# hands the Launcher and what a cell's drawer offers were read in modules of
# their own; about 668 KB since the builders, the legend's moving samples and
# new work's line, and since no comment is sent, view.js's and the styles'
# included, a little more with the drawer's acts and its whole question, and
# under the 677 KB allowed; new work's line and its hidden controls draw about
# 20 elements of the 611. One file more and two of the kilobytes are the kit's
# head.css, the head every live page shares, less the map's own head rules;
# the head and its link are seven elements more in the markup than the map's
# header was, and the budget rose by three, the rest within its room. One
# file more is the kit's early.js, the script the page once wrote into
# itself, which the sites' policy now runs only from a file. One file, three
# kilobytes and four elements are the asks beside the drawer: the watch for
# a waiting look, and the part it fills, whose rows load only once one waits.
BUDGET = Budget(files=36, kilobytes=682, elements=627)
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
ASKED_WORDS = f"owes {ASKER_SHORT} an answer: asked {{}} min ago, with no answer yet"
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
    # Still listed by the machine it left, an hour older, under a key of its
    # own, so the page holds both copies and has to choose.
    left = {**copy.deepcopy(cells[1]), "machine": "osprey", "since": "2026-11-16T08:00:00Z"}
    cells.append({**left, "key": f"{left['key']}@osprey"})
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


def namesakes() -> dict[str, list[dict[str, object]]]:
    """Node.json's realm with ASKER's question waiting on both NAMESAKES."""
    reading = json.loads((HERE / "node.json").read_text())["kinds"]
    groups = [f"@{cell}" for cell in NAMESAKES]
    asked = unanswered("namesakes", ASKER, ASKED_AGO, intent="question", groups=groups)
    return {**reading, "unanswered": [{**asked, "waiting_on": list(NAMESAKES)}]}


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
def namesakes_node(tmp_path: Path) -> Iterator[StandIn]:
    stand_in = StandIn(tmp_path / "namesakes.sock", "node", namesakes()).start()
    yield stand_in
    stand_in.leave()


@pytest.fixture
def namesakes_served(tmp_path: Path, namesakes_node: StandIn) -> Iterator[Served]:
    yield from serve(tmp_path, namesakes_node)


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


# The cells with a look waiting on Joel, each its own, and a machine whose drawer shows no asks.
ASKING = ("SKIFF1", "TERN02")
ASKLESS_MACHINE = "osprey"


def asking(cell: str) -> dict[str, object]:
    """Give a look waiting on Joel from `cell`, as the to-do daemon holds one."""
    look = {
        "v": 1,
        "id": f"look-{cell}",
        "at": datetime.now(UTC).isoformat(),
        "kind": "pr",
        "target": "https://example.com/pull/1",
        "title": f"{cell}'s Change",
        "why": "Its checks passed.",
        "ask": "Say whether it merges.",
        "choices": [
            {"value": "yes", "label": "Merge It"},
            {"value": "no", "label": "Leave It"},
        ],
        "state": "waiting",
        "address": cell.lower(),
    }
    return {"key": f"look:{look['id']}", "kind": "look", "look": look}


@pytest.fixture
def asks_served(tmp_path: Path, mycelium_node: StandIn) -> Iterator[tuple[Served, StandIn]]:
    """Serve the map beside a to-do daemon holding a look from each asking cell.

    The same stand-in takes the answer as the node would, so what was
    steered is read from it.
    """
    to_do = StandIn(tmp_path / "to-do.sock", "to-do", {"look": [asking(cell) for cell in ASKING]})
    to_do.start()
    node = {"socket": str(mycelium_node.path), "kinds": json.loads(os.environ["NODE_KINDS"])}
    feeds = {"node": node, "to-do": {"socket": str(to_do.path), "kinds": {"look": {}}}}
    answers = {"looks": str(to_do.path), "node": str(to_do.path)}
    site = {"mycelium": os.environ["MYCELIUM_SITE"]}
    with serving(tmp_path, site, feeds, answers=answers) as served:
        yield served, to_do
    to_do.leave()


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


# The networks node.json's realm leads, each drawn as a hub, and the one
# branching from another, as a mind map branches.
NETWORKS = ("orchestration.skiff1", "orchestration.tern01", "orchestration.heron1")
BRANCH, BRANCHED_FROM = "orchestration.tern01", "orchestration.skiff1"
# Each cell's box, and the box of the view they're drawn in.
IN_VIEW = """[document.getElementById('map-view'),
  ...document.querySelectorAll('#world circle.cell')]
  .map((e) => e.getBoundingClientRect())
  .map(({ left, right, top, bottom }) => ({ left, right, top, bottom }))"""


# On 14 November 2026 the first live map went on Joel's look list checked
# for its layout alone, and he said of it: "it shows no network at all ...
# no mind map like structure of how things connect ... i click on a thing
# and nothing changes in front of me ... i have to scroll so much." Each of
# those asks is held here at a desk's size, on the realm fitted to its
# window; the talk between cells is held where talk is (the talk test), and
# text too small or too faint by the page checks.
def test_the_map_draws_how_cells_connect_and_answers_a_click_in_one_window(
    mycelium_opened: Opened,
) -> None:
    page = mycelium_opened.page
    the_map_stands(page, "live")
    the_map_counts(page, COUNTED, "the map miscounted the realm")
    page.locator("#map-view").focus()
    page.keyboard.press("0")
    settled(page, "the view never stood still after 0")
    for network in NETWORKS:
        hub = page.locator(f'#world .hub[data-key="h:{network}"]')
        expect(hub, f"the map didn't draw the network {network}").to_be_visible()
    branch = page.locator(f'#world .hub[data-key="h:{BRANCH}"]')
    expect(branch, f"{BRANCH} wasn't drawn branching from {BRANCHED_FROM}").to_have_attribute(
        "data-branch-of", BRANCHED_FROM
    )
    expect(page.locator("#world circle.cell"), "the map left cells out").to_have_count(CELLS)
    across = page.evaluate(f"{PAINTED}.across")
    assert across > 0, "the map drew no line across networks, though cells share some"
    view, *cells = page.evaluate(IN_VIEW)
    outside = [
        cell
        for cell in cells
        if cell["left"] < view["left"]
        or cell["right"] > view["right"]
        or cell["top"] < view["top"]
        or cell["bottom"] > view["bottom"]
    ]
    assert not outside, f"0 left {len(outside)} cells outside the view {view}"
    window = page.evaluate(
        "[innerHeight, document.scrollingElement.scrollHeight,"
        " document.getElementById('map-view').getBoundingClientRect().bottom]"
    )
    assert window[1] <= window[0], f"the map's page scrolls: {window[1]} px in {window[0]}"
    assert window[2] <= window[0], f"the map runs {window[2]} px down a {window[0]} px window"
    box = page.locator('#world circle.cell[data-key="c:TERN03"]').bounding_box()
    assert box, "the map drew no TERN03 to click"
    page.mouse.click(box["x"] + box["width"] / 2, box["y"] + box["height"] / 2)
    expect(page, "a click on a cell didn't open it").to_have_url(re.compile("#cell=TERN03$"))
    expect(
        page.locator('#panel-body a[href^="mycelium:cell/TERN03"][href$="/go"]'),
        "a click on a cell left its drawer shut",
    ).to_be_visible()


def test_a_cell_that_moves_is_drawn_moving(mycelium_opened: Opened, mycelium_node: StandIn) -> None:
    page = mycelium_opened.page
    the_map_stands(page, "live")
    the_map_counts(page, COUNTED, "the map miscounted the realm")
    mycelium_node.tell("cell", changed=[{"key": "cell:TERN03", "standing": "moving"}])
    the_map_counts(page, "8 cells, 3 moving, 1 stuck", "the moved cell wasn't drawn moving")


# The two cells node.json names "Mend the Sails": the first moving, the
# second needing Joel.
NAMESAKES = ("TERN01", "TERN02")
NAMESAKES_OWE = f"hasn't answered {ASKER_SHORT}'s question of"


def standing_of(page: Page, cell: str) -> set[str]:
    """Read the classes a cell's marks are painted with, which say how it stands."""
    return {str(mark["cls"]) for mark in marks_of(page, f"c:{cell}")}


def still_owe(page: Page, why: str) -> None:
    """Hold that the question waiting on each of NAMESAKES is still said, age and all."""
    for cell in NAMESAKES:
        assert "m-waits" in standing_of(page, cell), f"{cell} lost its wait {why}"
        page.evaluate(f"location.hash = '#cell={cell}'")
        expect(page.locator("#panel-body"), f"{cell}'s drawer lost its wait {why}").to_contain_text(
            NAMESAKES_OWE
        )


# On 15 November 2026 the live map froze on "reading": two cells sharing a
# title moved, a reading left one's waits unset, working out how long they
# had waited threw, and a bare catch swallowed it, so the page kept its old
# counts and logged nothing. Here a question waits on both NAMESAKES, so
# each move works out its age. A reading that throws now logs, which fails
# the tab; one that's dropped leaves the counts or a cell's marks stale;
# and one that lands on the other cell of the two leaves the wrong marks.
def test_cells_sharing_a_title_that_move_are_drawn_moving(
    browser: Browser, namesakes_served: Served, namesakes_node: StandIn
) -> None:
    root, _ = namesakes_served
    opened = Opened(browser, f"{root}mycelium/", SIZES[-1], "dark")
    try:
        page = opened.page
        first, second = NAMESAKES
        the_map_stands(page, "live")
        the_map_counts(page, COUNTED, "the map miscounted the realm")
        still_owe(page, "before either moved")
        namesakes_node.tell("cell", changed=[{"key": f"cell:{second}", "standing": "moving"}])
        why = f"the map froze as {second}, sharing {first}'s title, moved"
        the_map_counts(page, "8 cells, 3 moving, 1 stuck", why)
        asks = page.locator(f'#world circle.cell[data-key="c:{second}"]')
        expect(asks, f"{second} still needs Joel once moving").not_to_have_attribute(
            "data-asks", ""
        )
        expect(page.locator("#asks-words"), "the map still says one needs Joel").to_have_text("")
        still_owe(page, f"once {second} moved")
        namesakes_node.tell("cell", changed=[{"key": f"cell:{first}", "standing": "quiet"}])
        the_map_counts(page, COUNTED, f"the map froze as {first} went quiet")
        assert "m-quiet" in standing_of(page, first), f"{first} wasn't drawn quiet"
        assert "m-nucleus" in standing_of(page, second), f"{second} stopped being drawn moving"
        still_owe(page, f"once {first} went quiet")
        the_map_stands(page, "live")
    finally:
        opened.close()


# The builders node.json's machine sends its builds to, by name as the map
# draws them under the map, and the one of them that takes none.
BUILDERS = ("b:gull", "b:heron", "b:tern")
IDLE_BUILDER = "gull"
IDLE_SAYS = "Nobody has heard how it stands: it is reached through another, which isn't dialled."
# What the lent builder's mark says under its name, from the fields the feed passes.
LENT_WORDS = "lent · running · 16 builds at once"
# Each mark among the machines, by its key, with its class and its box's corners.
MARKS_DRAWN = """[...document.querySelectorAll('.labels .t-mark')].map((g) => [
  g.dataset.mark, g.getAttribute('class'), getComputedStyle(g.querySelector('.t-box')).rx])"""
# Where each builder's mark stands against the view and against the
# lowest of everything else drawn: every other word on the map, the
# machines' marks and the networks' names among them, each network's
# hub, and each cell.
FOOT_SEEN = """(() => {
  const view = document.querySelector('#map-view').getBoundingClientRect();
  const box = (el) => el.getBoundingClientRect();
  const drawn = '.labels .tag:not(.t-builder), #world g.hub, #world circle.cell';
  const lowest = Math.max(...[...document.querySelectorAll(drawn)].map((g) => box(g).bottom));
  return [...document.querySelectorAll('.labels .t-builder')].map((g) => [
    g.dataset.mark, box(g).top >= lowest, box(g).top >= view.top && box(g).bottom <= view.bottom]);
})()"""


def test_the_builders_stand_under_the_machines_in_a_shape_of_their_own(
    mycelium_opened: Opened,
) -> None:
    page = mycelium_opened.page
    the_map_stands(page, "live")
    builders = page.locator(".labels .t-builder")
    expect(builders, "builders were drawn with the machines hidden").to_have_count(0)
    page.locator("#map-view").focus()
    page.keyboard.press("m")
    expect(builders, "m didn't draw the builders").to_have_count(len(BUILDERS))
    marks = page.evaluate(MARKS_DRAWN)
    keys = [key for key, _, _ in marks]
    assert keys[-len(BUILDERS) :] == list(BUILDERS), f"builders not after the machines: {keys}"
    # Under everything the ring draws, and still in view with the whole map.
    page.keyboard.press("0")
    settled(page, "the whole map never stood still")
    for key, under, seen in page.evaluate(FOOT_SEEN):
        assert under, f"{key} stood among the machines or the networks"
        assert seen, f"{key} fell out of the whole map's view"
    for key, look, corner in marks:
        if key in BUILDERS:
            assert corner == "7px", f"{key} isn't a box with rounded corners: {corner}"
            idle = key == f"b:{IDLE_BUILDER}"
            assert ("off" in look.split()) == idle, f"{key} drawn {look}, idle {idle}"
        else:
            assert corner != "7px", f"the machine {key} took a builder's corners"
    # With nothing open, each builder's mark says how it stands under its name.
    expect(
        page.locator('.labels .t-builder[data-mark="b:tern"]'), "a builder's mark misread it"
    ).to_contain_text(LENT_WORDS)
    page.evaluate(f"location.hash = '#builder={IDLE_BUILDER}'")
    expect(page.locator("#panel-title"), "#builder= didn't open its drawer").to_have_text(
        IDLE_BUILDER
    )
    expect(
        page.locator("#panel-body [data-builder-says]"), "the drawer misread the builder"
    ).to_have_text(IDLE_SAYS)
    # Hiding the machines lets go of a builder's drawer, and a builder's
    # link shows them again.
    page.locator("#map-view").focus()
    page.keyboard.press("m")
    expect(builders, "m didn't hide the builders").to_have_count(0)
    expect(page.locator("#panel-title"), "m kept a hidden builder open").not_to_have_text(
        IDLE_BUILDER
    )
    page.evaluate("location.hash = '#builder=heron'")
    expect(page.locator("#panel-title"), "#builder= didn't open heron").to_have_text("heron")
    expect(builders, "#builder= didn't show the builders").to_have_count(len(BUILDERS))


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
  if (!link) return;
  event.preventDefault();
  window.talkedTo = link.getAttribute("href");
  window.talks = (window.talks || 0) + 1;
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
    # Nor has it a window, and g says so.
    page.keyboard.press("g")
    expect(page.locator("#where"), "g on a receptor didn't say it has no window").to_contain_text(
        "There is no window to go to for"
    )
    page.keyboard.press("Escape")
    expect(page, "Escape didn't let the cell go").not_to_have_url(re.compile("#cell="))
    page.keyboard.press("t")
    expect(page.locator("#where"), "t with no cell open didn't say so").to_have_text(
        "No cell is open to talk to; a opens the next that needs you."
    )
    page.keyboard.press("g")
    expect(page.locator("#where"), "g with no cell open didn't say so").to_have_text(
        "No cell is open to go to; a opens the next that needs you."
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
    talk = page.locator('#panel-body a[href^="mycelium:cell/"]:not([data-go-to])')
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


# What node.json's TERN02 asks, whole, a paragraph a line, where its `asks`
# holds only the last; and where its Go to It goes, on the machine it runs on.
ASKED_WHOLE = (
    (
        "The waxed thread holds in salt but stiffens in the cold.\n"
        "The plain runs smoother and wears sooner."
    ),
    "Which thread: the waxed or the plain?",
)
GONE_TO = f"mycelium:cell/{NEEDING[0]}@kestrel/go"
# The drawer's first act, past its way back: the first link or button after it.
FIRST_ACT = """(() => {
  const act = document.querySelector("#panel-body a, #panel-body button:not(.back)");
  return [act?.textContent, act?.getAttribute("href")];
})()"""
# Whether the question comes after Go to It, each of its paragraphs' line
# breaks, which to_have_text folds away, and whether its lines keep their
# indentation, as pre-wrap sets them.
ASKED_SHAPE = """(() => {
  const body = document.querySelector("#panel-body");
  const go = body.querySelector("a[data-go-to]");
  const asked = body.querySelector(".asked");
  const after = !!go && !!asked
    && !!(go.compareDocumentPosition(asked) & Node.DOCUMENT_POSITION_FOLLOWING);
  const paragraphs = [...body.querySelectorAll(".asked p")];
  return [
    after,
    paragraphs.map((p) => p.querySelectorAll("br").length),
    paragraphs.map((p) => getComputedStyle(p).whiteSpace),
  ];
})()"""


def test_a_cell_that_needs_you_leads_with_its_question_whole_and_go_to_it(
    mycelium_opened: Opened,
) -> None:
    page = mycelium_opened.page
    the_map_stands(page, "live")
    page.evaluate(f"location.hash = '#cell={NEEDING[0]}'")
    body = page.locator("#panel-body")
    expect(body.locator("#panel-title"), "the asking cell's drawer never opened").to_be_visible()
    # Its words, whitespace folded: ASKED_SHAPE counts its line breaks.
    expect(body.locator(".asked p"), "its question wasn't said whole").to_have_text(
        list(ASKED_WHOLE), use_inner_text=True
    )
    first = page.evaluate(FIRST_ACT)
    assert first == ["Go to It", GONE_TO], f"the drawer's first act was {first}"
    shape = page.evaluate(ASKED_SHAPE)
    breaks = [paragraph.count("\n") for paragraph in ASKED_WHOLE]
    kept = ["pre-wrap"] * len(ASKED_WHOLE)
    assert shape == [True, breaks, kept], f"after Go to It, its breaks and spaces were {shape}"
    # g goes there as the link does, and t still talks rather than goes.
    page.evaluate(TALK_HELD)
    page.locator("#map-view").focus()
    page.keyboard.press("g")
    gone = page.evaluate("window.talkedTo")
    assert gone == GONE_TO, f"g went to {gone}"
    page.keyboard.press("t")
    talked = page.evaluate("window.talkedTo")
    assert talked == f"mycelium:cell/{NEEDING[0]}@kestrel", f"t talked to {talked}"
    # The tree's g is the tree's alone: it goes to its first row, and the
    # page's g never also answers it. . lets the cell go to show the tree,
    # so with no cell open the page's g would only refuse, saying so in the
    # status line a screen reader reads out over the tree's own step.
    talks = page.evaluate("window.talks")
    page.locator("#panel-title").press(".")
    items = page.locator("#tree [role=treeitem]")
    items.last.focus()
    page.keyboard.press("g")
    expect(items.first, "g didn't go to the tree's first").to_be_focused()
    expect(page.locator("#where"), "the page's g answered the tree's").not_to_contain_text(
        "No cell is open to go to"
    )
    after = page.evaluate("window.talks")
    assert after == talks, f"the tree's g opened a link: {after} links, not {talks}"


# What new work's line hands the Launcher, said back once the line closes,
# and what it ends with.
HANDED = "Sent to the Launcher's Start New Work"
HANDED_END = (
    "; nothing starts until you press Return there."
    " If it didn't open, wait a moment, then s brings the words back."
)
# Where a box lies on the page, scrolled or not: left, top, width, height.
PLACED = """(selector) => {
  const r = document.querySelector(selector).getBoundingClientRect();
  return [r.left + scrollX, r.top + scrollY, r.width, r.height];
}"""
# Whether the drawer has slid all the way in, on a page it fills.
SLID_IN = "() => document.getElementById('panel').getBoundingClientRect().left === 0"
# How far apart the middles of new work's drawn parts sit, up and down,
# which one row keeps near nothing, to within ONE_ROW_PX.
ONE_ROW_PX = 2
ROW_APART = """(() => {
  const middles = [...document.querySelectorAll("#sprout-line > *")]
    .map((e) => e.getBoundingClientRect())
    .filter((r) => r.width > 1 && r.height > 1)
    .map((r) => r.top + r.height / 2);
  return Math.max(...middles) - Math.min(...middles);
})()"""
# Gives the offered cell a name as long as an agent might write.
LONG_NAMED = """document.getElementById("sprout-from-words").textContent =
  "From " + "A Long Cell Name An Agent Wrote ".repeat(3)"""
# What the page says once the line has left letters off.
FULL_WORDS = "The line holds 1000 letters; any past that were left off."
# What the page says to Return with no words, and to Escape on the line.
NO_WORDS = "Say what the new work is first; the Launcher needs its words."
CLOSED_WORDS = "New work closed; nothing was handed on."
# Whether a repeat of the key whose code it's given, pressed now in new
# work's field, is held off: a function, so evaluate hands it the code.
REPEAT_HELD = """(code) => {
  const e = new KeyboardEvent("keydown", {
    code, repeat: true, bubbles: true, cancelable: true,
  });
  document.getElementById("sprout").dispatchEvent(e);
  return e.defaultPrevented;
}"""
# Where the field's chosen words start and end.
CHOSEN = """(() => {
  const f = document.getElementById("sprout");
  return [f.selectionStart, f.selectionEnd];
})()"""


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
    expect(page.locator("#where"), "Return with no words wasn't said").to_have_text(NO_WORDS)
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
    # From a cell's drawer, its address goes with the words, though the
    # line was opened on the whole map, and a tick lets it go.
    page.locator("#map-view").focus()
    page.keyboard.press("s")
    expect(page.locator("#sprout"), "the line's field has no cap").to_have_attribute(
        "maxlength", "1000"
    )
    page.locator("#map-view").focus()
    page.keyboard.press("a")
    expect(page, "a didn't open a cell").to_have_url(re.compile(rf"#cell={NEEDING[0]}$"))
    drawer = page.locator("#panel-body").get_by_role("button", name="Start New Work From It")
    drawer.click()
    offered = page.locator("#sprout-from-words")
    expect(offered, "the drawer's cell wasn't offered on the open line").to_be_visible()
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
    assert page.evaluate(CHOSEN) == [0, 9], "the handed words came back unchosen"
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
    expect(page.locator("#map-view"), "the keys didn't go back to the map").to_be_focused()
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
    expect(page.locator("#where"), "Escape's close wasn't said").to_have_text(CLOSED_WORDS)
    expect(page.locator("#trail"), "the trail didn't come back").to_be_visible()
    assert page.evaluate("window.talkedTo") is None, "Escape handed the words on"
    # Opened by the control, Escape gives the keys back to it.
    control = page.locator("#sprout-control")
    control.click()
    page.keyboard.press("Escape")
    expect(control, "the keys didn't go back to New Work").to_be_focused()


def test_find_and_new_work_never_stand_together(mycelium_opened: Opened) -> None:
    page = mycelium_opened.page
    the_map_stands(page, "live")
    line, finding, trail = (page.locator(f"#{e}") for e in ("sprout-line", "find-line", "trail"))
    page.locator("#map-view").focus()
    page.keyboard.press("s")
    expect(line, "s didn't open new work's line").to_be_visible()
    page.locator("#map-view").focus()
    page.keyboard.press("/")
    expect(finding, "/ didn't open find").to_be_visible()
    expect(line, "find stood beside new work").to_be_hidden()
    expect(trail, "the trail stood beside find").to_be_hidden()
    page.locator("#map-view").focus()
    page.keyboard.press("s")
    expect(line, "s didn't open new work over find").to_be_visible()
    expect(finding, "new work stood beside find").to_be_hidden()
    expect(trail, "the trail stood beside new work").to_be_hidden()


def sent_again(page: Page, want: str) -> None:
    """Open new work's line from the map and send its kept words again, as `want`."""
    page.evaluate("window.talkedTo = null")
    page.locator("#map-view").focus()
    page.keyboard.press("s")
    page.keyboard.press("Enter")
    handed = page.evaluate("window.talkedTo")
    assert handed == want, f"sent again as {handed}, not {want}"


def test_new_work_sends_again_from_the_cell_it_went_from(mycelium_opened: Opened) -> None:
    page = mycelium_opened.page
    the_map_stands(page, "live")
    page.evaluate(TALK_HELD)
    field, tick = page.locator("#sprout"), page.locator("#sprout-from")
    drawer = page.locator("#panel-body").get_by_role("button", name="Start New Work From It")

    # Words sent again go from the cell they went from, whatever is open,
    # and the tick names that cell's address where it can be seen.
    page.evaluate(f"location.hash = '#cell={NEEDING[0]}'")
    drawer.click()
    expect(page.locator("#sprout-from-line"), "the tick hid the address").to_have_attribute(
        "title", re.compile(rf"\({NEEDING[0]}\)$")
    )
    page.keyboard.type("first")
    page.keyboard.press("Enter")
    page.evaluate(f"location.hash = '#cell={NEEDING[1]}'")
    expect(page, "the second cell didn't open").to_have_url(re.compile(rf"#cell={NEEDING[1]}$"))
    sent_again(page, f"mycelium:sprout?words=first&from={NEEDING[0]}")
    # New words over kept ones are new work, from the cell open now.
    page.locator("#map-view").focus()
    page.keyboard.press("s")
    expect(field, "the handed words didn't come back").to_have_value("first")
    page.keyboard.type("second")
    page.keyboard.press("Enter")
    handed = page.evaluate("window.talkedTo")
    assert handed == f"mycelium:sprout?words=second&from={NEEDING[1]}", f"new words: {handed}"
    # A drawer's button on its own cell's open line ticks it back.
    drawer.click()
    page.keyboard.press("Escape")
    drawer.click()
    tick.uncheck()
    drawer.click()
    expect(tick, "the drawer's button left its own cell unticked").to_be_checked()


def test_new_works_kept_words_keep_a_cleared_tick_and_let_a_gone_cell_go(
    mycelium_opened: Opened, mycelium_node: StandIn
) -> None:
    page = mycelium_opened.page
    the_map_stands(page, "live")
    page.evaluate(TALK_HELD)
    field, tick = page.locator("#sprout"), page.locator("#sprout-from")
    drawer = page.locator("#panel-body").get_by_role("button", name="Start New Work From It")
    page.evaluate(f"location.hash = '#cell={NEEDING[1]}'")
    drawer.click()
    # A tick cleared on kept words stays cleared as new words go over them.
    page.keyboard.type("third")
    field.press("Enter")
    sent_again(page, f"mycelium:sprout?words=third&from={NEEDING[1]}")
    page.locator("#map-view").focus()
    page.keyboard.press("s")
    tick.uncheck()
    field.press_sequentially("x")
    expect(tick, "typing over kept words ticked a cleared cell back").not_to_be_checked()
    page.keyboard.press("Escape")
    drawer.click()
    # Words sent from no cell go again from none, though a cell is open.
    page.keyboard.type("bare")
    tick.uncheck()
    field.press("Enter")
    sent_again(page, "mycelium:sprout?words=bare")
    page.locator("#map-view").focus()
    page.keyboard.press("s")
    expect(page.locator("#sprout-from-line"), "a cell came back unasked").to_be_hidden()
    page.keyboard.press("Escape")
    # Kept words whose cell has left the map go again from the cell open now.
    drawer.click()
    page.keyboard.type("last")
    page.keyboard.press("Enter")
    page.evaluate(f"location.hash = '#cell={NEEDING[0]}'")
    expect(page, "the first cell didn't open").to_have_url(re.compile(rf"#cell={NEEDING[0]}$"))
    mycelium_node.tell("cell", gone=[f"cell:{NEEDING[1]}"])
    the_map_counts(page, "7 cells, 2 moving", "the gone cell was still counted")
    sent_again(page, f"mycelium:sprout?words=last&from={NEEDING[0]}")


def test_new_works_line_holds_its_keys(mycelium_opened: Opened) -> None:
    page = mycelium_opened.page
    the_map_stands(page, "live")
    field = page.locator("#sprout")
    # A held s opens the line once, its repeats typing nothing, and once
    # it comes up, even off the line, keys held in the field repeat.
    page.locator("#map-view").focus()
    for _ in range(3):
        page.keyboard.down("s")
    expect(field, "a held s typed into the line").to_have_value("")
    assert page.evaluate(REPEAT_HELD, "KeyS"), "a held s's repeat wasn't held off"
    # Another key coming up leaves it held; its own, even off the line,
    # lets it go.
    page.keyboard.down("Shift")
    page.keyboard.up("Shift")
    assert page.evaluate(REPEAT_HELD, "KeyS"), "another key coming up let a held s go"
    page.locator("#map-view").focus()
    page.keyboard.up("s")
    field.focus()
    page.keyboard.down("s")
    page.keyboard.down("s")
    page.keyboard.up("s")
    expect(field, "s coming up off the line didn't let it go").to_have_value("ss")
    field.fill("")
    # A key still held as the page gets the keys back, its press unheard,
    # repeats nowhere.
    page.locator("#map-view").focus()
    page.keyboard.press("Escape")
    page.keyboard.down("s")
    page.evaluate("window.dispatchEvent(new Event('blur'))")
    page.keyboard.down("s")
    expect(field, "a key held as the keys came back typed").to_have_value("")
    page.keyboard.up("s")
    field.focus()
    page.keyboard.down("x")
    page.keyboard.down("x")
    page.keyboard.up("x")
    expect(field, "a key held in the line didn't repeat").to_have_value("xx")
    # The cap is said only when something was left off.
    field.fill("x" * 1000)
    expect(page.locator("#where"), "the cap was said with nothing left off").not_to_have_text(
        FULL_WORDS
    )
    page.keyboard.press("y")
    expect(page.locator("#where"), "a letter left off wasn't said").to_have_text(FULL_WORDS)
    # On the line's own button, / leaves the words be.
    page.locator(".sprout-go").focus()
    page.keyboard.press("/")
    expect(page.locator("#find-line"), "/ on the line's button opened find").to_be_hidden()
    expect(field, "a key on the line's button let the words go").to_have_value("x" * 1000)
    field.focus()
    # Opened from find, which the line hides, Escape gives the keys to
    # the map.
    page.keyboard.press("Escape")
    page.locator("#map-view").focus()
    page.keyboard.press("/")
    page.evaluate("document.getElementById('sprout-control').click()")
    expect(field, "New Work didn't take the keys from find").to_be_focused()
    page.keyboard.press("Escape")
    expect(page.locator("#map-view"), "the keys went to the hidden find").to_be_focused()


def test_a_held_return_sends_new_work_once(mycelium_opened: Opened) -> None:
    page = mycelium_opened.page
    the_map_stands(page, "live")
    page.evaluate(TALK_HELD)
    before = page.url
    # Its repeats land on the map once the line closes, and neither reopen
    # the line nor open anything over what was said, though the last
    # letter, rolled over, comes up under it.
    page.locator("#map-view").focus()
    page.keyboard.press("s")
    page.keyboard.type("onc")
    page.keyboard.down("e")
    page.keyboard.down("Enter")
    page.keyboard.up("e")
    for _ in range(3):
        page.keyboard.down("Enter")
    page.keyboard.up("Enter")
    assert page.evaluate("window.talks") == 1, "a held Return sent more than once"
    handed = page.evaluate("window.talkedTo")
    assert handed == "mycelium:sprout?words=once", f"a held Return sent {handed}"
    expect(page.locator("#sprout-line"), "a held Return reopened the line").to_be_hidden()
    expect(page.locator("#where"), "a held Return said over the send").to_have_text(
        re.compile(r"^Sent to the Launcher's Start New Work;")
    )
    expect(page, "a held Return opened what the map points at").to_have_url(before)
    # On a drawer's button it opens the line, and sends nothing.
    page.evaluate(f"location.hash = '#cell={NEEDING[0]}'")
    page.locator("#panel-body").get_by_role("button", name="Start New Work From It").focus()
    for _ in range(3):
        page.keyboard.down("Enter")
    page.keyboard.up("Enter")
    expect(page.locator("#sprout"), "a drawer's button didn't open the line").to_be_focused()
    assert page.evaluate("window.talks") == 1, "a held Return on a drawer's button sent"


def test_a_held_escape_or_new_work_closes_the_line_once(mycelium_opened: Opened) -> None:
    page = mycelium_opened.page
    the_map_stands(page, "live")
    line, control = page.locator("#sprout-line"), page.locator("#sprout-control")
    page.evaluate(f"location.hash = '#cell={NEEDING[0]}'")
    opened = re.compile(rf"#cell={NEEDING[0]}$")
    expect(page, "the cell didn't open").to_have_url(opened)
    # A held Escape closes the line and lets go of nothing more.
    page.locator("#map-view").focus()
    page.keyboard.press("s")
    for _ in range(3):
        page.keyboard.down("Escape")
    page.keyboard.up("Escape")
    expect(line, "a held Escape left the line open").to_be_hidden()
    expect(page, "a held Escape let go of the open cell").to_have_url(opened)
    # A held Return on New Work, which opened the line, closes it for good.
    control.click()
    expect(line, "New Work didn't open the line").to_be_visible()
    control.focus()
    for _ in range(3):
        page.keyboard.down("Enter")
    page.keyboard.up("Enter")
    expect(line, "a held Return on New Work opened the line again").to_be_hidden()


@pytest.mark.parametrize("width", [SIZES[0][0], 560, 700, 780, 900])
def test_new_works_line_holds_one_row_over_a_full_drawer(
    browser: Browser, mycelium_served: Served, width: int
) -> None:
    root, _ = mycelium_served
    opened = Opened(browser, f"{root}mycelium/", (width, SIZES[0][1]), "dark")
    try:
        page = opened.page
        the_map_stands(page, "live")
        page.locator("#map-view").focus()
        page.keyboard.press("a")
        expect(page, "a didn't open a cell").to_have_url(re.compile(rf"#cell={NEEDING[0]}$"))
        # Measured once the drawer has slid in and shows its button, as
        # the click would scroll it there.
        page.wait_for_function(SLID_IN)
        start = page.locator("#panel-body").get_by_role("button", name="Start New Work From It")
        start.scroll_into_view_if_needed()
        was = [page.evaluate(PLACED, s) for s in ("#map", "#panel-body")]
        start.click()
        expect(page.locator("#sprout-from-words"), "the open cell wasn't offered").to_be_visible()
        # An agent names its cell at any length.
        page.evaluate(LONG_NAMED)
        # Wherever the drawer takes the page's width, the line says less
        # as it needs and holds one row, so it neither covers the drawer
        # nor moves it or the map.
        go = page.locator(".sprout-go")
        expect(go, "the button lost its whole name").to_have_accessible_name(
            "Start It in the Launcher"
        )
        if width == SIZES[0][0]:
            expect(go, "the button kept its long words").to_have_text("Start", use_inner_text=True)
        _, top, _, tall = page.evaluate(PLACED, "#sprout-line")
        # The drawer scrolls, so its edge, not its words, is where it shows.
        _, edge, _, _ = page.evaluate(PLACED, "#panel")
        apart = page.evaluate(ROW_APART)
        assert apart <= ONE_ROW_PX, (
            f"the line's parts sit {apart} px apart, so it took more than a row"
        )
        assert top + tall - 1 <= edge, f"the line, to {top + tall}, covers the drawer from {edge}"
        now = [page.evaluate(PLACED, s) for s in ("#map", "#panel-body")]
        assert now == was, f"the line moved the map or the drawer: {was} to {now}"
    finally:
        opened.close()


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
        # Its links to the Launcher name that machine too, never the basin.
        expect(
            body.locator('a[href^="mycelium:cell/"]:not([data-go-to])'),
            "the cell's link lost its machine",
        ).to_have_attribute("href", f"mycelium:cell/{BASIN_CELLS[0]}@{BASIN_MACHINES[0]}")
        expect(body.locator("a[data-go-to]"), "its Go to It lost its machine").to_have_attribute(
            "href", f"mycelium:cell/{BASIN_CELLS[0]}@{BASIN_MACHINES[0]}/go"
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
            "() => document.getElementById('map-ground').painted().fed === 1", timeout=PACE_MS
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
            "() => document.getElementById('map-ground').painted().fed === 2", timeout=FED_DRAWN_MS
        )
        page.wait_for_function(
            "() => document.getElementById('map-ground').painted().fed === 1",
            timeout=PACE_MS + FED_ENDING_S * 1000,
        )
        # Once the node stops saying one, its line goes too.
        fed_node.tell("fed", gone=[FED_KEY])
        page.wait_for_function(
            "() => document.getElementById('map-ground').painted().fed === 0", timeout=PACE_MS
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


def test_a_cell_whose_own_record_names_its_starter_grows_a_branch_with_no_forage(
    browser: Browser, mycelium_served: Served, mycelium_node: StandIn
) -> None:
    # This node serves no forages, so only the cell's record can say who
    # started it. That is all a map has for a cell on another machine, where
    # no forage reaches; the geometry is the forage case's, osprey to kestrel.
    root, _ = mycelium_served
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
        heard = int(datetime.now(UTC).timestamp() * 1000)
        said = {"key": f"cell:{GROWN_CELL}", "kind": "cell", "heard": heard}
        mycelium_node.tell(
            "cell", changed=[{**said, "started_by": GROWN_BY, "started_by_from": "book"}]
        )
        expect(hub, "a starter on another machine never grew the branch").to_have_attribute(
            "data-branch-of", GROWN_FROM, timeout=PACE_MS
        )
        the_map_counts(page, COUNTED, "a branch grown across machines changed the count")
        body = page.locator("#panel-body")
        expect(body, "the branch's drawer never said whose start grew it").to_contain_text(
            f"Started by {GROWN_BY_TITLE}: 1 of its 2 cells", timeout=PACE_MS
        )
        found = page.evaluate((HERE / "site-checks.js").read_text())
        assert found == [], f"site-checks.js on a branch grown across machines found {found}"
        # The cell's drawer says who started it from the record alone.
        page.evaluate(f"location.hash = '#cell={GROWN_CELL}'")
        expect(body, "the cell's drawer never said who its record named").to_contain_text(
            f"by {GROWN_BY_TITLE}", timeout=PACE_MS
        )
        # While it moves, the machine it left may list it too, under a key of
        # its own. The map draws the copy started latest and names that copy's
        # starter, so what it draws and the branch it grows change together.
        kinds = json.loads((HERE / "node.json").read_text())["kinds"]
        heron = next(cell for cell in kinds["cell"] if cell["name"] == GROWN_CELL)
        left = {
            **heron,
            "key": f"cell:{GROWN_CELL}@kestrel",
            "kind": "cell",
            "heard": heard,
            "machine": "kestrel",
        }
        mycelium_node.tell("cell", changed=[{**left, "since": "2026-11-16T09:00:00Z"}])
        expect(body, "the map never drew the copy started latest").to_contain_text(
            "network, on kestrel", timeout=PACE_MS
        )
        expect(body, "a copy the map doesn't draw named the starter").not_to_contain_text(
            f"by {GROWN_BY_TITLE}"
        )
        expect(hub, "a copy the map doesn't draw kept the branch").not_to_have_attribute(
            "data-branch-of", re.compile(".")
        )
        the_map_counts(page, COUNTED, "a cell two machines list was counted twice")
        # Told again as started earlier, it keeps its place as the copy told
        # last, so only the later copy standing gives the branch back.
        mycelium_node.tell("cell", changed=[{**left, "since": "2026-11-16T07:00:00Z"}])
        expect(body, "the map drew a copy started earlier").to_contain_text(
            "network, on osprey", timeout=PACE_MS
        )
        expect(hub, "the branch never came back with the copy that names it").to_have_attribute(
            "data-branch-of", GROWN_FROM, timeout=PACE_MS
        )
        expect(body, "the copy that stands never named its starter").to_contain_text(
            f"by {GROWN_BY_TITLE}", timeout=PACE_MS
        )
        mycelium_node.tell("cell", gone=[left["key"]])
        # Once the record stops naming its starter, the branch goes, and so
        # does the cell's word of who started it.
        mycelium_node.tell("cell", changed=[{**said, "started_by": None, "started_by_from": None}])
        expect(hub, "the branch outlived the word that grew it").not_to_have_attribute(
            "data-branch-of", re.compile("."), timeout=PACE_MS
        )
        expect(body, "the cell's drawer kept a starter its record dropped").not_to_contain_text(
            f"by {GROWN_BY_TITLE}"
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


# Marks the map's marks and the drawer's first element as drawn now, and
# counts the readings drawn after it, so a later look can say whether a
# reading built them again.
MARK_DRAWN = """() => {
  document.querySelector('#world g.nodes').dataset.kept = 'yes';
  document.querySelector('#panel-body').firstElementChild.dataset.kept = 'yes';
  const legend = document.querySelector('#panel-body details[data-keep="legend"]');
  legend.open = true;
  legend.querySelector('summary').focus();
  const mark = document.querySelector('.labels g.t-hub:not([display])');
  mark.dataset.kept = 'yes';
  mark.setAttribute('transform', 'translate(0.5,0.5)');
  document.addEventListener('mycelium:modelled', (e) => {
    window.tern = e.detail.model.cells.find((s) => s.address === 'TERN03') || null;
  });
}"""
# Whether the marks and the drawer marked by MARK_DRAWN are still the ones drawn,
# and whether the legend it opened is open and its summary focused.
STILL_DRAWN = """() => {
  const legend = document.querySelector('#panel-body details[data-keep="legend"]');
  return [
    document.querySelector('#world g.nodes').dataset.kept === 'yes',
    document.querySelector('#panel-body').firstElementChild.dataset.kept === 'yes',
    !!legend?.open,
    document.activeElement === legend?.querySelector('summary')];
}"""
# Whether the first network's name shown is the one MARK_DRAWN marked, and
# where it stands: placed again, it would stand by its network rather than
# where MARK_DRAWN nudged it, and written but never placed it has no place.
# None when no network's name is shown at all.
LABEL_KEPT = """() => {
  const mark = document.querySelector('.labels g.t-hub:not([display])');
  return mark ? [mark.dataset.kept === 'yes', mark.getAttribute('transform')] : null;
}"""
NUDGED = "translate(0.5,0.5)"
# Whether the name MARK_DRAWN nudged has been placed since: moved off the
# nudge, or hidden for want of room.
NUDGE_PLACED = """() => {
  const mark = document.querySelector('.labels g[data-kept]');
  return !!mark && (mark.getAttribute('transform') !== 'translate(0.5,0.5)'
    || mark.hasAttribute('display'));
}"""
# What TERN03 was in the reading last drawn, for a wait on the one told.
# Every wait is a function, never a bare expression: Playwright hands a
# function to the page through the browser's own protocol, but runs an
# expression through the page's eval, which the sites' policy refuses.
TERN_USED = "() => window.tern?.usage?.context?.used === 12000"
TERN_MOVING = "() => window.tern?.standing === 'moving'"
TERN_GONE = "() => window.tern === null"


def test_a_reading_that_moves_no_shape_is_painted_over_the_map_as_drawn(
    mycelium_opened: Opened, mycelium_node: StandIn
) -> None:
    page = mycelium_opened.page
    the_map_stands(page, "live")
    the_map_counts(page, COUNTED, "the map miscounted the realm")
    # The builders are watched apart, and their first answer lays the map
    # out again, so the case marks the map only once they stand on it.
    page.locator("#map-view").focus()
    page.keyboard.press("m")
    builders = page.locator(".labels .t-builder")
    expect(builders, "the builders never stood on the map").to_have_count(len(BUILDERS))
    # Now open, so its legend can be opened and focused as he would.
    page.keyboard.press(".")
    legend = page.locator('#panel-body details[data-keep="legend"] > summary')
    expect(legend, ". never opened Now with its legend").to_be_visible()
    page.evaluate(MARK_DRAWN)
    *_, focused = page.evaluate(STILL_DRAWN)
    assert focused, "the legend's summary took no focus"
    # A turn's worth of context, which moves no cell and no network.
    used = {"limit": 1000000, "used": 12000}
    last = "2026-11-16T10:00:00Z"
    turned = {"cost_today": 0.1, "context": used, "turns_today": 2, "last_turn": last}
    mycelium_node.tell("cell", changed=[{"key": "cell:TERN03", "usage": turned}])
    page.wait_for_function(TERN_USED, timeout=PACE_MS)
    kept_marks, kept_drawer, kept_fold, kept_focus = page.evaluate(STILL_DRAWN)
    assert kept_marks, "a reading that moved no shape built the map's marks again"
    assert kept_drawer, "a reading that left the drawer's words built the drawer again"
    assert kept_fold, "a reading that left the drawer's words shut the fold left open"
    assert kept_focus, "a reading that left the drawer's words took the focus off the fold"
    shown = page.evaluate(LABEL_KEPT)
    assert shown, "no network's name was shown after a reading that moved none"
    label_kept, label_at = shown
    assert label_kept, "a reading that named the same things wrote the map's names again"
    assert label_at == NUDGED, f"a reading that moved no name placed the names again: {label_at}"
    # A zoom moves the view, so the same names are placed again.
    page.locator("#map-view").focus()
    page.keyboard.press("+")
    page.wait_for_function(NUDGE_PLACED, timeout=PACE_MS)
    # TERN03 set moving beside TERN01, which already moves, keeps the shape
    # but not the inks: its nucleus is painted over the marks as drawn.
    mycelium_node.tell("cell", changed=[{"key": "cell:TERN03", "standing": "moving"}])
    page.wait_for_function(TERN_MOVING, timeout=PACE_MS)
    kept_marks, *_ = page.evaluate(STILL_DRAWN)
    assert kept_marks, "a cell set moving in a moving network built the map's marks again"
    painted = [mark["cls"] for mark in page.evaluate(MARKS.format("c:TERN03"))]
    assert "m-nucleus" in painted, f"a restyle left TERN03 unpainted as moving: {painted}"
    # A cell gone changes the shape, so the map is drawn again.
    mycelium_node.tell("cell", gone=["cell:TERN03"])
    page.wait_for_function(TERN_GONE, timeout=PACE_MS)
    kept_marks, *_ = page.evaluate(STILL_DRAWN)
    assert not kept_marks, "a cell gone left the map as it was drawn"
    shown = page.evaluate(LABEL_KEPT)
    assert shown, "no network's name was shown after a cell went"
    label_kept, label_at = shown
    assert not label_kept, "a cell gone left the map's names as they were written"
    assert (label_at or "").startswith("translate("), "the names written again were never placed"
    assert label_at != NUDGED, "the names written again stand where the old ones were nudged"


# The samples in the legend's What Moves, each with how many animations it runs.
SAMPLES_PLAYING = """() => Object.fromEntries(
  [...document.querySelectorAll('#panel-body .moves [data-moves]')].map((row) => [
    row.dataset.moves,
    [...row.querySelectorAll('svg *')].reduce((n, el) => n + el.getAnimations().length, 0)]))"""
SAMPLES = {"flight", "birth", "ended"}
# Two frames on, so whatever a frame would have started has started.
TWO_FRAMES = """() => new Promise((done) =>
  requestAnimationFrame(() => requestAnimationFrame(done)))"""
SAMPLES_STILL = f"() => Object.values(({SAMPLES_PLAYING})()).every((n) => n === 0)"
# A toggle with no click, as code opening the legend makes, never his hand.
REOPENED = """() => {
  const legend = document.querySelector('#panel-body details[data-keep="legend"]');
  legend.open = false;
  legend.open = true;
}"""
# Marks the legend drawn now, and notes TERN03 in each reading drawn after.
LEGEND_MARKED = """() => {
  document.querySelector('#panel-body details.legend').dataset.kept = 'yes';
  document.addEventListener('mycelium:modelled', (e) => {
    window.tern = e.detail.model.cells.find((s) => s.address === 'TERN03') || null;
  });
}"""
# Whether the legend marked by LEGEND_MARKED is the one drawn, and open.
LEGEND_KEPT = """() => {
  const legend = document.querySelector('#panel-body details.legend');
  return [legend?.dataset.kept === 'yes', !!legend?.open];
}"""
# Scrolls the drawer so the row of `kind` stands at its top, and says where
# its words are: the point a resting pointer sits under once the drawer has
# scrolled back by SCROLLED_PX.
ROW_AT_TOP = """(kind) => {
  const words = document.querySelector(`#panel-body [data-moves="${kind}"] dd`);
  words.scrollIntoView({ block: 'start' });
  const box = words.getBoundingClientRect();
  return [box.x + box.width / 2, box.y + box.height / 2];
}"""
SCROLLED_PX = 120
# When the ended sample's run began, so a run started over on top shows.
ENDED_BEGAN = """() => document.querySelector('#panel-body [data-moves="ended"] svg')
  .getAnimations({ subtree: true }).map((run) => run.startTime)"""
# Which sample's row is under the point given, or null.
ROW_UNDER = """([x, y]) =>
  document.elementFromPoint(x, y)?.closest('[data-moves]')?.dataset.moves ?? null"""


def playing_after_two_frames(page: Page) -> dict[str, int]:
    """How many animations each legend sample runs, two frames on."""
    page.evaluate(TWO_FRAMES)
    playing: dict[str, int] = page.evaluate(SAMPLES_PLAYING)
    return playing


def point_at(page: Page, kind: str) -> dict[str, int]:
    """Point at the words of the sample `kind`, and say what plays two frames on."""
    page.locator(f'#panel-body .moves [data-moves="{kind}"] dd').hover()
    return playing_after_two_frames(page)


def scrolled_under(page: Page, kind: str) -> dict[str, int]:
    """Scroll the row of `kind` under a resting pointer, and say what plays two frames on."""
    x, y = page.evaluate(ROW_AT_TOP, kind)
    page.mouse.move(x, y + SCROLLED_PX)
    page.mouse.wheel(0, -SCROLLED_PX)
    under = f"() => ({ROW_UNDER})([{x}, {y + SCROLLED_PX}]) === '{kind}'"
    page.wait_for_function(under, timeout=PACE_MS)
    return playing_after_two_frames(page)


def test_the_legend_shows_what_moves_moving_only_at_his_word(
    mycelium_opened: Opened, mycelium_node: StandIn
) -> None:
    page = mycelium_opened.page
    the_map_stands(page, "live")
    page.locator("#map-view").focus()
    page.keyboard.press(".")
    summary = page.locator('#panel-body details[data-keep="legend"] > summary')
    expect(summary, ". never opened Now with its legend").to_be_visible()
    # Opened by him, the legend plays every sample once.
    summary.click()
    playing = playing_after_two_frames(page)
    assert set(playing) == SAMPLES, f"the legend's What Moves holds {sorted(playing)}"
    idle = sorted(sample for sample, n in playing.items() if not n)
    assert not idle, f"opening the legend left samples still: {idle}"
    page.wait_for_function(SAMPLES_STILL, timeout=PACE_MS)
    # Pointed at, a row plays its own sample and no other; pointed at again
    # while it plays, it carries on rather than starting over on top.
    playing = point_at(page, "ended")
    moving = sorted(sample for sample, n in playing.items() if n)
    assert moving == ["ended"], f"pointing at Ended played {moving}"
    began = page.evaluate(ENDED_BEGAN)
    point_at(page, "birth")
    point_at(page, "ended")
    again = page.evaluate(ENDED_BEGAN)
    assert again == began, f"Ended started over on top of itself: {began} then {again}"
    page.wait_for_function(SAMPLES_STILL, timeout=PACE_MS)
    # A row scrolled under a resting pointer isn't pointed at: Ended, the
    # last, with the marks' rows under it to rest on first.
    playing = scrolled_under(page, "ended")
    assert not any(playing.values()), f"a row scrolled under a resting pointer played {playing}"
    # A toggle with no click plays nothing.
    page.evaluate(REOPENED)
    playing = playing_after_two_frames(page)
    assert not any(playing.values()), f"opening the legend with no click played {playing}"
    # A reading that changes the drawer's words leaves the legend drawn.
    page.evaluate(LEGEND_MARKED)
    mycelium_node.tell("cell", gone=["cell:TERN03"])
    page.wait_for_function(TERN_GONE, timeout=PACE_MS)
    kept, still_open = page.evaluate(LEGEND_KEPT)
    assert kept, "a reading that left the legend's words drew the legend again"
    assert still_open, "a reading shut the legend left open"
    # Asked for less motion, what plays stops, and pointing plays nothing.
    summary.click()
    summary.click()
    page.emulate_media(reduced_motion="reduce")
    playing = playing_after_two_frames(page)
    assert not any(playing.values()), f"less motion asked for left samples moving: {playing}"
    playing = point_at(page, "flight")
    assert not any(playing.values()), f"a sample moved with less motion asked for: {playing}"


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
        first = lines.index("Answers Owed") if "Answers Owed" in lines else len(lines)
        glance = next(n for n, line in enumerate(lines) if line.startswith("Your whole network"))
        assert first < glance, f"Now should say what holds the flow before the glance: {lines}"
        said = lines[first : first + 6]
        assert said[:2] == ["Answers Owed", WAITED_SHORT], f"Now misread who owes: {said}"
        assert said[2:3] in [[ASKED_WORDS.format(age)] for age in ages], (
            f"Now misread the question waiting: {said}"
        )
        assert said[3:5] == ["Holds Standing", HOLDER_SHORT], f"Now misread who holds: {said}"
        held_words = [f"{one}, {other}" for one, other in zip(reach, until, strict=True)]
        assert said[5:] in [[words] for words in held_words], f"Now misread the hold: {said}"
        page.evaluate(f"location.hash = '#cell={WAITED}'")
        title = page.locator("#panel-title")
        expect(title, f"{WAITED}'s drawer never opened").to_have_text(WAITED_SHORT)
        waits = f"hasn't answered {ASKER_SHORT}'s question of"
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
    page.wait_for_function(f"() => {PAINTED}.folded > 0", timeout=PACE_MS)
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
        f"() => !{MARKS.format(SENSOR)}.some((mark) => mark.cls === 'm-sensed')", timeout=PACE_MS
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
        page.wait_for_function(f"() => {PAINTED}.said === {len(TALKERS)}", timeout=PACE_MS)
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
            f"() => !{PAINTED}.talks.some((t) => t.includes('faint'))", timeout=PACE_MS
        )
        drawn = page.evaluate(f"{PAINTED}.talks")
        assert drawn == ["m-talk warm2"], f"a network open drew its talk as {drawn}"
        # The line open spreads into the pairs it stands for, its own lit.
        pair = "encodeURIComponent('c:HERON2|c:TERN03')"
        page.evaluate(f"location.hash = '#talk=' + {pair}")
        page.wait_for_function(f"() => {PAINTED}.talk === {len(TALKERS)}", timeout=PACE_MS)
        drawn = sorted(page.evaluate(f"{PAINTED}.talks"))
        assert drawn == ["m-talk warm1 lit", "m-talk warm2"], f"a talk open drew {drawn}"
        ends = page.evaluate(f"{PAINTED}.talkEnds")
        assert all(end.startswith("c:") for end in ends), f"a talk open drew lines to {ends}"
        page.evaluate(f"location.hash = '#cell={HEARD}'")
        page.wait_for_function(
            f"() => {PAINTED}.talks.filter((t) => !t.includes('faint')).length === {len(TALKERS)}",
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


def test_the_asks_beside_the_drawer_follow_what_it_shows_and_are_answered_there(
    browser: Browser, asks_served: tuple[Served, StandIn]
) -> None:
    (root, _), to_do = asks_served
    opened = Opened(browser, f"{root}mycelium/", SIZES[-1], "dark")
    try:
        page = opened.page
        the_map_stands(page, "live")
        part = page.locator("#asks-part")
        rows = part.locator("article.ask")
        expect(rows, "Now didn't hold every look waiting on you").to_have_count(len(ASKING))
        page.evaluate(f"location.hash = '#cell={ASKING[1]}'")
        expect(rows, "a cell's drawer showed another's asks").to_have_count(1)
        expect(rows.locator("h3")).to_have_text(f"{ASKING[1]}'s Change")
        page.evaluate(f"location.hash = '#machine={ASKLESS_MACHINE}'")
        expect(part, "a machine's drawer showed the asks").to_be_hidden()
        page.evaluate("location.hash = ''")
        expect(rows, "let go, the asks didn't widen again").to_have_count(len(ASKING))
        page.evaluate(f"location.hash = '#cell={ASKING[0]}'")
        expect(part, "a cell's own ask wasn't shown beside its drawer").to_be_visible()
        rows.get_by_role("button", name="Merge It").click()
        expect(rows.locator(".ask-told")).to_have_text(
            "Answered Merge It: sent to the session that asked."
        )
        expect(rows.get_by_role("button"), "answers stayed once one was pressed").to_have_count(0)
        steered = [(asked["address"], asked["steer"]) for asked in to_do.steered]
        assert len(steered) == 1, f"the map's answer was carried {len(steered)} times"
        address, steer = steered[0]
        assert address == ASKING[0], f"the answer went to {address}"
        assert isinstance(steer, dict), f"the steer was {steer!r}"
        assert (steer["look"], steer["value"]) == (f"look-{ASKING[0]}", "yes"), steer
    finally:
        opened.close()

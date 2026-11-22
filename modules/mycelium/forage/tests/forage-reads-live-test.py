"""The forage page reads live from the node, through the sites' server, in a browser.

The page is the one Nix builds, served by the real local-sites server beside a
stand-in node (modules/local-sites/tests/stand_in_daemon.py) holding four cells'
forage on kestrel (node.json): one with a purpose, a plan and a thing it waits
on its person for, relying on a need the one it started serves, a start the
node's book holds, one with no purpose written, split from one whose lineage
names nothing but that split, which is only claimed.
Its feed is trimmed to the fields home.nix declares for a machine in a realm
(NODE_KINDS). It must say it's live with every cell drawn, each started or
split cell under the one it grew from, the booked start drawn whole and the
claimed split dashed and said as claimed at both ends, say what waits on its person and the
plan furthest from done in its lede, draw each cell's plans, quest, needs and
split in its panel and the need as a dotted arc, step its cursor by keys,
follow a purpose written, drawn as words though it holds markup, a wait
answered and a cell going, say broken when the node stops with its cells kept
and live again, its cells read again, when it's back, say stale when the
server goes and keep what it drew, keep to its budget, and pass the
web-design skill's page checks at a phone's width and a desk's in both looks.
Beside a node holding more cells than the floor draws, its floor must follow the
cursor to a booked start past the floor's 48th, drawn solid, and hold still as
the cursor steps on it, a find narrows the list and a record arrives, until a
step leaves it.
Anything the page logs fails it, but for the browser's own lines once the test
takes the server away. The server, the browser and the tab are
modules/local-sites/tests/live_page.py's.
"""

import json
import os
import re
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
# A page that dropped what it drew on a change shows it within a redraw, and one that lost its
# server tries again a second on (feed.ts); waiting this long lets both happen first.
REDRAWN_MS = 2500
# A purpose a cell's writer chose, with markup in it.
WRITTEN = "OSPREY keeps watch over the <i>harbour</i>."
# The need SKIFFS serves KESTRL, as node.json says it, with markup in it.
NEED = "a <b>calm</b> field"
# Every cell node.json holds, in the order the page lists them: those that grew from none here
# by address, each followed by the cells started by it or split from it.
CELLS = ["KESTRL", "SKIFFS", "TERNAA", "OSPREY"]
# What a first visit to the forage page may cost, drawn from the stand-in's cells. Its files sit
# at what the page asks for now, on purpose: each module is one more file a first visit waits
# on, so splitting one out raises this as a choice rather than slipping past it. The kit's
# cell.ts is the 23rd, the one way every site names a cell.
BUDGET = Budget(files=23, kilobytes=513, elements=255)


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


def opened_on(page: Page, name: str) -> Locator:
    """Press `name`'s row, and give its panel once it draws that cell."""
    cell(page, name).click()
    panel = page.locator("#panel-body")
    expect(panel.locator("h3"), f"the panel didn't open {name}").to_have_text(name)
    return panel


def fact(panel: Locator, said: str) -> Locator:
    """Give what the panel says beside the term `said`, whole, so Started isn't Started by."""
    term = panel.page.locator("dt", has_text=re.compile(f"^{re.escape(said)}$"))
    return panel.locator(".facts > div").filter(has=term).locator("dd")


def selected(page: Page, name: str, why: str) -> None:
    expect(cell(page, name), why).to_have_attribute("aria-selected", "true")


def test_the_forage_page_draws_every_cell_live(forage_page: Page) -> None:
    says_it_stands(forage_page, "live")
    every_cell_drawn(forage_page)
    expect(forage_page.locator("#headline"), "the headline didn't count the cells").to_have_text(
        "4 cells, 1 without purpose."
    )
    expect(forage_page.locator("#substrate"), "the page didn't say where they stand").to_have_text(
        "kestrel"
    )


def test_a_cell_started_or_split_stands_under_the_one_it_grew_from(forage_page: Page) -> None:
    says_it_stands(forage_page, "live")
    every_cell_drawn(forage_page)
    for name, depth in [("KESTRL", 0), ("SKIFFS", 1), ("TERNAA", 0), ("OSPREY", 1)]:
        expect(cell(forage_page, name), f"{name} wasn't drawn {depth} deep").to_have_attribute(
            "style", f"--depth:{depth}"
        )
    # SKIFFS's start is in the node's book, so only OSPREY's split is drawn as claimed.
    branch = forage_page.locator('path.hypha[data-cell="SKIFFS"]')
    expect(branch, "SKIFFS's booked branch was dashed").to_have_css("stroke-dasharray", "none")
    branch = forage_page.locator('path.hypha[data-cell="OSPREY"]')
    expect(branch, "OSPREY's split wasn't drawn").to_have_count(1)
    expect(branch, "OSPREY's claimed split wasn't dashed").not_to_have_css(
        "stroke-dasharray", "none"
    )
    # Each booked end says its cell whole, so a fact gone missing can't pass for one unhedged.
    # These read text, which is the same however a line wraps; the markup in draw.test.ts
    # pins which comma each tie holds, and here a tie is checked to hold, as its style says.
    panel = opened_on(forage_page, "SKIFFS")
    expect(fact(panel, "Started by"), "SKIFFS's booked start didn't read whole").to_have_text(
        "KESTRL"
    )
    panel = opened_on(forage_page, "KESTRL")
    expect(fact(panel, "Started"), "KESTRL's booked start didn't read whole").to_have_text(
        "One cell: SKIFFS"
    )
    panel = opened_on(forage_page, "OSPREY")
    expect(fact(panel, "Split from"), "OSPREY didn't say it split from TERNAA").to_contain_text(
        "TERNAA, as claimed"
    )
    tie = fact(panel, "Split from").locator(".tie")
    expect(tie, "OSPREY's hedged ancestor could break from its comma").to_have_css(
        "white-space", "nowrap"
    )
    panel = opened_on(forage_page, "TERNAA")
    expect(fact(panel, "Split into"), "TERNAA didn't say it split into OSPREY").to_contain_text(
        "One cell, as claimed: OSPREY"
    )


def test_what_waits_on_its_person_is_said_first(forage_page: Page) -> None:
    says_it_stands(forage_page, "live")
    every_cell_drawn(forage_page)
    lede = forage_page.locator("#lede")
    expect(lede, "the lede didn't say what waits").to_contain_text(
        "One thing waits on you, from KESTRL."
    )
    expect(lede, "the lede didn't say the plan furthest to go").to_contain_text(
        "KESTRL has furthest to go, 2 of 5 steps done."
    )
    # The cursor stands on the first row, so its panel is open from the start.
    panel = forage_page.locator("#panel-body")
    expect(panel.locator("h3"), "the panel didn't open the first row").to_have_text("KESTRL")
    for said in ["Say which field to hover over", "Ready now", "Hover Over the Field"]:
        expect(panel, f"KESTRL's panel didn't say {said!r}").to_contain_text(said)
    expect(fact(panel, "Its quest"), "KESTRL's panel didn't name its quest").to_have_text(
        "QEST-20261116-KESTRL active"
    )
    panel = opened_on(forage_page, "SKIFFS")
    expect(panel, "SKIFFS's panel didn't say its purpose").to_contain_text(
        "SKIFFS rigs the skiff for the season."
    )
    expect(panel, "SKIFFS's panel drew KESTRL's wait").not_to_contain_text("What It Waits On")


def test_a_need_one_cell_serves_another_is_drawn_at_both_ends(forage_page: Page) -> None:
    says_it_stands(forage_page, "live")
    every_cell_drawn(forage_page)
    expect(forage_page.locator("#floor path.join"), "the need drew no arc").to_have_count(1)
    panel = opened_on(forage_page, "SKIFFS")
    serves = fact(panel, "Serves")
    expect(serves, "SKIFFS didn't say whom it serves").to_contain_text("KESTRL")
    expect(serves, "the need wasn't drawn as words").to_contain_text(NEED)
    expect(panel.locator("b"), "the need was read as markup").to_have_count(0)
    panel = opened_on(forage_page, "KESTRL")
    expect(fact(panel, "Relies on"), "KESTRL didn't say whom it relies on").to_contain_text(
        "SKIFFS"
    )


def test_the_keys_step_the_cursor(forage_page: Page) -> None:
    says_it_stands(forage_page, "live")
    every_cell_drawn(forage_page)
    selected(forage_page, "KESTRL", "the cursor didn't start on the first row")
    forage_page.keyboard.press("j")
    selected(forage_page, "SKIFFS", "j didn't step down")
    forage_page.keyboard.press("-")
    selected(forage_page, "KESTRL", "- didn't step up to the starter")
    forage_page.keyboard.press("G")
    selected(forage_page, "OSPREY", "G didn't go to the last row")
    expect(forage_page.locator("#panel-body h3"), "the panel didn't follow").to_have_text("OSPREY")
    forage_page.keyboard.press("g")
    selected(forage_page, "KESTRL", "g didn't go to the first row")


# The tops a node longer than the floor holds, CELLAA to CELLCG, then the one
# the last of them started, as its book holds: the list's 60th.
TOPS = [f"CELL{chr(65 + at // 26)}{chr(65 + at % 26)}" for at in range(59)]
SHOOT = "SHOOTS"


def long_record(cell: str, started_by: str | None = None) -> dict[str, object]:
    """Give a forage record for `cell` as the node does, started by `started_by` by its book."""
    lineage: dict[str, object] = {"started_by": started_by, "parent": None}
    if started_by is not None:
        lineage |= {"started_by_from": "book", "parent": f"QEST-20261116-{started_by}"}
    return {
        "key": f"forage:{cell}",
        "kind": "forage",
        "heard": 0,
        "cell": cell,
        "session": f"0000-{cell.lower()}",
        "machine": "kestrel",
        "quest": f"QEST-20261116-{cell}",
        "purpose": f"{cell} keeps one field.",
        "status": "active",
        "documents": [],
        "lineage": lineage,
    }


@pytest.fixture
def long_node(tmp_path: Path) -> Iterator[StandIn]:
    records = [long_record(cell) for cell in TOPS] + [long_record(SHOOT, TOPS[-1])]
    reading = tmp_path / "long.json"
    reading.write_text(json.dumps({"domain": "node", "kinds": {"forage": records}}))
    yield from standing_in(tmp_path / "long.sock", "node", reading)


@pytest.fixture
def long_page(browser: Browser, tmp_path: Path, long_node: StandIn) -> Iterator[Page]:
    feed = {"socket": str(long_node.path), "kinds": json.loads(os.environ["NODE_KINDS"])}
    with serving(tmp_path, {"forage": os.environ["FORAGE_SITE"]}, {"node": feed}) as (root, _):
        opened = Opened(browser, f"{root}forage/", SIZES[-1], "dark")
        yield opened.page
        opened.close()


def on_the_floor(page: Page) -> list[str]:
    """Give the cells the floor draws a compartment for, in its order."""
    marks = page.locator("#floor rect[data-cell]")
    return [str(each) for each in marks.evaluate_all("all => all.map((m) => m.dataset.cell)")]


def ringed_on_the_floor(page: Page, name: str, why: str) -> None:
    expect(page.locator("#floor rect.on"), why).to_have_attribute("data-cell", name)


def test_the_floor_follows_the_cursor_down_a_long_list_and_holds_still_on_it(
    long_page: Page, long_node: StandIn
) -> None:
    says_it_stands(long_page, "live")
    marks = long_page.locator("#floor rect[data-cell]")
    expect(marks, "the floor didn't draw 48 of the 60 cells").to_have_count(48)
    cell(long_page, SHOOT).click()
    selected(long_page, SHOOT, f"{SHOOT}'s row didn't take the cursor")
    branch = long_page.locator(f'#floor path.hypha[data-cell="{SHOOT}"]')
    expect(branch, f"the floor didn't follow the cursor to {SHOOT}").to_have_count(1)
    expect(branch, f"{SHOOT}'s booked branch was dashed").to_have_css("stroke-dasharray", "none")
    drawn = on_the_floor(long_page)
    # The floor around the shoot holds the list's 21st; one drawn around it draws the first 48.
    # A mark on the floor drawn shows whether a step draws it again.
    held = TOPS[20]
    long_page.evaluate("document.querySelector('#floor svg').dataset.kept = 'yes'")
    cell(long_page, held).click()
    selected(long_page, held, f"{held}'s row didn't take the cursor")
    ringed_on_the_floor(long_page, held, "the ring didn't follow a step on the floor")
    assert on_the_floor(long_page) == drawn, "a step the floor holds moved it"
    kept = long_page.evaluate("document.querySelector('#floor svg')?.dataset.kept ?? ''")
    assert kept == "yes", "a step the floor holds drew it again"
    # A find leaving only the cell the cursor is on narrows the list, not the floor.
    long_page.keyboard.press("/")
    long_page.locator("#find").fill(held)
    expect(cell(long_page, TOPS[0]), "the find left a cell it doesn't match").to_have_count(0)
    ringed_on_the_floor(long_page, held, "a find moved the ring")
    assert on_the_floor(long_page) == drawn, "a find moved the floor"
    long_page.locator("#find").press("Escape")
    expect(cell(long_page, TOPS[0]), "ending the find didn't bring the list back").to_have_count(1)
    assert on_the_floor(long_page) == drawn, "ending a find moved the floor"
    # A find that matches nothing, as a typo does, keeps the cursor and the floor through its end.
    long_page.keyboard.press("/")
    long_page.locator("#find").fill("ZZZZZZ")
    expect(long_page.locator("#cells .none"), "the find matched a cell").to_have_count(1)
    ringed_on_the_floor(long_page, held, "a find that matched nothing moved the ring")
    assert on_the_floor(long_page) == drawn, "a find that matched nothing moved the floor"
    long_page.locator("#find").press("Escape")
    selected(long_page, held, "ending a find that matched nothing lost the cursor")
    assert on_the_floor(long_page) == drawn, "ending a find that matched nothing moved the floor"
    # A record hollows a cell the floor draws: it's drawn hollow, and the floor holds still.
    # The floor a step off draws holds that cell too, so it shows which list the step drew.
    recorded = TOPS[30]
    long_node.tell("forage", changed=[{"key": f"forage:{recorded}", "purpose": ""}])
    hollowed = long_page.locator(f'#floor rect.hollow[data-cell="{recorded}"]')
    expect(hollowed, "the floor didn't draw the record").to_have_count(1, timeout=PACE_MS)
    ringed_on_the_floor(long_page, held, "a record moved the ring")
    assert on_the_floor(long_page) == drawn, "a record moved the floor"
    long_page.keyboard.press("g")
    selected(long_page, TOPS[0], "g didn't go to the first row")
    ringed_on_the_floor(long_page, TOPS[0], "a step off the floor didn't draw it around the cursor")
    expect(hollowed, "a step off the floor drew the list from before the record").to_have_count(1)


def test_a_find_matching_nothing_keeps_the_cursor_until_a_link_or_its_cell_ending(
    long_page: Page, long_node: StandIn
) -> None:
    says_it_stands(long_page, "live")
    cell(long_page, SHOOT).click()
    branch = long_page.locator(f'#floor path.hypha[data-cell="{SHOOT}"]')
    expect(branch, f"the floor didn't follow the cursor to {SHOOT}").to_have_count(1)
    # The floor around the shoot holds the list's 21st, so a step there keeps it.
    held = TOPS[20]
    cell(long_page, held).click()
    ringed_on_the_floor(long_page, held, "the ring didn't follow a step on the floor")
    # A link naming the held cell, in another form, ends the find that hides it.
    long_page.keyboard.press("/")
    long_page.locator("#find").fill("ZZZZZZ")
    expect(cell(long_page, held), "the find left the held cell").to_have_count(0)
    long_page.evaluate(f"location.hash = '#cell={held}%40kestrel'")
    selected(long_page, held, "a link to the held cell didn't bring its row back")
    # The held cell ending during the find leaves the floor drawn around the shoot.
    long_page.keyboard.press("/")
    long_page.locator("#find").fill("ZZZZZZ")
    expect(long_page.locator("#cells .none"), "the find matched a cell").to_have_count(1)
    long_node.tell("forage", gone=[f"forage:{held}"])
    expect(
        long_page.locator(f'#floor rect[data-cell="{held}"]'), "the floor kept a cell gone"
    ).to_have_count(0, timeout=PACE_MS)
    expect(branch, "a cell gone during the find moved the floor").to_have_count(1)


# A cell named by its whole link, as a notice or another page gives it.
LINKED = "TERNAA"
LINKED_AS = f"mycelium:cell/{LINKED}@kestrel#notice:4"


def test_a_cell_opens_by_its_whole_link_as_its_address(
    browser: Browser, forage_served: Served
) -> None:
    root, _ = forage_served
    opened = Opened(browser, f"{root}forage/#cell={LINKED_AS}", SIZES[-1], "dark")
    try:
        page = opened.page
        says_it_stands(page, "live")
        selected(page, LINKED, f"#cell={LINKED_AS} didn't open {LINKED}")
        expect(page, "the hash wasn't said as its address").to_have_url(
            re.compile(f"#cell={LINKED}$")
        )
        page.evaluate("location.hash = '#cell=SKIFFS%40kestrel'")
        selected(page, "SKIFFS", "an address on its machine didn't open its cell")
    finally:
        opened.close()


def test_a_purpose_written_and_a_cell_gone_are_followed(
    forage_page: Page, mycelium_node: StandIn
) -> None:
    says_it_stands(forage_page, "live")
    every_cell_drawn(forage_page)
    headline = forage_page.locator("#headline")
    # A change carries only what moved, as the node sends it.
    mycelium_node.tell("forage", changed=[{"key": "forage:OSPREY", "purpose": WRITTEN}])
    expect(headline, "a purpose written wasn't followed").to_have_text(
        "4 cells, each with its purpose.", timeout=PACE_MS
    )
    expect(cell(forage_page, "OSPREY"), "OSPREY's purpose wasn't drawn as words").to_contain_text(
        WRITTEN
    )
    panel = opened_on(forage_page, "OSPREY")
    expect(panel.locator(".lead"), "OSPREY's panel didn't draw its purpose").to_have_text(WRITTEN)
    expect(forage_page.locator("main i"), "OSPREY's purpose was read as markup").to_have_count(0)
    mycelium_node.tell("forage", gone=["forage:TERNAA"])
    expect(cell(forage_page, "TERNAA"), "the gone cell was still drawn").to_have_count(
        0, timeout=PACE_MS
    )
    expect(headline, "the count didn't follow the cell going").to_have_text(
        "3 cells, each with its purpose."
    )


def test_a_wait_answered_is_no_longer_said(forage_page: Page, mycelium_node: StandIn) -> None:
    says_it_stands(forage_page, "live")
    every_cell_drawn(forage_page)
    lede = forage_page.locator("#lede")
    expect(lede, "the lede didn't say what waits").to_contain_text("waits on you")
    mycelium_node.tell("forage", changed=[{"key": "forage:KESTRL", "waits": None}])
    expect(lede, "the answered wait was still said").not_to_contain_text(
        "waits on you", timeout=PACE_MS
    )
    expect(lede, "the lede lost what follows the wait").to_contain_text(
        "OSPREY has no purpose written yet."
    )
    expect(forage_page.locator("#panel-body"), "KESTRL's panel kept its wait").not_to_contain_text(
        "What It Waits On"
    )


def test_a_node_that_stops_is_said_broken_then_live(
    forage_page: Page, mycelium_node: StandIn
) -> None:
    says_it_stands(forage_page, "live")
    every_cell_drawn(forage_page)
    mycelium_node.leave()
    says_it_stands(forage_page, "broken")
    forage_page.wait_for_timeout(REDRAWN_MS)
    every_cell_drawn(forage_page)
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
    _, server = forage_served
    forage_opened.loses(server)
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
        # KESTRL's panel, open from the start, holds its wait, its plan and the need it relies on.
        opened.passes_the_skills_checks()
    finally:
        # Closed whatever failed, so what the page logged is said too.
        opened.close()

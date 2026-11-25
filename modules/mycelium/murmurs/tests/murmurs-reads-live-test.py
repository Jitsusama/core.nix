"""The murmurs page reads live from its node, through the sites' server, in a real browser.

The page is the one Nix builds, served by the real local-sites server beside a
stand-in node (modules/local-sites/tests/stand_in_daemon.py) holding three
murmurs under test names, said minutes ago, through the node feed exactly as
home.nix declares it for a machine in a realm (NODE_KINDS). It must say it's
live with every murmur drawn and counted, take a murmur said, settled or gone
within its pace, say a quiet node in a line with no empty list and no bare
sky, never call a node it never heard quiet, say what it couldn't read, say
broken when the node stops while keeping what it drew and live once it's back,
say stale when the server goes, draw a murmur's words as words and never as
markup, step its cursor by the keys and by a tap on the keys its head shows,
and pass the web-design skill's page checks at a phone's width and a desk's in
both looks. Anything the page logs fails it. The server, the browser and the tab are
modules/local-sites/tests/live_page.py's, which opens once more a tab whose
renderer died before its first reading, as a busy builder's sometimes does.
"""

import contextlib
import json
import os
import time
import warnings
from pathlib import Path
from typing import TYPE_CHECKING

import pytest
from live_page import LOOKS, SIZES, Budget, Opened, Served, serving
from playwright.sync_api import Browser, Error, Page, expect
from stand_in_daemon import StandIn

if TYPE_CHECKING:
    from collections.abc import Callable, Iterator

# A murmur shows the moment the feed hands it on, so a change must show well inside this.
PACE_MS = 6000
# The server watches a daemon that went again every 2 seconds (upstream.rs's
# AGAIN) while the page stays connected to it, so a node back is live well
# inside this.
BACK_MS = 12000
# A page that dropped what it drew on a change shows it within a redraw;
# waiting this long lets it, before the test looks for what should stay.
REDRAWN_MS = 1500
# How many times a tab crashed at its first reading is read: once in the tab
# that crashed, once in the tab opened in its place. And what it warns.
TWICE = 2
REOPENED = "crashed and was opened once more"
# What the three murmurs come to: all said, one of them settled.
COUNTED = "Three murmured, two still in the air."
# What a first visit to the page may cost, drawn from the three murmurs. The quiet
# day's line is two of the elements and one of the kilobytes.
BUDGET = Budget(files=20, kilobytes=423, elements=147)
# How long ago each murmur was said, in seconds, oldest first.
AGES = (40 * 60, 25 * 60, 10 * 60)
# The newest murmur still in the air, which the lede quotes. Nearly every murmur a cell says
# runs past the 140 characters the lede quotes before it trims, as this one does, so the lede
# this draws is the one a first visit usually ends on.
NEWEST = (
    "Gulls are back on the breakwater, a week before the almanac said they would be, and "
    "they're nesting low on the seaward side where the spring tides reach the rocks."
)
# A desk as wide as the laptop's monitor shows the page, past SIZES' desk.
WIDE_DESK = (1920, 1080)
# The height in pixels each of the head's parts takes with the words index.html opens with and
# with those the feed wrote, by its id. The opening words are put back for a moment to be
# measured, so both are laid out in the same box.
OPENING_AND_LIVE = """async (ids) => {
  const opening = new DOMParser().parseFromString(
    await (await fetch(location.href)).text(), "text/html");
  return Object.fromEntries(ids.map((id) => {
    const part = document.getElementById(id);
    const live = part.innerHTML;
    const liveHeight = part.getBoundingClientRect().height;
    part.innerHTML = opening.getElementById(id).innerHTML;
    const openingHeight = part.getBoundingClientRect().height;
    part.innerHTML = live;
    return [id, [Math.round(openingHeight), Math.round(liveHeight)]];
  }));
}"""


def murmur(writer: str, at: int, words: str, **more: object) -> dict[str, object]:
    """Make a murmur record by `writer` at `at`, as the node keys and tells one."""
    return {
        "key": f"murmur:{writer}:{at}",
        "writer": writer,
        "at": at,
        "text": words,
        "tied_to": [],
        "purpose": f"Watch the {writer.lower()} shore: the tide's hours, read each morning.",
        "settled": None,
        **more,
    }


@pytest.fixture
def said_at() -> tuple[int, ...]:
    """When each murmur was said, in seconds since 1970, oldest first."""
    now = int(time.time())
    return tuple(now - age for age in AGES)


@pytest.fixture
def murmuring_node(tmp_path: Path, said_at: tuple[int, ...]) -> Iterator[StandIn]:
    kestrel, osprey, tern = said_at
    murmurs = [
        murmur(
            "KESTRL",
            kestrel,
            "The tide turned an hour early at the outer buoy.",
            tied_to=["QEST-20261116-OSPREY"],
        ),
        murmur(
            "OSPREY", osprey, "The harbour chart is a day behind.", settled="charted this morning"
        ),
        murmur("TERNAA", tern, NEWEST),
    ]
    stand_in = StandIn(tmp_path / "node.sock", "node", {"murmur": murmurs}).start()
    yield stand_in
    stand_in.leave()


@pytest.fixture
def murmurs_served(tmp_path: Path, murmuring_node: StandIn) -> Iterator[Served]:
    feed = {"socket": str(murmuring_node.path), "kinds": json.loads(os.environ["NODE_KINDS"])}
    with serving(tmp_path, {"web": os.environ["MURMURS_SITE"]}, {"node": feed}) as served:
        yield served


@pytest.fixture
def murmurs_opened(browser: Browser, murmurs_served: Served) -> Iterator[Opened]:
    root, _ = murmurs_served
    opened = Opened(browser, f"{root}web/", SIZES[-1], "dark")
    try:
        opened.first(drawn)
        yield opened
    finally:
        opened.close()


def drawn(page: Page) -> None:
    """Read the page live with every murmur counted, its first reading."""
    the_page_stands(page, "live")
    the_page_counts(page, COUNTED, "the page miscounted the murmurs")


def the_page_stands(page: Page, state: str, timeout: int = PACE_MS) -> None:
    standing = page.locator("#asof")
    expect(standing, f"the murmurs page never said it was {state}").to_have_attribute(
        "data-state", state, timeout=timeout
    )


def the_page_counts(page: Page, counted: str, why: str) -> None:
    expect(page.locator("#headline"), why).to_have_text(counted, timeout=PACE_MS)


def rows(page: Page) -> list[str]:
    """Read the murmurs' keys as the list draws them, top to bottom."""
    keys: list[str] = page.locator("#murmurs [role=option]").evaluate_all(
        "rows => rows.map(row => row.dataset.key)"
    )
    return keys


def the_cursor_is_on(page: Page, key: str, why: str) -> None:
    chosen = page.locator('#murmurs [aria-selected="true"]')
    expect(chosen, why).to_have_attribute("data-key", key, timeout=PACE_MS)


def test_the_murmurs_are_drawn_live_newest_first(
    murmurs_opened: Opened, said_at: tuple[int, ...]
) -> None:
    page = murmurs_opened.page
    the_page_stands(page, "live")
    the_page_counts(page, COUNTED, "the page miscounted the murmurs")
    kestrel, osprey, tern = said_at
    newest_first = [f"murmur:TERNAA:{tern}", f"murmur:OSPREY:{osprey}", f"murmur:KESTRL:{kestrel}"]
    assert rows(page) == newest_first, f"the murmurs were listed {rows(page)}"
    settled = page.locator(f'#murmurs [data-key="murmur:OSPREY:{osprey}"] .settled')
    expect(settled, "the settled murmur didn't say how").to_have_text(
        "Settled: charted this morning"
    )
    # Its quest's name and its ties are what the node feed lets through as
    # purpose and tied_to, so either withheld shows here.
    said = page.locator(f'#murmurs [data-key="murmur:KESTRL:{kestrel}"] .said')
    expect(said, "the murmur wasn't named by its quest's purpose").to_contain_text(
        "from Watch the kestrl shore"
    )
    expect(said, "the murmur didn't say what it's tied to").to_contain_text("tied to OSPREY")


def test_a_murmur_said_arrives(murmurs_opened: Opened, murmuring_node: StandIn) -> None:
    page = murmurs_opened.page
    the_page_counts(page, COUNTED, "the page miscounted the murmurs")
    now = int(time.time())
    murmuring_node.tell("murmur", changed=[murmur("HERONS", now, "Fog on the far shore.")])
    the_page_counts(page, "Four murmured, three still in the air.", "the new murmur wasn't heard")
    assert rows(page)[0] == f"murmur:HERONS:{now}", "the newest murmur wasn't listed first"


def test_a_murmur_settled_comes_to_roost(
    murmurs_opened: Opened, murmuring_node: StandIn, said_at: tuple[int, ...]
) -> None:
    page = murmurs_opened.page
    the_page_counts(page, COUNTED, "the page miscounted the murmurs")
    key = f"murmur:TERNAA:{said_at[2]}"
    murmuring_node.tell("murmur", changed=[{"key": key, "settled": "counted at noon"}])
    the_page_counts(page, "Three murmured, one still in the air.", "the settling wasn't heard")
    expect(
        page.locator(f'#murmurs [data-key="{key}"] .settled'), "the row didn't say it settled"
    ).to_have_text("Settled: counted at noon")


def test_a_murmur_that_goes_leaves_the_page(
    murmurs_opened: Opened, murmuring_node: StandIn, said_at: tuple[int, ...]
) -> None:
    page = murmurs_opened.page
    the_page_counts(page, COUNTED, "the page miscounted the murmurs")
    key = f"murmur:KESTRL:{said_at[0]}"
    murmuring_node.tell("murmur", gone=[key])
    the_page_counts(page, "Two murmured, one still in the air.", "the gone murmur still counted")
    expect(
        page.locator(f'[data-key="{key}"]'), "the gone murmur's row or bird stayed"
    ).to_have_count(0)


def test_a_murmur_it_cannot_read_is_said_while_the_rest_draw(
    murmurs_opened: Opened, murmuring_node: StandIn
) -> None:
    page = murmurs_opened.page
    the_page_counts(page, COUNTED, "the page miscounted the murmurs")
    murmuring_node.tell("murmur", changed=[{"key": "murmur:GULLSS:1", "writer": "GULLSS"}])
    expect(page.locator("#asof"), "the unread murmur wasn't said").to_contain_text(
        "Couldn't read: murmur:GULLSS:1, which names no writer, words or time", timeout=PACE_MS
    )
    the_page_counts(page, COUNTED, "the unread murmur cost the page its count")


def test_a_node_that_stops_is_said_broken_with_the_murmurs_kept_then_live(
    murmurs_opened: Opened, murmuring_node: StandIn
) -> None:
    page = murmurs_opened.page
    the_page_stands(page, "live")
    the_page_counts(page, COUNTED, "the page miscounted the murmurs")
    murmuring_node.leave()
    the_page_stands(page, "broken")
    page.wait_for_timeout(REDRAWN_MS)
    the_page_counts(page, COUNTED, "the broken page lost its murmurs")
    murmuring_node.start()
    the_page_stands(page, "live", BACK_MS)
    # A murmur after it's back shows the watch delivers again, not just the old page.
    now = int(time.time())
    murmuring_node.tell("murmur", changed=[murmur("HERONS", now, "Fog on the far shore.")])
    the_page_counts(page, "Four murmured, three still in the air.", "the page back live missed one")


def test_a_server_that_goes_is_said_stale_with_the_murmurs_kept(
    murmurs_opened: Opened, murmurs_served: Served
) -> None:
    page = murmurs_opened.page
    the_page_stands(page, "live")
    _, server = murmurs_served
    murmurs_opened.loses(server)
    the_page_stands(page, "stale")
    page.wait_for_timeout(REDRAWN_MS)
    the_page_counts(page, COUNTED, "the stale page lost its murmurs")


def test_a_murmurs_words_are_drawn_as_words(
    murmurs_opened: Opened, murmuring_node: StandIn
) -> None:
    page = murmurs_opened.page
    the_page_counts(page, COUNTED, "the page miscounted the murmurs")
    # Markup, and a quote that would break out of an attribute it's written into.
    hostile = "Mend the <b>Hull</b> & \"oars\" ' onmouseover='x' <img src=x>"
    now = int(time.time())
    hostile_murmur = murmur(
        "HERONS", now, hostile, settled=hostile, purpose=hostile, tied_to=[f"QEST-{hostile}"]
    )
    murmuring_node.tell("murmur", changed=[hostile_murmur])
    row = page.locator(f'#murmurs [data-key="murmur:HERONS:{now}"]')
    expect(row.locator(".words"), "the murmur's words never reached its row").to_have_text(
        hostile, timeout=PACE_MS
    )
    expect(row.locator(".said"), "the hostile quest name never reached its row").to_contain_text(
        f"from {hostile}"
    )
    # Past a redraw, so the lede, the flock and the row have each drawn every field.
    page.wait_for_timeout(REDRAWN_MS)
    drawn = page.locator('b:has-text("Hull"), img[src="x"], [onmouseover]')
    expect(drawn, "a murmur's words were drawn as markup somewhere").to_have_count(0)


def test_the_keys_step_the_cursor_and_show_themselves(
    murmurs_opened: Opened, said_at: tuple[int, ...]
) -> None:
    page = murmurs_opened.page
    the_page_counts(page, COUNTED, "the page miscounted the murmurs")
    kestrel, osprey, tern = said_at
    the_cursor_is_on(page, f"murmur:TERNAA:{tern}", "the cursor didn't start on the newest")
    page.keyboard.press("j")
    the_cursor_is_on(page, f"murmur:OSPREY:{osprey}", "j didn't step to the next murmur")
    page.keyboard.press("G")
    the_cursor_is_on(page, f"murmur:KESTRL:{kestrel}", "G didn't go to the oldest")
    page.keyboard.press("j")
    the_cursor_is_on(page, f"murmur:KESTRL:{kestrel}", "j stepped past the oldest")
    page.keyboard.press("k")
    the_cursor_is_on(page, f"murmur:OSPREY:{osprey}", "k didn't step back")
    page.keyboard.press("g")
    the_cursor_is_on(page, f"murmur:TERNAA:{tern}", "g didn't go to the newest")
    page.keyboard.press("?")
    expect(page.locator("#keys"), "? didn't show the keys").to_be_visible()
    page.keyboard.press("Escape")
    expect(page.locator("#keys"), "Escape didn't hide the keys").to_be_hidden()


def test_a_tap_on_a_key_the_head_shows_does_what_typing_it_does(
    murmurs_opened: Opened, said_at: tuple[int, ...]
) -> None:
    page = murmurs_opened.page
    the_page_counts(page, COUNTED, "the page miscounted the murmurs")
    _, osprey, tern = said_at
    shown = page.locator(".hero .keys kbd")
    holding = page.locator("#murmurs")
    shown.filter(has_text="j").click()
    the_cursor_is_on(page, f"murmur:OSPREY:{osprey}", "a tap on j didn't step")
    expect(holding, "the list didn't hold the keys after a tap on j").to_be_focused()
    shown.filter(has_text="k").click()
    the_cursor_is_on(page, f"murmur:TERNAA:{tern}", "a tap on k didn't step back")
    expect(holding, "the list didn't hold the keys after a tap on k").to_be_focused()
    # Typed now, so a tap that left the keys on the kbd would leave the cursor.
    page.keyboard.press("j")
    the_cursor_is_on(page, f"murmur:OSPREY:{osprey}", "a key typed after a tap didn't step")
    shown.filter(has_text="?").click()
    expect(page.locator("#keys"), "a tap on ? didn't show the keys").to_be_visible()


def test_a_tab_that_crashes_before_its_first_reading_is_opened_once_more(
    browser: Browser, murmurs_served: Served
) -> None:
    root, _ = murmurs_served
    read: list[Page] = []

    def crashed_then_counted(page: Page) -> None:
        read.append(page)
        if len(read) == 1:
            crash(page)
        the_page_counts(page, COUNTED, "the tab opened once more miscounted the murmurs")

    warned: list[str] = []
    read_first(browser, f"{root}web/", crashed_then_counted, warned)
    assert len(read) == TWICE, f"the first reading was taken {len(read)} times, not twice"
    assert read[0] is not read[1], "the reading was taken again in the tab that crashed"
    assert warned, "the tab was opened once more without a warning"
    here = f"{Path(__file__).name}: "
    astray = [said for said in warned if not said.startswith(here) or REOPENED not in said]
    assert astray == [], f"a warning said {astray}, not the test's tab opened once more"


def test_a_tab_that_crashes_again_at_its_second_reading_fails(
    browser: Browser, murmurs_served: Served
) -> None:
    root, _ = murmurs_served
    read: list[Page] = []

    def crashed(page: Page) -> None:
        read.append(page)
        crash(page)
        the_page_counts(page, COUNTED, "the crashed tab counted the murmurs")

    warned: list[str] = []
    with pytest.raises(AssertionError, match="Target crashed"):
        read_first(browser, f"{root}web/", crashed, warned)
    assert len(read) == TWICE, f"a tab crashing at each reading was read {len(read)} times"
    assert any(REOPENED in said for said in warned), f"the tab was reopened saying {warned}"


def test_a_first_reading_that_fails_without_a_crash_is_not_taken_again(
    browser: Browser, murmurs_served: Served
) -> None:
    root, _ = murmurs_served
    read: list[Page] = []

    def miscounted(page: Page) -> None:
        read.append(page)
        # Waiting for the crash's own words, which the failure then quotes, as
        # it would a page that drew them: a quote is not a crash.
        expect(page.locator("#headline"), "the page counted").to_have_text("Target crashed")

    # Two readings would say it was taken again; a warning alone could be a
    # load that crashed on a busy builder.
    with pytest.raises(AssertionError, match="the page counted"):
        read_first(browser, f"{root}web/", miscounted, [])
    assert len(read) == 1, f"a miscount was read {len(read)} times, not once"


def read_first(
    browser: Browser, address: str, reading: Callable[[Page], None], warned: list[str]
) -> None:
    """Open the page, take its first reading, and keep each warning with where it named."""
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        try:
            opened = Opened(browser, address, SIZES[-1], "dark")
            try:
                opened.first(reading)
            finally:
                opened.close()
        finally:
            warned.extend(f"{Path(said.filename).name}: {said.message}" for said in caught)


def crash(page: Page) -> None:
    """Kill the tab's renderer, as a builder short of memory does."""
    # The navigation's own error says nothing the next reading's won't.
    with contextlib.suppress(Error):
        page.goto("chrome://crash")


@pytest.fixture
def quiet_node(tmp_path: Path) -> Iterator[StandIn]:
    stand_in = StandIn(tmp_path / "quiet.sock", "node", {"murmur": []}).start()
    yield stand_in
    stand_in.leave()


def heard_nothing(page: Page) -> None:
    """Read the page live with nothing murmured, its first reading."""
    the_page_stands(page, "live")
    the_page_counts(page, "Nothing murmured yet.", "the quiet page miscounted")


def said_quiet(page: Page, why: str) -> None:
    """Expect the page to say a quiet day: its line, the crest, and no list or flock."""
    expect(page.locator("#quiet"), why).to_be_visible(timeout=PACE_MS)
    expect(page.locator("#quiet"), why).to_contain_text("Nothing murmured in the last day.")
    # A listbox must hold options, so a list with none is no listbox at all.
    expect(page.locator('[role="listbox"]'), "an empty listbox was drawn").to_have_count(0)
    expect(page.locator("#murmurs"), "an empty list was drawn").to_be_hidden()
    expect(page.locator("#sky svg:not(.crest)"), "a bare sky was drawn").to_have_count(0)
    expect(page.locator("#sky .crest"), "the crest didn't stand in").to_be_visible()
    expect(page.locator("#sky-says"), "the legend told of no flock").to_be_hidden()
    expect(page.locator("figure.dusk"), "the crest was read out").to_have_attribute(
        "aria-hidden", "true"
    )


# The kit's italic code face, 66 KB, which a quiet day's line once sent for the
# two words of the command it names.
ITALIC_CODE = "monaspace-italic"


def test_a_quiet_day_sends_no_italic_code_face(
    browser: Browser, tmp_path: Path, quiet_node: StandIn
) -> None:
    """A quiet day's line names a command, set upright, so no italic code face is sent."""
    feed = {"socket": str(quiet_node.path), "kinds": json.loads(os.environ["NODE_KINDS"])}
    with serving(tmp_path, {"web": os.environ["MURMURS_SITE"]}, {"node": feed}) as served:
        root, _ = served
        opened = Opened(browser, f"{root}web/", SIZES[-1], "dark")
        try:
            opened.first(heard_nothing)
            said_quiet(opened.page, "the quiet day wasn't said")
            opened.page.wait_for_load_state("networkidle")
            faces = [request.url for request in opened.asked if ITALIC_CODE in request.url]
            assert faces == [], f"a quiet day's line sent the italic code face: {faces}"
            opened.keeps_to(BUDGET)
        finally:
            opened.close()


def test_a_quiet_node_is_said_in_a_line_with_no_empty_list_or_bare_sky(
    browser: Browser, tmp_path: Path, quiet_node: StandIn
) -> None:
    feed = {"socket": str(quiet_node.path), "kinds": json.loads(os.environ["NODE_KINDS"])}
    with serving(tmp_path, {"web": os.environ["MURMURS_SITE"]}, {"node": feed}) as served:
        root, _ = served
        opened = Opened(browser, f"{root}web/", SIZES[0], "light")
        try:
            opened.first(heard_nothing)
            page = opened.page
            said_quiet(page, "the quiet day wasn't said")
            opened.passes_the_skills_checks()
            # A murmur it can't read is still a murmur, so the day isn't called quiet.
            unreadable = "murmur:GULLSS:1"
            quiet_node.tell("murmur", changed=[{"key": unreadable, "writer": "GULLSS"}])
            expect(page.locator("#asof"), "the unread murmur wasn't said").to_contain_text(
                "Couldn't read: murmur:GULLSS:1", timeout=PACE_MS
            )
            expect(page.locator("#quiet"), "an unread murmur was called quiet").to_be_hidden()
            quiet_node.tell("murmur", gone=[unreadable])
            said_quiet(page, "the unread murmur going left no quiet line")
            at = int(time.time())
            quiet_node.tell("murmur", changed=[murmur("HERONS", at, "Fog.")])
            the_page_counts(page, "One murmured, all still in the air.", "the murmur went unheard")
            listed = page.locator("#murmurs")
            expect(listed, "the first murmur's list").to_have_attribute("role", "listbox")
            expect(listed, "the list went unnamed").to_have_accessible_name("The murmurs")
            expect(page.locator("#quiet"), "the quiet line outlived the quiet").to_be_hidden()
            expect(page.locator("#sky svg:not(.crest)"), "the flock didn't fly").to_have_count(1)
            expect(page.locator("#sky-says"), "the flock went unexplained").to_be_visible()
            expect(page.locator("figure.dusk"), "the flock was hidden").not_to_have_attribute(
                "aria-hidden", "true"
            )
            # Showing only what's in the air, then settling the last of it, shows every
            # murmur again rather than an empty list the quiet line would call a quiet day.
            roosted = murmur("OSPREY", at - 60, "Charted.", settled="charted this morning")
            quiet_node.tell("murmur", changed=[roosted])
            the_page_counts(page, "Two murmured, one still in the air.", "the roost went unheard")
            page.locator('#headline a[data-show="air"]').click()
            options = listed.locator('[role="option"]')
            title = page.locator("#murmurs-title")
            expect(title, "the air filter didn't take").to_have_text("The murmurs still in the air")
            key = f"murmur:HERONS:{at}"
            expect(options, "the air filter showed a settled murmur").to_have_count(1)
            expect(options, "the air filter showed the wrong murmur").to_have_attribute(
                "data-key", key
            )
            quiet_node.tell("murmur", changed=[{"key": key, "settled": "cleared by noon"}])
            roosts = "Two murmured, every one come to roost."
            the_page_counts(page, roosts, "the settling went unheard")
            expect(title, "the air filter outlived the air").to_have_text("The murmurs")
            expect(options, "the settled murmurs hid").to_have_count(2)
            expect(page.locator("#quiet"), "settled murmurs were called quiet").to_be_hidden()
            quiet_node.tell("murmur", gone=[key, str(roosted["key"])])
            said_quiet(page, "the last murmur going left no quiet line")
        finally:
            opened.close()


def never_reached(page: Page) -> None:
    """Read the page broken with nothing heard, as a node never reached leaves it."""
    the_page_stands(page, "broken")
    the_page_counts(page, "Nothing to hear yet.", "a node never reached wasn't said unheard")


def test_a_node_never_reached_is_said_unheard_and_never_quiet(
    browser: Browser, tmp_path: Path
) -> None:
    absent = tmp_path / "absent.sock"
    feed = {"socket": str(absent), "kinds": json.loads(os.environ["NODE_KINDS"])}
    with serving(tmp_path, {"web": os.environ["MURMURS_SITE"]}, {"node": feed}) as served:
        root, _ = served
        first = Opened(browser, f"{root}web/", SIZES[0], "light")
        second: Opened | None = None
        try:
            first.first(never_reached)
            # A tab joining the watch once it's broken is told the server holds no
            # records for it, which say nothing of the node's day.
            second = Opened(browser, f"{root}web/", SIZES[0], "light")
            second.first(never_reached)
            second.page.wait_for_timeout(REDRAWN_MS)
            for page in (first.page, second.page):
                quiet = page.locator("#quiet")
                expect(quiet, "a node never reached was called quiet").to_be_hidden()
                the_page_counts(page, "Nothing to hear yet.", "the unheard page changed its word")
        finally:
            if second is not None:
                second.close()
            first.close()


def test_the_murmurs_page_keeps_to_its_budget(browser: Browser, murmurs_served: Served) -> None:
    root, _ = murmurs_served
    opened = Opened(browser, f"{root}web/", SIZES[-1], "dark")
    try:
        opened.first(drawn)
        opened.keeps_to(BUDGET)
        the_page_stands(opened.page, "live")
    finally:
        opened.close()


@pytest.mark.parametrize("size", [*SIZES, WIDE_DESK])
def test_the_opening_words_take_the_lines_the_head_will(
    browser: Browser, murmurs_served: Served, size: tuple[int, int]
) -> None:
    # The words the page opens with must take the lines its headline and lede will once the
    # node answers, or everything under them moves as it does.
    root, _ = murmurs_served
    opened = Opened(browser, f"{root}web/", size, "light")
    try:
        opened.first(drawn)
        opened.page.wait_for_timeout(REDRAWN_MS)
        heights = opened.page.evaluate(OPENING_AND_LIVE, ["headline", "lede"])
        moved = {part: each for part, each in heights.items() if each[0] != each[1]}
        assert not moved, f"at {size} the head's opening and live heights differ: {moved}"
    finally:
        opened.close()


@pytest.mark.parametrize("look", LOOKS)
@pytest.mark.parametrize("size", SIZES)
def test_the_murmurs_page_passes_the_skills_page_checks(
    browser: Browser, murmurs_served: Served, size: tuple[int, int], look: str
) -> None:
    root, _ = murmurs_served
    opened = Opened(browser, f"{root}web/", size, look)
    try:
        opened.first(drawn)
        opened.passes_the_skills_checks()
    finally:
        opened.close()

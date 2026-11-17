"""The murmurs page reads live from its node, through the sites' server, in a real browser.

The page is the one Nix builds, served by the real local-sites server beside a
stand-in node (modules/local-sites/tests/stand_in_daemon.py) holding three
murmurs under test names, said minutes ago, through the node feed exactly as
home.nix declares it for a machine in a realm (NODE_KINDS). It must say it's
live with every murmur drawn and counted, take a murmur said, settled or gone
within its pace, say what it couldn't read, say broken when the node stops
while keeping what it drew and live once it's back, say stale when the server
goes, draw a murmur's words as words and never as markup, step its cursor by
the keys and by a tap on the keys its head shows, and pass the web-design
skill's page checks at a phone's width and a desk's in both looks. Anything
the page logs fails it. The server, the browser and the tab are
modules/local-sites/tests/live_page.py's.
"""

import json
import os
import time
from typing import TYPE_CHECKING

import pytest
from live_page import LOOKS, SIZES, Opened, Served, serving
from playwright.sync_api import Browser, Page, expect
from stand_in_daemon import StandIn

if TYPE_CHECKING:
    from collections.abc import Iterator
    from pathlib import Path

# A murmur shows the moment the feed hands it on, so a change must show well inside this.
PACE_MS = 6000
# The server watches a daemon that went again every 2 seconds (upstream.rs's
# AGAIN) while the page stays connected to it, so a node back is live well
# inside this.
BACK_MS = 12000
# A page that dropped what it drew on a change shows it within a redraw;
# waiting this long lets it, before the test looks for what should stay.
REDRAWN_MS = 1500
# The browser's own line for a feed that couldn't connect, once the server is gone.
UNREACHED = "WebSocket connection to"
# What the three murmurs come to: all said, one of them settled.
COUNTED = "Three murmured, two still in the air."
# How long ago each murmur was said, in seconds, oldest first.
AGES = (40 * 60, 25 * 60, 10 * 60)


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
        murmur("TERNAA", tern, "Gulls are back on the breakwater."),
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
    yield opened
    opened.close()


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
    murmuring_node.tell("murmur", gone=[f"murmur:KESTRL:{said_at[0]}"])
    the_page_counts(page, "Two murmured, one still in the air.", "the gone murmur still counted")


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
    # The feed tries the server again a second on, which the browser logs.
    murmurs_opened.expecting(UNREACHED)
    _, server = murmurs_served
    server.kill()
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
    murmuring_node.tell("murmur", changed=[murmur("HERONS", now, hostile, settled=hostile)])
    row = page.locator(f'#murmurs [data-key="murmur:HERONS:{now}"]')
    expect(row.locator(".words"), "the murmur's words never reached its row").to_have_text(
        hostile, timeout=PACE_MS
    )
    # Past a redraw, so the lede, the flock and the row have each drawn it.
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
    shown.filter(has_text="j").click()
    the_cursor_is_on(page, f"murmur:OSPREY:{osprey}", "a tap on j didn't step")
    shown.filter(has_text="k").click()
    the_cursor_is_on(page, f"murmur:TERNAA:{tern}", "a tap on k didn't step back")
    shown.filter(has_text="?").click()
    expect(page.locator("#keys"), "a tap on ? didn't show the keys").to_be_visible()


@pytest.mark.parametrize("look", LOOKS)
@pytest.mark.parametrize("size", SIZES)
def test_the_murmurs_page_passes_the_skills_page_checks(
    browser: Browser, murmurs_served: Served, size: tuple[int, int], look: str
) -> None:
    root, _ = murmurs_served
    opened = Opened(browser, f"{root}web/", size, look)
    try:
        the_page_stands(opened.page, "live")
        the_page_counts(opened.page, COUNTED, "the murmurs page didn't draw")
        opened.passes_the_skills_checks()
    finally:
        opened.close()

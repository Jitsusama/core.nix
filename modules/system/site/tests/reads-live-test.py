"""The system page reads live from the system daemon, through the sites' server, in a browser.

The page is the one Nix builds, served by the real local-sites server beside a
stand-in system daemon (modules/local-sites/tests/stand_in_daemon.py) holding
a summary, four parts and two machines under test names, system.json. It must
say it's live with every machine drawn and counted, follow a machine that
turns and one that goes, say broken when the daemon stops and live again
when it's back, say stale when the server goes, and pass the web-design skill's
page checks at a phone's width and a desk's in both looks. Its keys and places
must move the cursor from section to section. Anything the page
logs fails it, since that is how it drew four empty parts unseen on 16 November.
The server, the browser and the tab are modules/local-sites/tests/live_page.py's.
"""

import json
import os
import re
from pathlib import Path
from typing import TYPE_CHECKING

import pytest
from live_page import LOOKS, SIZES, Budget, Opened, Served, serving
from playwright.sync_api import Browser, Page, expect
from stand_in_daemon import StandIn

if TYPE_CHECKING:
    from collections.abc import Iterator

HERE = Path(__file__).parent
# The system daemon reads its sources every few seconds, so a change must show well inside this.
PACE_MS = 6000
# The feed waits up to 30 seconds between tries once the daemon has been gone a while.
RETRY_MS = 35000
# What system.json comes to: two machines, each with a filling disk.
COUNTED = "Two machines, both worth a look."
# What a first visit to the page may cost, drawn from system.json.
BUDGET = Budget(files=24, kilobytes=470, elements=370)


@pytest.fixture
def system_daemon(tmp_path: Path) -> Iterator[StandIn]:
    reading = json.loads((HERE / "system.json").read_text())
    stand_in = StandIn(tmp_path / "system.sock", "system", reading["kinds"]).start()
    yield stand_in
    stand_in.leave()


@pytest.fixture
def system_served(tmp_path: Path, system_daemon: StandIn) -> Iterator[Served]:
    whole = {"socket": str(system_daemon.path), "kinds": {"summary": {}, "part": {}, "machine": {}}}
    with serving(tmp_path, {"system": os.environ["SYSTEM_SITE"]}, {"system": whole}) as served:
        yield served


@pytest.fixture
def system_page(browser: Browser, system_served: Served) -> Iterator[Page]:
    root, _ = system_served
    opened = Opened(browser, f"{root}system/", SIZES[-1], "dark")
    yield opened.page
    opened.close()


def says_it_stands(page: Page, state: str, timeout: int = PACE_MS) -> None:
    expect(page.locator("#standing"), f"the page never said it was {state}").to_have_attribute(
        "data-state", state, timeout=timeout
    )


def counted(page: Page, said: str = COUNTED) -> None:
    expect(page, f"the page's title never counted {said}").to_have_title(
        f"{said} Your machines, now", timeout=PACE_MS
    )


def test_the_system_page_draws_every_machine_live(system_page: Page) -> None:
    says_it_stands(system_page, "live")
    counted(system_page)
    expect(system_page.locator("#standing-words"), "the page didn't say where").to_contain_text(
        "kestrel"
    )
    for machine in ["kestrel", "osprey"]:
        row = system_page.locator(f'#parts a[data-row="machine:{machine}"]')
        expect(row, f"the page drew no row for {machine}").to_be_visible()


# How far under the masthead's foot a row scrolled up to it may land, its
# own margin and a rounding: further, and it never scrolled to the edge.
SCROLLED_TO = 40
# How far over its foot a row may land, from the browser snapping a scroll
# to whole pixels; a row under the masthead would be over by its height.
SNAPPED = 1


def section_of_the_cursor(page: Page) -> str | None:
    """Say which section holds what has focus, by its id, or None."""
    held = page.evaluate("document.activeElement?.closest('section[id]')?.id ?? null")
    return held if isinstance(held, str) else None


def test_the_keys_and_places_move_the_cursor(system_page: Page) -> None:
    says_it_stands(system_page, "live")
    counted(system_page)
    keyboard = system_page.keyboard
    keyboard.press("j")
    assert section_of_the_cursor(system_page) == "machine-1"
    keyboard.press("2")
    assert section_of_the_cursor(system_page) == "machine-2"
    # With machine 1's row scrolled up out of sight, 1 scrolls back to it and
    # lands it at the top edge: clear of the masthead only while the scroll
    # padding holds it there.
    size = system_page.viewport_size
    system_page.set_viewport_size({"width": 1280, "height": 400})
    above = system_page.evaluate(
        "(() => { const row = document.querySelector('#machine-1 a[data-row]');"
        " window.scrollTo(0, row.getBoundingClientRect().bottom + window.scrollY + 50);"
        " return row.getBoundingClientRect().bottom < 0; })()"
    )
    assert above is True, "machine 1's row couldn't be scrolled out of sight"
    keyboard.press("1")
    assert section_of_the_cursor(system_page) == "machine-1"
    gap = system_page.evaluate(
        "document.activeElement.getBoundingClientRect().top"
        " - document.querySelector('.mast').getBoundingClientRect().bottom"
    )
    assert isinstance(gap, int | float), f"the gap measured {gap!r}, not a number"
    assert -SNAPPED < gap < SCROLLED_TO, f"the row landed {gap}px below the masthead's foot"
    if size is not None:
        system_page.set_viewport_size(size)
    keyboard.press("2")
    keyboard.press("k")
    assert section_of_the_cursor(system_page) == "machine-1"
    place = system_page.locator('#places a[data-place="machine-2"]')
    place.click()
    assert section_of_the_cursor(system_page) == "machine-2"
    expect(place, "the place the cursor is in wasn't marked").to_have_attribute(
        "aria-current", "location"
    )
    keyboard.press("y")
    # A headless browser may refuse the clipboard; either way y says what it
    # did, and with the command of the machine the cursor is on.
    expect(system_page.locator("#note"), "y didn't copy osprey's command").to_have_text(
        re.compile(r"^(Copied|Couldn't copy).*mycelium sprout --on osprey\b")
    )
    # Off every machine, y copies nothing and says so.
    system_page.evaluate("document.activeElement?.blur()")
    keyboard.press("y")
    expect(system_page.locator("#note"), "y off a machine didn't say why").to_have_text(
        re.compile(r"^Nothing to copy here")
    )


def test_a_machine_that_turns_is_counted(system_page: Page, system_daemon: StandIn) -> None:
    says_it_stands(system_page, "live")
    counted(system_page)
    turned: dict[str, object] = {
        "key": "machine:osprey",
        "health": "bad",
        "side": ["out of reach"],
    }
    system_daemon.tell("machine", changed=[turned])
    counted(system_page, "Two machines, one in trouble and one worth a look.")


def test_a_machine_that_goes_leaves_its_part(system_page: Page, system_daemon: StandIn) -> None:
    says_it_stands(system_page, "live")
    row = system_page.locator('#parts a[data-row="machine:osprey"]')
    expect(row, "the page drew no row for osprey").to_be_visible()
    system_daemon.tell("machine", gone=["machine:osprey"])
    expect(row, "the gone machine was still drawn").to_have_count(0, timeout=PACE_MS)


def test_a_system_daemon_that_stops_is_said_broken_then_live(
    system_page: Page, system_daemon: StandIn
) -> None:
    says_it_stands(system_page, "live")
    system_daemon.leave()
    says_it_stands(system_page, "broken")
    system_daemon.start()
    says_it_stands(system_page, "live", RETRY_MS)
    counted(system_page)


def test_a_sites_server_that_goes_is_said_stale(system_page: Page, system_served: Served) -> None:
    says_it_stands(system_page, "live")
    _, server = system_served
    server.kill()
    says_it_stands(system_page, "stale")


def test_the_system_page_keeps_to_its_budget(browser: Browser, system_served: Served) -> None:
    root, _ = system_served
    opened = Opened(browser, f"{root}system/", SIZES[-1], "dark")
    try:
        says_it_stands(opened.page, "live")
        counted(opened.page)
        opened.keeps_to(BUDGET)
        says_it_stands(opened.page, "live")
    finally:
        opened.close()


@pytest.mark.parametrize("look", LOOKS)
@pytest.mark.parametrize("size", SIZES)
def test_the_system_page_passes_the_skills_page_checks(
    browser: Browser, system_served: Served, size: tuple[int, int], look: str
) -> None:
    root, _ = system_served
    opened = Opened(browser, f"{root}system/", size, look)
    says_it_stands(opened.page, "live")
    counted(opened.page)
    opened.passes_the_skills_checks()
    opened.close()

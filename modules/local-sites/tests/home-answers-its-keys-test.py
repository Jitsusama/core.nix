"""The home of the local sites, as its builder writes it, answers its keys in a browser.

The home is the one home.py builds from a record of three sites and the
theme's web kit, served by the real local-sites server with the kit's
scripts mounted where every site reads them. In headless Chromium j and k,
and the arrows, step its cursor from site to site and hold it at the ends,
g twice and G go to the first and the last, Return opens the site the
cursor is on, ? and F1 open its list of keys and Escape closes it, the
arrows on the theme switch choose a look and leave the cursor where it was,
and the look chosen holds through a reload. It passes the web-design
skill's page checks at a phone's width and a desk's in both looks, and
anything it logs fails it. The server, the browser and the tab are
modules/local-sites/tests/live_page.py's.
"""

import json
import os
import shutil
import subprocess
import sys
from pathlib import Path
from typing import TYPE_CHECKING

import pytest
from live_page import LOOKS, SIZES, Opened, serving
from playwright.sync_api import Browser, Page, expect

if TYPE_CHECKING:
    from collections.abc import Iterator

# Three sites, one in each of the home's beds and in the order they show, as a
# session records them; each is a page of its own the cursor can open.
SITES = [
    {"path": "tern/", "name": "Tern", "what": "Why it's so.", "kind": "explainer"},
    {"path": "kestrel/", "name": "Kestrel", "what": "A first look.", "kind": "mock"},
    {"path": "osprey/", "name": "Osprey", "what": "How it went.", "kind": "record"},
]
NAMES = [site["name"] for site in SITES]
# The name of the site whose link says it's the cursor.
CURSOR = '.sites a[aria-current="true"] .name'


@pytest.fixture
def home(tmp_path: Path) -> Iterator[str]:
    """Serve the home as home.py builds it among the documents, giving its address."""
    with serving(tmp_path, {}, {}) as (address, _):
        documents = tmp_path / "documents"
        # The theme's kit sits in the sites' folder, since the server follows
        # no link out of it.
        shutil.copytree(os.environ["WEB_KIT"], documents / "kit")
        record = {"about": "a stand-in", "sites": SITES}
        (documents / "sites.json").write_text(json.dumps(record))
        for site in SITES:
            (documents / site["path"]).mkdir()
            (documents / site["path"] / "index.html").write_text(
                f"<!doctype html>\n<title>{site['name']}</title>\n"
            )
        sources = Path(os.environ["HOME_SOURCES"])
        subprocess.run([sys.executable, sources / "home.py", sources, documents], check=True)
        yield address


@pytest.fixture
def tab(browser: Browser, home: str) -> Iterator[Page]:
    """Open a desk-sized tab on the home in the dark look, failing if the page logs."""
    opened = Opened(browser, home, SIZES[-1], "dark")
    yield opened.page
    opened.close()


def pressing(page: Page, keys: str, then: str) -> None:
    """Press each of `keys` in turn, after which the cursor is on the site named `then`."""
    for key in keys.split():
        page.keyboard.press(key)
    expect(page.locator(CURSOR), f"{keys} left the cursor elsewhere").to_have_text(then)


def test_the_home_steps_from_site_to_site_and_holds_at_the_ends(tab: Page) -> None:
    """j, k and the arrows step the cursor; neither end is passed."""
    expect(tab.locator(CURSOR), "the cursor didn't start on the newest").to_have_text(NAMES[0])
    pressing(tab, "j", NAMES[1])
    pressing(tab, "j j", NAMES[2])
    pressing(tab, "k", NAMES[1])
    pressing(tab, "ArrowDown", NAMES[2])
    pressing(tab, "ArrowUp k k", NAMES[0])


def test_g_twice_and_capital_g_go_to_the_first_and_the_last(tab: Page) -> None:
    """G goes to the last site, and g pressed twice back to the first."""
    pressing(tab, "G", NAMES[-1])
    pressing(tab, "g g", NAMES[0])


def test_return_opens_the_site_the_cursor_is_on(tab: Page, home: str) -> None:
    """Return on the cursor goes to its site."""
    pressing(tab, "j", NAMES[1])
    tab.keyboard.press("Enter")
    expect(tab, "Return didn't open the site under the cursor").to_have_url(f"{home}kestrel/")


def test_question_mark_and_f1_open_the_keys_and_escape_closes_them(tab: Page) -> None:
    """? and F1 each open the list of keys, Escape closes it."""
    keys = tab.locator("#keys")
    for key in ["?", "F1"]:
        tab.keyboard.press(key)
        expect(keys, f"{key} didn't open the list of keys").to_be_visible()
        tab.keyboard.press("Escape")
        expect(keys, f"Escape didn't close the list {key} opened").to_be_hidden()


def test_the_arrows_on_the_switch_choose_a_look_and_leave_the_cursor(tab: Page) -> None:
    """On the theme switch the arrows are its own, a look kept through a reload."""
    pressing(tab, "j", NAMES[1])
    # Down is the cursor's key too, so only the switch's taking it keeps the cursor.
    tab.locator('.switch input[value="system"]').focus()
    tab.keyboard.press("ArrowDown")
    root = tab.locator("html")
    expect(root, "the arrow didn't choose the dark look").to_have_attribute("data-theme", "dark")
    expect(tab.locator(CURSOR), "the arrow on the switch moved the cursor").to_have_text(NAMES[1])
    pressing(tab, "j", NAMES[2])
    tab.reload()
    expect(root, "the look chosen didn't hold through a reload").to_have_attribute(
        "data-theme", "dark"
    )


@pytest.mark.parametrize("look", LOOKS)
@pytest.mark.parametrize("size", SIZES, ids=[f"{width}px" for width, _ in SIZES])
def test_the_home_passes_the_skills_page_checks(
    browser: Browser, home: str, size: tuple[int, int], look: str
) -> None:
    """The web-design skill's checks find nothing on the home at either width in either look."""
    opened = Opened(browser, home, size, look)
    expect(opened.page.locator(CURSOR), "the home drew no sites").to_have_text(NAMES[0])
    opened.passes_the_skills_checks()
    opened.close()

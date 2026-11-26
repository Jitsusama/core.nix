"""The home of the local sites, as its builder writes it, answers its keys in a browser.

The home is the one home.py builds from a record of three sites and the theme's
web kit, served by the real local-sites server with the kit's scripts and the
web kit mounted where every site reads them, so a theme or font it can't reach
there is logged. In headless Chromium j and k, and the arrows, step its cursor
from site to site and hold it at the ends, g twice (and not once) and G go to
the first and the last, Return opens the site the cursor is on, ? and F1 open
its list of keys and Escape closes it, the keys its head shows answer a tap,
the arrows on the theme switch choose a look and leave the cursor where it was,
and the look chosen holds on the next visit. Its mark is drawn in each look's
own tint from the kit. It passes the web-design skill's page checks at a
phone's width and a desk's in both looks, and anything it logs fails it. On a
laptop's short screen its first site stands whole in the first screen, and a
tall one draws its crest whole. A document whose head is a hero rather than a
head answers a tap on its shown keys too. The server, the browser and the tab
are modules/local-sites/tests/live_page.py's.
"""

import json
import os
import re
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
# A document on the kit's script whose header is a hero, as one-kit's is. The
# build sandbox has no font a key's letter can be drawn in, and the kit's own
# faces don't load there, so each key is given a box of its own to be tapped.
DOCUMENT = """<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>Heron</title>
<link rel="icon" href="data:,">
<link rel="stylesheet" href="../kit/theme.css">
<link rel="stylesheet" href="../kit/base.css">
<link rel="stylesheet" href="/local-sites/kit/switch.css">
<style>.keys kbd { display: inline-block; min-inline-size: 1em; min-block-size: 1em; }</style>
<script type="module" src="/local-sites/kit/document.js"></script>
</head>
<body>
<header class="hero"><p class="keys"><kbd>j</kbd> and <kbd>k</kbd> step</p></header>
<main>
<section class="part"><h2>First</h2></section>
<section class="part"><h2>Second</h2></section>
</main>
</body>
</html>
"""
# The name of the site whose link says it's the cursor.
CURSOR = '.sites a[aria-current="true"] .name'
# Longer than the kit's 600 ms for a second g to count as the pair.
PAST_TWICE_MS = 1000
# Where the kit's theme.css starts its light look, and the mark its dark one.
LIGHT = "@media (prefers-color-scheme: light)"
DARK_STROKE = re.compile(r"dark\) \{ g \{ stroke: ([^;]+); \}")
LIGHT_STROKE = re.compile(r"^    g \{ stroke: ([^;]+); \}", re.MULTILINE)
# A laptop's short screens, on which the first site must stand whole in the
# first screen; one tall enough for the crest at its 13rem, as home.css draws
# it from about 70rem tall; and one between, past the 60rem the threshold
# tightens under, whose crest still follows its height: 30svh less 8rem.
# Each crest is said as pixels of the height and a number of rems, since an
# interface's rem is its body's size, which grows with the width from 15 px
# to 16 px (theme/web/web.nix).
LAPTOPS = [(1280, 800), (1024, 768)]
TALL = (1280, 1200)
FULL_CREST = (0, 13)
BETWEEN = (1280, 1000)
BETWEEN_CREST = (300, -8)
# A phone, narrow and short, whose crest goes by its width alone, at 9rem: a
# short screen's shrinking is the wide one's, so the crest doesn't fall to its
# least on a phone or change size as its address bar slides.
PHONE = (375, 667)
PHONE_CREST = (0, 9)
# The page's rem, in pixels.
REM = "() => parseFloat(getComputedStyle(document.documentElement).fontSize)"


@pytest.fixture
def home(tmp_path: Path) -> Iterator[str]:
    """Serve the home as home.py builds it among the documents, giving its address."""
    with serving(tmp_path, {}, {}) as (address, _):
        documents = tmp_path / "documents"
        # The documents keep a kit of their own, as a document must stand
        # alone; the home reads none of it, but the copy the sites share.
        shutil.copytree(os.environ["WEB_KIT"], documents / "kit")
        record = {"about": "a stand-in", "sites": SITES}
        (documents / "sites.json").write_text(json.dumps(record))
        for site in SITES:
            (documents / site["path"]).mkdir()
            (documents / site["path"] / "index.html").write_text(
                f"<!doctype html>\n<title>{site['name']}</title>\n"
            )
        (documents / "heron").mkdir()
        (documents / "heron" / "index.html").write_text(DOCUMENT)
        sources = Path(os.environ["HOME_SOURCES"])
        theme = Path(os.environ["WEB_KIT"]) / "theme.css"
        subprocess.run([sys.executable, sources / "home.py", sources, documents, theme], check=True)
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
    expect(tab.locator(CURSOR), "the cursor didn't start on the first").to_have_text(NAMES[0])
    pressing(tab, "j", NAMES[1])
    pressing(tab, "j j", NAMES[2])
    pressing(tab, "k", NAMES[1])
    pressing(tab, "ArrowDown", NAMES[2])
    pressing(tab, "ArrowUp k k", NAMES[0])


def test_g_twice_and_capital_g_go_to_the_first_and_the_last(tab: Page) -> None:
    """G goes to the last site, and g pressed twice back to the first."""
    pressing(tab, "G", NAMES[-1])
    tab.keyboard.press("g")
    # Past the pair's window, so a lone g has had its chance to move the cursor.
    tab.wait_for_timeout(PAST_TWICE_MS)
    expect(tab.locator(CURSOR), "a single g moved the cursor").to_have_text(NAMES[-1])
    pressing(tab, "g g", NAMES[0])


def test_the_keys_the_head_shows_answer_a_tap(tab: Page) -> None:
    """A tap on the head's j steps the cursor as the key does."""
    tab.locator(".page-head .keys kbd", has_text="j").click()
    expect(tab.locator(CURSOR), "a tap on j left the cursor").to_have_text(NAMES[1])


def test_a_documents_keys_answer_a_tap_under_a_hero(browser: Browser, home: str) -> None:
    """A tap on j in a document's hero steps to a part, whose heading takes focus."""
    opened = Opened(browser, f"{home}heron/", SIZES[-1], "dark")
    try:
        opened.page.locator("header .keys kbd", has_text="j").click()
        reached = opened.page.locator(".part h2:focus")
        expect(reached, "a tap on j under a hero didn't step").to_have_count(1)
    finally:
        opened.close()


def test_the_mark_is_drawn_in_each_looks_tint(home: str, tmp_path: Path) -> None:
    """The real kit names a tint for each look, and each look's mark takes its own."""
    assert home, "the home wasn't served"
    mark = (tmp_path / "documents" / "favicon.svg").read_text()
    dark, light = DARK_STROKE.search(mark), LIGHT_STROKE.search(mark)
    assert dark, f"the mark strokes no dark look: {mark}"
    assert light, f"the mark strokes no light look: {mark}"
    theme = (Path(os.environ["WEB_KIT"]) / "theme.css").read_text()
    dark_side, _, light_side = theme.partition(LIGHT)
    assert f"--action: {dark[1]};" in dark_side, f"the dark mark isn't the kit's: {dark[1]}"
    assert f"--action: {light[1]};" in light_side, f"the light mark isn't the kit's: {light[1]}"


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
    """On the theme switch the arrows are its own, a look kept to the next visit."""
    pressing(tab, "j", NAMES[1])
    # Down is the cursor's key too, so only the switch's taking it keeps the cursor.
    tab.locator('.switch input[value="system"]').focus()
    tab.keyboard.press("ArrowDown")
    root = tab.locator("html")
    expect(root, "the arrow didn't choose the dark look").to_have_attribute("data-theme", "dark")
    expect(tab.locator(CURSOR), "the arrow on the switch moved the cursor").to_have_text(NAMES[1])
    pressing(tab, "j", NAMES[2])
    # Visited afresh rather than reloaded, since a reload lets Chromium put the
    # radios back itself.
    tab.goto(tab.url)
    expect(root, "the look chosen didn't hold to the next visit").to_have_attribute(
        "data-theme", "dark"
    )
    dark = tab.locator('.switch input[value="dark"]')
    expect(dark, "the switch forgot its look by the next visit").to_be_checked()


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


@pytest.mark.parametrize("size", LAPTOPS, ids=[f"{width}x{height}" for width, height in LAPTOPS])
def test_a_laptops_first_screen_shows_a_site_whole(
    browser: Browser, home: str, size: tuple[int, int]
) -> None:
    """On a laptop's short screen the threshold gives way, and the first site stands whole."""
    opened = Opened(browser, home, size, "dark")
    try:
        first = opened.page.locator(".bed .sites a").first
        expect(first, "the home drew no sites").to_be_visible()
        box = first.bounding_box()
        assert box, "the first site was drawn with no box"
        ends, (_, height) = box["y"] + box["height"], size
        assert ends <= height, f"the first site ends at {ends:.0f} px, past the {height} px screen"
    finally:
        opened.close()


@pytest.mark.parametrize(
    ("size", "crest"),
    [(TALL, FULL_CREST), (BETWEEN, BETWEEN_CREST), (PHONE, PHONE_CREST)],
    ids=["tall", "between", "phone"],
)
def test_a_screens_crest_follows_its_height_when_wide_and_its_width_on_a_phone(
    browser: Browser, home: str, size: tuple[int, int], crest: tuple[int, int]
) -> None:
    """A tall screen draws its crest whole, a shorter one smaller, and a phone at its width's."""
    opened = Opened(browser, home, size, "dark")
    try:
        box = opened.page.locator(".crest").bounding_box()
        assert box, "the crest was drawn with no box"
        pixels, rems = crest
        wanted = pixels + rems * opened.page.evaluate(REM)
        # Within a pixel, since the browser lays boxes out in 1/64 px steps.
        drawn = box["width"]
        assert abs(drawn - wanted) < 1, f"a {size} screen drew the crest {drawn} px, not {wanted}"
    finally:
        opened.close()


# What the threshold's own gaps measure: the crest's below it and the
# contents' above them, which a short wide screen takes to nothing.
GAPS = """() => [".crest", ".contents"].map((part) => {
    const style = getComputedStyle(document.querySelector(part));
    return part === ".crest" ? style.marginBlockEnd : style.marginBlockStart;
})"""


# The threshold's gaps once a short wide screen has taken them to nothing.
SHUT = ["0px", "0px"]


@pytest.mark.parametrize(
    ("size", "threshold"),
    [(LAPTOPS[0], "shut"), (BETWEEN, "open")],
    ids=["laptop", "between"],
)
def test_only_a_screen_under_60rem_tall_tightens_the_threshold(
    browser: Browser, home: str, size: tuple[int, int], threshold: str
) -> None:
    """The threshold's gaps go only on a wide screen shorter than 60rem."""
    opened = Opened(browser, home, size, "dark")
    try:
        gaps = opened.page.evaluate(GAPS)
        shut = gaps == SHUT
        assert shut == (threshold == "shut"), f"a {size} screen's gaps were {gaps}"
    finally:
        opened.close()

"""The web-design skill's gate (skill/gate.js), held to where it finds a site's mark, in Chromium.

The gate takes a page's mark from the first <header> or the one element
just before it, since a page on the live sites' head draws its mark there
(sites.md). Its first cases are the misses that bounded that: a skip link
just before a head with no mark, or further up, carrying an icon the gate
took for the mark, and the theme switch's icons, which are no mark. Each
case runs the gate whole, as a live page's test does (live_page.py's
PAGE_CHECKS), and reads only whether it names the mark as missing, since
the small pages here lack much else it asks for. The gate's path comes
from WEB_SKILL, as the check running this sets it, and its browser from
probe_browser.py, loaded as a plugin.
"""

import os
from pathlib import Path
from typing import TYPE_CHECKING

import pytest

if TYPE_CHECKING:
    from collections.abc import Iterator

    from playwright.sync_api import Browser, Page

GATE = (Path(os.environ["WEB_SKILL"]) / "gate.js").read_text()
ORIGIN = "http://gate.test/"
# What the gate says of a page whose masthead draws no mark.
UNMARKED = "the site's mark, drawn as inline SVG in the first <header> or just before it"
MARK = '<svg viewBox="0 0 16 16" aria-hidden="true"><circle cx="8" cy="8" r="6"/></svg>'
SKIP = f'<a class="skip" href="#main">{MARK}Skip to the page</a>'
SWITCH = f'<div role="radiogroup" aria-label="Theme">{MARK}</div>'
HEAD = '<header class="page-head"><h1>A page</h1></header>'
MAIN = '<main id="main"><p>Its words.</p></main>'


@pytest.fixture
def tab(browser: Browser) -> Iterator[Page]:
    """Open a fresh tab at a desk's width."""
    opened = browser.new_page(viewport={"width": 1024, "height": 640})
    yield opened
    opened.close()


def unmarked(tab: Page, body: str) -> bool:
    """Open a page holding `body`, and say whether the gate names its mark as missing.

    The page comes by a real navigation, as a live page does, and anything
    else it asks for isn't there, so the gate reads only what's written.
    """
    page = f'<!doctype html><html lang="en"><head><title>A page</title></head><body>{body}</body>'
    tab.route("**/*", lambda route: route.fulfill(status=404, body=""))
    sent = "text/html; charset=utf-8"
    tab.route(ORIGIN, lambda route: route.fulfill(content_type=sent, body=page))
    tab.goto(ORIGIN)
    return UNMARKED in tab.evaluate(GATE)


def test_a_mark_in_the_head_is_found(tab: Page) -> None:
    """A mark drawn inside the first <header> is the page's mark."""
    marked = HEAD.replace("<h1>", f"{MARK}<h1>")
    assert not unmarked(tab, f"{SKIP}{marked}{MAIN}")


def test_a_mark_just_before_the_head_is_found(tab: Page) -> None:
    """A mark just before the head, as the live sites draw it, is the page's mark."""
    assert not unmarked(tab, f'{SKIP}<a class="crest" href="/">{MARK}</a>{HEAD}{MAIN}')


def test_a_skip_links_icon_just_before_the_head_is_no_mark(tab: Page) -> None:
    """A skip link's icon just before a head with no mark of its own is no mark."""
    assert unmarked(tab, f"{SKIP}{HEAD}{MAIN}")


def test_a_skip_links_icon_further_up_is_no_mark(tab: Page) -> None:
    """A skip link's icon above a banner, above the head, is no mark either."""
    assert unmarked(tab, f'{SKIP}<div class="banner">Read only</div>{HEAD}{MAIN}')


def test_the_switchs_icons_are_no_mark(tab: Page) -> None:
    """The theme switch in the head draws icons, which are none of them the mark."""
    switched = HEAD.replace("</header>", f"{SWITCH}</header>")
    assert unmarked(tab, f"{switched}{MAIN}")


def test_a_page_with_no_head_is_unmarked(tab: Page) -> None:
    """A page with no <header> has no masthead to hold a mark."""
    assert unmarked(tab, f"{SKIP}{MARK}{MAIN}")

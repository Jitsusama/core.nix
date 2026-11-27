"""The web-design skill's first probe (skill/gate.js), held to its masthead's mark in Chromium.

A page on the live sites' head draws its mark just before the head rather
than in it, so the probe counts the one element before the first <header>
as its masthead too. Each case opens a small page whose masthead holds a
mark or doesn't, runs the probe on it as a live page's test does
(live_page.py's PAGE_CHECKS), and reads only whether its answer asks for
the mark, since a page this small lacks much else the probe asks for. The
cases pin that rule from both sides: a mark in the head or just before it
passes, and one only further up, or only in the switch's icons, fails. The
probe's path comes from WEB_SKILL, as the check running this sets it, and
its browser from probe_browser.py, loaded as a plugin.
"""

import os
from pathlib import Path
from typing import TYPE_CHECKING

import pytest

if TYPE_CHECKING:
    from collections.abc import Iterator

    from playwright.sync_api import Browser, Page

PROBE = (Path(os.environ["WEB_SKILL"]) / "gate.js").read_text()
ORIGIN = "http://gate.test/"
# What the probe says while the masthead holds no mark of the site's.
NO_MARK = "the site's mark, drawn as inline SVG in the first <header> or just before it"
MARK = '<svg viewBox="0 0 16 16" aria-hidden="true"><circle cx="8" cy="8" r="6"/></svg>'
SWITCH = f'<div role="radiogroup" aria-label="Theme"><button role="radio">{MARK}</button></div>'
SKIP = '<a class="skip" href="#main">Skip to the page</a>'


@pytest.fixture
def tab(browser: Browser) -> Iterator[Page]:
    """Open a fresh tab at a desk's width."""
    opened = browser.new_page(viewport={"width": 1024, "height": 640})
    yield opened
    opened.close()


def probed(tab: Page, body: str) -> list[str]:
    """Open a page whose body is `body`, and give what the probe answers.

    The page comes by a real navigation, as a live page does, and every
    other address it asks for is answered as missing, so none waits on a
    name nothing serves.
    """
    page = f'<!doctype html><html lang="en-CA"><head><title>A page</title></head><body>{body}'
    sent = "text/html; charset=utf-8"
    tab.route("**/*", lambda route: route.fulfill(status=404, body=""))
    tab.route(ORIGIN, lambda route: route.fulfill(content_type=sent, body=page))
    tab.goto(ORIGIN)
    return list(tab.evaluate(PROBE))


def masthead(before: str, inside: str) -> str:
    """Give a body with `before` ahead of its first header, which holds `inside` and the switch."""
    return f'{before}<header><h1>A page</h1>{inside}{SWITCH}</header><main id="main"></main>'


def test_a_mark_in_the_head_passes(tab: Page) -> None:
    assert NO_MARK not in probed(tab, masthead(SKIP, MARK))


def test_a_mark_drawn_just_before_the_head_passes(tab: Page) -> None:
    """The live sites' head: the skip link, then the mark, then the head of parts alone."""
    assert NO_MARK not in probed(tab, masthead(SKIP + MARK, ""))


def test_a_mark_inside_the_element_just_before_the_head_passes(tab: Page) -> None:
    assert NO_MARK not in probed(tab, masthead(f'{SKIP}<a href="/">{MARK}</a>', ""))


def test_an_element_just_before_the_head_with_no_mark_fails(tab: Page) -> None:
    """Only the one element before the head counts, so a mark further up stands in for none.

    Here the element just before the head is a skip link, and the drawing
    sits ahead of it, as a banner's would.
    """
    assert NO_MARK in probed(tab, masthead(MARK + SKIP, ""))


def test_the_switchs_icons_are_no_mark(tab: Page) -> None:
    """A head whose only drawings are the theme switch's icons has no mark of the site's."""
    assert NO_MARK in probed(tab, masthead(SKIP, ""))


def test_a_page_with_no_head_has_no_mark(tab: Page) -> None:
    assert NO_MARK in probed(tab, f'{MARK}<main id="main"><h1>A page</h1></main>')

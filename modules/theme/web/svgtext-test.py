"""The web-design skill's drawing words probe (skill/svgtext.js), held to its cases in Chromium.

Each case sets a small page holding an inline drawing and runs the probe on
it as a live page's test does (live_page.py's PAGE_CHECKS), reading the
lines it answers. The first case is the miss that made the probe: on 15
November 2026 the forage research page's diagram drew light words on a
block whose fill named a colour the kit doesn't define, so it painted its
light fallback in the dark look, at 1.45:1 there (1.16:1 in the case's own
colours), and the gate passed it, since it measured no words inside a
drawing. The probe's bar of 7:1 and its edge of 64 px for a drawing kept in
an image are each pinned from both sides. The probe's path comes from
WEB_SKILL, as the check running this sets it, and its browser and tab from
probe_browser.py, loaded as a plugin.
"""

import os
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from playwright.sync_api import Page

PROBE = (Path(os.environ["WEB_SKILL"]) / "svgtext.js").read_text()
ORIGIN = "http://svgtext.test/"
# The dark look's ground and its words, near the kit's own.
DARK = "#121a16"
LIGHT_WORDS = "#e6e1cf"
# Greys either side of 7:1 on white: 7.00:1 and 6.90:1.
AT_THE_BAR, UNDER_THE_BAR = "#595959", "#5a5a5a"
# A drawing an image loads, so a case's images are laid out as loaded ones are.
DRAWING = (
    '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 4 1"><rect width="4" height="1"/></svg>'
)


def probed(tab: Page, drawing: str, style: str = "") -> list[str]:
    """Open `drawing` in a page styled by `style`, and give what the probe answers.

    The page comes by a real navigation, as a live page does, so the probe
    reads it as published.
    """
    page = (
        f"<!doctype html><head><style>body {{ margin: 0; }} {style}</style></head>"
        f"<body><main>{drawing}</main></body>"
    )
    sent = "text/html; charset=utf-8"
    tab.route(ORIGIN, lambda route: route.fulfill(content_type=sent, body=page))
    drawn = "image/svg+xml"
    tab.route("**/*.svg", lambda route: route.fulfill(content_type=drawn, body=DRAWING))
    tab.goto(ORIGIN)
    return list(tab.evaluate(PROBE))


def drawn(inside: str) -> str:
    """Give an inline drawing 320 by 80 holding `inside`."""
    return f'<svg width="320" height="80" viewBox="0 0 320 80">{inside}</svg>'


def test_a_shape_falling_back_to_paper_under_light_words_is_named(tab: Page) -> None:
    """Light words on a block whose fill names no colour the kit defines paint its fallback.

    The research page's block of 15 November 2026: its fill fell back to
    paper in the dark look, under the dark look's light words.
    """
    drawing = drawn(
        '<rect width="320" height="80" style="fill: var(--paper, #f6f1e4)"/>'
        f'<text x="16" y="48" style="fill: {LIGHT_WORDS}; font-size: 16px">the floor cut</text>'
    )
    found = probed(tab, drawing, f"body {{ background: {DARK}; }}")
    assert len(found) == 1, found
    assert found[0].startswith('the drawn words "the floor cut" stand at 1.'), found
    assert "on rgb(246, 241, 228), under 7:1" in found[0], found


def test_light_words_on_a_shape_the_kit_defines_pass(tab: Page) -> None:
    """The same words on a block filled with the page's own ground answer nothing."""
    drawing = drawn(
        '<rect width="320" height="80" style="fill: var(--ground)"/>'
        f'<text x="16" y="48" style="fill: {LIGHT_WORDS}; font-size: 16px">the floor cut</text>'
    )
    style = f":root {{ --ground: {DARK}; }} body {{ background: #f6f1e4; }}"
    assert probed(tab, drawing, style) == []


def test_faint_words_on_the_page_alone_are_named(tab: Page) -> None:
    """Grey words on a white page, with no shape behind them, are measured on the page."""
    drawing = drawn('<text x="16" y="48" style="fill: #999; font-size: 16px">C12</text>')
    found = probed(tab, drawing)
    assert len(found) == 1, found
    assert found[0].startswith('the drawn words "C12" stand at 2.'), found
    assert "on rgb(255, 255, 255)" in found[0], found


def test_dark_words_faded_by_their_group_are_named(tab: Page) -> None:
    """Words dark enough alone fall under 7:1 once the group they're in fades them."""
    drawing = drawn(
        '<g style="opacity: 0.3"><text x="16" y="48" style="fill: #111; font-size: 16px">'
        "C12</text></g>"
    )
    found = probed(tab, drawing)
    assert len(found) == 1, found
    assert 'the drawn words "C12"' in found[0], found


def test_strong_words_pass(tab: Page) -> None:
    """Near-black words on a white page answer nothing."""
    drawing = drawn('<text x="16" y="48" style="fill: #111; font-size: 16px">C12</text>')
    assert probed(tab, drawing) == []


def test_words_at_the_bar_pass_and_words_just_under_it_are_named(tab: Page) -> None:
    """Grey at 7.00:1 on white answers nothing; grey at 6.90:1 is named."""
    at = drawn(f'<text x="16" y="48" style="fill: {AT_THE_BAR}; font-size: 16px">C12</text>')
    assert probed(tab, at) == []
    under = drawn(f'<text x="16" y="48" style="fill: {UNDER_THE_BAR}; font-size: 16px">C12</text>')
    found = probed(tab, under)
    assert len(found) == 1, found
    assert found[0].startswith('the drawn words "C12" stand at 6.90:1'), found


def test_a_drawing_kept_in_an_image_is_named_and_an_icon_is_not(tab: Page) -> None:
    """A drawing 65 px wide in an img can't be measured, so it's named; one 64 px wide isn't."""
    images = '<img src="floor.svg" width="65" height="16" alt="">'
    images += '<img src="cap.svg" width="64" height="16" alt="">'
    found = probed(tab, images)
    assert len(found) == 1, found
    assert found[0].startswith("the drawing floor.svg sits in an img"), found

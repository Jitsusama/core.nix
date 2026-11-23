"""The web-design skill's pixels probe (skill/pixels.js), held to its cases in headless Chromium.

Each case sets a small page in a phone's 390 px window and runs the probe
on it as a live page's test does, reading the lines it answers: a page
scrolling sideways, a shift as a part arrives, text under the floor, and a
control named apart from its words. The cases come from the live sites as
1HE06W measured them on 22 November 2026: forage 601 px wide on a phone and
shifting 0.81, its labels under 10 px, and the Network's buttons named
"Find a cell" while showing "/". The probe's path comes from WEB_SKILL, as
the check running this sets it, and its browser from probe_browser.py,
loaded as a plugin.
"""

import os
from pathlib import Path
from typing import TYPE_CHECKING

import pytest

if TYPE_CHECKING:
    from collections.abc import Iterator

    from playwright.sync_api import Browser, Page, ViewportSize

PROBE = (Path(os.environ["WEB_SKILL"]) / "pixels.js").read_text()
PHONE: ViewportSize = {"width": 390, "height": 844}
ORIGIN = "http://pixels.test/"
# The kit's own text-for-the-ear rule, so a case can hide words as a page does.
STYLE = """
body { margin: 0; font: 16px serif; }
.sr { position: absolute; width: 1px; height: 1px; overflow: hidden;
      clip-path: inset(50%); white-space: nowrap; }
"""


@pytest.fixture
def tab(browser: Browser) -> Iterator[Page]:
    """Open a fresh tab at a phone's width."""
    opened = browser.new_page(viewport=PHONE)
    yield opened
    opened.close()


def probed(tab: Page, body: str, settle_ms: int = 0) -> list[str]:
    """Open `body` as a page, let it settle `settle_ms`, and give what the probe answers.

    The page comes by a real navigation, not `set_content`, whose written
    document keeps no layout shifts in its timeline as a loaded page does.
    """
    page = f"<!doctype html><style>{STYLE}</style><body>{body}</body>"
    tab.route(ORIGIN, lambda route: route.fulfill(content_type="text/html", body=page))
    tab.goto(ORIGIN)
    if settle_ms:
        tab.wait_for_timeout(settle_ms)
    found = tab.evaluate(PROBE)
    assert isinstance(found, list)
    return [str(line) for line in found]


def test_a_page_that_fits_passes(tab: Page) -> None:
    """A page of words that wrap in its window answers nothing."""
    assert probed(tab, "<main><h1>Forage</h1><p>Each cell, and what it works on.</p></main>") == []


def test_a_page_wider_than_a_phone_names_what_reaches_past(tab: Page) -> None:
    """A 601 px drawing in a 390 px window says the page's width and what reaches past."""
    found = probed(tab, '<main><div style="width: 601px; height: 20px"></div></main>')
    assert len(found) == 1
    assert "390 px window" in found[0]
    assert "div reaches 601 px" in found[0]


def test_a_wide_part_in_its_own_scroller_passes(tab: Page) -> None:
    """Code wider than the window, held in a scroller of its own, doesn't widen the page."""
    body = '<main><pre style="overflow-x: auto"><code>{}</code></pre></main>'.format("x" * 200)
    assert probed(tab, body) == []


def test_text_under_the_floor_is_named(tab: Page) -> None:
    """A count at 11.76 px, forage's --step-small on a phone, is under the 12 px floor."""
    found = probed(tab, '<main><p style="font-size: 11.76px">12 cells</p></main>')
    assert len(found) == 1
    assert found[0].startswith('text at 11.8 px ("12 cells"), under 12 px')


def test_text_at_the_floor_passes(tab: Page) -> None:
    """A count at exactly 12 px holds the floor."""
    assert probed(tab, '<main><p style="font-size: 12px">12 cells</p></main>') == []


def test_words_for_the_ear_alone_are_no_reading_size(tab: Page) -> None:
    """Text the kit clips to a pixel for a screen reader isn't held to the floor."""
    assert (
        probed(tab, '<main><p>Cells</p><span class="sr" style="font-size: 4px">so</span></main>')
        == []
    )


def drawing(label: str) -> str:
    """Draw `label`, a text element, in a drawing at its own size."""
    return f'<main><svg width="200" height="60" viewBox="0 0 200 60">{label}</svg></main>'


def test_a_drawings_unmarked_small_label_is_held_to_the_floor(tab: Page) -> None:
    """A cell's 10 px label, unmarked, is under 12 px like any other text."""
    found = probed(tab, drawing('<text x="4" y="30" font-size="10">1GB1PI</text>'))
    assert len(found) == 1
    assert found[0].startswith('text at 10.0 px ("1GB1PI"), under 12 px')


def test_a_drawings_label_that_enlarges_may_be_small(tab: Page) -> None:
    """A 10 px label marked data-enlarges passes, since zoom or focus shows it larger."""
    label = '<g data-enlarges><text x="4" y="30" font-size="10">reef</text></g>'
    assert probed(tab, drawing(label)) == []


def test_nothing_is_under_nine_px_even_where_it_enlarges(tab: Page) -> None:
    """A label at 8 px is too small to read even marked, so it's named."""
    label = '<text data-enlarges x="4" y="30" font-size="8">reef</text>'
    found = probed(tab, drawing(label))
    assert len(found) == 1
    assert found[0].startswith('a drawing\'s label at 8.0 px ("reef"), under 9 px')


def test_a_drawings_scale_sets_its_labels_size(tab: Page) -> None:
    """A 20 px label in a drawing shown at half its size reads at 10 px, under the floor."""
    body = (
        '<main><svg width="100" height="30" viewBox="0 0 200 60">'
        '<text x="4" y="30" font-size="20">reef</text></svg></main>'
    )
    found = probed(tab, body)
    assert len(found) == 1
    assert found[0].startswith('text at 10.0 px ("reef")')


SHIFTING = """
<main><div id="above"></div><p>Each cell, and what it works on.</p></main>
<script>
  const grow = () => {{ document.getElementById("above").style.height = "{height}"; }};
  requestAnimationFrame(() => requestAnimationFrame(() => setTimeout(grow, 100)));
</script>
"""


def test_a_part_arriving_above_the_words_is_a_shift(tab: Page) -> None:
    """A 600 px part filled in above the words after they're drawn shifts the page past 0.1."""
    found = probed(tab, SHIFTING.format(height="600px"), 400)
    assert len(found) == 1
    assert found[0].startswith("the page shifts ")
    assert "over 0.1" in found[0]


def test_a_part_whose_box_is_kept_doesnt_shift(tab: Page) -> None:
    """The same part with its box reserved before it's filled moves nothing."""
    body = SHIFTING.format(height="600px").replace(
        '<div id="above">', '<div id="above" style="height: 600px">'
    )
    assert probed(tab, body, 400) == []


def test_a_name_apart_from_what_shows_is_named(tab: Page) -> None:
    """A button showing "Find" but named "Search cells" can't be said by what it shows."""
    found = probed(tab, '<main><button aria-label="Search cells">Find</button></main>')
    assert found == [
        (
            'a control showing "Find" named "Search cells": '
            "start its name with the words it shows, or drop the aria-label"
        )
    ]


def test_a_name_starting_with_what_shows_passes(tab: Page) -> None:
    """A button showing "Find" and its keycap, named "Find a cell", passes."""
    body = '<main><button aria-label="Find a cell">Find <kbd>/</kbd></button></main>'
    assert probed(tab, body) == []


def test_a_mark_alone_shows_a_word_too(tab: Page) -> None:
    """A keycap button showing "/" and named "Find a cell", as the Network's were, is named."""
    found = probed(
        tab, '<main><kbd role="button" tabindex="0" aria-label="Find a cell">/</kbd></main>'
    )
    assert found == [
        (
            'a control showing only "/", named "Find a cell": '
            "show its word beside the mark, its name starting with that word"
        )
    ]


def test_a_keycap_showing_its_key_passes(tab: Page) -> None:
    """A tappable keycap showing j, named for its key first, passes."""
    body = '<main><kbd role="button" tabindex="0" aria-label="j, next site">j</kbd></main>'
    assert probed(tab, body) == []


def test_a_picture_named_by_its_label_passes(tab: Page) -> None:
    """A theme switch drawn only as a picture shows no words, so its label names it."""
    body = (
        '<main><button aria-label="Dark"><svg width="24" height="24" aria-hidden="true">'
        '<circle cx="12" cy="12" r="6"/></svg></button></main>'
    )
    assert probed(tab, body) == []

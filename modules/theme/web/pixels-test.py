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
    assert isinstance(found, list), f"the probe answered {found!r}, not a list of lines"
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


def test_a_wide_part_in_its_own_scroller_is_not_what_it_names(tab: Page) -> None:
    """A 900 px part in its own scroller is passed over for the 601 px one widening the page."""
    body = (
        '<main><div style="overflow-x: auto"><div style="width: 900px; height: 20px"></div>'
        '</div><div style="width: 601px; height: 20px"></div></main>'
    )
    found = probed(tab, body)
    assert len(found) == 1
    assert "div reaches 601 px" in found[0], found


def test_text_under_the_floor_is_named(tab: Page) -> None:
    """A count at 11.76 px, forage's --step-small on a phone, is under the 12 px floor."""
    found = probed(tab, '<main><p style="font-size: 11.76px">12 cells</p></main>')
    assert len(found) == 1
    assert found[0].startswith('text at 11.8 px ("12 cells"), under 12 px'), found


def test_text_at_the_floor_passes(tab: Page) -> None:
    """A count at exactly 12 px holds the floor."""
    assert probed(tab, '<main><p style="font-size: 12px">12 cells</p></main>') == []


def test_a_small_box_at_the_floor_passes(tab: Page) -> None:
    """A 12 px keycap in a box 10.6 px wide holds the floor, however its width rounds."""
    body = (
        '<main><p><kbd style="display: inline-block; width: 10.6px; font-size: 12px">j</kbd>'
        "</p></main>"
    )
    assert probed(tab, body) == []


def test_text_scaled_down_is_measured_scaled(tab: Page) -> None:
    """Words set at 20 px in a box scaled to half read at 10 px, under the floor."""
    body = (
        '<main><div style="transform: scale(0.5)"><p style="font-size: 20px">Reef</p></div></main>'
    )
    found = probed(tab, body)
    assert len(found) == 1
    assert found[0].startswith('text at 10.0 px ("Reef")'), found


def test_every_small_text_is_counted(tab: Page) -> None:
    """The smallest text is named and the rest counted, so a page sees how much is left."""
    body = (
        '<main><p style="font-size: 10px">Ten</p><p style="font-size: 11px">Eleven</p>'
        '<p style="font-size: 11.5px">More</p></main>'
    )
    found = probed(tab, body)
    assert len(found) == 1
    assert found[0].startswith('text at 10.0 px ("Ten"), under 12 px (and 2 more): '), found


def test_a_fields_words_are_held_to_the_floor(tab: Page) -> None:
    """A search field's hint at 10 px is text like any other."""
    found = probed(tab, '<main><input style="font-size: 10px" placeholder="Find a cell"></main>')
    assert len(found) == 1
    assert found[0].startswith('text at 10.0 px ("Find a cell")'), found


def test_words_a_style_writes_are_held_to_the_floor(tab: Page) -> None:
    """A label written by ::after at 10 px is text like any other."""
    body = (
        "<style>.n::after { content: 'cells'; font-size: 10px; }</style>"
        '<main><p class="n">12 </p></main>'
    )
    found = probed(tab, body)
    assert len(found) == 1
    assert found[0].startswith('text at 10.0 px ("cells")'), found


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
    assert found[0].startswith('text at 10.0 px ("1GB1PI"), under 12 px'), found


def test_a_drawings_label_that_enlarges_may_be_small(tab: Page) -> None:
    """A 10 px label marked data-enlarges passes, since zoom or focus shows it larger."""
    label = '<g data-enlarges><text x="4" y="30" font-size="10">reef</text></g>'
    assert probed(tab, drawing(label)) == []


def test_nothing_is_under_nine_px_even_where_it_enlarges(tab: Page) -> None:
    """A label at 8 px is too small to read even marked, so it's named."""
    label = '<text data-enlarges x="4" y="30" font-size="8">reef</text>'
    found = probed(tab, drawing(label))
    assert len(found) == 1
    assert found[0].startswith('a drawing\'s label at 8.0 px ("reef"), under 9 px'), found


def test_a_drawings_scale_sets_its_labels_size(tab: Page) -> None:
    """A 20 px label in a drawing shown at half its size reads at 10 px, under the floor."""
    body = (
        '<main><svg width="100" height="30" viewBox="0 0 200 60">'
        '<text x="4" y="30" font-size="20">reef</text></svg></main>'
    )
    found = probed(tab, body)
    assert len(found) == 1
    assert found[0].startswith('text at 10.0 px ("reef")'), found


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
    assert found[0].startswith("the page shifts "), found
    assert "over 0.1" in found[0]


# A grey block of known height, so a shift's size is the window's arithmetic
# rather than a font's: moved down by `by` px in a 390 by 844 window, it shifts
# (400 + by) / 844 of the window by by / 844 of its height.
MOVING = """
<main><div id="above"></div><div style="height: 400px; background: #ccc"></div></main>
<script>
  const above = document.getElementById("above");
  const grow = (px) => () => {{ above.style.height = px; }};
  requestAnimationFrame(() => requestAnimationFrame(() => {{
    setTimeout(grow("{first}"), 100);
    setTimeout(grow("{then}"), 1600);
  }}));
</script>
"""


def test_a_shift_is_said_by_its_size(tab: Page) -> None:
    """A 400 px block pushed down 200 px shifts 600 / 844 times 200 / 844, so 0.17."""
    found = probed(tab, MOVING.format(first="200px", then="200px"), 400)
    assert len(found) == 1
    assert found[0].startswith("the page shifts 0.17 as it arrives"), found


def test_shifts_far_apart_are_windows_apart(tab: Page) -> None:
    """Two shifts of 0.07 a second and a half apart are two windows, neither over 0.1."""
    assert probed(tab, MOVING.format(first="100px", then="200px"), 2000) == []


def test_a_shift_after_a_press_is_the_press(tab: Page) -> None:
    """A part opened by a press moves the page as the press asked, so it isn't counted."""
    body = (
        '<main><button id="open">Open</button><div id="above"></div>'
        '<div style="height: 400px; background: #ccc"></div></main><script>'
        'document.getElementById("open").onclick = () => {'
        ' document.getElementById("above").style.height = "300px"; };</script>'
    )
    page = f"<!doctype html><style>{STYLE}</style><body>{body}</body>"
    tab.route(ORIGIN, lambda route: route.fulfill(content_type="text/html", body=page))
    tab.goto(ORIGIN)
    tab.click("#open")
    assert tab.evaluate(PROBE) == []


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


def test_a_name_only_beginning_with_the_same_letters_is_named(tab: Page) -> None:
    """A button showing "Find" named "Finder" can't be said as "Find", so it's named."""
    found = probed(tab, '<main><button aria-label="Finder">Find</button></main>')
    assert len(found) == 1
    assert found[0].startswith('a control showing "Find" named "Finder"'), found


def test_words_a_narrow_screen_hides_arent_shown(tab: Page) -> None:
    """A label hidden at a phone's width isn't among the words the button shows."""
    body = (
        '<main><button aria-label="Find a cell">Find'
        '<span style="display: none"> everything</span></button></main>'
    )
    assert probed(tab, body) == []


def test_an_empty_name_is_no_name(tab: Page) -> None:
    """An empty aria-label leaves the button named by its words, so it passes."""
    assert probed(tab, '<main><button aria-label="">Find</button></main>') == []


def test_every_misnamed_control_is_counted_in_one_line(tab: Page) -> None:
    """Three misnamed buttons give one line naming the first and counting the rest."""
    body = (
        '<main><button aria-label="Search">Find</button>'
        '<button aria-label="Leave">Go</button><button aria-label="Shut">Close</button></main>'
    )
    found = probed(tab, body)
    assert len(found) == 1
    assert found[0].startswith('a control showing "Find" named "Search" (and 2 more): '), found


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


def test_faint_text_is_named_with_its_ratio(tab: Page) -> None:
    """Grey #999 on white is 2.85:1, under 4.5:1 for body text."""
    found = probed(tab, '<main><p style="color: #999">12 cells</p></main>')
    assert len(found) == 1
    assert found[0].startswith('text "12 cells" at 2.85:1 on its ground, under 4.5:1: '), found


def test_text_at_four_and_a_half_passes(tab: Page) -> None:
    """Grey #767676 on white is 4.54:1, just over the bar."""
    assert probed(tab, '<main><p style="color: #767676">12 cells</p></main>') == []


def test_large_text_needs_only_three(tab: Page) -> None:
    """Grey #949494 is 3.03:1: enough at 24 px, not at 16 px."""
    large = '<main><h1 style="color: #949494; font-size: 24px">Forage</h1></main>'
    assert probed(tab, large) == []
    small = '<main><p style="color: #949494">Forage</p></main>'
    assert len(probed(tab, small)) == 1


def test_text_faded_by_opacity_is_measured_faded(tab: Page) -> None:
    """Black text in a box at half opacity reads as mid grey, under 4.5:1."""
    found = probed(tab, '<main><p style="opacity: 0.5">Quiet</p></main>')
    assert len(found) == 1
    assert found[0].startswith('text "Quiet" at 3.9'), found


def test_a_box_ground_is_what_the_words_sit_on(tab: Page) -> None:
    """White on the page is named first, #777 on a #222 box is counted, #eee on it passes."""
    body = (
        '<main><p style="color: #fff">Light</p>'
        '<div style="background: #222"><p style="color: #777">Muted</p>'
        '<p style="color: #eee">Read</p></div></main>'
    )
    found = probed(tab, body)
    assert len(found) == 1
    assert found[0].startswith('text "Light" at 1.00:1'), found
    assert "(and 1 more)" in found[0]

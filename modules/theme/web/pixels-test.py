"""The web-design skill's pixels, words and names probes, held to their cases in headless Chromium.

Each case sets a small page in a phone's 390 px window and runs the probes
on it as a live page's test does, reading the lines they answer: a page
scrolling sideways and a shift as a part arrives (skill/pixels.js), text
under the floor or faint (skill/words.js), and a control named apart from
its words (skill/names.js). The cases come from the live sites as
1HE06W measured them on 22 November 2026: forage 601 px wide on a phone and
shifting 0.81, its labels under 10 px, and the Network's buttons named
"Find a cell" while showing "/". The probe's path comes from WEB_SKILL, as
the check running this sets it, and its browser from probe_browser.py,
loaded as a plugin.

Long because each case is a small page of its own, and the three probes'
cases share one browser, one window and one way of opening a page, which
splitting them would copy into each.
"""

import os
import struct
import time
import zlib
from pathlib import Path
from typing import TYPE_CHECKING

import pytest

if TYPE_CHECKING:
    from collections.abc import Iterator

    from playwright.sync_api import Browser, Page, ViewportSize

PROBE = (Path(os.environ["WEB_SKILL"]) / "pixels.js").read_text()
WORDS = (Path(os.environ["WEB_SKILL"]) / "words.js").read_text()
NAMES = (Path(os.environ["WEB_SKILL"]) / "names.js").read_text()
# The words probe's line for paint it can't work out, given how many places.
UNWORKED = (
    "words in {} drawn through a filter, blend, mask, stroke, shadow alone or too many "
    "layers to work out, so their contrast went unmeasured: paint them and their grounds plainly"
)
# Longest the probe may take on a page that never rests: its three seconds'
# wait, its last half second of quiet, and a builder's slack.
RESTLESS_SECONDS = 5
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


def opened(tab: Page, body: str, head: str = "") -> None:
    """Open `body` as a page, `head` written in its head.

    The page comes by a real navigation, not `set_content`, whose written
    document keeps no layout shifts in its timeline as a loaded page does.
    """
    page = f"<!doctype html><head>{head}<style>{STYLE}</style></head><body>{body}</body>"
    tab.route(ORIGIN, lambda route: route.fulfill(content_type="text/html", body=page))
    tab.goto(ORIGIN)


def canvas(tab: Page) -> tuple[int, ...]:
    """Give the colour the page paints at its window's last pixel, where no case draws.

    A one-pixel PNG's only row reads the same under every filter PNG has,
    since nothing lies left of it or above, so its bytes after the filter's
    are the pixel.
    """
    shot = tab.screenshot(
        clip={"x": PHONE["width"] - 1, "y": PHONE["height"] - 1, "width": 1, "height": 1}
    )
    chunks: dict[bytes, bytes] = {}
    at = 8
    while at < len(shot):
        (length,) = struct.unpack(">I", shot[at : at + 4])
        kind = shot[at + 4 : at + 8]
        chunks[kind] = chunks.get(kind, b"") + shot[at + 8 : at + 8 + length]
        at += 12 + length
    return tuple(zlib.decompress(chunks[b"IDAT"])[1:4])


def answered(tab: Page, probe: str) -> list[str]:
    """Give what `probe` answers on the page open in `tab`."""
    found = tab.evaluate(probe)
    assert isinstance(found, list), f"the probe answered {found!r}, not a list of lines"
    return [str(line) for line in found]


def probed(
    tab: Page,
    body: str,
    settle_ms: int = 0,
    probes: tuple[str, ...] = (PROBE, WORDS),
    head: str = "",
) -> list[str]:
    """Open `body` as a page, let it settle `settle_ms`, and give what `probes` answer."""
    opened(tab, body, head)
    if settle_ms:
        tab.wait_for_timeout(settle_ms)
    return [line for probe in probes for line in answered(tab, probe)]


def named(tab: Page, body: str) -> list[str]:
    """Open `body` as a page and give what names.js answers."""
    return probed(tab, body, probes=(NAMES,))


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
    setTimeout(() => {{ grow("{first}")(); setTimeout(grow("{then}"), 1500); }}, 100);
  }}));
</script>
"""

# A block moved down 100 px twice, at 300 ms and 650 ms after the probe is
# called, which the case sets going in the call itself once the page is
# drawn: two shifts of 0.07 in one window, the second landing after the
# probe's first half second of quiet would have ended, so only a wait that
# starts again on a shift hears it, however slow the builder.
AGAIN = """
<main><div id="above"></div><div style="height: 400px; background: #ccc"></div></main>
<script>
  const above = document.getElementById("above");
  window.drawn = new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done)));
  window.shiftTwice = () => {
    setTimeout(() => { above.style.height = "100px"; }, 300);
    setTimeout(() => { above.style.height = "200px"; }, 650);
  };
</script>
"""

# The block moved 40 px every 200 ms for as long as the page is open.
RESTLESS = """
<main><div id="above"></div><div style="height: 400px; background: #ccc"></div></main>
<script>
  const above = document.getElementById("above");
  let down = false;
  setInterval(() => { down = !down; above.style.height = down ? "40px" : "0"; }, 200);
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


def test_a_shift_as_the_probe_waits_starts_its_wait_again(tab: Page) -> None:
    """A second shift landing in the probe's wait is heard, so the two make 0.14."""
    opened(tab, AGAIN)
    tab.evaluate("drawn")
    # The probe is one expression opening with comments, so it's returned
    # whole in brackets: a bare return before a new line returns nothing.
    found = answered(tab, f"(() => {{ shiftTwice(); return (\n{PROBE}\n); }})()")
    assert len(found) == 1, found
    assert found[0].startswith("the page shifts 0.14 as it arrives"), found


def test_a_page_that_never_rests_ends_the_wait(tab: Page) -> None:
    """A page shifting for ever is named within the wait's three seconds, and a little."""
    opened(tab, RESTLESS)
    began = time.monotonic()
    found = answered(tab, PROBE)
    took = time.monotonic() - began
    assert took < RESTLESS_SECONDS, f"the probe took {took:.1f} s on a page that never rests"
    assert found[0].startswith("the page shifts "), found


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
    found = named(tab, '<main><button aria-label="Search cells">Find</button></main>')
    assert found == [
        (
            'a control showing "Find" named "Search cells": '
            "start its name with the words it shows, or drop the aria-label"
        )
    ]


def test_a_name_only_beginning_with_the_same_letters_is_named(tab: Page) -> None:
    """A button showing "Find" named "Finder" can't be said as "Find", so it's named."""
    found = named(tab, '<main><button aria-label="Finder">Find</button></main>')
    assert len(found) == 1
    assert found[0].startswith('a control showing "Find" named "Finder"'), found


def test_words_a_narrow_screen_hides_arent_shown(tab: Page) -> None:
    """A label hidden at a phone's width isn't among the words the button shows."""
    body = (
        '<main><button aria-label="Find a cell">Find'
        '<span style="display: none"> everything</span></button></main>'
    )
    assert named(tab, body) == []


def test_an_empty_name_is_no_name(tab: Page) -> None:
    """An empty aria-label leaves the button named by its words, so it passes."""
    assert named(tab, '<main><button aria-label="">Find</button></main>') == []


def test_every_misnamed_control_is_counted_in_one_line(tab: Page) -> None:
    """Three misnamed buttons give one line naming the first and counting the rest."""
    body = (
        '<main><button aria-label="Search">Find</button>'
        '<button aria-label="Leave">Go</button><button aria-label="Shut">Close</button></main>'
    )
    found = named(tab, body)
    assert len(found) == 1
    assert found[0].startswith('a control showing "Find" named "Search" (and 2 more): '), found


def test_a_name_starting_with_what_shows_passes(tab: Page) -> None:
    """A button showing "Find" and its keycap, named "Find a cell", passes."""
    body = '<main><button aria-label="Find a cell">Find <kbd>/</kbd></button></main>'
    assert named(tab, body) == []


def test_a_mark_alone_shows_a_word_too(tab: Page) -> None:
    """A keycap button showing "/" and named "Find a cell", as the Network's were, is named."""
    found = named(
        tab, '<main><kbd role="button" tabindex="0" aria-label="Find a cell">/</kbd></main>'
    )
    assert found == [
        (
            'a control showing only "/", named "Find a cell": '
            "show its word beside the mark, its name starting with that word"
        )
    ]


def test_a_mark_alone_is_named_without_a_label_too(tab: Page) -> None:
    """Dropping a mark's label leaves "/" as its name, which no voice can say either."""
    found = named(tab, '<main><kbd role="button" tabindex="0">/</kbd></main>')
    assert found == [
        (
            'a control showing only "/": '
            "show its word beside the mark, its name starting with that word"
        )
    ]


def test_the_skills_key_line_passes_once_its_keys_are_pressable(tab: Page) -> None:
    """The head's key line, its keycaps made buttons as pressable makes them, passes."""
    key = '<kbd role="button" tabindex="0"{}>{}</kbd>'
    body = (
        '<main><p class="keys">'
        f"{key.format(' data-tap', 'j')} and {key.format(' data-tap', 'k')} step from"
        f" site to site, and {key.format('', '?')} lists every key</p></main>"
    )
    assert named(tab, body) == []


def test_a_keycap_whose_line_says_nothing_after_it_is_named(tab: Page) -> None:
    """A "/" keycap with only another keycap after it, and a "?" ending its line, are named."""
    key = '<kbd role="button" tabindex="0">{}</kbd>'
    body = f'<main><p class="keys">{key.format("/")} {key.format("?")}</p></main>'
    assert named(tab, body) == [
        (
            'a control showing only "/" (and 1 more): '
            "show its word beside the mark, its name starting with that word"
        )
    ]


def test_a_keycap_beside_words_no_one_sees_or_a_joining_word_is_named(tab: Page) -> None:
    """A comment, words kept for a screen reader, or "or" alone say nothing beside a key."""
    key = '<kbd role="button" tabindex="0">{}</kbd>'
    heard = (
        '<span style="position: absolute; width: 1px; height: 1px; overflow: hidden;'
        ' clip-path: inset(50%)">lists every key</span>'
    )
    body = (
        f"<main><p>{key.format('/')}<!-- searches --></p>"
        f"<p>{key.format('?')}{heard}</p>"
        f"<p>{key.format('[')} or {key.format(']')}</p></main>"
    )
    assert named(tab, body) == [
        (
            'a control showing only "/" (and 3 more): '
            "show its word beside the mark, its name starting with that word"
        )
    ]


def test_a_keycap_its_line_passes_is_named_for_that_line(tab: Page) -> None:
    """A ? keycap named "Delete everything" beside "lists every key" is named; its line passes."""
    key = '<main><p><kbd role="button" tabindex="0" aria-label="{}">?</kbd> lists every key</p>'
    assert named(tab, key.format("Lists every key") + "</main>") == []
    assert named(tab, key.format("Delete everything") + "</main>") == [
        (
            'a keycap "?" beside "lists every key" named "Delete everything": '
            "start its name with the words its line shows beside it, or drop the aria-label"
        )
    ]


def test_a_controls_words_in_a_box_drawn_as_its_children_are_held_to_its_name(
    tab: Page,
) -> None:
    """A button showing "Find" through a display: contents span, named "Search", is named."""
    body = (
        '<main><button aria-label="Search"><span style="display: contents">Find</span>'
        "</button></main>"
    )
    found = named(tab, body)
    assert len(found) == 1, found
    assert found[0].startswith('a control showing "Find" named "Search"'), found


def test_a_button_showing_only_a_mark_is_named_whatever_follows(tab: Page) -> None:
    """An icon button showing "+" beside words is named: only a keycap reads its line."""
    body = '<main><p><button aria-label="Add">+</button> adds a cell</p></main>'
    found = named(tab, body)
    assert len(found) == 1, found
    assert found[0].startswith('a control showing only "+", named "Add"'), found


def test_a_name_given_by_another_element_is_held_to_what_shows(tab: Page) -> None:
    """A button labelled by a heading reading "Cells" while showing "Find" is named."""
    body = '<main><h2 id="cells">Cells</h2><button aria-labelledby="cells">Find</button></main>'
    found = named(tab, body)
    assert len(found) == 1, found
    assert found[0].startswith('a control showing "Find" named "Cells"'), found


def test_a_button_showing_only_its_keycap_takes_the_key_as_its_word(tab: Page) -> None:
    """A button showing only a j keycap is named for j first, as a keycap control is."""
    good = '<main><button aria-label="j, next site"><kbd>j</kbd></button></main>'
    assert named(tab, good) == []
    found = named(tab, '<main><button aria-label="Next site"><kbd>j</kbd></button></main>')
    assert len(found) == 1, found
    assert found[0].startswith('a control showing "j" named "Next site"'), found


def test_a_keycap_showing_its_key_passes(tab: Page) -> None:
    """A tappable keycap showing j, named for its key first, passes."""
    body = '<main><kbd role="button" tabindex="0" aria-label="j, next site">j</kbd></main>'
    assert named(tab, body) == []


def test_a_picture_named_by_its_label_passes(tab: Page) -> None:
    """A theme switch drawn only as a picture shows no words, so its label names it."""
    body = (
        '<main><button aria-label="Dark"><svg width="24" height="24" aria-hidden="true">'
        '<circle cx="12" cy="12" r="6"/></svg></button></main>'
    )
    assert named(tab, body) == []


def test_faint_text_is_named_with_its_ratio(tab: Page) -> None:
    """Grey #999 on white is 2.85:1, under 4.5:1 for body text."""
    found = probed(tab, '<main><p style="color: #999">12 cells</p></main>')
    assert len(found) == 1
    assert found[0].startswith('text "12 cells" at 2.85:1 on its ground, under 4.5:1: '), found


def test_text_at_four_and_a_half_passes(tab: Page) -> None:
    """Grey #767676 on white is 4.54:1, just over the bar."""
    assert probed(tab, '<main><p style="color: #767676">12 cells</p></main>') == []


def test_text_just_under_four_and_a_half_is_named(tab: Page) -> None:
    """Grey #777 on white is 4.48:1, and WCAG's bar has no rounding."""
    found = probed(tab, '<main><p style="color: #777">12 cells</p></main>')
    assert len(found) == 1, found
    assert found[0].startswith('text "12 cells" at 4.48:1'), found


def test_large_text_needs_only_three(tab: Page) -> None:
    """Grey #949494 is 3.03:1: enough at 24 px, not at 16 px."""
    large = '<main><h1 style="color: #949494; font-size: 24px">Forage</h1></main>'
    assert probed(tab, large) == []
    small = '<main><p style="color: #949494">Forage</p></main>'
    found = probed(tab, small)
    assert len(found) == 1, found
    assert found[0].startswith('text "Forage" at 3.03:1 on its ground, under 4.5:1'), found


def test_bold_text_is_large_from_eighteen_and_two_thirds(tab: Page) -> None:
    """#949494 at 19 px is enough bold, and not at a normal weight."""
    bold = '<main><p style="color: #949494; font: 700 19px serif">Forage</p></main>'
    assert probed(tab, bold) == []
    normal = '<main><p style="color: #949494; font: 400 19px serif">Forage</p></main>'
    found = probed(tab, normal)
    assert len(found) == 1, found
    assert found[0].startswith('text "Forage" at 3.03:1'), found


def test_text_faded_by_opacity_is_measured_faded(tab: Page) -> None:
    """Black text in a box at half opacity reads as mid grey, under 4.5:1."""
    found = probed(tab, '<main><p style="opacity: 0.5">Quiet</p></main>')
    assert len(found) == 1
    assert found[0].startswith('text "Quiet" at 3.9'), found


def test_a_faded_box_fades_its_ground_and_words_as_one(tab: Page) -> None:
    """#888 on black at 0.8 opacity paints #a0a0a0 on #333 over white, 4.8:1."""
    body = (
        '<main><div style="background: #000; opacity: 0.8">'
        '<p style="color: #888">Calm</p></div></main>'
    )
    assert probed(tab, body) == []


def test_a_faded_control_on_its_own_ground_is_measured_as_drawn(tab: Page) -> None:
    """White on #444 at half opacity over black paints mid grey on #222, about 4:1."""
    body = (
        '<main style="background: #000"><div style="background: #444; opacity: 0.5">'
        '<p style="color: #fff">Off</p></div></main>'
    )
    found = probed(tab, body)
    assert len(found) == 1, found
    assert found[0].startswith('text "Off" at 4.0'), found


def test_words_nobody_sees_are_not_faint(tab: Page) -> None:
    """Words at no opacity, still to arrive, aren't measured."""
    assert probed(tab, '<main><p style="opacity: 0; color: #999">Copied</p></main>') == []


def test_a_placeholder_is_measured_in_its_own_colour(tab: Page) -> None:
    """A field's #bbb placeholder is faint, though the field's words are black."""
    body = (
        "<style>input::placeholder { color: #bbb; }</style>"
        '<main><input placeholder="Find" style="color: #000"></main>'
    )
    found = probed(tab, body)
    assert len(found) == 1, found
    assert found[0].startswith('text "Find" at 1.9'), found


def test_words_a_style_writes_are_measured_in_their_colour(tab: Page) -> None:
    """An ::after's #bbb words are faint beside black ones."""
    body = (
        '<style>.n::after { content: " cells"; color: #bbb; }</style>'
        '<main><p class="n">12</p></main>'
    )
    found = probed(tab, body)
    assert len(found) == 1, found
    assert found[0].startswith('text "cells" at 1.9'), found


def test_a_gradient_is_measured_at_its_worst(tab: Page) -> None:
    """#777 on a gradient from white to #999 is named at its #999 end."""
    body = (
        '<main><div style="background: linear-gradient(#fff, #999)">'
        '<p style="color: #777">Fade</p></div></main>'
    )
    found = probed(tab, body)
    assert len(found) == 1, found
    assert found[0].startswith('text "Fade" at 1.57'), found


def test_a_gradient_crossing_the_words_colour_is_named(tab: Page) -> None:
    """#757575 passes on white and on black, but a gradient between them passes through it."""
    body = (
        '<main><div style="background: linear-gradient(#fff, #000)">'
        '<p style="color: #757575">Fade</p></div></main>'
    )
    found = probed(tab, body)
    assert len(found) == 1, found
    assert found[0].startswith('text "Fade" at 1.'), found


def test_a_mix_of_colours_in_a_gradient_is_read_mixed(tab: Page) -> None:
    """A 9% tint in a dark page's glow, as the system page draws, isn't read as solid tint."""
    body = (
        "<style>:root { color-scheme: dark; } body { color: #ddd; background: #111 "
        "radial-gradient(ellipse 70rem 28rem at 50% -6rem, "
        "color-mix(in oklab, rgb(220, 220, 255) 9%, transparent), transparent 70%); }</style>"
        "<main><p>Glow</p></main>"
    )
    assert probed(tab, body) == []


def test_a_dark_page_with_no_ground_sits_on_a_dark_canvas(tab: Page) -> None:
    """#eee words on a dark scheme's canvas read, though they'd be faint on white."""
    body = (
        '<style>:root { color-scheme: dark; }</style><main><p style="color: #eee">Pale</p></main>'
    )
    assert probed(tab, body) == []


def test_a_dark_scheme_named_in_the_head_sits_on_a_dark_canvas(tab: Page) -> None:
    """#eee words read on the canvas a meta tag's dark scheme sets, the style saying none.

    The canvas's own pixel is read too, so the case holds the browser to the
    dark canvas the probe takes the meta to mean.
    """
    body = '<main><p style="color: #eee">Pale</p></main>'
    head = '<meta name="color-scheme" content="dark">'
    assert probed(tab, body, head=head) == []
    painted = canvas(tab)
    assert painted == (18, 18, 18), f"the canvas a dark meta sets is {painted}, not #121212"


def test_words_in_a_box_drawn_as_its_children_are_measured(tab: Page) -> None:
    """#ccc words held by a display: contents span are read where its parent is drawn."""
    body = '<main><p style="color: #ccc"><span style="display: contents">Faint</span></p></main>'
    found = probed(tab, body)
    assert len(found) == 1, found
    assert found[0].startswith('text "Faint" at 1.6'), found


def test_a_box_drawn_as_its_children_paints_nothing_under_them(tab: Page) -> None:
    """#111 words on black are named though the contents span holding them sets white."""
    body = (
        '<main><div style="background: #000"><span style="display: contents; background: #fff;'
        ' opacity: 0.3; color: #111">Dim</span></div></main>'
    )
    found = probed(tab, body)
    assert len(found) == 1, found
    assert found[0].startswith('text "Dim" at 1.'), found


def test_a_layer_of_none_leaves_the_layers_under_it(tab: Page) -> None:
    """#111 words over a none layer above black are named on the black."""
    body = (
        '<main><p style="color: #111; background-image: none, linear-gradient(#000, #000)">'
        "Dim</p></main>"
    )
    found = probed(tab, body)
    assert len(found) == 1, found
    assert found[0].startswith('text "Dim" at 1.'), found


def test_a_layer_sized_past_its_box_in_pixels_is_its_ground(tab: Page) -> None:
    """#111 words on a black layer 4000 px wide, or contained, are named on the black."""
    for size in ("4000px 100%", "contain"):
        body = (
            f'<main><p style="color: #111; background: linear-gradient(#000, #000) no-repeat'
            f' 0 0 / {size}, #fff">Dim</p></main>'
        )
        found = probed(tab, body)
        assert len(found) == 1, (size, found)
        assert found[0].startswith('text "Dim" at 1.'), (size, found)


def test_a_stripe_tiled_along_one_axis_is_a_ground(tab: Page) -> None:
    """#111 words on white striped with black, tiled across and full height, are named."""
    body = (
        '<main><p style="color: #111; background: linear-gradient(90deg, #000 50%,'
        ' transparent 50%) repeat-x 0 0 / 20px 100%, #fff">Dim</p></main>'
    )
    found = probed(tab, body)
    assert len(found) == 1, found
    assert found[0].startswith('text "Dim" at 1.'), found


def test_a_layer_placed_off_its_origin_leaves_the_ground_under_it(tab: Page) -> None:
    """#eee words on black are named for the white strip a black layer, moved over, leaves."""
    body = (
        '<main><p style="color: #eee; background: linear-gradient(#000, #000) no-repeat'
        ' 3rem 0 / 100% 100%, #fff">Pale</p></main>'
    )
    found = probed(tab, body)
    assert len(found) == 1, found
    assert found[0].startswith('text "Pale" at 1.'), found


def test_a_ground_under_a_mark_beside_the_words_is_still_their_ground(tab: Page) -> None:
    """#111 words on black are named though a small red mark is drawn over the black."""
    body = (
        '<main><p style="color: #111; background: linear-gradient(#f00, #f00) no-repeat 0 0 / '
        '4px 4px, linear-gradient(#000, #000)">Dim</p></main>'
    )
    found = probed(tab, body)
    assert len(found) == 1, found
    assert found[0].startswith('text "Dim" at 1.'), found


def test_a_tiled_layer_is_the_ground_under_the_words(tab: Page) -> None:
    """An 8 px black tile, repeated across the box, is what #111 words sit on."""
    body = (
        '<main><p style="color: #111; background: linear-gradient(#000, #000) 0 0 / 8px 8px">'
        "Dim</p></main>"
    )
    found = probed(tab, body)
    assert len(found) == 1, found
    assert found[0].startswith('text "Dim" at 1.'), found


def test_a_hard_stop_is_not_blended_across(tab: Page) -> None:
    """#757575 passes on white and black halves split by a hard stop, with no grey between."""
    body = (
        '<main><div style="background: linear-gradient(90deg, #fff 50%, #000 50%)">'
        '<p style="color: #757575">Split</p></div></main>'
    )
    assert probed(tab, body) == []


def test_a_hard_stop_in_pixels_is_not_blended_across(tab: Page) -> None:
    """#757575 passes on a white band 200 px wide, or one to 100%, that ends hard on black."""
    for stops in ("#fff 0 200px, #000 200px", "#fff 100%, #000"):
        body = (
            f'<main><div style="background: linear-gradient(90deg, {stops})">'
            '<p style="color: #757575">Split</p></div></main>'
        )
        assert probed(tab, body) == [], stops


def test_a_shimmer_twice_its_box_wide_is_measured(tab: Page) -> None:
    """A clipped gradient sized past its box, as a shimmer is, still paints the words."""
    body = (
        '<main><h1 style="background: linear-gradient(90deg, #ddd, #fff, #ddd); '
        'background-size: 200% 100%; background-clip: text; color: transparent">Shine</h1></main>'
    )
    found = probed(tab, body)
    assert len(found) == 1, found
    assert found[0].startswith('text "Shine" at 1.'), found


def test_a_line_drawn_beside_the_words_is_not_their_ground(tab: Page) -> None:
    """A 1 px black line drawn as a gradient leaves the words on white."""
    body = (
        '<main><div style="background: linear-gradient(#000, #000) no-repeat; '
        'background-size: 1px 100%"><p>Beside a line</p></div></main>'
    )
    assert probed(tab, body) == []


def test_words_painted_with_a_gradient_are_measured_by_it(tab: Page) -> None:
    """Words clipped to a black to #ccc gradient are named at #ccc."""
    body = (
        '<main><p style="background: linear-gradient(#000, #ccc); background-clip: text; '
        'color: transparent">Glow</p></main>'
    )
    found = probed(tab, body)
    assert len(found) == 1, found
    assert found[0].startswith('text "Glow" at 1.6'), found


def test_words_filled_with_a_gradient_are_measured_by_it(tab: Page) -> None:
    """Black words filled transparent over a pale clipped gradient show the gradient."""
    body = (
        '<main><p style="background: linear-gradient(#eee, #ddd); background-clip: text; '
        'color: #000; -webkit-text-fill-color: transparent">Glow</p></main>'
    )
    found = probed(tab, body)
    assert len(found) == 1, found
    assert found[0].startswith('text "Glow" at 1.'), found


def test_words_in_a_box_painting_them_with_a_gradient_are_measured_by_it(tab: Page) -> None:
    """A heading's clipped gradient paints the words of the span inside it."""
    body = (
        '<main><h1 style="background: linear-gradient(#eee, #ddd); background-clip: text; '
        'color: transparent"><span>Glow</span></h1></main>'
    )
    found = probed(tab, body)
    assert len(found) == 1, found
    assert found[0].startswith('text "Glow" at 1.'), found


def test_words_through_a_filter_or_a_blend_are_named_for_a_person(tab: Page) -> None:
    """A blur and a blend change the paint in ways the probe can't work out."""
    body = (
        '<main><p style="filter: blur(1px)">Blurred</p>'
        '<p style="mix-blend-mode: difference">Blended</p></main>'
    )
    assert probed(tab, body) == [UNWORKED.format("2 places are")]


def test_words_masked_stroked_or_drawn_by_a_shadow_are_named_for_a_person(tab: Page) -> None:
    """A blended ground, a mask, a stroke and a shadow alone are paint it can't work out."""
    body = (
        '<main><p style="background: linear-gradient(#fff, #eee); '
        'background-blend-mode: multiply">Multiplied</p>'
        '<p style="-webkit-mask-image: linear-gradient(#000, transparent)">Masked</p>'
        '<p style="-webkit-text-stroke: 1px #000">Stroked</p>'
        '<p style="color: transparent; text-shadow: 0 0 2px #000">Shadowed</p></main>'
    )
    assert probed(tab, body) == [UNWORKED.format("4 places are")]


def test_words_on_too_many_grounds_are_named_for_a_person(tab: Page) -> None:
    """Three gradients, one in another, are more ways than the probe works out."""
    ground = '<div style="background: linear-gradient(#fff, #eee)">'
    found = probed(tab, f"<main>{ground * 3}<p>Deep</p></div></div></div></main>")
    assert len(found) == 1, found
    assert found == [UNWORKED.format("1 place are")], found


def test_words_a_style_hides_are_not_measured(tab: Page) -> None:
    """An ::after left undrawn, or at no opacity, has no size or colour anyone reads."""
    body = (
        '<style>.a::after { content: "later"; display: none; font-size: 8px; }'
        '.b::after { content: "later"; opacity: 0; color: #fff; }</style>'
        '<main><p class="a">Now</p><p class="b">Then</p></main>'
    )
    assert probed(tab, body) == []


def test_words_shrunk_by_scale_are_measured_shrunk(tab: Page) -> None:
    """16 px words at half scale are 8 px on screen."""
    found = probed(tab, '<main><p style="scale: 0.5">Halved</p></main>')
    assert len(found) == 1, found
    assert found[0].startswith('text at 8.0 px ("Halved")'), found


def test_words_on_a_picture_are_named_for_a_person(tab: Page) -> None:
    """A picture can't be measured, so its words are named, not passed in silence."""
    picture = "data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7"
    body = f'<main><div style="background-image: url({picture})"><p>On a picture</p></div></main>'
    found = probed(tab, body)
    assert found == [
        (
            "words in 1 place sit on a picture, so their contrast went unmeasured: "
            "set them on a plain ground or a gradient"
        )
    ], found


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

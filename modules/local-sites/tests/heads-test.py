"""The heads gate passes the kit's own head and names each way a page strays from it.

The head sites.md writes for every live page is the case it must pass, read
from the skill itself, so the gate and the pattern it holds pages to can't
drift apart. Each other case changes one thing about that head and expects
the one line saying what to change back.

Long because it holds a case for each way a browser reads a head and each
rule heads.py holds it to, and they are read together, beside heads.py.
"""

import os
import re
import sys
from pathlib import Path
from typing import TYPE_CHECKING

import heads
import pytest

if TYPE_CHECKING:
    from collections.abc import Callable

SITES_MD = Path(os.environ["WEB_SKILL"]) / "sites.md"

# The head sites.md writes, the html block under its heading for it.
WRITTEN = re.search(
    r"\*\*The head every live page opens with\*\*.*?```html\n(.*?)```",
    SITES_MD.read_text(),
    re.DOTALL,
)

# A page's head links the kit's sheet before the head it draws.
LINK = '<link rel="stylesheet" href="../local-sites/kit/head.css">'


def written() -> str:
    """Give the head sites.md writes."""
    assert WRITTEN is not None, "sites.md no longer writes the head as an html block"
    return WRITTEN.group(1)


def page(head: str) -> str:
    """Give a page linking the kit's sheet and opening with `head`, before its main."""
    return f"<!doctype html><html><head>{LINK}</head><body>{head}<main></main></body></html>"


def changed(old: str, new: str) -> str:
    """Give the written head with `old` changed to `new`, which it must hold."""
    head = written()
    assert old in head, f"sites.md's head no longer holds {old!r}"
    return head.replace(old, new, 1)


def test_the_head_sites_md_writes_is_the_kits() -> None:
    """The pattern every page copies passes, as a head and as a page."""
    assert heads.head_faults(written()) == []
    assert heads.page_faults(page(written())) == []


def test_a_page_not_linking_the_sheet_is_named() -> None:
    """A head the kit's sheet never reaches would be drawn as the page's own."""
    markup = f"<!doctype html><html><head></head><body>{written()}</body></html>"
    assert heads.page_faults(markup) == [NO_SHEET]


# What the gate says of a page whose head the kit's sheet never draws.
NO_SHEET = heads.NO_SHEET


@pytest.mark.parametrize(
    "link",
    [
        '<link rel="preload" as="style" href="../local-sites/kit/head.css">',
        '<link rel="stylesheet" href="kit/head.css">',
        '<link rel="stylesheet" href="local-sites/kit/head.css">',
        '<link rel="stylesheet" href="../my-local-sites/kit/head.css">',
        '<link rel="alternate stylesheet" title="x" href="../local-sites/kit/head.css">',
        '<link rel="stylesheet" href="../local-sites/kit/head.css" disabled>',
        '<link rel="stylesheet" href="../local-sites/kit/head.css" media="print">',
        '<link rel="stylesheet" type="text/plain" href="../local-sites/kit/head.css">',
        '<link rel="stylesheet" integrity="sha384-x" href="../local-sites/kit/head.css">',
    ],
)
def test_a_sheet_no_screen_draws_the_head_with_is_named(link: str) -> None:
    """Preloaded, a page's own copy, a miss, an alternate, off, print-only, not CSS, or pinned.

    A pinned digest goes stale as the kit's sheet changes, and a browser then
    applies nothing.
    """
    markup = f"<!doctype html><html><head>{link}</head><body>{written()}</body></html>"
    assert heads.page_faults(markup) == [NO_SHEET]


def test_a_sheet_linked_in_the_body_is_named() -> None:
    """A sheet linked after the head draws it late, once it has shown bare."""
    markup = f"<!doctype html><html><head></head><body>{LINK}{written()}</body></html>"
    assert heads.page_faults(markup) == [NO_SHEET]


@pytest.mark.parametrize(
    "link",
    [
        '<link rel="stylesheet" href="/local-sites/kit/head.css">',
        '<link rel="stylesheet" href="../local-sites/kit/head.css" media="all">',
        '<link rel="stylesheet" type="Text/CSS; charset=utf-8" href="../local-sites/kit/head.css">',
    ],
)
def test_a_sheet_from_the_root_or_for_all_media_draws_the_head(link: str) -> None:
    """Home links the kit from the root, every screen is all media, and CSS is CSS."""
    markup = f"<!doctype html><html><head>{link}</head><body>{written()}</body></html>"
    assert heads.page_faults(markup) == []


@pytest.mark.parametrize(
    "markup",
    [
        f"<!doctype html><html>{LINK}<body>{{head}}</body></html>",
        f"<!doctype html><html><head></head>{LINK}<body>{{head}}</body></html>",
        f"<!doctype html><html>{LINK.replace('>', '/>')}<body>{{head}}</body></html>",
    ],
)
def test_a_sheet_in_a_head_the_parser_implies_draws_the_head(markup: str) -> None:
    """A link before the body goes in the head, written or not, as a parser puts it."""
    assert heads.page_faults(markup.format(head=written())) == []


def test_a_base_moving_the_sheets_path_is_named() -> None:
    """A base resolves the kit's path somewhere else, so the gate can't say it's the kit's."""
    markup = (
        f'<!doctype html><html><head><base href="/elsewhere/a/">{LINK}</head>'
        f"<body>{written()}</body></html>"
    )
    assert heads.page_faults(markup) == [heads.BASED]


@pytest.mark.parametrize(
    "holder", [("<div hidden>", "</div>"), ("<dialog>", "</dialog>"), ("<div popover>", "</div>")]
)
def test_a_base_written_where_nothing_is_drawn_is_named(holder: tuple[str, str]) -> None:
    """A browser reads links from a base wherever it's parsed, drawn or not."""
    opens, closes = holder
    base = f'{opens}<base href="/elsewhere/a/">{closes}'
    assert heads.page_faults(page(base + written())) == [heads.BASED]


def test_a_base_in_a_template_moves_nothing() -> None:
    """A template's markup is its own fragment, so a base in it moves no link."""
    base = '<template><base href="/elsewhere/a/"></template>'
    assert heads.page_faults(page(base + written())) == []


@pytest.mark.parametrize("holder", ["template", "noscript"])
def test_a_template_or_noscript_in_the_head_keeps_the_head(holder: str) -> None:
    """What a head's template or noscript holds is its own, so the body doesn't begin there."""
    markup = (
        f"<!doctype html><html><head><{holder}><div></div></{holder}>{LINK}</head>"
        f"<body>{written()}</body></html>"
    )
    assert heads.page_faults(markup) == []


def test_a_link_after_a_no_break_space_in_the_head_is_in_the_body() -> None:
    """A no-break space is a word to HTML, so the body begins at it."""
    markup = f"<!doctype html><html><head>&nbsp;{LINK}</head><body>{written()}</body></html>"
    assert heads.page_faults(markup) == [NO_SHEET]


def test_an_attribute_written_twice_is_read_as_its_first() -> None:
    """A browser keeps a repeated attribute's first value, so own.css is what loads."""
    link = '<link rel="stylesheet" href="own.css" href="../local-sites/kit/head.css">'
    markup = f"<!doctype html><html><head>{link}</head><body>{written()}</body></html>"
    assert heads.page_faults(markup) == [NO_SHEET]
    lede = changed('<p class="lede">', '<p class="lede" aria-live="polite" aria-live="off">')
    assert heads.head_faults(lede) == [SPEAKS]


# What the gate says of a skip link sharing its line with what follows.
SKIP_LINE = heads.SKIP_LINE


def test_a_skip_link_sharing_its_line_is_named() -> None:
    """The strip goes after the skip link's line, so the head on it would hold the strip."""
    skip = '<a class="skip" href="#main">Skip to the sites</a>'
    assert heads.page_faults(page(skip + written())) == [SKIP_LINE]
    assert heads.page_faults(page(f"\n{skip} \n{written()}")) == []


def test_a_skip_link_written_over_two_lines_is_named() -> None:
    """The strip goes after the line its start tag opens on, inside the link."""
    skip = '<a class="skip"\n  href="#main">Skip to the sites</a>'
    assert heads.page_faults(page(f"{skip}\n{written()}")) == [SKIP_LINE]


def test_a_skip_link_in_the_head_is_named() -> None:
    """The strip's nav, put after a skip link in the eyebrow, would end the eyebrow early."""
    head = changed(
        '<p class="eyebrow">your sites',
        '<p class="eyebrow"><a class="skip" href="#main">Skip</a>\nyour sites',
    )
    assert (
        "its skip link sits in or after its head, where the strip site.sh puts after it"
        " would split what holds it: write it before the head"
    ) in heads.page_faults(page(head))


@pytest.mark.parametrize(
    "markup",
    [
        f"<!doctype html><html><head></head><div>{{head}}</div>{LINK}</html>",
        f"<!doctype html><html><div></div>{LINK}<body>{{head}}</body></html>",
        (
            f"<!doctype html><html><head><title>x</title>Stray{LINK}</head>"
            "<body>{head}</body></html>"
        ),
        f"<!doctype html><html><body><head>{LINK}</head>{{head}}</body></html>",
        (
            f"<!doctype html><html><head></head><noscript></noscript>{LINK}"
            "<body>{head}</body></html>"
        ),
        f"<!doctype html><html><head><br/>{LINK}</head><body>{{head}}</body></html>",
        f'<!doctype html><html><img src="x" /><head>{LINK}</head><body>{{head}}</body></html>',
        f"<!doctype html><html><head><svg/>{LINK}</head><body>{{head}}</body></html>",
    ],
)
def test_a_sheet_after_the_body_begins_is_in_the_body(markup: str) -> None:
    """The body begins where a browser begins it, and a head written in it is ignored.

    After the head, before it, at words in it, as a second head in the body, at a
    noscript after the head, or at a void or SVG element written closed: each
    link lands in the body, where it draws the head late.
    """
    assert heads.page_faults(markup.format(head=written())) == [NO_SHEET]


def test_a_head_left_open_ends_where_the_body_starts() -> None:
    """With </head> left off, a link in the body is still in the body."""
    markup = f"<!doctype html><html><head><title>x</title><body>{LINK}{written()}</body></html>"
    assert heads.page_faults(markup) == [NO_SHEET]


def test_a_sheet_asked_for_afresh_still_draws_the_head() -> None:
    """A query asking past a cache names the same sheet."""
    link = LINK.replace("head.css", "head.css?v=2")
    markup = f"<!doctype html><html><head>{link}</head><body>{written()}</body></html>"
    assert heads.page_faults(markup) == []


def test_a_page_with_no_head_is_named() -> None:
    """A page drawing its own header instead says how many heads it has."""
    faults = heads.head_faults('<header class="hero"><h1>Mine</h1></header>')
    assert faults == ['it draws 0 heads: open it with one <header class="page-head">']


def test_a_page_with_two_heads_is_named() -> None:
    """A page opens with one head, never the kit's twice."""
    faults = heads.head_faults(written() + written())
    assert faults == ['it draws 2 heads: open it with one <header class="page-head">']


def test_a_banner_kept_above_the_head_is_named() -> None:
    """A page half moved, its own banner still drawn first, isn't one of the family."""
    faults = heads.head_faults(
        '<header class="hero"><p class="wordmark">Mine</p></header>' + written()
    )
    assert faults == [
        'it draws a <header class="hero"> above its head: let the kit\'s head be its one banner'
    ]


def test_a_banner_kept_below_the_head_is_named() -> None:
    """A page half moved, its own banner still drawn after the kit's, is heard as two."""
    faults = heads.head_faults(
        written() + '<header class="hero"><p class="wordmark">Mine</p></header>'
    )
    assert faults == [
        (
            'it draws a <header class="hero"> below its head as well:'
            " let the kit's head be its one banner"
        )
    ]


def test_a_sections_own_header_below_the_head_is_no_banner() -> None:
    """A header in an article is the article's, not a second banner."""
    post = "<main><article><header><h2>A post</h2></header></article></main>"
    assert heads.head_faults(written() + post) == []
    region = '<div role="region"><header><h2>A part</h2></header></div>'
    assert heads.head_faults(written() + region) == []


def test_a_banner_after_a_head_that_isnt_one_is_named_below() -> None:
    """A head in a main is no banner, and the page's own header after it is below it."""
    faults = heads.head_faults(f'<main>{written()}</main><header class="hero"><p>Mine</p></header>')
    assert (
        'it draws a <header class="hero"> below its head: let the kit\'s head be its one banner'
    ) in faults


def test_a_banner_marked_inside_the_head_is_a_second() -> None:
    """A role is heard wherever it's marked, so a banner in the lede is one more."""
    faults = heads.head_faults(changed("Seven sites", '<span role="banner">Seven</span> sites'))
    assert faults == [
        ("it draws a <span> below its head as well: let the kit's head be its one banner")
    ]


def test_a_banner_wrapped_around_the_head_is_named() -> None:
    """A page's own banner holding the kit's head is still drawn first, and heard as the banner."""
    faults = heads.head_faults('<header class="mast"><p>Mine</p>' + written() + "</header>")
    assert faults == [
        'it draws a <header class="mast"> around its head: let the kit\'s head be its one banner'
    ]


@pytest.mark.parametrize("holder", ["template", "noscript"])
def test_a_head_never_drawn_is_no_head(holder: str) -> None:
    """A head held where no browser draws it leaves the page's own showing."""
    faults = heads.head_faults(f"<{holder}>{written()}</{holder}>")
    assert faults == ['it draws 0 heads: open it with one <header class="page-head">']


@pytest.mark.parametrize("holder", ["<dialog>", "<details><summary>more</summary>"])
def test_a_head_shut_away_is_no_head(holder: str) -> None:
    """A dialog not open, or details not open past its summary, draws none of the head."""
    closing = "</dialog>" if holder == "<dialog>" else "</details>"
    markup = f'<div class="hero"><p class="big">Mine</p></div>{holder}{written()}{closing}'
    assert heads.head_faults(markup) == [
        'it draws 0 heads: open it with one <header class="page-head">'
    ]


def test_a_head_in_open_details_is_drawn() -> None:
    """Details marked open draw what they hold."""
    markup = f"<details open><summary>more</summary>{written()}</details>"
    assert heads.head_faults(markup) == []


@pytest.mark.parametrize(
    "marked",
    [
        "hidden",
        "popover",
        'style="display:none"',
        'style="color: red; display: NONE !important"',
        'style="display: none ! important"',
        'style="display: block; display: none"',
        'style="display: none !important; display: block"',
        'style="opacity: 0"',
        'hidden="until-found" style="display: block"',
    ],
)
def test_a_hidden_head_is_no_head(marked: str) -> None:
    """A head hidden, a popover no script has shown, or styled out is never drawn.

    A property's last value counts, but not over an important one.
    """
    head = changed('<header class="page-head">', f'<header class="page-head" {marked}>')
    faults = heads.head_faults('<div class="hero"><h2>Mine</h2></div>' + head)
    assert faults == ['it draws 0 heads: open it with one <header class="page-head">']


def test_a_head_held_in_a_popover_is_no_head() -> None:
    """A popover draws nothing it holds until a script shows it."""
    faults = heads.head_faults(
        f'<div class="hero"><h2>Mine</h2></div><div popover>{written()}</div>'
    )
    assert faults == ['it draws 0 heads: open it with one <header class="page-head">']


@pytest.mark.parametrize(
    "marked",
    [
        'style="display: block"',
        'style="overflow: hidden"',
        'style="border-style: none"',
        'style="visibility: visible"',
        'style="--x: none"',
        'style="display: none; display: block"',
        'hidden style="display: block"',
        'popover style="display: flex"',
    ],
)
def test_a_head_styled_otherwise_is_drawn(marked: str) -> None:
    """A style that hides nothing leaves the head drawn, and its own display beats hidden."""
    head = changed('<header class="page-head">', f'<header class="page-head" {marked}>')
    assert heads.head_faults(head) == []


def test_words_loose_in_the_head_are_named() -> None:
    """Every word in the head belongs to one of its parts."""
    head = changed('<p class="eyebrow">', 'Stray words<p class="eyebrow">')
    assert heads.head_faults(head) == [
        "its head writes words outside its parts: keep every word in a part"
    ]


def test_a_banner_marked_by_role_above_the_head_is_named() -> None:
    """A div a screen reader hears as the banner is a banner, whatever its tag."""
    faults = heads.head_faults('<div role="banner"><p>Mine</p></div>' + written())
    assert faults == ["it draws a <div> above its head: let the kit's head be its one banner"]


def test_a_banner_marked_by_role_around_the_head_is_named() -> None:
    """A banner landmark holding the kit's head is still heard first."""
    faults = heads.head_faults('<div class="mast" role="banner"><p>Mine</p>' + written() + "</div>")
    assert faults == [
        'it draws a <div class="mast"> around its head: let the kit\'s head be its one banner'
    ]


def test_a_heading_marked_by_role_before_the_head_is_named() -> None:
    """A div heard as a heading above the head is the page's first heading."""
    faults = heads.head_faults('<div role="heading" aria-level="2">Mine</div>' + written())
    assert faults == [
        "its first heading, <div>, sits outside its head: make the head's title first"
    ]


@pytest.mark.parametrize("role", ["none", "presentation", "region"])
def test_a_head_marked_another_role_is_named(role: str) -> None:
    """A head marked another role is no longer the banner a screen reader lands on."""
    head = changed('<header class="page-head">', f'<header class="page-head" role="{role}">')
    assert heads.head_faults(head) == [
        f'its head is marked role="{role}": leave the header its own role, the banner'
    ]


def test_a_head_marked_the_banner_passes() -> None:
    """Saying the header's own role again changes nothing."""
    head = changed('<header class="page-head">', '<header class="page-head" role="banner">')
    assert heads.head_faults(head) == []


@pytest.mark.parametrize("role", ["none", "presentation", "generic"])
def test_a_title_marked_no_heading_is_named(role: str) -> None:
    """A title a screen reader isn't told is a heading isn't the page's title to it."""
    head = changed("<h1>", f'<h1 role="{role}">')
    assert heads.head_faults(head) == [
        f'its title is marked role="{role}": leave the <h1> a heading'
    ]


@pytest.mark.parametrize("role", ["main", "region", "article", "navigation", "complementary"])
def test_a_head_inside_an_element_marked_a_section_is_named(role: str) -> None:
    """A header in an element marked a section is as much the section's as in its tag."""
    markup = page(f'<div role="{role}">{written()}</div>')
    assert heads.page_faults(markup) == [
        (
            f'its head sits in a <div role="{role}">, where a header is no banner:'
            " draw it before, as the page's own"
        )
    ]


@pytest.mark.parametrize("section", ["main", "article", "section", "aside", "nav"])
def test_a_head_inside_a_section_is_named(section: str) -> None:
    """A header inside a section is that section's, not the page's banner."""
    markup = (
        f"<!doctype html><html><head>{LINK}</head>"
        f"<body><{section}>{written()}</{section}></body></html>"
    )
    assert heads.page_faults(markup) == [
        (
            f"its head sits in a <{section}>, where a header is no banner:"
            " draw it before, as the page's own"
        )
    ]


@pytest.mark.parametrize(
    ("old", "name"),
    [
        ('<p class="eyebrow">', '<p class="eyebrow">'),
        ('<p class="lede">', '<p class="lede">'),
        ('<p class="standing">', '<p class="standing">'),
        ('<div class="quiet">', '<div class="quiet">'),
    ],
)
def test_a_hidden_part_is_named(old: str, name: str) -> None:
    """A part marked hidden is never drawn or heard, its status least of all."""
    head = changed(old, old.replace(">", " hidden>", 1))
    assert heads.head_faults(head) == [f"its head hides its {name}: draw every part"]


def test_a_hidden_title_is_named() -> None:
    """A hidden title leaves the page with no first heading in its head."""
    assert heads.head_faults(changed("<h1>", "<h1 hidden>")) == [
        "it draws no heading: make the head's title its first",
        "its head hides its <h1>: draw every part",
    ]


def test_a_heading_in_the_head_before_its_title_is_named() -> None:
    """An eyebrow marked a heading is heard as the page's first, before the title."""
    head = changed("your sites", '<span role="heading" aria-level="1">your sites</span>')
    assert heads.head_faults(head) == [
        "its first heading is a <span> in its head: make the head's title its first"
    ]


@pytest.mark.parametrize(
    ("old", "name"),
    [
        ('<p class="eyebrow">', '<p class="eyebrow">'),
        ("<h1>", "<h1>"),
        ('<p class="lede">', '<p class="lede">'),
        ('<div class="quiet">', '<div class="quiet">'),
    ],
)
def test_a_part_screen_readers_never_hear_is_named(old: str, name: str) -> None:
    """A part drawn but aria-hidden or inert says nothing to a screen reader."""
    for marked in (' aria-hidden="true">', " inert>"):
        head = changed(old, old.replace(">", marked, 1))
        assert heads.head_faults(head) == [
            f"its head hides its {name} from screen readers: let them hear every part"
        ], marked


def test_a_head_screen_readers_never_hear_is_named() -> None:
    """An aria-hidden head is drawn, but heard as nothing at all."""
    for marked in (' aria-hidden="true">', " inert>"):
        head = changed('<header class="page-head">', f'<header class="page-head"{marked}')
        assert heads.head_faults(head) == [
            "its head is hidden from screen readers: let them hear it"
        ], marked


def test_a_head_drawn_as_another_element_is_named() -> None:
    """The head is a header, which a screen reader lands on as the banner."""
    head = changed('<header class="page-head">', '<div class="page-head">')
    head = head.replace("</header>", "</div>")
    assert heads.head_faults(head) == ['its head is a <div>: draw it as <header class="page-head">']


def test_a_title_before_the_head_is_named() -> None:
    """The page's first heading is the head's title, not one drawn above it."""
    faults = heads.head_faults("<h1>Above</h1>" + written())
    assert faults == ["its first heading, <h1>, sits outside its head: make the head's title first"]


def test_a_lesser_heading_before_the_head_is_named() -> None:
    """A page's old title kept as an <h2> above the head is still its first heading."""
    faults = heads.head_faults('<div class="mast"><h2>Mine</h2></div>' + written())
    assert faults == ["its first heading, <h2>, sits outside its head: make the head's title first"]


def test_parts_out_of_order_are_named() -> None:
    """A lede before the title breaks the order every page reads in."""
    head = written()
    lede = re.search(r'\s*<p class="lede">.*?</p>', head, re.DOTALL)
    assert lede is not None, "sites.md's head no longer has a lede"
    moved = head.replace(lede.group(0), "", 1).replace(
        '<header class="page-head">', '<header class="page-head">' + lede.group(0), 1
    )
    faults = heads.head_faults(moved)
    assert len(faults) == 1, faults
    assert faults[0].startswith('its head holds <p class="lede">, <p class="eyebrow">'), faults


def test_a_part_drawn_apart_is_named() -> None:
    """A site's own row inside the head is a part the family doesn't have."""
    faults = heads.head_faults(changed("</header>", "<p>Mine too</p></header>"))
    assert len(faults) == 1, faults
    assert ', <div class="quiet">, <p>: hold ' in faults[0], faults


def test_an_eyebrow_of_three_phrases_is_named() -> None:
    """An eyebrow says what this is and whose, never three clauses."""
    head = changed("on this machine</p>", 'on this machine<span class="sep">·</span>now</p>')
    assert heads.head_faults(head) == [f"its eyebrow has 3 phrases: {heads.EYEBROW}"]


def test_an_eyebrow_of_one_phrase_is_named() -> None:
    """An eyebrow with no dot says one thing where it should say two."""
    head = changed('your sites<span class="sep">·</span>on', "your sites on")
    assert heads.head_faults(head) == [f"its eyebrow has 1 phrase: {heads.EYEBROW}"]


def test_an_eyebrow_with_its_dot_hidden_is_one_phrase() -> None:
    """A sep hidden draws no dot, so the eyebrow reads as one phrase, not a dot as text."""
    head = changed('<span class="sep">', '<span class="sep" hidden>')
    assert heads.head_faults(head) == [f"its eyebrow has 1 phrase: {heads.EYEBROW}"]


def test_an_eyebrow_dotted_as_text_is_named() -> None:
    """The pages write their dot as text today; the kit spaces only its own."""
    head = changed('<span class="sep">·</span>', " · ")
    assert heads.head_faults(head) == [
        'its eyebrow writes its dot as text: write it as <span class="sep">·</span>'
    ]


def test_an_eyebrow_dotted_as_text_beside_the_kits_is_named() -> None:
    """A text dot beside the kit's draws three clauses as two."""
    head = changed("your sites<span", "your · sites<span")
    assert heads.head_faults(head) == [
        "its eyebrow writes a dot as text beside the kit's: keep the one sep"
    ]


@pytest.mark.parametrize(
    ("old", "new"),
    [
        ("your sites<span", "<span"),
        ("</span>on this machine", "</span> "),
        ("your sites<span", "<span hidden>your sites</span><span"),
        ("your sites<span", '<span aria-hidden="true">your sites</span><span'),
        (
            'your sites<span class="sep">·</span>',
            '<span aria-hidden="true">your sites<span class="sep">·</span></span>',
        ),
        ("</span>on this machine", "</span><template>on this machine</template>"),
    ],
)
def test_an_eyebrow_with_an_empty_phrase_is_named(old: str, new: str) -> None:
    """A dot with nothing on one side of it is one phrase drawn as two."""
    assert heads.head_faults(changed(old, new)) == [
        f"its eyebrow has an empty phrase: {heads.EYEBROW}"
    ]


# What the gate says of a standing line that doesn't say the feed's state alone.
NO_STATUS = (
    'its standing line needs one polite role="status" for the feed\'s state,'
    " heard, and nothing else live"
)


def test_a_standing_line_with_no_status_is_named() -> None:
    """The feed's state is said through the standing line's status alone."""
    head = changed(' role="status"', "")
    assert heads.head_faults(head) == [NO_STATUS]


@pytest.mark.parametrize(
    ("old", "new"),
    [
        ('role="status"', 'role="alert"'),
        ('role="status"', 'aria-live="assertive"'),
        ('role="status"', 'role="status" aria-live="off"'),
        ('role="status"', 'role="status" aria-live="assertive"'),
        ('role="status"', 'role="status" hidden'),
        ('role="status"', 'role="status" inert'),
        ('role="status"', 'role="status" style="visibility:hidden"'),
        ('<p class="standing">', '<p class="standing" aria-hidden="true">'),
        (
            '<span class="glyph" aria-hidden="true"></span>',
            '<span class="glyph" aria-hidden="true"/>',
        ),
        (
            '<span class="glyph" aria-hidden="true">',
            '<span class="glyph" aria-hidden="false" role="status">',
        ),
    ],
)
def test_a_standing_line_speaking_otherwise_is_named(old: str, new: str) -> None:
    """Another live role, an alert, a status silenced, cutting in, unheard, or two of them.

    A glyph written closed stays open, as a browser leaves it, so the status
    lands under its aria-hidden and is never heard.
    """
    assert heads.head_faults(changed(old, new)) == [NO_STATUS]


@pytest.mark.parametrize(
    ("old", "new"),
    [
        ('role="status"', 'role="wobble status"'),
        ('role="status"', 'role="status" aria-live=""'),
        ("Reaching its daemon.</span>", 'Reaching its daemon.</span><span aria-live="">.</span>'),
        ("Reaching its daemon.</span>", 'Reaching its daemon.</span><span role="timer">1s</span>'),
        ("Reaching its daemon.</span>", 'Reaching its daemon.</span><span role="marquee">.</span>'),
    ],
)
def test_a_status_as_a_browser_reads_it_passes(old: str, new: str) -> None:
    """A role skipped to reach status, an empty aria-live, and a timer or marquee, all off."""
    assert heads.head_faults(changed(old, new)) == []


def test_a_lede_that_speaks_is_named() -> None:
    """A lede rewritten as the feed moves would be heard at every change."""
    head = changed('<p class="lede">', '<p class="lede" aria-live="polite">')
    assert heads.head_faults(head) == [SPEAKS]


# What the gate says of one more part of the head heard as it changes.
SPEAKS = "1 more of its head speaks as it changes: only the standing line's status may be live"


@pytest.mark.parametrize(
    ("old", "new"),
    [
        ("Seven sites", "<output>Seven</output> sites"),
        ('<p class="lede">', '<p class="lede" role="wobble status">'),
        ('<header class="page-head">', '<header class="page-head" aria-live="polite">'),
    ],
)
def test_a_head_speaking_beyond_its_status_is_named(old: str, new: str) -> None:
    """An output is a status, and so is a part marked one, and a head live is heard whole."""
    assert heads.head_faults(changed(old, new)) == [SPEAKS]


def test_a_head_marked_a_status_is_named_twice() -> None:
    """A head marked a status is no banner, and speaks at every change."""
    head = changed('<header class="page-head">', '<header class="page-head" role="status">')
    assert heads.head_faults(head) == [
        'its head is marked role="status": leave the header its own role, the banner',
        SPEAKS,
    ]


# What the gate says of a live region holding the head.
AROUND = "1 around its head speaks as it changes: nothing holding the head may be live"


@pytest.mark.parametrize(
    ("old", "new"),
    [("<body>", '<body aria-live="polite">'), ("<body>", '<body role="log">')],
)
def test_a_live_region_around_the_head_is_named(old: str, new: str) -> None:
    """A live region speaks all it holds, so one around the head speaks every lede."""
    assert heads.page_faults(page(written()).replace(old, new, 1)) == [AROUND]


def test_a_role_list_is_read_as_its_first_known_role() -> None:
    """A browser takes none from none status, so the lede stays quiet."""
    head = changed('<p class="lede">', '<p class="lede" role="none status">')
    assert heads.head_faults(head) == []


def test_a_p_left_open_ends_where_the_next_starts() -> None:
    """An end tag left off, as HTML allows, reads as a browser reads it."""
    head = re.sub(r"</p>(\s*<(p|div|h1))", r"\1", written())
    assert head != written(), "sites.md's head no longer ends a <p> before a block"
    assert heads.head_faults(head) == []


def test_a_break_or_an_svg_drawn_closed_holds_nothing() -> None:
    """A <wbr> in the title, and an icon's path written closed, leave the parts in order."""
    head = changed("Every Site", "Every<wbr> Site")
    head = head.replace(
        '<span class="glyph" aria-hidden="true"></span>',
        '<span class="glyph" aria-hidden="true"><svg><path d="M0 0"/></svg></span>',
    )
    assert heads.head_faults(head) == []


@pytest.mark.parametrize(
    ("markup", "tag", "parent"),
    [
        ("<div><svg/><span>x</span></div>", "span", "div"),
        ("<div><math/><span>x</span></div>", "span", "div"),
        ("<div><p>a<dialog>b</dialog></div>", "dialog", "div"),
        ("<ul><p>a<li>b</li></ul>", "li", "ul"),
        ("<div><p>a<search>b</search></div>", "search", "div"),
    ],
)
def test_markup_is_nested_as_a_browser_nests_it(markup: str, tag: str, parent: str) -> None:
    """SVG and MathML written closed close, and the blocks HTML ends a <p> at end it."""
    element = next(each for each in heads.read(markup).within() if each.tag == tag)
    assert element.parent is not None, f"<{tag}> sits under nothing"
    assert element.parent.tag == parent, f"<{tag}> sits under <{element.parent.tag}>"


# What the gate says of a quiet row its key line doesn't open.
NO_KEYS = 'its quiet row doesn\'t open with a <p class="keys">: put the key line first in it'


def test_a_quiet_row_with_no_key_line_is_named() -> None:
    """The key line opens the quiet row, with the switch under it."""
    head = re.sub(r'<p class="keys">.*?</p>', "", written(), count=1, flags=re.DOTALL)
    assert heads.head_faults(head) == [NO_KEYS]


def test_a_hidden_key_line_is_no_key_line() -> None:
    """A key line marked hidden doesn't open the quiet row it isn't drawn in."""
    head = changed('<p class="keys">', '<p class="keys" hidden>')
    assert heads.head_faults(head) == [NO_KEYS]


def test_a_key_line_under_the_switch_is_named() -> None:
    """A switch drawn above the key line turns the quiet row over."""
    head = changed('<div class="quiet">', '<div class="quiet"><div class="switch"></div>')
    assert heads.head_faults(head) == [NO_KEYS]


@pytest.fixture
def judged(
    tmp_path: Path, capsys: pytest.CaptureFixture[str], monkeypatch: pytest.MonkeyPatch
) -> Callable[..., tuple[int, list[str]]]:
    """Run the gate over pages, each written to a file, with those waiting named."""

    def judge(waiting: str, **pages: str) -> tuple[int, list[str]]:
        named = []
        for name, markup in pages.items():
            path = tmp_path / f"{name}.html"
            path.write_text(markup)
            named.append(f"{name}={path}")
        # As checks.nix writes it, the list one word however short.
        monkeypatch.setattr(sys, "argv", ["heads.py", f"--waiting={waiting}", *named])
        code = heads.main()
        return code, capsys.readouterr().out.splitlines()

    return judge


# A page keeping its own head, as every live page did before the kit's.
OWN = '<header class="head"><h1>Mine</h1></header>'


def test_a_page_waiting_to_move_passes(judged: Callable[..., tuple[int, list[str]]]) -> None:
    """A page on the waiting list may keep its own head until it moves."""
    assert judged("Code", Code=OWN) == (0, [])


def test_a_moved_page_is_held_to_the_head(judged: Callable[..., tuple[int, list[str]]]) -> None:
    """A page off the waiting list is named for each way it strays."""
    code, said = judged("", Code=OWN)
    assert code == 1, said
    assert said[0] == f"Code: {NO_SHEET}", said


def test_an_empty_list_judges_every_page(judged: Callable[..., tuple[int, list[str]]]) -> None:
    """Once the last page moves, the list is empty and every page is held."""
    assert judged("", Code=page(written()), Home=page(written())) == (0, [])


def test_a_page_joining_the_list_is_named(judged: Callable[..., tuple[int, list[str]]]) -> None:
    """A page new to the sites opens with the head; it never waits for it."""
    assert judged("Code,Garden", Code=OWN, Garden=OWN) == (
        1,
        ["Garden joined the waiting list: a page that joins the sites opens with the head"],
    )


def test_a_waiting_page_that_moved_must_leave_the_list(
    judged: Callable[..., tuple[int, list[str]]],
) -> None:
    """The list only shrinks: a page keeping to the head is taken off it."""
    assert judged("Code", Code=page(written())) == (
        1,
        ["Code opens with the kit's head now: take it off the waiting list"],
    )


def test_a_waiting_name_no_page_answers_to_is_named(
    judged: Callable[..., tuple[int, list[str]]],
) -> None:
    """A page that's gone, or misspelt, can't stay on the list unseen."""
    assert judged("Cod", Code=page(written())) == (1, ["Cod waits, but isn't a live page here"])


if __name__ == "__main__":
    sys.exit(pytest.main([__file__]))

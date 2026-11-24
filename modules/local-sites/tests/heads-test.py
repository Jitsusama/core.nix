"""The heads gate passes the kit's own head and names each way a page strays from it.

The head sites.md writes for every live page is the case it must pass, read
from the skill itself, so the gate and the pattern it holds pages to can't
drift apart. Each other case changes one thing about that head and expects
the one line saying what to change back.
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
    """Give a page linking the kit's sheet and opening with `head`."""
    return f"<!doctype html><html><head>{LINK}</head><body><main>{head}</main></body></html>"


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
    ],
)
def test_a_sheet_no_screen_draws_the_head_with_is_named(link: str) -> None:
    """Preloaded, a page's own copy, a path that misses, an alternate, off, or print-only."""
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
    ],
)
def test_a_sheet_from_the_root_or_for_all_media_draws_the_head(link: str) -> None:
    """Home links the kit from the root, and every screen is all media."""
    markup = f"<!doctype html><html><head>{link}</head><body>{written()}</body></html>"
    assert heads.page_faults(markup) == []


@pytest.mark.parametrize(
    "markup",
    [
        f"<!doctype html><html>{LINK}<body>{{head}}</body></html>",
        f"<!doctype html><html><head></head>{LINK}<body>{{head}}</body></html>",
    ],
)
def test_a_sheet_in_a_head_the_parser_implies_draws_the_head(markup: str) -> None:
    """A link before the body goes in the head, written or not, as a parser puts it."""
    assert heads.page_faults(markup.format(head=written())) == []


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
        'it draws a <header class="hero"> above its head: let the kit\'s head be its first'
    ]


def test_a_banner_wrapped_around_the_head_is_named() -> None:
    """A page's own banner holding the kit's head is still drawn first, and heard as the banner."""
    faults = heads.head_faults('<header class="mast"><p>Mine</p>' + written() + "</header>")
    assert faults == [
        'it draws a <header class="mast"> around its head: let the kit\'s head be its first'
    ]


@pytest.mark.parametrize("holder", ["template", "noscript"])
def test_a_head_never_drawn_is_no_head(holder: str) -> None:
    """A head held where no browser draws it leaves the page's own showing."""
    faults = heads.head_faults(f"<{holder}>{written()}</{holder}>")
    assert faults == ['it draws 0 heads: open it with one <header class="page-head">']


def test_a_hidden_head_is_no_head() -> None:
    """A head marked hidden is never drawn, so the page's own shows."""
    head = changed('<header class="page-head">', '<header class="page-head" hidden>')
    faults = heads.head_faults('<div class="hero"><h2>Mine</h2></div>' + head)
    assert faults == ['it draws 0 heads: open it with one <header class="page-head">']


def test_words_loose_in_the_head_are_named() -> None:
    """Every word in the head belongs to one of its parts."""
    head = changed('<p class="eyebrow">', 'Stray words<p class="eyebrow">')
    assert heads.head_faults(head) == [
        "its head writes words outside its parts: keep every word in a part"
    ]


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
    [("your sites<span", "<span"), ("</span>on this machine", "</span> ")],
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
    ],
)
def test_a_status_as_a_browser_reads_it_passes(old: str, new: str) -> None:
    """A role a browser skips to reach status, and an empty aria-live, which is off."""
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
        ('<header class="page-head">', '<header class="page-head" role="status">'),
    ],
)
def test_a_head_speaking_beyond_its_status_is_named(old: str, new: str) -> None:
    """An output is a status, and so is a part or a head marked one, each heard at every change."""
    assert heads.head_faults(changed(old, new)) == [SPEAKS]


# What the gate says of a live region holding the head.
AROUND = "1 around its head speaks as it changes: nothing holding the head may be live"


@pytest.mark.parametrize(
    ("old", "new"),
    [("<main>", '<main aria-live="polite">'), ("<body>", '<body role="log">')],
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

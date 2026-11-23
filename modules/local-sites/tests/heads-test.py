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
    assert heads.page_faults(markup) == ["it doesn't link the kit's head.css, which draws the head"]


def test_a_page_with_no_head_is_named() -> None:
    """A page drawing its own header instead says how many heads it has."""
    faults = heads.head_faults('<header class="hero"><h1>Mine</h1></header>')
    assert faults == ['it draws 0 heads: open it with one <header class="page-head">']


def test_a_head_drawn_as_another_element_is_named() -> None:
    """The head is a header, which a screen reader lands on as the banner."""
    head = changed('<header class="page-head">', '<div class="page-head">')
    head = head.replace("</header>", "</div>")
    assert heads.head_faults(head) == ['its head is a <div>: draw it as <header class="page-head">']


def test_a_title_before_the_head_is_named() -> None:
    """The page's first heading is the head's title, not one drawn above it."""
    faults = heads.head_faults("<h1>Above</h1>" + written())
    assert faults == ["its first <h1> sits outside its head: make it the head's title"]


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
    assert heads.head_faults(head) == [
        'its eyebrow has 3 phrases: say two, what this is <span class="sep">·</span> whose or where'
    ]


def test_a_standing_line_with_no_status_is_named() -> None:
    """The feed's state is said through the standing line's status alone."""
    head = changed(' role="status"', "")
    assert heads.head_faults(head) == [
        'its standing line needs one role="status" for the feed\'s state'
    ]


def test_a_lede_that_speaks_is_named() -> None:
    """A lede rewritten as the feed moves would be heard at every change."""
    head = changed('<p class="lede">', '<p class="lede" aria-live="polite">')
    assert heads.head_faults(head) == [
        "1 more of its head speaks as it changes: only the standing line's status may be live"
    ]


def test_a_quiet_row_with_no_key_line_is_named() -> None:
    """The key line opens the quiet row, with the switch under it."""
    head = re.sub(r'<p class="keys">.*?</p>', "", written(), count=1, flags=re.DOTALL)
    assert heads.head_faults(head) == [
        'its quiet row has no <p class="keys">: put the key line first in it'
    ]


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
        monkeypatch.setattr(sys, "argv", ["heads.py", "--waiting", waiting, *named])
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
    assert said[0] == "Code: it doesn't link the kit's head.css, which draws the head", said


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

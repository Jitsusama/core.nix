"""The web-design skill's looks probe (skill/looks.js), held to its cases in headless Chromium.

Each case serves a small site, runs the probe on its page as checks.md runs
it before a deploy, writes the page as the deploy leaves it, and runs the
probe again with the first run's looks, reading what it names lost. The
first case is the deploy of 18 November 2026 that kept a tracker guide's
words and lost its masthead, heading art and stylesheets. The probe's path
comes from WEB_SKILL and a face to load from FACE, as the check running
this sets them.
"""

import os
import shutil
import threading
from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from typing import TYPE_CHECKING

import pytest
from playwright.sync_api import Browser, sync_playwright

if TYPE_CHECKING:
    from collections.abc import Iterator, Mapping

PROBE = Path(os.environ["WEB_SKILL"]) / "looks.js"
STYLE = """
@font-face { font-family: "Kit Face"; src: url("face.ttf"); }
body { background: rgb(240, 236, 228); color: rgb(30, 30, 30); font-family: "Kit Face", serif; }
h1 { font-family: "Kit Face", serif; }
.masthead { padding: 2rem; }
"""
MARK = '<svg width="32" height="32" aria-label="The mark"><circle cx="16" cy="16" r="8"/></svg>'
HEAD = '<link rel="stylesheet" href="site.css"><script src="site.js"></script>'
MASTHEAD = f'<header class="masthead">{MARK}<h1>The Guide</h1></header>'
ART = (
    '<section id="flow"><h2>How It Flows</h2>'
    '<svg width="200" height="80" data-drawing="flow"><rect width="200" height="80"/></svg>'
    "</section>"
)
WORDS = "<main><p>What each word means.</p></main>"
GUIDE = f"{MASTHEAD}{ART}{WORDS}"
NOTHING = "the page draws nothing of its own: no stylesheet, face, masthead or picture"


class Quiet(SimpleHTTPRequestHandler):
    """Serves a folder, saying nothing of each request."""

    def log_message(self, *_: object) -> None:
        """Say nothing, so the test's output is what it finds."""


def page(head: str, body: str) -> str:
    """Write a page with `head` in its head and `body` as its body."""
    return (
        '<!doctype html><html lang="en"><head><meta charset="utf-8">'
        f"<title>The Guide</title>{head}</head><body>{body}</body></html>"
    )


@pytest.fixture(scope="module")
def browser() -> Iterator[Browser]:
    """One headless Chromium for every case."""
    with sync_playwright() as playwright:
        opened = playwright.chromium.launch()
        yield opened
        opened.close()


class Site:
    """A small site served from a folder, whose page a deploy rewrites."""

    def __init__(self, browser: Browser, root: Path) -> None:
        """Serve `root`, with the kit's stylesheet, face, script and picture in it."""
        self.browser = browser
        self.root = root
        (root / "site.css").write_text(STYLE)
        (root / "site.js").write_text("document.documentElement.dataset.ready = 'yes';\n")
        shutil.copy(os.environ["FACE"], root / "face.ttf")
        (root / "picture.svg").write_text(
            '<svg xmlns="http://www.w3.org/2000/svg" width="40" height="40">'
            '<rect width="40" height="40"/></svg>',
        )
        handler = partial(Quiet, directory=str(root))
        self.server = ThreadingHTTPServer(("127.0.0.1", 0), handler)
        threading.Thread(target=self.server.serve_forever, daemon=True).start()

    def probe(
        self,
        html: str,
        before: list[str] | None = None,
        meant: Mapping[str, str] | None = None,
    ) -> dict[str, list[str]]:
        """Serve `html` as the page and run the probe on it, told `before` and `meant`."""
        (self.root / "guide.html").write_text(html, encoding="utf-8")
        context = self.browser.new_context()
        tab = context.new_page()
        tab.goto(f"http://127.0.0.1:{self.server.server_address[1]}/guide.html")
        if before is not None:
            tab.evaluate(
                "([before, meant]) => { globalThis.looksBefore = before; "
                "globalThis.looksMeant = meant; }",
                [before, meant or {}],
            )
        found: dict[str, list[str]] = tab.evaluate(PROBE.read_text())
        context.close()
        return found


@pytest.fixture
def site(browser: Browser, tmp_path: Path) -> Iterator[Site]:
    """Serve a site of its own for each case."""
    served = Site(browser, tmp_path)
    yield served
    served.server.shutdown()


def test_the_page_says_what_it_draws(site: Site) -> None:
    """A whole page lists its stylesheet, face, type, colours, masthead, mark, art and script."""
    found = site.probe(page(HEAD, f'{GUIDE}<img src="picture.svg" alt="A square">'))
    assert found["broken"] == []
    assert {
        "stylesheet site.css",
        "face Kit Face normal 400",
        "type body Kit Face",
        "type title Kit Face",
        "colour ground rgb(240, 236, 228)",
        "colour ink rgb(30, 30, 30)",
        "masthead",
        "mark",
        "drawing The mark",
        "drawing flow",
        "image picture.svg",
        "script site.js",
    } <= set(found["looks"])


def test_a_plain_shell_loses_the_looks_its_words_kept(site: Site) -> None:
    """The 18 November deploy: the words stay, and the masthead, art and styles are named lost."""
    before = site.probe(page(HEAD, GUIDE))["looks"]
    after = site.probe(page("", WORDS), before)
    assert set(after["lost"]) == {
        "stylesheet site.css",
        "face Kit Face normal 400",
        "type body Kit Face",
        "type title Kit Face",
        "colour ground rgb(240, 236, 228)",
        "colour ink rgb(30, 30, 30)",
        "masthead",
        "mark",
        "drawing The mark",
        "drawing flow",
        "script site.js",
    }
    assert after["broken"] == [NOTHING]


def test_a_stylesheet_still_in_the_markup_that_answers_404_is_lost(site: Site) -> None:
    """A link to a stylesheet the deploy left out is broken, and its looks lost."""
    before = site.probe(page(HEAD, GUIDE))["looks"]
    (site.root / "site.css").unlink()
    after = site.probe(page(HEAD, GUIDE), before)
    assert after["broken"] == ["site.css answered 404"]
    assert {"stylesheet site.css", "face Kit Face normal 400", "type body Kit Face"} <= set(
        after["lost"],
    )


def test_a_loss_named_as_meant_is_said_and_passes(site: Site) -> None:
    """A drawing the deploy meant to take away is named with why, and nothing is lost."""
    before = site.probe(page(HEAD, GUIDE))["looks"]
    after = site.probe(page(HEAD, f"{MASTHEAD}{WORDS}"), before, {"drawing flow": "no flow now"})
    assert (after["lost"], after["meant"], after["broken"]) == (
        [],
        ["drawing flow: no flow now"],
        [],
    )


def test_the_same_page_again_loses_nothing(site: Site) -> None:
    """A deploy that changes only the words keeps every look."""
    before = site.probe(page(HEAD, GUIDE))["looks"]
    for _ in range(10):
        after = site.probe(page(HEAD, GUIDE.replace("each word", "every word")), before)
        assert (after["lost"], after["meant"], after["broken"]) == ([], [], [])
        assert after["looks"] == before


def test_a_drawing_in_a_shut_fold_is_counted_and_the_fold_shut_again(site: Site) -> None:
    """A fold's drawing is measured open, so losing it is seen, and the fold is left shut."""
    fold = f"<details><summary>More</summary>{ART}</details>"
    found = site.probe(page(HEAD, f"{MASTHEAD}{fold}{WORDS}"))
    assert "drawing flow" in found["looks"]
    after = site.probe(
        page(HEAD, f"{MASTHEAD}<details><summary>More</summary></details>"), found["looks"]
    )
    assert after["lost"] == ["drawing flow"]


def test_a_page_that_draws_nothing_is_a_fault(site: Site) -> None:
    """A page with no styles, masthead or picture says so rather than passing quietly."""
    found = site.probe("<!doctype html><html><body><p>Words.</p></body></html>")
    assert "colour ground none" in found["looks"]
    assert found["broken"] == [NOTHING]

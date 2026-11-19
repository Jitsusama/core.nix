"""The web-design skill's looks probe (skill/looks.js), held to its cases in headless Chromium.

Each case serves a small site, runs the probe on its page as checks.md runs
it before a deploy, writes the page as the deploy leaves it, and runs the
probe again with the first run's answer, reading what it names lost. The
second case is the deploy of 18 November 2026 that kept a tracker guide's
words and lost its masthead, heading art and stylesheets. Each run reads
the page in a browser context of its own, with nothing cached, as
checks.md runs it. The probe's path comes from WEB_SKILL and a face to
load from FACE, as the check running this sets them, and its browser from
probe_browser.py, loaded as a plugin.
"""

import os
import shutil
import threading
from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from typing import TYPE_CHECKING, Any

import pytest

if TYPE_CHECKING:
    from collections.abc import Iterator, Mapping

    from playwright.sync_api import Browser

# What a run answers: a before run's looks and broken, an after run's lost,
# meant, broken, still and now.
Answer = dict[str, Any]

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
THE_LOOKS = {
    "stylesheet site.css",
    "face Kit Face normal 400",
    "type body Kit Face",
    "type title Kit Face",
    "colour ground rgb(240, 236, 228)",
    "colour ink rgb(30, 30, 30)",
    "masthead",
    "mark",
    "drawing page 1",
    "drawing flow",
    "script site.js",
}


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
        before: Answer | None = None,
        meant: Mapping[str, str] | None = None,
        then: str = "null",
    ) -> Answer:
        """Serve `html` as the page and run the probe on it, told `before` and `meant`.

        `then` is asked of the page once the probe has answered, and its
        answer is kept as the answer's `then`.
        """
        (self.root / "guide.html").write_text(html, encoding="utf-8")
        with self.browser.new_context() as context:
            tab = context.new_page()
            tab.goto(f"http://127.0.0.1:{self.server.server_address[1]}/guide.html")
            if before is not None:
                tab.evaluate(
                    "([before, meant]) => { globalThis.looksBefore = before; "
                    "globalThis.looksMeant = meant; }",
                    [before, meant or {}],
                )
            found: Answer = tab.evaluate(PROBE.read_text())
            found["then"] = tab.evaluate(then)
        return found

    def close(self) -> None:
        """Stop serving, and give the socket back."""
        self.server.shutdown()
        self.server.server_close()


@pytest.fixture
def site(browser: Browser, tmp_path: Path) -> Iterator[Site]:
    """Serve a site of its own for each case."""
    served = Site(browser, tmp_path)
    yield served
    served.close()


def passed(after: Answer) -> tuple[list[str], list[str], list[str]]:
    """Answer what an after run lost, was told was meant and found newly broken."""
    return (after["lost"], after["meant"], after["broken"])


def test_the_page_says_what_it_draws(site: Site) -> None:
    """A whole page lists its stylesheet, face, type, colours, masthead, mark, art and script."""
    found = site.probe(page(HEAD, f'{GUIDE}<img src="picture.svg" alt="A square">'))
    assert found["broken"] == []
    assert {*THE_LOOKS, "image picture.svg"} <= set(found["looks"])


def test_a_plain_shell_loses_the_looks_its_words_kept(site: Site) -> None:
    """The 18 November deploy: the words stay, and the masthead, art and styles are named lost."""
    before = site.probe(page(HEAD, GUIDE))
    after = site.probe(page("", WORDS), before)
    assert set(after["lost"]) == THE_LOOKS
    assert after["broken"] == [NOTHING]


def test_a_stylesheet_still_in_the_markup_that_answers_404_is_lost(site: Site) -> None:
    """A link to a stylesheet the deploy left out is broken, and its looks lost."""
    before = site.probe(page(HEAD, GUIDE))
    (site.root / "site.css").unlink()
    after = site.probe(page(HEAD, GUIDE), before)
    assert after["broken"] == ["site.css answered 404"]
    assert {"stylesheet site.css", "face Kit Face normal 400", "type body Kit Face"} <= set(
        after["lost"],
    )


def test_a_face_that_never_loads_is_lost_and_broken(site: Site) -> None:
    """A face still declared whose file the deploy left out is broken, and lost."""
    before = site.probe(page(HEAD, GUIDE))
    (site.root / "site.css").write_text(STYLE.replace("face.ttf", "gone.ttf"))
    after = site.probe(page(HEAD, GUIDE), before)
    assert after["lost"] == ["face Kit Face normal 400"]
    assert set(after["broken"]) == {
        "gone.ttf answered 404",
        "face Kit Face normal 400 is declared and won't load",
    }


def test_a_picture_the_deploy_left_out_is_lost_and_broken(site: Site) -> None:
    """An image still in the markup that answers 404 is broken, and lost."""
    before = site.probe(page(HEAD, f'{GUIDE}<img src="picture.svg" alt="A square">'))
    (site.root / "picture.svg").unlink()
    after = site.probe(page(HEAD, f'{GUIDE}<img src="picture.svg" alt="A square">'), before)
    assert passed(after) == (["image picture.svg"], [], ["picture.svg answered 404"])


def test_what_was_broken_before_stays_still_rather_than_failing(site: Site) -> None:
    """A 404 the live page already had is still broken, and fails no deploy."""
    missing = '<img src="missing.svg" alt="Never there">'
    before = site.probe(page(HEAD, f"{GUIDE}{missing}"))
    assert before["broken"] == ["missing.svg answered 404"]
    after = site.probe(page(HEAD, f"{GUIDE}{missing}"), before)
    assert passed(after) == ([], [], [])
    assert after["still"] == ["missing.svg answered 404"]


def test_art_a_stylesheet_paints_is_a_look(site: Site) -> None:
    """A picture drawn by a stylesheet, and the sheet in the page, are lost with it."""
    painted = "<style>.masthead::before { content: ''; background: url(picture.svg); }</style>"
    before = site.probe(page(f"{HEAD}{painted}", GUIDE))
    assert {"stylesheet inline", "art picture.svg"} <= set(before["looks"])
    after = site.probe(page(HEAD, GUIDE), before)
    assert sorted(after["lost"]) == ["art picture.svg", "stylesheet inline"]


def test_a_rebuilt_asset_with_a_new_hash_keeps_its_name(site: Site) -> None:
    """A script whose content hash changed is the same look, not one lost and one new."""
    (site.root / "app.3f9a2c1b.js").write_text("// The first build.\n")
    (site.root / "app.77d0e4a9.js").write_text("// The next build.\n")
    before = site.probe(page(f'{HEAD}<script src="app.3f9a2c1b.js"></script>', GUIDE))
    assert "script app.*.js" in before["looks"]
    after = site.probe(page(f'{HEAD}<script src="app.77d0e4a9.js"></script>', GUIDE), before)
    assert passed(after) == ([], [], [])


def test_a_loss_named_as_meant_is_said_and_passes(site: Site) -> None:
    """A drawing the deploy meant to take away is named with why, and nothing is lost."""
    before = site.probe(page(HEAD, GUIDE))
    after = site.probe(page(HEAD, f"{MASTHEAD}{WORDS}"), before, {"drawing flow": "no flow now"})
    assert passed(after) == ([], ["drawing flow: no flow now"], [])


def test_the_same_page_again_loses_nothing(site: Site) -> None:
    """A deploy that changes only the words keeps every look."""
    before = site.probe(page(HEAD, GUIDE))
    for _ in range(10):
        after = site.probe(page(HEAD, GUIDE.replace("each word", "every word")), before)
        assert passed(after) == ([], [], [])
        assert after["now"] == {"looks": before["looks"], "broken": []}


def test_a_drawing_in_a_shut_fold_is_counted_and_the_fold_shut_again(site: Site) -> None:
    """Folds are measured open, an exclusive group's too, and left shut and grouped again."""
    folds = (
        f'<details name="more"><summary>More</summary>{ART}</details>'
        '<details name="more"><summary>Yet more</summary>'
        '<svg width="40" height="40" id="lexicon"><rect width="40" height="40"/></svg></details>'
    )
    shut = "[...document.querySelectorAll('details')].map((fold) => [fold.open, fold.name])"
    found = site.probe(page(HEAD, f"{MASTHEAD}{folds}{WORDS}"), then=shut)
    assert {"drawing flow", "drawing lexicon"} <= set(found["looks"])
    assert found["then"] == [[False, "more"], [False, "more"]]
    after = site.probe(
        page(HEAD, f"{MASTHEAD}<details><summary>More</summary></details>"),
        found,
    )
    assert sorted(after["lost"]) == ["drawing flow", "drawing lexicon"]


def test_a_page_that_draws_nothing_is_a_fault(site: Site) -> None:
    """A page with no styles, masthead or picture says so rather than passing quietly."""
    found = site.probe("<!doctype html><html><body><p>Words.</p></body></html>")
    assert "colour ground none" in found["looks"]
    assert found["broken"] == [NOTHING]


def test_a_before_that_is_not_a_before_run_is_refused(site: Site) -> None:
    """A before list set the wrong way fails loudly rather than answering as a before run."""
    with pytest.raises(Exception, match="looksBefore is set"):
        site.probe(page(HEAD, GUIDE), {"looks": "stylesheet site.css"})

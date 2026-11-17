"""The web-design skill's homes probe (skill/homes.js), held to its cases in headless Chromium.

Each case writes a small sites folder, a sites.json, any rows.json and
homes.json, and a page, serves it, runs the probe in the page as checks.md
runs it, and reads what it names. The rules a row and a record are held to are the system reader's
(modules/system/src/homes.rs), so a case here is one the reader decides too.
The probe's path comes from WEB_SKILL, as the check running this sets it.
"""

import json
import os
import threading
from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from typing import TYPE_CHECKING

import pytest
from playwright.sync_api import Browser, sync_playwright

if TYPE_CHECKING:
    from collections.abc import Iterator

PROBE = Path(os.environ["WEB_SKILL"]) / "homes.js"
CODE = {"path": "code/", "homes": ["repository"]}
MACHINES = {"path": "mycelium/", "homes": [{"kind": "machine", "address": "#machine={name}"}]}
NOTEBOOK = {
    "kind": "repository",
    "key": "github:example/notebook",
    "name": "example/notebook",
    "names": ["notebook"],
    "address": "code/#repo=example%2Fnotebook",
}
NOTHING = "<p>Nothing named.</p>"


class Quiet(SimpleHTTPRequestHandler):
    """Serves a folder, saying nothing of each request."""

    def log_message(self, *_: object) -> None:
        """Say nothing, so the test's output is what it finds."""


@pytest.fixture(scope="module")
def browser() -> Iterator[Browser]:
    """One headless Chromium for every case."""
    with sync_playwright() as playwright:
        opened = playwright.chromium.launch()
        yield opened
        opened.close()


def probed(
    browser: Browser,
    root: Path,
    sites: list[object],
    page: str,
    rows: object = None,
    **homes: object,
) -> object:
    """Run the probe on `page`, served beside `sites`, any `rows` and each site's homes.json."""
    (root / "sites.json").write_text(json.dumps({"sites": sites}))
    if rows is not None:
        (root / "rows.json").write_text(json.dumps(rows))
    for site, things in homes.items():
        (root / site).mkdir()
        (root / site / "homes.json").write_text(json.dumps({"things": things}))
    (root / "page.html").write_text(
        f'<!doctype html><html><head><meta charset="utf-8"></head><body>{page}</body></html>',
        encoding="utf-8",
    )
    server = ThreadingHTTPServer(("127.0.0.1", 0), partial(Quiet, directory=str(root)))
    threading.Thread(target=server.serve_forever, daemon=True).start()
    try:
        tab = browser.new_page()
        tab.goto(f"http://127.0.0.1:{server.server_address[1]}/page.html")
        found = tab.evaluate(PROBE.read_text())
        tab.close()
    finally:
        server.shutdown()
    return found


def unlinked(name: str, where: str, kind: str, address: str) -> str:
    """Say what the probe says of a mention of `name`, in `where`, with no link home."""
    return f'"{name}" in "{where}" names a {kind} at {address}: link it there'


def test_a_pattern_learns_names_from_links(browser: Browser, tmp_path: Path) -> None:
    """A linked name's bare mention is named; a link off the sites or quoted teaches nothing."""
    page = (
        '<ul><li><a href="/mycelium/#machine=studio">studio</a> is busy.</li>'
        "<li>Then <code>studio</code> idles.</li>"
        '<li><a href="http://far.example/mycelium/#machine=faraway">x</a> <code>faraway</code>'
        '</li><li><blockquote data-said-by="joel"><a href="/mycelium/#machine=quoted">q</a>'
        "</blockquote> <code>quoted</code></li>"
        '<li><a href="/mycelium/#machine=deep&amp;view=x">d</a> <code>deep</code></li></ul>'
        '<svg><a href="/mycelium/#machine=drawn"><text>drawn</text></a></svg>'
        "<p>And <code>drawn</code> too.</p><p>See <code>notebook</code>.</p>"
    )
    assert probed(browser, tmp_path, [CODE, MACHINES], page, code=[NOTEBOOK]) == [
        unlinked("studio", "Then studio idles.", "machine", "mycelium/#machine=studio"),
        unlinked("drawn", "And drawn too.", "machine", "mycelium/#machine=drawn"),
        unlinked("notebook", "See notebook.", "repository", "code/#repo=example%2Fnotebook"),
    ]


def test_a_name_left_raw_is_learned(browser: Browser, tmp_path: Path) -> None:
    """A name a page encodes with encodeURIComponent, which leaves an apostrophe, is read."""
    page = "<p><a href=\"/mycelium/#machine=joe's\">joe's</a></p><p><code>joe's</code> again.</p>"
    assert probed(browser, tmp_path, [MACHINES], page) == [
        unlinked("joe's", "joe's again.", "machine", "mycelium/#machine=joe's"),
    ]


def test_what_the_reader_drops_is_named(browser: Browser, tmp_path: Path) -> None:
    """A row on no plain path, homes not a list, a pattern or a record off its page."""
    sites = [
        CODE,
        {"path": "../up/", "homes": ["thing"]},
        {"path": "loose/", "homes": {"kind": "cell", "address": "#cell={name}"}},
        {"path": "far/", "homes": [{"kind": "net", "address": "https://x/{name}"}]},
        {"path": "odd/", "homes": [{"kind": "tag", "address": "#tag=%zz{name}"}]},
    ]
    stray = {"kind": "repository", "name": "stray", "address": "/elsewhere/#x"}
    assert probed(browser, tmp_path, sites, NOTHING, code=[NOTEBOOK, stray]) == [
        (
            'code/homes.json gives stray "/elsewhere/#x": '
            "an address is code/ and a fragment (#), in plain bytes"
        ),
        (
            '"../up/" in sites.json homes things on no plain path: '
            "steps of letters, digits and -._~, none starting with a dot"
        ),
        (
            '"loose/"\'s homes in sites.json are {"kind":"cell","address":"#cell={name}"}: '
            "a list of kinds and patterns"
        ),
        (
            'far/\'s pattern for net, "https://x/{name}", must be a fragment (#) '
            "on its page holding {name}, in plain bytes"
        ),
    ]


def test_a_kind_is_its_first_rows(browser: Browser, tmp_path: Path) -> None:
    """A kind a row gives by name and by pattern is the pattern's; a later row's is a fault."""
    both = ["machine", {"kind": "machine", "address": "#machine={name}"}]
    sites = [{"path": "mycelium/", "homes": both}, {"path": "health/", "homes": ["machine"]}]
    assert probed(browser, tmp_path, sites, NOTHING) == [
        "machine has two homes, mycelium/ and health/: keep one in sites.json",
    ]


def test_a_page_linking_no_patterns_thing_passes(browser: Browser, tmp_path: Path) -> None:
    """With patterns alone and nothing of theirs linked, there is nothing to measure."""
    assert probed(browser, tmp_path, [MACHINES], "<p><code>studio</code></p>") == []


def test_no_row_home_to_anything_is_a_fault(browser: Browser, tmp_path: Path) -> None:
    """A check that finds nothing to test fails rather than passing."""
    assert probed(browser, tmp_path, [{"path": "notes/", "homes": []}], NOTHING) == [
        "a row in sites.json home to a kind, for any mention to be measured",
    ]


def test_the_homes_rows_are_read_before_the_record(browser: Browser, tmp_path: Path) -> None:
    """rows.json, every row the home lists, is read in place of sites.json when it's there."""
    rows = {"sites": [CODE, {"path": "../up/", "homes": ["thing"]}]}
    unread = [{"path": "../unread/", "homes": ["thing"]}]
    page = "<p>See <code>notebook</code>.</p>"
    assert probed(browser, tmp_path, unread, page, rows, code=[NOTEBOOK]) == [
        (
            '"../up/" in rows.json homes things on no plain path: '
            "steps of letters, digits and -._~, none starting with a dot"
        ),
        unlinked("notebook", "See notebook.", "repository", "code/#repo=example%2Fnotebook"),
    ]


def test_rows_that_list_no_sites_are_a_fault(browser: Browser, tmp_path: Path) -> None:
    """A rows.json there but not the home's rows is named, never passed over for sites.json."""
    assert probed(browser, tmp_path, [CODE], NOTHING, {"rows": []}, code=[NOTEBOOK]) == [
        "rows.json at the sites' root, the home's rows as {\"sites\": [...]}, for the homes",
    ]


def test_a_leading_dot_step_is_named_for_the_system(browser: Browser, tmp_path: Path) -> None:
    """The home lists a row written ./code/, which the system's homes refuse, so it's named."""
    rows = {"sites": [{"path": "./code/", "homes": ["repository"]}]}
    assert probed(browser, tmp_path, [], NOTHING, rows, code=[NOTEBOOK]) == [
        '"./code/" in rows.json homes things, which the system\'s homes refuse: drop its ./',
        "a row in rows.json home to a kind, for any mention to be measured",
    ]


def test_a_shortened_names_tail_is_no_mention(browser: Browser, tmp_path: Path) -> None:
    """The tail after an ellipsis is part of a name cut short; a name before one counts."""
    page = (
        '<ul><li data-home="code/#repo=example%2Fnotebook">'
        "<code>example/notebook</code></li></ul>"
        "<p><code>long.name\u2026notebook</code></p>"
        "<p>Then <code>notebook\u2026</code> trails off.</p>"
    )
    assert probed(browser, tmp_path, [CODE], page, code=[NOTEBOOK]) == [
        unlinked(
            "notebook",
            "Then notebook\u2026 trails off.",
            "repository",
            "code/#repo=example%2Fnotebook",
        ),
    ]

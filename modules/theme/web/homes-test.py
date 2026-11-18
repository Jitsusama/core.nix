"""The web-design skill's homes probe (skill/homes.js), held to its cases in headless Chromium.

Each case writes a small sites folder, a sites.json, any rows.json and
homes.json, and a page, serves it, runs the probe in the page as checks.md
runs it, and reads what it names. The rules a row and a record are held to
are the system reader's (modules/system/src/homes.rs), so a row or a record
the reader would drop is named here. Where the rows come from is the
probe's own: rows.json, which the home writes (modules/local-sites/home),
or sites.json where the sites serve none.
The probe's path comes from WEB_SKILL, as the check running this sets it.
"""

import json
import os
import threading
from functools import partial
from http import HTTPStatus
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from typing import TYPE_CHECKING

import pytest
from playwright.sync_api import Browser, sync_playwright

if TYPE_CHECKING:
    import socket
    from collections.abc import Iterator, Mapping, Sequence
    from socketserver import BaseServer

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
    """Serves a folder, saying nothing of each request, and failing the files named failing."""

    def __init__(
        self,
        request: socket.socket,
        client_address: tuple[str, int],
        server: BaseServer,
        *,
        directory: str,
        failing: Mapping[str, HTTPStatus],
    ) -> None:
        """Serve `directory`, answering each file `failing` names with its status."""
        self.failing = failing
        super().__init__(request, client_address, server, directory=directory)

    def do_GET(self) -> None:
        """Answer a failing file with its status, and the rest from the folder."""
        status = self.failing.get(self.path.removeprefix("/"))
        if status is not None:
            self.send_error(status)
            return
        super().do_GET()

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
    sites: Sequence[object] | str | None,
    page: str,
    rows: str | HTTPStatus | None = None,
    **homes: object,
) -> object:
    """Run the probe on `page`, served beside any `sites`, `rows` as written, and homes.json.

    `sites` written as a string is served as written, and `rows` given as a
    status is what the server answers for rows.json.
    """
    if isinstance(sites, str):
        (root / "sites.json").write_text(sites)
    elif sites is not None:
        (root / "sites.json").write_text(json.dumps({"sites": sites}))
    failing = {"rows.json": rows} if isinstance(rows, HTTPStatus) else {}
    if isinstance(rows, str):
        (root / "rows.json").write_text(rows)
    for site, things in homes.items():
        (root / site).mkdir()
        (root / site / "homes.json").write_text(json.dumps({"things": things}))
    (root / "page.html").write_text(
        f'<!doctype html><html><head><meta charset="utf-8"></head><body>{page}</body></html>',
        encoding="utf-8",
    )
    handler = partial(Quiet, directory=str(root), failing=failing)
    server = ThreadingHTTPServer(("127.0.0.1", 0), handler)
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


def test_a_live_rows_declared_homes_are_read(browser: Browser, tmp_path: Path) -> None:
    """A live row as Nix declares it, in rows.json, stands over its stale sites.json twin."""
    stale = [{"path": "code/", "homes": []}]
    rows = json.dumps({"sites": [CODE]})
    page = "<p>See <code>notebook</code>.</p>"
    assert probed(browser, tmp_path, stale, page, rows, code=[NOTEBOOK]) == [
        unlinked("notebook", "See notebook.", "repository", "code/#repo=example%2Fnotebook"),
    ]


def test_rows_with_no_record_beside_them_are_read(browser: Browser, tmp_path: Path) -> None:
    """A home with declared rows and no sites.json yet still has rows to read."""
    rows = json.dumps({"sites": [CODE]})
    page = "<p>See <code>notebook</code>.</p>"
    assert probed(browser, tmp_path, None, page, rows, code=[NOTEBOOK]) == [
        unlinked("notebook", "See notebook.", "repository", "code/#repo=example%2Fnotebook"),
    ]


def rows_fault(fault: str) -> str:
    """Say what the probe says of a rows.json it can't read, for `fault`."""
    return (
        f"rows.json at the sites' root {fault}: read the home that writes it and the server "
        "that serves it, `journalctl --user -u local-sites-home -u local-sites`"
    )


@pytest.mark.parametrize("rows", ['{"rows": []}', "null", '{"sites": ['])
def test_rows_that_wont_read_are_a_fault(browser: Browser, tmp_path: Path, rows: str) -> None:
    """A rows.json there but of another shape, or broken, is named, never passed over."""
    assert probed(browser, tmp_path, [CODE], NOTHING, rows, code=[NOTEBOOK]) == [
        rows_fault('won\'t read as rows, {"sites": [...]}'),
    ]


def test_rows_the_server_fails_are_a_fault(browser: Browser, tmp_path: Path) -> None:
    """Only a rows.json that isn't there falls back; one the server fails is named."""
    failing = HTTPStatus.INTERNAL_SERVER_ERROR
    found = probed(browser, tmp_path, [CODE], NOTHING, failing, code=[NOTEBOOK])
    assert found == [rows_fault("answered 500 from the sites' server")]


def test_a_record_that_wont_read_beside_rows_is_named(browser: Browser, tmp_path: Path) -> None:
    """A broken sites.json leaves the last build's rows serving, so it's named over them."""
    rows = json.dumps({"sites": [CODE]})
    assert probed(browser, tmp_path, '{"sites": [', NOTHING, rows, code=[NOTEBOOK]) == [
        (
            "sites.json at the sites' root won't read as rows, {\"sites\": [...]}, so the home "
            "kept its last rows.json: fix sites.json"
        ),
    ]


def test_a_row_the_home_left_out_is_named(browser: Browser, tmp_path: Path) -> None:
    """A record's row on no plain path, which the home leaves out of its rows, is named."""
    stray = {"path": "../up/", "homes": ["thing"]}
    rows = json.dumps({"sites": [CODE]})
    assert probed(browser, tmp_path, [CODE, stray], NOTHING, rows, code=[NOTEBOOK]) == [
        (
            '"../up/" in sites.json homes things on no plain path, so the home leaves it out: '
            "steps of letters, digits and -._~, none starting with a dot"
        ),
    ]


def test_a_row_the_home_has_not_built_is_named(browser: Browser, tmp_path: Path) -> None:
    """A record's row missing from rows.json, which the home hasn't built yet, is named."""
    notes = {"path": "notes/", "homes": ["note"]}
    rows = json.dumps({"sites": [CODE]})
    assert probed(browser, tmp_path, [CODE, notes], NOTHING, rows, code=[NOTEBOOK]) == [
        (
            "\"notes/\" in sites.json isn't in rows.json, so the home hasn't built since it "
            "was written: read `journalctl --user -u local-sites-home`"
        ),
    ]


def test_no_record_at_all_is_a_fault(browser: Browser, tmp_path: Path) -> None:
    """Sites serving neither rows.json nor sites.json have no homes to read."""
    assert probed(browser, tmp_path, None, NOTHING) == [
        "sites.json at the sites' root, for the homes to be read from",
    ]


def test_a_leading_dot_step_is_named_for_code(browser: Browser, tmp_path: Path) -> None:
    """The home lists a row written ./code/, which Code's homes refuse, so it's named."""
    dotted = [{"path": "./code/", "homes": ["repository"]}]
    rows = json.dumps({"sites": dotted})
    assert probed(browser, tmp_path, dotted, NOTHING, rows, code=[NOTEBOOK]) == [
        (
            '"./code/" in rows.json homes things, which Code\'s homes refuse: '
            "drop its ./ in sites.json"
        ),
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

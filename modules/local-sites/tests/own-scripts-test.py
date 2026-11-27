"""What own-scripts.py refuses a live page, and what it leaves be."""

import importlib.util
import os
from typing import TYPE_CHECKING, cast

import pytest

if TYPE_CHECKING:
    from pathlib import Path

# The check as localSite's build runs it, named by the gate that runs this.
SPEC = importlib.util.spec_from_file_location("own_scripts", os.environ["OWN_SCRIPTS"])
assert SPEC is not None, "OWN_SCRIPTS names no Python file"
assert SPEC.loader is not None, "OWN_SCRIPTS names no file Python can load"
own_scripts = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(own_scripts)

# A page as the live sites write one, its scripts served beside it.
OWN = """<!doctype html>
<html lang="en"><head><script src="../local-sites/kit/early.js"></script>
<script type="application/ld+json">{"@type": "WebSite"}</script></head>
<body><a href="#main">Skip</a><svg><a xlink:href="#top"/></svg>
<script type="module" src="site.js"></script></body></html>
"""


def judged(folder: Path, body: str) -> list[str]:
    """Say what the check finds in a page holding `body` in `folder`."""
    (folder / "index.html").write_text(f"<!doctype html><body>\n{body}\n</body>")
    # Loaded by its path, the check is a module mypy can't read the types of.
    return cast("list[str]", own_scripts.faults(folder))


def test_a_page_of_its_own_scripts_passes(tmp_path: Path) -> None:
    """A page whose every script is a file, or data, holds nothing to refuse."""
    (tmp_path / "index.html").write_text(OWN)
    assert own_scripts.faults(tmp_path) == [], "a page of its own scripts was refused"


@pytest.mark.parametrize(
    ("body", "said"),
    [
        ("<script>document.title = 'x';</script>", "2: a script written into the page"),
        ('<script type="module">import "./a.js";</script>', "2: a script written into the page"),
        ('<script type="importmap">{}</script>', "2: a script written into the page"),
        ('<button onclick="go()">Go</button>', "2: onclick=, a handler written as an attribute"),
        ('<img src="a.png" ONERROR="go()">', "2: onerror=, a handler written as an attribute"),
        ('<a href=" java\tscript:go()">Go</a>', "2: a javascript: address in href="),
        ('<form action="JavaScript:go()"></form>', "2: a javascript: address in action="),
        ('<svg><a xlink:href="javascript:go()"/></svg>', "2: a javascript: address in xlink:href="),
    ],
)
def test_a_script_the_policy_won_t_run_is_refused(tmp_path: Path, body: str, said: str) -> None:
    """Each kind the policy turns down is named by its page and line, with what to do."""
    assert judged(tmp_path, body) == [f"index.html:{said}; {own_scripts.INSTEAD}"], body


def test_only_the_pages_at_the_folder_s_top_are_read(tmp_path: Path) -> None:
    """The kit a site carries is its own build's to hold, so only the site's pages are read."""
    (tmp_path / "kit").mkdir()
    (tmp_path / "kit" / "index.html").write_text("<script>go()</script>")
    (tmp_path / "index.html").write_text(OWN)
    assert own_scripts.faults(tmp_path) == [], "a page below the site's top was read"

"""Whether the map this machine serves reaches every cell of its realm.

The same reading map-reads-live-test.py makes of a stand-in, made of the
live page at MAP_CHECK_ROOT: the map opened with `?reach` says, once it
has drawn the node's reading, which cells it misses and why (reach.ts).
What it said is written to MAP_CHECK_SAID, for mycelium-map-check
(map-check.sh) to print; nothing is written when the map never read the
node.
"""

import json
import os
import re
from collections.abc import Iterator
from pathlib import Path

import pytest
from playwright.sync_api import Browser, ViewportSize, expect, sync_playwright

ROOT = os.environ["MAP_CHECK_ROOT"]
SAID = Path(os.environ["MAP_CHECK_SAID"])
# How long the map may take to hear the node and draw its whole realm.
LIVE_MS = 60_000
SIZE: ViewportSize = {"width": 1440, "height": 900}


@pytest.fixture
def browser() -> Iterator[Browser]:
    """Headless Chromium in its own sandbox, since it draws what the realm's peers wrote."""
    with sync_playwright() as playwright:
        launched = playwright.chromium.launch(
            chromium_sandbox=True, args=["--host-resolver-rules=MAP sites.localhost 127.0.0.1"]
        )
        yield launched
        launched.close()


def test_the_map_on_this_machine_reaches_every_cell(browser: Browser) -> None:
    context = browser.new_context(viewport=SIZE)
    page = context.new_page()
    page.goto(f"{ROOT}mycelium/?reach")
    expect(page.locator("#asof"), "the map never read the node").to_have_attribute(
        "data-state", "live", timeout=LIVE_MS
    )
    html = page.locator("html")
    expect(html, "the map never said its reach").to_have_attribute(
        "data-reach", re.compile("."), timeout=LIVE_MS
    )
    said = json.loads(html.get_attribute("data-reach") or "{}")
    context.close()
    SAID.write_text(json.dumps(said))
    assert said["failures"] == [], f"the map misses {said['count']} cells"

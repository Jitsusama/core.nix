"""The headless Chromium every test of the web-design skill's probes reads its pages in.

Each probe's test (homes-test.py, looks-test.py) loads this as a pytest
plugin (`-p probe_browser`), with this folder on Python's path as its check
sets it, and takes the `browser` fixture it gives, so the browser is opened
one way, once per test file.
"""

from typing import TYPE_CHECKING

import pytest
from playwright.sync_api import Browser, sync_playwright

if TYPE_CHECKING:
    from collections.abc import Iterator


@pytest.fixture(scope="module")
def browser() -> Iterator[Browser]:
    """One headless Chromium for every case."""
    with sync_playwright() as playwright:
        # The build sandbox's /dev/shm is small, and a tab that outgrows it
        # crashes mid-load, so the browser keeps that memory in a temporary
        # file instead, as the live pages' tests do (live_page.py).
        opened = playwright.chromium.launch(args=["--disable-dev-shm-usage"])
        yield opened
        opened.close()

"""What every live page's browser test shares: the sites' server, Chromium and a tab.

A page's test serves the page as Nix builds it through the real local-sites
server, beside a stand-in daemon (stand_in_daemon.py) in place of the one its
feed reads, and reads it in headless Chromium. The server, the browser, a tab
keeping what the page logs and the web-design skill's page checks are the same
for every page, so they live here once and each page's test says only what its
page holds. A test loads this as a pytest plugin (`-p live_page`), which gives
it the `browser` fixture. The server and the kit come from LOCAL_SITES and
LOCAL_SITES_KIT, the home's strip styles from STRIP and the skill from
WEB_SKILL, as the check running the test sets them.
"""

import json
import os
import socket
import subprocess
import time
import warnings
from contextlib import contextmanager
from pathlib import Path
from typing import TYPE_CHECKING

import pytest
from playwright.sync_api import (
    Browser,
    BrowserContext,
    ConsoleMessage,
    Error,
    Page,
    sync_playwright,
)

if TYPE_CHECKING:
    from collections.abc import Iterator, Mapping

# How long the sites' server has to start listening.
STARTING_S = 5
# What Playwright says when a tab's renderer died under it, rather than the page
# failing. Should a later Playwright word it otherwise, a crash fails the test at
# once, as it did before Opened retried it.
CRASHED = "Page crashed"
# A phone's width and a desk's, the two the skill's checks are held at.
SIZES = [(390, 844), (1440, 900)]
LOOKS = ["dark", "light"]
# The web-design skill's checks a published page passes, each answering [] when it does.
PAGE_CHECKS = ["gate.js", "layout.js", "svgtext.js"]

type Served = tuple[str, subprocess.Popen[bytes]]


def a_free_port() -> int:
    """Give a port nothing on this machine listens on."""
    with socket.socket() as probe:
        probe.bind(("127.0.0.1", 0))
        return int(probe.getsockname()[1])


@contextmanager
def serving(
    folder: Path, mounts: Mapping[str, str], feeds: Mapping[str, object]
) -> Iterator[Served]:
    """Run the sites' server on `mounts` and `feeds`, giving its root and its process."""
    port = a_free_port()
    documents = folder / "documents"
    documents.mkdir()
    # The home, among the documents, styles the strip over a mounted page and
    # says what the sites hold, which a page's checks ask of its root.
    (documents / "strip.css").write_text(Path(os.environ["STRIP"]).read_text())
    (documents / "llms.txt").write_text("# Sites\n\nThe sites on this machine.\n")
    config = {
        "listen": f"127.0.0.1:{port}",
        "names": ["sites.localhost"],
        "documents": str(documents),
        "mounts": {**mounts, "local-sites": os.environ["LOCAL_SITES_KIT"]},
        "feeds": feeds,
    }
    (folder / "config.json").write_text(json.dumps(config))
    server = subprocess.Popen([os.environ["LOCAL_SITES"], "--config", str(folder / "config.json")])
    deadline = time.monotonic() + STARTING_S
    while True:
        try:
            socket.create_connection(("127.0.0.1", port), timeout=1).close()
            break
        except OSError:
            assert time.monotonic() < deadline, f"the sites' server never listened on {port}"
            time.sleep(0.05)
    try:
        yield f"http://sites.localhost:{port}/", server
    finally:
        server.kill()
        server.wait()


@pytest.fixture
def browser() -> Iterator[Browser]:
    """Headless Chromium, reaching the sites by the name the server insists on."""
    with sync_playwright() as playwright:
        # The page is reached by the name a browser sends to this machine by
        # itself, with no resolver in the sandbox to say where it is.
        launched = playwright.chromium.launch(
            # The build sandbox's /dev/shm is small, and a tab that outgrows it
            # crashes mid-load ("Page crashed"), so the browser keeps that memory
            # in a temporary file instead. A busy builder still crashes one now
            # and then, which Opened opens once more.
            args=["--host-resolver-rules=MAP sites.localhost 127.0.0.1", "--disable-dev-shm-usage"]
        )
        yield launched
        launched.close()


class Opened:
    """A tab on a page, keeping every error and warning the page logs."""

    context: BrowserContext
    page: Page

    def __init__(self, browser: Browser, address: str, size: tuple[int, int], look: str) -> None:
        """Open `address` at `size` in `look`, listening before it loads."""
        self.size, self.look = size, look
        self.logged: list[str] = []
        # Words a test expects the page to log from some line on, each with
        # where in the log it began expecting them (see expecting).
        self.expected: list[tuple[str, int]] = []
        try:
            self._open(browser, address)
        except Error as error:
            # A renderer dies mid-load on a builder short of memory, with no
            # word from the page, so the tab is opened once more in a fresh
            # context. A page that crashes its tab itself crashes it again and
            # fails here. What the dead tab logged is dropped, since the second
            # load logs again whatever the page says, but the warning keeps it
            # in pytest's summary, so a page that starts crashing is seen. Only
            # the load is retried: a tab that dies later fails the test.
            if CRASHED not in error.message:
                raise
            self.context.close()
            warnings.warn(
                f"the tab on {address} at {size} in {look} crashed and was opened once more, "
                f"having logged {self.logged}",
                stacklevel=2,
            )
            self.logged.clear()
            self._open(browser, address)

    def _open(self, browser: Browser, address: str) -> None:
        width, height = self.size
        self.context = browser.new_context(
            viewport={"width": width, "height": height},
            color_scheme="dark" if self.look == "dark" else "light",
        )
        self.page = self.context.new_page()
        self.page.on("console", self._heard)
        self.page.on("pageerror", lambda error: self.logged.append(f"thrown: {error}"))
        self.page.goto(address)

    def _heard(self, message: ConsoleMessage) -> None:
        if message.type in {"error", "warning"}:
            self.logged.append(f"{message.type}: {message.text}")

    def expecting(self, words: str) -> None:
        """Let the browser log `words` from now on, without holding it against the page.

        For a line a test causes, such as the browser's own for a WebSocket that
        couldn't connect once the server is taken away. It never covers what was
        logged before, nor anything the page threw.
        """
        self.expected.append((words, len(self.logged)))

    def _expected(self, at: int, line: str) -> bool:
        if line.startswith("thrown: "):
            return False
        return any(words in line and at >= since for words, since in self.expected)

    def passes_the_skills_checks(self) -> None:
        """Hold the page to the web-design skill's checks, each finding nothing."""
        skill = Path(os.environ["WEB_SKILL"])
        for check in PAGE_CHECKS:
            found = self.page.evaluate((skill / check).read_text())
            assert found == [], f"{check} at {self.size} in the {self.look} look found {found}"

    def close(self) -> None:
        """Close the tab, failing if the page logged anything while it was open."""
        self.context.close()
        unexpected = [line for at, line in enumerate(self.logged) if not self._expected(at, line)]
        assert unexpected == [], f"the page logged {unexpected}"

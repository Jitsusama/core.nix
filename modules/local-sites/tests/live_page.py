"""What every live page's browser test shares: the sites' server, Chromium and a tab.

A page's test serves the page as Nix builds it through the real local-sites
server, beside a stand-in daemon (stand_in_daemon.py) in place of the one its
feed reads, and reads it in headless Chromium. The server, the browser, a tab
keeping what the page logs and the web-design skill's page checks are the same
for every page, so they live here once and each page's test says only what its
page holds. A test loads this as a pytest plugin (`-p live_page`), which gives
it the `browser` fixture. The server and the kit come from LOCAL_SITES and
LOCAL_SITES_SHARED (what every site shares: the kit, and the theme's web kit
as web/), the home's strip styles from STRIP and the skill from
WEB_SKILL, as the check running the test sets them. Each page also keeps to
a budget (Budget, Opened.keeps_to), so the cost Joel asked to be cut on 17
November can't creep back unseen.
"""

import json
import os
import socket
import subprocess
import time
import warnings
from contextlib import contextmanager
from dataclasses import dataclass
from pathlib import Path
from typing import TYPE_CHECKING
from urllib.parse import urlsplit

import pytest
from playwright.sync_api import (
    Browser,
    BrowserContext,
    ConsoleMessage,
    Error,
    Page,
    Request,
    WebSocket,
    sync_playwright,
)

if TYPE_CHECKING:
    from collections.abc import Callable, Iterator, Mapping

# How long the sites' server has to start listening.
STARTING_S = 5
# What Playwright says when a tab's renderer died under it, rather than the page
# failing: the first while it loads, the second to an expect waiting on it.
# Should a later Playwright word it otherwise, a crash fails the test at once,
# as it did before Opened retried it.
CRASHES = ("Page crashed", "Target crashed")
# A phone's width and a desk's, the two the skill's checks are held at.
SIZES = [(390, 844), (1440, 900)]
LOOKS = ["dark", "light"]
KILOBYTE = 1024
# The browser's own lines once the sites' server is gone: the feed's next try to
# connect, and a file or fetch the server was answering at the kill or is asked
# for after. Only the errors a server going away gives, so a stale page that
# fails some other way, such as a name that won't resolve, still fails. The
# browser's line names no address, so one refused at another port passes too.
SERVER_GONE = [
    "WebSocket connection to",
    *(
        f"Failed to load resource: net::ERR_{error}"
        for error in [
            "CONNECTION_REFUSED",
            "CONNECTION_RESET",
            "CONNECTION_ABORTED",
            "CONNECTION_CLOSED",
            "EMPTY_RESPONSE",
            "CONTENT_LENGTH_MISMATCH",
            "INCOMPLETE_CHUNKED_ENCODING",
        ]
    ),
]
# What the server answers a file the browser already holds unchanged.
NOT_MODIFIED = 304
OK = 200
# The tags a page draws most of, most first, to say what grew past a budget.
MOST_DRAWN = """() => {
  const counts = {};
  for (const each of document.getElementsByTagName('*')) {
    counts[each.localName] = (counts[each.localName] ?? 0) + 1;
  }
  return Object.entries(counts).sort((a, b) => b[1] - a[1]).slice(0, 5);
}"""
# The web-design skill's checks a published page passes, each answering [] when it does.
PAGE_CHECKS = ["gate.js", "layout.js", "svgtext.js"]
# A desk as wide as the laptop's monitor, past SIZES' desk, for a page whose head is still
# fluid there.
WIDE_DESK = (1920, 1080)
# Each of the head's parts, by its id, laid out with the words index.html opens with, with the
# words drawn live, and with each of `forms`: their heights in pixels and the part's line
# height. Each is put in for a moment and the live words put back, so all are laid out in the
# same box, and only once the webfonts are in, so no fallback face is measured.
HEAD_HEIGHTS = """async ([ids, forms]) => {
  await document.fonts.ready;
  const opening = new DOMParser().parseFromString(
    await (await fetch(location.href)).text(), "text/html");
  return Object.fromEntries(ids.map((id) => {
    const part = document.getElementById(id);
    const live = part.innerHTML;
    const tall = (words) => {
      part.innerHTML = words;
      return Math.round(part.getBoundingClientRect().height);
    };
    const said = {
      opening: tall(opening.getElementById(id).innerHTML),
      forms: (forms[id] ?? []).map(tall),
      live: tall(live),
      line: parseFloat(getComputedStyle(part).lineHeight),
    };
    return [id, said];
  }));
}"""

type Served = tuple[str, subprocess.Popen[bytes]]


@dataclass(frozen=True)
class Budget:
    """What a page may cost a first visit: the files and bytes it asks for, the elements it draws.

    Each is set a little above what the page costs now, measured under its test's
    stand-in, so a page that grows past it says so and its owner decides whether
    the cost is worth it, raising the budget in the same change if it is. Bytes
    are the bodies the server sent, which a build fixes, so they don't vary.
    """

    files: int
    kilobytes: int
    elements: int


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
        "mounts": {**mounts, "local-sites": os.environ["LOCAL_SITES_SHARED"]},
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


def _crashed(error: Exception) -> bool:
    # Playwright's own error says a crash in its message. A failed expect says
    # it on a line of its own, after the text the page drew, so only that line
    # counts: a page that draws the words has not crashed.
    if isinstance(error, Error):
        return any(words in error.message for words in CRASHES)
    lines = {line.strip() for line in str(error).splitlines()}
    return any(f"Error: {words}" in lines for words in CRASHES)


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
        self.browser = browser
        try:
            self._open(browser, address)
        except Error as error:
            if not _crashed(error):
                raise
            self._open_again()

    def first(self, reading: Callable[[Page], None]) -> None:
        """Take the page's first reading, opening the tab once more if it crashed meanwhile.

        A renderer can die just after the load, while the page draws what its
        feed first sent, so a reading taken before a test tells its stand-in
        anything can be taken again in a fresh tab. A tab that dies after it
        fails the test, as does one that dies again at its second reading, and
        a reading that fails for any other reason fails at once.
        """
        try:
            reading(self.page)
        except (AssertionError, Error) as error:
            if not _crashed(error):
                raise
            self._open_again()
            reading(self.page)

    def _open_again(self) -> None:
        # A renderer dies on a builder short of memory, with no word from the
        # page, so the tab is opened once more in a fresh context, but only
        # once for each of the load and the first reading: a page that crashes
        # its tab itself crashes it again and fails. What the dead tab logged
        # is dropped, since the second load logs again whatever the page says,
        # and what a test expected is expected from the new log's start; the
        # warning keeps it in pytest's summary.
        self.context.close()
        warnings.warn(
            f"the tab on {self.address} at {self.size} in {self.look} crashed and was opened "
            f"once more, having logged {self.logged}",
            stacklevel=3,
        )
        self.logged.clear()
        self.expected = [(words, 0) for words, _ in self.expected]
        self._open(self.browser, self.address)

    def _open(self, browser: Browser, address: str) -> None:
        width, height = self.size
        self.context = browser.new_context(
            viewport={"width": width, "height": height},
            color_scheme="dark" if self.look == "dark" else "light",
        )
        self.page = self.context.new_page()
        self.address = address
        # Whether the skill's checks have opened the page's folds, after which
        # its shift is theirs as much as the page's (see holds_every_pixel).
        self.unfolded = False
        # Every file the tab asks for, and every socket it opens, from its first
        # load on, which keeps_to reads.
        self.asked: list[Request] = []
        self.sockets: list[str] = []
        self.page.on("request", self._asked)
        self.page.on("websocket", self._opened)
        self.page.on("console", self._heard)
        self.page.on("pageerror", lambda error: self.logged.append(f"thrown: {error}"))
        self.page.goto(address)

    def _asked(self, request: Request) -> None:
        self.asked.append(request)

    def _opened(self, socket: WebSocket) -> None:
        self.sockets.append(socket.url)

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

    def loses(self, server: subprocess.Popen[bytes]) -> None:
        """Take the sites' `server` away, letting the browser say so from then on.

        One step, so no case can kill the server before it says what to expect.
        """
        for words in SERVER_GONE:
            self.expecting(words)
        server.kill()
        server.wait()

    def _expected(self, at: int, line: str) -> bool:
        if line.startswith("thrown: "):
            return False
        return any(words in line and at >= since for words, since in self.expected)

    def passes_the_skills_checks(self) -> None:
        """Hold the page to the web-design skill's checks, each finding nothing."""
        skill = Path(os.environ["WEB_SKILL"])
        self.unfolded = True
        for check in PAGE_CHECKS:
            found = self.page.evaluate((skill / check).read_text())
            assert found == [], f"{check} at {self.size} in the {self.look} look found {found}"

    def holds_every_pixel(self) -> None:
        """Hold the page to the pixels, words and names probes: width, shift, type, contrast, names.

        A page's test calls this once its page keeps to the rules, so each page
        takes the probe as it's moved onto them (1HE06W's plan of 22 November
        2026), rather than every page failing at once before it's moved. The
        probe counts every layout shift since the load, waiting for the page
        to go quiet, so call this once the page's first reading is drawn, as
        its own wait for that says, and before anything presses, scrolls or
        opens a fold, passes_the_skills_checks among them, whose layout.js
        opens each one. Called sooner, a feed answering after half a second's
        quiet lands unmeasured. Called after them, it refuses, rather than
        name a shift the page never made.
        """
        assert not self.unfolded, (
            "holds_every_pixel ran after passes_the_skills_checks, whose layout.js opened "
            "every fold, so the shift it would count is theirs: call it first"
        )
        for probe in ("pixels.js", "floor.js", "words.js", "names.js"):
            found = self.page.evaluate((Path(os.environ["WEB_SKILL"]) / probe).read_text())
            assert found == [], f"{probe} at {self.size} in the {self.look} look found {found}"

    def keeps_to(self, budget: Budget) -> None:
        """Hold the drawn page to `budget`, then visit it again and find nothing sent twice.

        A first visit asks only the sites' server, files and sockets alike, for
        no more files and bytes than the budget says, and draws no more
        elements. A second visit asks only that server too, and the server
        answers every file it asks for as unchanged (304), so no body comes
        again. Each visit is counted once the network has gone quiet, so a file
        asked for late is counted, and none of the first visit's runs into the
        second's. Call it once the page is drawn.
        """
        self.page.wait_for_load_state("networkidle")
        self._stays_home("a first visit")
        asked = [request.url for request in self.asked]
        assert len(asked) <= budget.files, (
            f"a first visit asked for {len(asked)} files, past its budget of {budget.files}: "
            f"{asked}"
        )
        sent = sum(
            request.sizes()["responseBodySize"] for request in self.asked if request.response()
        )
        assert sent <= budget.kilobytes * KILOBYTE, (
            f"a first visit was sent {sent / KILOBYTE:.1f} KB, past its budget of "
            f"{budget.kilobytes} KB"
        )
        elements = self.page.evaluate("document.getElementsByTagName('*').length")
        assert elements <= budget.elements, (
            f"the page drew {elements} elements, past its budget of {budget.elements}, "
            f"most of them {self.page.evaluate(MOST_DRAWN)}"
        )
        self.asked.clear()
        self.sockets.clear()
        self.page.reload(wait_until="load")
        self.page.wait_for_load_state("networkidle")
        self._stays_home("a second visit")
        assert self.asked, "a second visit asked for nothing, so it can't say what came again"
        again = [answer for request in self.asked if (answer := self._sent_again(request))]
        assert again == [], (
            f"a second visit had these files answered other than as unchanged: {again}"
        )

    @staticmethod
    def _sent_again(request: Request) -> tuple[str, int | None, int] | None:
        """Say a file the server answered other than as unchanged, with its status and body.

        The server answers 304, which Chromium reports as it is for a page's own
        files, but as the 200 it holds for the page itself and for a fetch that
        asks again, with no body come over the wire. So a 304, or a 200 with no
        body, is unchanged; a file unanswered, or sent with a body, is not.
        """
        answer = request.response()
        if answer is None:
            return (request.url, None, 0)
        body = request.sizes()["responseBodySize"]
        if answer.status == NOT_MODIFIED or (answer.status == OK and body == 0):
            return None
        return (request.url, answer.status, body)

    def _stays_home(self, visit: str) -> None:
        origin = urlsplit(self.address).netloc
        files = [request.url for request in self.asked]
        elsewhere = [url for url in files + self.sockets if urlsplit(url).netloc != origin]
        assert elsewhere == [], f"{visit} reached beyond the sites' server for {elsewhere}"

    def close(self) -> None:
        """Close the tab, failing if the page logged anything while it was open."""
        self.context.close()
        unexpected = [line for at, line in enumerate(self.logged) if not self._expected(at, line)]
        assert unexpected == [], f"the page logged {unexpected}"

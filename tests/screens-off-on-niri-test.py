"""Every screen goes off from the launcher by niri's own power-off, on niri.

modules/quickshell/checks/displays.nix runs this through pytest in tests/nested-niri.sh, with the
shell launcher_on_niri.py starts once a run. Turn Every Screen Off and :set noscreens each ask
niri to power its monitors off, which keeps every output connected and enabled, so a monitor's
hub and network card stay powered (Joel, 2026-10-24: "is there a way to turn my external
display off without shutting down its hub or network card or anything? i want the laptop to be
able to keep going without blinding me in my sleep"). A nested niri draws nothing once its
monitors are off and says nothing of it, so the requests are read on their way to niri, through a
proxy on niri's socket that the shell and the windows daemon, which turns every screen off since
11 November 2026, are started on.

The shell is started as on a niri that powers one output off by name
(modules/niri/powers-off-each-output.patch), so a screen's own Turn It Off asks for that one by
its name (Joel, 2026-11-07: "i want to be able to control the screen that gets shut off, one,
the other, all"). What each screen offers is held in Qt's engine (tests/screens.qml); here, only
that the act reaches niri.

A change to a screen niri refuses says so on the : line, in niri's own words for why, where
Return otherwise looked done and only the shell's log said otherwise. niri refuses a screen's
change only where it can't read it or won't make its mode, which the windows daemon never asks
for, so the proxy turns the daemon's request into niri's own ReturnError, which niri answers with
its example error. What the daemon says of each refusal, and of a screen gone, is held in its
own tests (modules/windows, screens/acts.rs and screens/line.rs); here, only that niri's words
reach the line.
"""

from __future__ import annotations

import os
import socket
import threading
import time
from contextlib import contextmanager
from pathlib import Path
from typing import TYPE_CHECKING

import launcher_on_niri
import pytest

if TYPE_CHECKING:
    from collections.abc import Callable, Iterator

    from launcher_on_niri import Shell

# How long the shell may take to ask, past the moment it waits for the key to come up.
ASKED = 10.0
POLL = 0.05
CHUNK = 65536
# What niri answers its ReturnError request with, as its src/ipc/server.rs says it.
NIRIS_EXAMPLE_ERROR = "example compositor error"


class Proxy:
    """niri's socket, passed through, keeping every byte a client sent it.

    While a test has it refusing what a request holds, that request reaches niri as ReturnError,
    which niri refuses, so it answers in the request's place in line.
    """

    def __init__(self, niri: str, path: Path) -> None:
        """Listen at the path, passing each connection on to niri's own socket."""
        self.niri = niri
        self.path = path
        self.sent = bytearray()
        self.refused: bytes | None = None
        self.lock = threading.Lock()
        self.server = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.server.bind(str(path))
        self.server.listen()
        threading.Thread(target=self.accept, daemon=True).start()

    def accept(self) -> None:
        """Take each client as it connects, for as long as the run lasts."""
        while True:
            client, _ = self.server.accept()
            upstream = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
            upstream.connect(self.niri)
            threading.Thread(target=carry, args=(client, upstream, self.keep), daemon=True).start()
            threading.Thread(target=carry, args=(upstream, client, unchanged), daemon=True).start()

    def keep(self, chunk: bytes) -> bytes:
        """Keep what a client sent niri, and give what goes on to it.

        Each request is a line of its own, sent whole, so a chunk holds whole lines.
        """
        with self.lock:
            self.sent.extend(chunk)
            refused = self.refused
        if refused is None:
            return chunk
        return b"\n".join(
            b'"ReturnError"' if refused in line else line for line in chunk.split(b"\n")
        )

    @contextmanager
    def refusing(self, asked: bytes) -> Iterator[None]:
        """Have niri refuse each request holding what's asked, until the block ends."""
        with self.lock:
            self.refused = asked
        try:
            yield
        finally:
            with self.lock:
                self.refused = None

    def since(self, mark: int) -> bytes:
        """Give what clients have sent niri since the mark."""
        with self.lock:
            return bytes(self.sent[mark:])

    def mark(self) -> int:
        """Give where what clients have sent ends now."""
        with self.lock:
            return len(self.sent)


def carry(source: socket.socket, sink: socket.socket, heard: Callable[[bytes], bytes]) -> None:
    """Pass what one end says to the other, each chunk heard on its way and passed as heard."""
    while chunk := source.recv(CHUNK):
        sink.sendall(heard(chunk))
    sink.shutdown(socket.SHUT_WR)


def unchanged(chunk: bytes) -> bytes:
    """Hear what niri answers, and pass it on as it is, keeping none of it."""
    return chunk


@pytest.fixture(scope="module")
def proxy() -> Proxy:
    """Stand the proxy on niri's socket, and point what starts after it there."""
    niri = os.environ["NIRI_SOCKET"]
    standing = Proxy(niri, Path(os.environ["LOGS"]).resolve() / "niri-proxy.sock")
    os.environ["NIRI_SOCKET"] = str(standing.path)
    return standing


@pytest.fixture(scope="module")
def shell(proxy: Proxy) -> Iterator[Shell]:
    """Give the shell, started once a run on the proxy, as on a niri powering each screen."""
    assert os.environ["NIRI_SOCKET"] == str(proxy.path)
    os.environ["NIRI_POWERS_OFF_EACH_OUTPUT"] = "1"
    started = launcher_on_niri.started()
    yield started
    started.stop()


@pytest.fixture(autouse=True)
def screens_back_on(shell: Shell) -> Iterator[None]:
    """Turn the screens back on and close the launcher after each test."""
    yield
    shell.as_account("niri msg action power-on-monitors")
    shell.launcher_left_closed()


def niri_asked(proxy: Proxy, mark: int, asked: bytes = b'"PowerOffMonitors"') -> None:
    """Wait until a client has asked niri for what's asked since the mark: power off, unsaid."""
    deadline = time.monotonic() + ASKED
    while time.monotonic() < deadline:
        if asked in proxy.since(mark):
            return
        time.sleep(POLL)
    failed = f"the shell never asked niri for {asked!r}: {proxy.since(mark)!r}"
    raise AssertionError(failed)


def test_turn_every_screen_off_powers_niris_monitors_off(shell: Shell, proxy: Proxy) -> None:
    """Turn Every Screen Off closes the launcher, then asks niri to power its monitors off."""
    shell.answer("every screen off")
    shell.launcher_until(
        lambda shown: (shown["selected"] or {}).get("name") == "Turn Every Screen Off"
    )
    # From the find, Return lands on a row that changes something, and the next runs it, once
    # the landing's own moment has passed.
    shell.send_key("ret")
    shell.launcher_until(lambda shown: shown["mode"] == "normal" and not shown["landedFresh"])
    mark = proxy.mark()
    shell.send_key("ret")
    shell.launcher_closed()
    niri_asked(proxy, mark)


def test_a_screens_turn_it_off_powers_that_one_off_by_name(shell: Shell, proxy: Proxy) -> None:
    """A screen's Turn It Off, a step inside it, asks niri to power that one off by name."""
    shell.answer("screens")
    shell.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Screens")
    shell.step_in()
    # The nested niri's one screen leads Screens, and Tab steps inside it.
    screen = (shell.showing()["selected"] or {}).get("name")
    shell.step_in("tab")
    shell.normal()
    shell.down_to("Turn It Off")
    mark = proxy.mark()
    shell.send_key("ret")
    # The nested niri's one screen is the last lit, so the launcher closes, nothing being left
    # to show it, and asks once the key is up.
    shell.launcher_closed()
    niri_asked(proxy, mark, b'"PowerOffMonitor":{"output":')
    assert b'"PowerOffMonitors"' not in proxy.since(mark), f"{screen} asked for every screen"


def test_set_noscreens_powers_niris_monitors_off(shell: Shell, proxy: Proxy) -> None:
    """:set noscreens on the command line asks niri to power its monitors off."""
    shell.answer("screen")
    shell.normal()
    shell.send_chars(":set noscreens")
    shell.launcher_until(lambda shown: shown["command"] is not None)
    mark = proxy.mark()
    shell.send_key("ret")
    niri_asked(proxy, mark)


def test_a_scale_niri_refuses_says_niris_why(shell: Shell, proxy: Proxy) -> None:
    """:setl scale=1.25 that niri refuses says on the : line it wasn't set, and why."""
    shell.answer("screens")
    shell.launcher_until(lambda shown: (shown["selected"] or {}).get("name") == "Screens")
    shell.step_in()
    shell.step_in("tab")
    shell.normal()
    shell.launcher_until(lambda shown: "Scale" in shown["rows"])
    shell.send_chars(":setl scale=1.25")
    shell.launcher_until(lambda shown: shown["command"] is not None)
    mark = proxy.mark()
    with proxy.refusing(b'"Scale":'):
        shell.send_key("ret")
        niri_asked(proxy, mark, b'"Scale":{"scale":{"Specific":1.25}}')
        wanted = f"1.25\u00d7 not set \u00b7 {NIRIS_EXAMPLE_ERROR}"
        shell.launcher_until(lambda shown: shown["commandSaid"] == wanted)

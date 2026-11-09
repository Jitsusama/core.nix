"""Every screen goes off from the launcher by niri's own power-off, on niri.

modules/quickshell/checks.nix runs this through pytest inside tests/nested-niri.sh, with the
shell launcher_on_niri.py starts once a run. Turn Every Screen Off and :set noscreens each ask
niri to power its monitors off, which keeps every output connected and enabled, so a monitor's
hub and network card stay powered (Joel, 2026-10-24: "is there a way to turn my external
display off without shutting down its hub or network card or anything? i want the laptop to be
able to keep going without blinding me in my sleep"). A nested niri draws nothing once its
monitors are off and says nothing of it, so the shell's requests are read on their way to niri,
through a proxy on niri's socket that the shell is started on.

The shell is started as on a niri that powers one output off by name
(modules/niri/powers-off-each-output.patch), so a screen's own Turn It Off asks for that one by
its name (Joel, 2026-11-07: "i want to be able to control the screen that gets shut off, one,
the other, all"). What each screen offers is held in Qt's engine (tests/screens.qml); here, only
that the act reaches niri.
"""

from __future__ import annotations

import os
import socket
import threading
import time
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


class Proxy:
    """niri's socket, passed through, keeping every byte a client sent it."""

    def __init__(self, niri: str, path: Path) -> None:
        """Listen at the path, passing each connection on to niri's own socket."""
        self.niri = niri
        self.path = path
        self.sent = bytearray()
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
            threading.Thread(target=carry, args=(upstream, client, ignore), daemon=True).start()

    def keep(self, chunk: bytes) -> None:
        """Keep what a client sent niri."""
        with self.lock:
            self.sent.extend(chunk)

    def since(self, mark: int) -> bytes:
        """Give what clients have sent niri since the mark."""
        with self.lock:
            return bytes(self.sent[mark:])

    def mark(self) -> int:
        """Give where what clients have sent ends now."""
        with self.lock:
            return len(self.sent)


def carry(source: socket.socket, sink: socket.socket, heard: Callable[[bytes], None]) -> None:
    """Pass what one end says to the other, each chunk heard on its way."""
    while chunk := source.recv(CHUNK):
        heard(chunk)
        sink.sendall(chunk)
    sink.shutdown(socket.SHUT_WR)


def ignore(_chunk: bytes) -> None:
    """Hear what niri answers, and keep none of it."""


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


def asked_to_power_off(proxy: Proxy, mark: int, asked: bytes = b'"PowerOffMonitors"') -> None:
    """Wait until a client has asked niri to power its monitors off since the mark."""
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
    asked_to_power_off(proxy, mark)


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
    asked_to_power_off(proxy, mark, b'"PowerOffMonitor":{"output":')
    assert b'"PowerOffMonitors"' not in proxy.since(mark), f"{screen} asked for every screen"


def test_set_noscreens_powers_niris_monitors_off(shell: Shell, proxy: Proxy) -> None:
    """:set noscreens on the command line asks niri to power its monitors off."""
    shell.answer("screen")
    shell.normal()
    shell.send_chars(":set noscreens")
    shell.launcher_until(lambda shown: shown["command"] is not None)
    mark = proxy.mark()
    shell.send_key("ret")
    asked_to_power_off(proxy, mark)

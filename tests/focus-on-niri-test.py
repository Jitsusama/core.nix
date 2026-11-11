"""The launcher, the held notices and a window share the keys by one rule, on niri.

modules/quickshell/checks/core.nix runs this through pytest inside tests/nested-niri.sh, with the
shell launcher_on_niri.py starts once a run. The newest surface Joel opened takes the keys, a
notice arriving never does, and a click on a window gives that window the keys and closes the
launcher, since one left open without them hears nothing he can see (Joel, 2026-11-07: "we
have contention when say a notification window is up and you launch the launcher, or when the
launcher is up and you load a notification window. you can't change focus and shit. similar
issues when in the launcher and you try to click off and interact with a window."). The rule
itself is niri's, which its own tests hold (modules/niri/on-demand-when-asked.patch); these
cases are the shell asking for it. A window a program puts up for Joel because he asked it
from the open launcher, as 1Password's unlock, closes the launcher and takes the focus, where
it came up under the launcher holding the keys (Joel, 2026-11-10: "the unlock screen shows
below the launcher. very poor UX"); any other window opening meanwhile leaves it be.
Every test starts with the launcher closed and no notice held, and leaves them so.
"""

from __future__ import annotations

import json
import os
import time
from pathlib import Path
from typing import TYPE_CHECKING

import launcher_on_niri
import pytest

if TYPE_CHECKING:
    from collections.abc import Iterator

    from launcher_on_niri import Json, Shell

# Far enough to take the pointer to the screen's top left corner from anywhere on it.
TO_THE_CORNER = -10000
# How far inside a window's bottom left corner it's clicked, clear of the launcher's card.
INSIDE = 24
# The program handed to, by the app id its windows carry, as 1Password's say 1Password.
HANDED_TO = "stand-in-manager"
# How long the program handed to has to put its window up, in ms: long enough for foot
# to start on a loaded builder.
PATIENCE = 60000


@pytest.fixture(scope="module")
def shell() -> Iterator[Shell]:
    """Give the shell, started once a run."""
    started = launcher_on_niri.started()
    yield started
    started.stop()


@pytest.fixture(autouse=True)
def launcher_closed_after(shell: Shell) -> Iterator[None]:
    """Close the launcher after each test, so one failing with it open fails alone."""
    yield
    shell.launcher_left_closed()


def holds_the_notices(shell: Shell) -> None:
    """Take the keys to the held notices, a card having come for them to hold."""
    with shell.told("shows card Alpha"):
        shell.as_account("notify-send --expire-time=60000 'Alpha' 'held'")
    with shell.told("keys notices"):
        shell.take_notices()


def notices_answer_then_let_go(shell: Shell) -> None:
    """See the held notices answer a key, then let them go by :q, and put the card away."""
    with shell.told("notice mode command"):
        shell.send_chars(":")
    with shell.told("lets notices go"):
        shell.send_chars("q")
        shell.send_key("ret")
    shell.as_account("qs ipc call notices putAway")
    shell.notices_gone()


def opened(shell: Shell) -> None:
    """Open the launcher, by what Super+Space runs, and wait until it holds the keys."""
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()


def window_of(shell: Shell, app: str, title: str) -> Json:
    """Open a foot window carrying this app id and title, and give it once niri lists it."""
    shell.as_account(f"niri msg action spawn -- foot --app-id '{app}' --title '{title}' sleep 3600")
    deadline = time.monotonic() + launcher_on_niri.STARTING.total_seconds()
    while True:
        window = next((one for one in shell.windows() if one["title"] == title), None)
        if window is not None:
            return window
        assert time.monotonic() < deadline, f"no window titled {title!r} opened"
        time.sleep(launcher_on_niri.POLL)


def focused_window(shell: Shell) -> Json | None:
    """Give the window niri focuses, or None."""
    focused: Json | None = json.loads(shell.as_account("niri msg --json focused-window"))
    return focused


def focus_reaches(shell: Shell, window: Json) -> None:
    """Wait for niri to focus this window, which the launcher asks of it as it closes.

    The ask goes down niri's socket after the launcher has said it closed, so a busy
    builder can read the focus before niri has done it (2026-11-10).
    """
    deadline = time.monotonic() + launcher_on_niri.WAITING.total_seconds()
    focused = focused_window(shell)
    while focused is None or focused["id"] != window["id"]:
        assert time.monotonic() < deadline, f"niri never focused {window['title']}: {focused}"
        time.sleep(launcher_on_niri.POLL)
        focused = focused_window(shell)


def click(shell: Shell, x: int, y: int) -> None:
    """Click the left button at a point on the screen, through a virtual pointer."""
    wlrctl = os.environ["WLRCTL"]
    shell.as_account(f"{wlrctl} pointer move {TO_THE_CORNER} {TO_THE_CORNER}")
    shell.as_account(f"{wlrctl} pointer move {x} {y}")
    shell.as_account(f"{wlrctl} pointer click left")


def test_a_launcher_opened_over_held_notices_takes_the_keys_and_gives_them_back(
    shell: Shell,
) -> None:
    """The launcher, opened while the notices hold the keys, takes them, and gives them back."""
    holds_the_notices(shell)
    opened(shell)
    shell.send_chars("k")
    shell.launcher_until(lambda shown: shown["typed"] == "k")
    with shell.told("keys notices"):
        shell.as_account(shell.ipc("toggle"))
        shell.launcher_closed()
    notices_answer_then_let_go(shell)


def test_held_notices_take_the_keys_from_the_launcher_and_a_notice_arriving_never_does(
    shell: Shell,
) -> None:
    """A notice coming leaves the launcher the keys; the notices held take them, and give back."""
    opened(shell)
    with shell.told("shows card Bravo"):
        shell.as_account("notify-send --expire-time=60000 'Bravo' 'arriving'")
    shell.send_chars("a")
    shell.launcher_until(lambda shown: shown["keys"] and shown["typed"] == "a")
    with shell.told("keys notices"):
        shell.take_notices()
    shell.launcher_until(lambda shown: not shown["keys"])
    notices_answer_then_let_go(shell)
    # Let go, the notices give the keys back to the launcher, still open beneath them.
    shell.launcher_until(lambda shown: shown["keys"])
    shell.send_chars("b")
    shell.launcher_until(lambda shown: shown["typed"] == "ab")


def test_a_click_on_a_window_takes_the_keys_and_closes_the_launcher(shell: Shell) -> None:
    """A click on a window gives it the keys and closes the launcher; what's typed reaches it."""
    # Where foot's cat writes what reaches it, from wherever niri starts it.
    typed = Path(os.environ["LOGS"]).resolve() / "typed-in-the-window"
    shell.as_account(
        f"niri msg action spawn -- foot --title 'Clicked' sh -c 'cat > {typed}'",
    )
    deadline = time.monotonic() + launcher_on_niri.STARTING.total_seconds()
    window = None
    while window is None:
        window = next((one for one in shell.windows() if one["title"] == "Clicked"), None)
        assert time.monotonic() < deadline, "foot never opened"
        time.sleep(launcher_on_niri.POLL)
    place = window["layout"]["tile_pos_in_workspace_view"]
    size = window["layout"]["tile_size"]
    opened(shell)
    click(shell, int(place[0]) + INSIDE, int(place[1] + size[1]) - INSIDE)
    shell.launcher_closed()
    focus_reaches(shell, window)
    # A key reaches only the window: foot writes the line, and the launcher's find is empty.
    shell.send_chars("z\n")
    deadline = time.monotonic() + launcher_on_niri.WAITING.total_seconds()
    while shell.execute(f"grep -q z {typed}")[0] != 0:
        assert time.monotonic() < deadline, "the key never reached the window clicked"
        time.sleep(launcher_on_niri.POLL)
    assert not shell.showing()["open"], "the launcher opened again"
    shell.close(window)


def test_a_window_of_the_program_handed_to_closes_the_launcher_and_takes_the_focus(
    shell: Shell,
) -> None:
    """Handed to a program, the launcher stays open over another's window and goes for its own."""
    opened(shell)
    shell.as_account(shell.ipc("handsOver", HANDED_TO, str(PATIENCE)))
    # A window of anything else, as a session opening, leaves the launcher open with the keys.
    other = window_of(shell, "stand-in-other", "Other")
    shell.send_chars("k")
    shell.launcher_until(lambda shown: shown["keys"] and shown["typed"] == "k")
    with shell.told(f"handed to {HANDED_TO}"):
        handed = window_of(shell, HANDED_TO, "Unlock")
    shell.launcher_closed()
    focus_reaches(shell, handed)
    # Handed over once, the next opening is the launcher's own again.
    opened(shell)
    again = window_of(shell, HANDED_TO, "Again")
    shell.send_chars("j")
    shell.launcher_until(lambda shown: shown["keys"] and shown["typed"] == "j")
    for window in (other, handed, again):
        shell.close(window)


def test_the_launcher_says_whether_it_is_open_in_its_mark(shell: Shell) -> None:
    """The launcher's mark reads open while it's open and closed once it closes.

    A switch run by root can't ask the shell, so it reads this to wait until Joel isn't in it.
    """
    mark = "$XDG_RUNTIME_DIR/quickshell-launcher"
    shell.as_account(f"grep -qx closed {mark}")
    opened(shell)
    shell.as_account(f"grep -qx open {mark}")
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_closed()
    shell.as_account(f"grep -qx closed {mark}")

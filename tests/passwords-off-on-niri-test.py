"""The Passwords place while it's off, as Joel's is until agents are fenced off, on niri.

modules/quickshell/checks/passwords.nix runs this through pytest inside tests/nested-niri.sh,
with the place off as the module leaves it, so no daemon is reached. Chrome is a stand-in
found by its desktop entry, as 1Password's extension fills sign-ins in there. Home's
Passwords row and the place's one row each have a drawer saying why it's off and what
Return opens meanwhile, rather than none, which a drawer only repeating the row's name was.
"""

from __future__ import annotations

import os
import shutil
from pathlib import Path
from typing import TYPE_CHECKING

import launcher_on_niri
import pytest

if TYPE_CHECKING:
    from collections.abc import Iterator

    from launcher_on_niri import Shell

LOGS = Path(os.environ["LOGS"])
# The place's one row while it's off.
OFF = "Passwords Is Off While It's Made Safe"
# What both drawers say: why it's off, then what Return opens, where 1Password fills in.
ABOUT = (
    "Off until agents are fenced off: each runs as you, so any of them could ask it for a"
    " password. Meanwhile, Return opens Google Chrome, where 1Password's extension fills in"
    " sign-ins."
)


def stand_in_chrome() -> None:
    """Give the shell a Chrome to find by its desktop entry, which does nothing."""
    bash = shutil.which("bash")
    assert bash is not None, "no bash to stand in for Chrome"
    program = LOGS.resolve() / "google-chrome"
    program.write_text(f"#!{bash}\n")
    program.chmod(0o755)
    data = Path(os.environ.get("XDG_DATA_HOME", Path.home() / ".local" / "share"))
    entry = data / "applications" / "google-chrome.desktop"
    entry.parent.mkdir(parents=True, exist_ok=True)
    entry.write_text(f"[Desktop Entry]\nType=Application\nName=Google Chrome\nExec={program} %U\n")


@pytest.fixture(scope="module")
def shell() -> Iterator[Shell]:
    """Give the shell, started once a run after Chrome's stand-in is in place."""
    LOGS.mkdir(parents=True, exist_ok=True)
    stand_in_chrome()
    started = launcher_on_niri.started()
    yield started
    started.stop()


@pytest.fixture(autouse=True)
def launcher_closed_after(shell: Shell) -> Iterator[None]:
    """Close the launcher after each test, so one failing with it open fails alone."""
    yield
    shell.close_launcher()


def drawer_says(shell: Shell, body: str) -> None:
    """Wait for the drawer to be out, saying this."""
    shell.launcher_until(
        lambda shown: (
            (shown.get("drawer") or {}).get("out") is True and shown["drawer"].get("body") == body
        )
    )


def test_homes_passwords_row_says_why_its_off_in_its_drawer(shell: Shell) -> None:
    """Home's Passwords row says why it's off in its drawer, not only its one row's name."""
    shell.as_account(shell.ipc("toggle"))
    shell.launcher_ready()
    shell.down_to("Passwords")
    drawer_says(shell, ABOUT)


def test_the_off_row_says_why_its_off_in_its_drawer(shell: Shell) -> None:
    """The place's one row while it's off says why in its drawer, which its name alone doesn't."""
    shell.as_account(shell.ipc("collection", "passwords"))
    shell.launcher_ready()
    shell.launcher_until(lambda shown: shown["held"] == ["passwords"] and shown["rows"] == [OFF])
    drawer_says(shell, ABOUT)

"""The Passwords place while it's off, with no password manager set up, on niri.

modules/quickshell/checks/passwords.nix runs this through pytest inside tests/nested-niri.sh. Off, the
place holds one row saying so. With no manager, nothing can open from it, so its Return goes
into its drawer, which says why and what would, rather than doing nothing, since every row's
Return does something.
"""

from __future__ import annotations

from typing import TYPE_CHECKING, cast

import launcher_on_niri
import pytest

if TYPE_CHECKING:
    from collections.abc import Iterator

    from launcher_on_niri import Json, Shell

# The place's one row while it's off.
OFF = "Passwords Is Off While It's Made Safe"
# What its drawer says of why nothing opens from it.
WHY = "No password manager is set up, so nothing opens from here."


@pytest.fixture(scope="module")
def shell() -> Iterator[Shell]:
    """Give the shell, started once a run."""
    started = launcher_on_niri.started()
    yield started
    started.stop()


def into_drawer(shown: Json) -> bool:
    """Say whether the keys are in the off row's drawer, which says why nothing opens."""
    drawer = cast("Json", shown["drawer"] or {})
    return bool(shown["open"]) and shown["mode"] == "page" and WHY in str(drawer.get("body", ""))


def test_the_off_rows_return_goes_into_its_drawer_saying_why(shell: Shell) -> None:
    """With nothing to open, Return on the off row puts the keys in its drawer, saying why."""
    shell.as_account(shell.ipc("collection", "passwords"))
    shell.launcher_ready()
    shell.launcher_until(lambda shown: shown["rows"] == [OFF])
    selected = cast("Json", shell.showing()["selected"])
    assert selected["verb"] == "show", f"the off row's Return isn't its drawer: {selected}"
    shell.send_key("ret")
    # Return from the find may land on the row first, as on one that has just changed, and a
    # second does its act.
    shell.launcher_until(
        lambda shown: (
            into_drawer(shown)
            or (shown["mode"] == "normal" and (shown["selected"] or {}).get("name") == OFF)
        )
    )
    if not into_drawer(shell.showing()):
        shell.send_key("ret")
    shell.launcher_until(into_drawer)
    # Escape does nothing in a drawer, so Ctrl+W h, as the foot says, gives the keys back to
    # the rows before the launcher closes.
    shell.as_account("wtype -M ctrl -k w -m ctrl -k h")
    shell.launcher_until(lambda shown: shown["mode"] == "normal")
    shell.close_launcher()

"""A meter stays its moment from the frame it's first drawn in, on niri.

modules/quickshell/checks/shell.nix runs this through pytest inside tests/nested-niri.sh, with the
shell launcher_on_niri.py starts. A meter (modules/quickshell/Meter.qml) shows for a moment as a
level changes. Its moment was timed from the change, so on a builder whose frames came 1.3 to 2 s
apart it was gone before niri had drawn it, and the desktop test's meter over dictation never
said it was drawn (17 and 18 November). Here niri is held stopped for longer than the moment as
the screen's brightness is picked, so the meter's surface can't be drawn until niri goes on, and
the meter must still be drawn, then stay its moment.
"""

from __future__ import annotations

import time
from typing import TYPE_CHECKING

import launcher_on_niri
import pytest

if TYPE_CHECKING:
    from collections.abc import Iterator

    from launcher_on_niri import Shell

# A meter's moment (modules/quickshell/Meter.qml), and the least of it a case allows, since its
# reports are read every 50 ms.
MOMENT, AT_LEAST = 1.5, 1.3
# How long niri is held: longer than the moment, as the slowest builder frame was.
HELD = 2.5
# How long a meter's reports may take to come once niri goes on.
COMES = 10.0
# Where the backlight daemon writes each level Joel picks (modules/backlight), which the
# brightness meter hears.
CHOSEN = '"$XDG_RUNTIME_DIR/backlight-chosen"'


@pytest.fixture(scope="module")
def shell() -> Iterator[Shell]:
    """Give the shell, started once for the file."""
    with launcher_on_niri.running() as shell:
        yield shell


def heard(shell: Shell, since: int, until: str) -> dict[str, float]:
    """Give when each of the brightness meter's reports since the mark came, until `until`."""
    came: dict[str, float] = {}
    deadline = time.monotonic() + COMES
    while until not in came and time.monotonic() < deadline:
        reports, since = shell.said_since(since)
        for said in reports:
            if said in ("drawn brightness", "gone brightness"):
                came.setdefault(said.split(" ")[0], time.monotonic())
        time.sleep(launcher_on_niri.POLL)
    return came


def test_a_meter_niri_draws_late_is_drawn_and_stays_its_moment(shell: Shell) -> None:
    """Pick a brightness while niri is held, and see the meter drawn, then gone a moment after."""
    # The file's first writing may go unheard, since it's made by it; any meter it shows has
    # gone by the end of the wait.
    since = shell.mark()
    shell.as_account(f"printf 0.3 > {CHOSEN}")
    heard(shell, since, "gone")
    since = shell.mark()
    with launcher_on_niri.niri_held():
        shell.as_account(f"printf 0.6 > {CHOSEN}")
        time.sleep(HELD)
    came = heard(shell, since, "gone")
    assert "drawn" in came, f"the meter was never drawn: {sorted(came)}\n{shell.tail()}"
    assert "gone" in came, f"the meter never went: {sorted(came)}\n{shell.tail()}"
    stayed = came["gone"] - came["drawn"]
    assert stayed >= AT_LEAST, f"the meter stayed {stayed:.2f} s once drawn, not {MOMENT}"

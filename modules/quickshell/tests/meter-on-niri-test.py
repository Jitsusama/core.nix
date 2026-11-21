"""A meter stays its moment from the frame niri first shows of it, on niri.

modules/quickshell/checks/shell.nix runs this through pytest inside tests/nested-niri.sh, with the
shell launcher_on_niri.py starts. A meter (modules/quickshell/Meter.qml) shows for a moment as a
level changes. Its moment was timed from the change, so on a builder whose frames came 1.3 to 2 s
apart it was gone before niri had drawn it, and the desktop test's meter over dictation never
said it was drawn (17 and 18 November). Here niri is held stopped for longer than the moment as
the screen's brightness is picked, so the meter's surface can't be drawn until niri goes on, and
the meter must still be drawn, then stay its moment, and no longer, so it was timed from its
frame rather than spent by its cap. When each frame is spent is held in Qt's engine
(modules/quickshell/tests/meter-moment.qml).

The moment is timed from "seen", said as niri shows the meter's frame, not from "drawn", said
as Qt hands niri that frame, since the moment begins at the first: timed from "drawn", a gate
under load saw the meter stay 3.44 s (20 November), as it would with niri showing the frame
some two seconds after it was handed over. How long it stays once seen is Moment's own timer,
which Qt's engine holds; what only niri shows is that its moment began at niri's presented
frame, never Qt's swapped one, which "seen" says, and that it then went in its moment.
"""

from __future__ import annotations

import os
import time
from pathlib import Path
from typing import TYPE_CHECKING

import launcher_on_niri
import pytest

if TYPE_CHECKING:
    from collections.abc import Iterator

    from launcher_on_niri import Shell

# A meter's moment (modules/quickshell/Moment.qml), and the least and most of it a case allows:
# each report is timed as a 50 ms poll reads it, so a stalled reader can shorten what it
# measures by a few polls, and its timer fires late on a loaded builder. Its cap, ten seconds
# unseen, would keep one niri held for 2.5 s some 7 s past its frame.
MOMENT, AT_LEAST, AT_MOST = 1.5, 1.2, 3.0
# What "seen" says when the moment began at niri's presented frame, and at Qt's swapped one.
PRESENTED, SWAPPED = "seen brightness presented", "seen brightness swapped"
# How long niri is held: longer than the moment, as the slowest builder frame was.
HELD = 2.5
# How long a meter's reports may take to come once niri goes on.
COMES = 10.0
# Where the backlight daemon writes each level Joel picks (modules/backlight), which the
# brightness meter hears.
CHOSEN = '"$XDG_RUNTIME_DIR/backlight-chosen"'


@pytest.fixture(scope="module")
def shell() -> Iterator[Shell]:
    """Give the shell, started once for the file, with a level already picked before it.

    Made before the shell starts, the file is one it watches from the start, and a pick made
    before it isn't news (modules/quickshell/Brightness.qml), so the case's pick is the meter's
    first showing.
    """
    (Path(os.environ["XDG_RUNTIME_DIR"]) / "backlight-chosen").write_text("0.3")
    with launcher_on_niri.running() as shell:
        yield shell


def heard(shell: Shell, since: int, until: str) -> dict[str, float]:
    """Give when each of the brightness meter's reports since the mark came, until `until`."""
    came: dict[str, float] = {}
    deadline = time.monotonic() + COMES
    while until not in came and time.monotonic() < deadline:
        reports, since = shell.said_since(since)
        for said in reports:
            if said in ("drawn brightness", PRESENTED, SWAPPED, "gone brightness"):
                came.setdefault(said, time.monotonic())
        time.sleep(launcher_on_niri.POLL)
    return came


def test_a_meter_niri_shows_late_stays_its_moment_from_niris_frame(shell: Shell) -> None:
    """Pick a brightness while niri is held, and see the meter shown, then gone a moment after."""
    since = shell.mark()
    with launcher_on_niri.niri_held():
        shell.as_account(f"printf 0.6 > {CHOSEN}")
        time.sleep(HELD)
    came = heard(shell, since, "gone brightness")
    told = f"{sorted(came)}\n{shell.tail()}"
    assert "drawn brightness" in came, f"the meter was never drawn: {told}"
    assert SWAPPED not in came, (
        "its moment began at Qt's frame, not niri's, so the shell found no"
        f" presentation-time from niri: {told}"
    )
    assert PRESENTED in came, f"niri never showed the meter: {told}"
    assert "gone brightness" in came, f"the meter never went: {told}"
    stayed = came["gone brightness"] - came[PRESENTED]
    late = came[PRESENTED] - came["drawn brightness"]
    assert AT_LEAST <= stayed <= AT_MOST, (
        f"the meter stayed {stayed:.2f} s once seen, not {MOMENT}, seen {late:.2f} s after drawn"
    )

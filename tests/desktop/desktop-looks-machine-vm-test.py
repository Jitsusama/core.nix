"""The desktop test's looks area for Machine and Machines.

Here Machine's things are shot at Joel's two scales, on a screen as large as the laptop's, as the
looks area shoots the launcher's own: 1.5, as the HP draws it, and 2, as the laptop does. Every
figure is the machine daemon's (modules/machine), read from the guest's kernel, or, for Machines,
the realm's mycelium node's record of this machine, so this is where the design looks at what
each gives. It's an area of its own, rather than more of looks, since the
pyramid keeps a VM test to twenty subtests (docs/testing.md).

tests/desktop.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), and its
opening comment says what the test proves. Every test asks for the desktop, started once a run
(desktop_session.py), and leaves it as it found it.
"""

from __future__ import annotations

from datetime import timedelta
from typing import TYPE_CHECKING, Any, cast

import desktop_session
import pytest
from desktop_session import LAUNCHER_OPEN

if TYPE_CHECKING:
    from collections.abc import Iterator

    from desktop_session import Desktop
    from test_driver.machine import BaseMachine

# The scales Joel's screens are drawn at, as niri spells them.
JOELS_SCALES = ("1.5", "2")

# Machine's things, each with a part the machine daemon alone gives, and that part's own parts,
# as the daemon reads the guest's kernel: what moves most leads and the model ends
# (VKCF3Z, 2026-11-07).
MACHINES_PARTS = [
    ("processor", "Time Spent", "Programs"),
    ("memory", "Held By", "Programs"),
    ("disk", "Speed", "Reading"),
    ("uptime", "Asleep", None),
]


@pytest.fixture(scope="module")
def desktop(machine: BaseMachine, values: dict[str, Any]) -> Desktop:
    """Give the desktop, started once a run."""
    return desktop_session.started(machine, cast("desktop_session.Values", values))


@pytest.fixture
def at_each_scale(desktop: Desktop) -> Iterator[list[str]]:
    """Give Joel's scales to set in turn, and put the screen back at 1 afterwards."""
    yield list(JOELS_SCALES)
    # A test that failed with the launcher open leaves it closed for the next.
    if desktop.machine.execute(desktop.in_session(LAUNCHER_OPEN))[0] == 0:
        desktop.close_launcher()
    desktop.as_account("niri msg output winit scale 1")


def at_scale(desktop: Desktop, scale: str) -> None:
    """Set the screen's scale, with the launcher closed, so it opens laid out for it."""
    machine = desktop.machine
    machine.wait_until_fails(desktop.in_session(LAUNCHER_OPEN), timeout=timedelta(seconds=10))
    desktop.as_account(f"niri msg output winit scale {scale}")


def test_machines_parts_are_shot_at_each_scale(desktop: Desktop, at_each_scale: list[str]) -> None:
    """Shoot the processor's, memory's, disk's and uptime's parts, and a part's own."""
    for scale in at_each_scale:
        named = scale.replace(".", "-")
        at_scale(desktop, scale)
        for asked, part, inside in MACHINES_PARTS:
            desktop.answer(asked)
            desktop.launcher_until(lambda shown: shown["kind"] == "answer")
            desktop.step_in()
            desktop.launcher_until(lambda shown: part in shown["rows"])  # noqa: B023
            desktop.down_to(part)
            desktop.shoot(f"launcher-machine-{asked}-at-{named}")
            if inside is not None:
                desktop.step_in()
                desktop.launcher_until(lambda shown: inside in shown["rows"])  # noqa: B023
                held = part.lower().replace(" ", "-")
                desktop.shoot(f"launcher-machine-{asked}-{held}-at-{named}")
            desktop.close_launcher()


def test_machines_is_shot_at_each_scale(desktop: Desktop, at_each_scale: list[str]) -> None:
    """Shoot Machines, this machine's row first, saying how it stands from its node's record.

    The node here links no other machine, so this one is the realm's only row. It has no
    drawer, since Return takes it to the Machine place, whose own places hold its figures;
    another machine's drawer, and one out of reach dimmed, are tests/machines.qml's.
    """
    for scale in at_each_scale:
        named = scale.replace(".", "-")
        at_scale(desktop, scale)
        desktop.as_account("qs ipc call launcher toggle")
        desktop.launcher_ready()
        desktop.type_in("machines", selecting="Machines")
        desktop.step_in()
        desktop.launcher_until(
            lambda shown: (
                shown["rows"][:1] == ["machine"]
                and "NixOS" in shown["selected"]["note"]
                and shown["selected"]["side"] == "this machine"
                and shown["drawer"]["out"] is False
            )
        )
        desktop.shoot(f"launcher-machines-at-{named}")
        desktop.close_launcher()

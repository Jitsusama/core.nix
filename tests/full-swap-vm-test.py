"""Full swap ends the work that filled it, through systemd-oomd, and nothing else.

tests/full-swap.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), and
its opening comment says what fills memory. grow, on the machine's path, takes data that
compresses about two to one up to the MiB it's told and keeps it. The tests need no value
from Nix.
"""

from __future__ import annotations

from datetime import timedelta
from typing import TYPE_CHECKING

import pytest

if TYPE_CHECKING:
    from test_driver.machine import BaseMachine

# How long the big one may take to fill memory and swap until oomd marks it.
FILLING = timedelta(minutes=5)
# How long systemd may take to end what oomd marked.
ENDING = timedelta(seconds=30)


@pytest.fixture(scope="module")
def watched(machine: BaseMachine) -> BaseMachine:
    """Give the machine with systemd-oomd watching both of its swaps."""
    machine.wait_for_unit("systemd-oomd.service")
    machine.wait_for_unit("dev-zram0.swap")
    machine.wait_for_unit("var-swapfile.swap")
    return machine


@pytest.fixture(scope="module")
def filled(watched: BaseMachine) -> BaseMachine:
    """Give the machine with a small program holding a little and a big one filling the rest.

    The small one starts first, so some of what it holds is in swap when the big one starts
    filling the rest.
    """
    watched.succeed("systemd-run --unit=small -p Type=exec grow 64")
    watched.succeed("systemd-run --unit=big -p Type=exec grow 4096")
    return watched


def test_swap_is_watched_from_the_root_slice(watched: BaseMachine) -> None:
    """Swap is watched from the root slice, so oomd may end anything that fills it."""
    assert "Path: /\n" in watched.succeed("oomctl")


def test_the_work_that_filled_swap_is_ended_by_systemd_oomd(filled: BaseMachine) -> None:
    """The work that filled swap is ended by systemd-oomd, before the kernel's killer picks."""
    filled.wait_until_succeeds(
        "journalctl -u systemd-oomd -o cat"
        " | grep -F 'Marked /system.slice/big.service for killing'",
        timeout=FILLING,
    )
    # systemd says who ended it, which tells oomd from the kernel's killer.
    filled.wait_until_succeeds(
        "journalctl -u big.service -o cat | grep -F 'systemd-oomd killed'", timeout=ENDING
    )
    assert "Out of memory" not in filled.succeed("journalctl -k -o cat")


def test_what_held_a_little_is_left_running(filled: BaseMachine) -> None:
    """What held a little is left running once the big one has gone."""
    filled.wait_until_succeeds(
        "systemctl show -P ActiveState big.service | grep -q -x -E 'inactive|failed'",
        timeout=FILLING + ENDING,
    )
    assert filled.succeed("systemctl is-active small.service").strip() == "active"

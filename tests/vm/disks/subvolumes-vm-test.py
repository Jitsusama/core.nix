"""An account's folders move into Btrfs subvolumes of their own as its manager starts, failing open.

subvolumes.nix, beside this, runs this through pytest inside the driver (tests/lib/vm_pytest.py),
and its opening comment says what the test proves. The account's folders are made once a run, one
held open by a process of its own, one whose writer runs and one with a file it can't read,
then each folder's move runs as a login or a switch runs it, so any test runs alone with `-k`.
The tests need no value from Nix.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

import pytest
from vm_helpers import as_account

if TYPE_CHECKING:
    from test_driver.machine import BaseMachine

HOME = "/home/joel"
FREE = ("moved", "held", "missing", "busy")


def unit(folder: str) -> str:
    """Give the unit that moves a folder, as home.nix names it."""
    return f"own-subvolume-{folder}.service"


def inode(machine: BaseMachine, path: str) -> str:
    """Give a path's inode number, which is 256 at a subvolume's root."""
    return machine.succeed(f"stat -c %i {path}").strip()


def said(machine: BaseMachine, folder: str) -> str:
    """Give what a folder's move said in the journal."""
    return as_account(machine, f"journalctl --user -u {unit(folder)} -o cat")


@pytest.fixture(scope="module")
def moved(machine: BaseMachine) -> BaseMachine:
    """Give the machine once its account's folders are made and each move has run on them."""
    machine.wait_for_unit("multi-user.target")
    machine.succeed("loginctl enable-linger joel")
    machine.wait_for_unit("user@1000.service")
    as_account(
        machine,
        "mkdir -p moved/deep held unreadable busy && echo kept > moved/deep/file"
        " && echo written > busy/file"
        " && ln moved/deep/file moved/linked && ln -s deep/file moved/pointing"
        " && echo open > held/file && echo hidden > unreadable/file && chmod 000 unreadable/file",
    )
    # A process of the account's holds a file in it open, as a running program does.
    as_account(
        machine,
        f"systemd-run --user --unit holder bash -c 'exec 3< {HOME}/held/file; sleep infinity'",
    )
    holds = "test -e /proc/$(systemctl --user show -P MainPID holder)/fd/3"
    as_account(machine, f"timeout 30 sh -c 'until {holds}; do sleep 0.2; done'")
    as_account(machine, "systemctl --user stop writer.service")
    # The busy folder's writer runs, as a switch finds it, so its move is started under it.
    as_account(machine, "systemctl --user start busy.service")
    as_account(machine, "systemctl --user restart " + " ".join(unit(f) for f in FREE))
    # The folder that can't be copied fails its unit, and says so.
    as_account(machine, f"systemctl --user restart {unit('unreadable')}", succeed=False)
    return machine


def test_each_move_comes_with_the_manager_before_its_writers(moved: BaseMachine) -> None:
    """Each folder's move is wanted as the account's manager starts, ordered before its writers."""
    for folder in (*FREE, "unreadable"):
        assert as_account(moved, f"systemctl --user is-enabled {unit(folder)}").strip() == "enabled"
    before = as_account(moved, f"systemctl --user show -P Before {unit('unreadable')}")
    assert "writer.service" in before.split()
    requires = as_account(moved, "systemctl --user show -P Requires,Requisite,BindsTo writer")
    assert "own-subvolume" not in requires, "a writer never needs its folder's move"


def test_a_folder_moves_into_a_subvolume_of_its_own_whole(moved: BaseMachine) -> None:
    """A free folder becomes a subvolume's root with its files and links, the old one kept."""
    assert inode(moved, f"{HOME}/moved") == "256"
    assert moved.succeed(f"cat {HOME}/moved/deep/file").strip() == "kept"
    assert inode(moved, f"{HOME}/moved/deep/file") == inode(moved, f"{HOME}/moved/linked")
    assert moved.succeed(f"readlink {HOME}/moved/pointing").strip() == "deep/file"
    assert moved.succeed(f"stat -c %U {HOME}/moved").strip() == "joel"
    kept = moved.succeed(f"ls -d {HOME}/moved.before-subvolume-*").split()
    assert len(kept) == 1, f"the folder as it was is kept once, not {kept}"
    assert moved.succeed(f"cat {kept[0]}/deep/file").strip() == "kept"
    assert "is a subvolume of its own now" in said(moved, "moved")


def test_a_folder_held_open_stays_and_says_why(moved: BaseMachine) -> None:
    """A folder a process of the account's holds a file in is left where it is."""
    assert inode(moved, f"{HOME}/held") != "256"
    assert "held/file open, so" in said(moved, "held")
    moved.fail(f"ls -d {HOME}/held.before-subvolume-* {HOME}/held.subvolume-new")


def test_a_folder_whose_writer_runs_stays_though_nothing_is_open(moved: BaseMachine) -> None:
    """A writer running between writes holds nothing open, so its running keeps its folder."""
    assert inode(moved, f"{HOME}/busy") != "256"
    assert "busy.service is running, so" in said(moved, "busy")
    moved.fail(f"ls -d {HOME}/busy.before-subvolume-* {HOME}/busy.subvolume-new")


def test_a_folder_missing_is_left_and_its_unit_passes(moved: BaseMachine) -> None:
    """A folder that isn't there is nothing to move, and isn't a failure."""
    assert "missing isn't a folder here" in said(moved, "missing")
    assert as_account(moved, f"systemctl --user is-active {unit('missing')}").strip() == "active"


def test_a_folder_that_cant_be_copied_is_left_whole_and_its_writer_starts(
    moved: BaseMachine,
) -> None:
    """A copy that fails leaves the folder as it was, its half copy gone, and the writer starts."""
    assert inode(moved, f"{HOME}/unreadable") != "256"
    moved.succeed(f"test $(stat -c %a {HOME}/unreadable/file) = 0")
    # The half copy was a subvolume the account made and then removed itself.
    moved.fail(f"ls -d {HOME}/unreadable.before-subvolume-* {HOME}/unreadable.subvolume-new")
    assert "couldn't copy" in said(moved, "unreadable")
    as_account(moved, "systemctl --user start writer.service")
    assert as_account(moved, "systemctl --user is-active writer.service").strip() == "active"


def test_a_second_run_leaves_a_subvolume_as_it_is(moved: BaseMachine) -> None:
    """A subvolume's root isn't copied again, and folders freed since move at the next run."""
    before = moved.succeed(f"ls -d {HOME}/moved.before-subvolume-*")
    as_account(moved, "chmod 600 unreadable/file && systemctl --user stop holder writer busy")
    freed = ("moved", "held", "unreadable", "busy")
    as_account(moved, "systemctl --user restart " + " ".join(unit(f) for f in freed))
    assert moved.succeed(f"ls -d {HOME}/moved.before-subvolume-*") == before
    assert inode(moved, f"{HOME}/held") == "256"
    assert inode(moved, f"{HOME}/unreadable") == "256"
    assert inode(moved, f"{HOME}/busy") == "256"
    assert moved.succeed(f"cat {HOME}/busy/file").strip() == "written"

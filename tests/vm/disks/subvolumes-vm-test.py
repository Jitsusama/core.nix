"""An account's folders move into Btrfs subvolumes of their own as its manager starts, failing open.

subvolumes.nix, beside this, runs this through pytest inside the driver (tests/lib/vm_pytest.py),
and its opening comment says what the test proves. The account's folders are made once a run, one
held open by a process of its own, one it only maps, one whose writer runs, one with a file it
can't read, one held by a process that quits later and one a racer opens as it's copied, then each
folder's move runs as a login or a switch runs it, so any test runs alone
with `-k`. The tests need no value from Nix.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

import pytest
from vm_helpers import as_account

if TYPE_CHECKING:
    from test_driver.machine import BaseMachine

HOME = "/home/joel"
FREE = ("moved", "held", "mapped", "missing", "busy")


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
    # The moves the manager starts find no folder yet, and a target is reached only after what
    # it wants, so none of them can still be running as the folders are made.
    as_account(
        machine,
        "timeout 60 sh -c 'until systemctl --user is-active -q default.target; do sleep 0.2; done'",
    )
    as_account(
        machine,
        "mkdir -p moved/deep held mapped unreadable busy && echo kept > moved/deep/file"
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
    # A program run from the folder maps it with no file in it open, as Chrome maps its
    # histograms; the process's own maps are readable to the account, init's aren't.
    as_account(machine, "cp $(readlink -f $(command -v sleep)) mapped/sleep")
    as_account(machine, f"systemd-run --user --unit mapper {HOME}/mapped/sleep infinity")
    maps = "grep -q mapped/sleep /proc/$(systemctl --user show -P MainPID mapper)/maps"
    as_account(machine, f"timeout 30 sh -c 'until {maps}; do sleep 0.2; done'")
    as_account(machine, "systemctl --user stop writer.service")
    # The busy folder's writer runs, as a switch finds it, so its move is started under it.
    as_account(machine, "systemctl --user start busy.service")
    as_account(machine, "systemctl --user restart " + " ".join(unit(f) for f in FREE))
    # The folder that can't be copied fails its unit, and says so.
    as_account(machine, f"systemctl --user restart {unit('unreadable')}", succeed=False)
    # A process holds this one, as Chrome holds its folder at a switch, and quits later.
    as_account(machine, "mkdir quitting && echo held > quitting/file")
    as_account(
        machine,
        f"systemd-run --user --unit quitter bash -c 'exec 3< {HOME}/quitting/file; sleep infinity'",
    )
    holds = "test -e /proc/$(systemctl --user show -P MainPID quitter)/fd/3"
    as_account(machine, f"timeout 30 sh -c 'until {holds}; do sleep 0.2; done'")
    # Enough files that the racer, started as the copy begins, opens one long before it ends.
    as_account(machine, "mkdir raced && for i in $(seq 8000); do echo $i > raced/f$i; done")
    as_account(machine, f"systemctl --user restart {unit('quitting')} {unit('raced')}")
    return machine


def waiter(folder: str) -> str:
    """Give the unit that waits out a folder's holder, as home.nix names it."""
    return f"own-subvolume-{folder}-after-quit.service"


def waited(machine: BaseMachine, folder: str) -> str:
    """Give what a folder's waiter said in the journal."""
    return as_account(machine, f"journalctl --user -u {waiter(folder)} -o cat")


def again(folder: str) -> str:
    """Give a command that runs a folder's move once more, however often it ran just now.

    systemd refuses a sixth start within ten seconds, and a subtest staging case after case
    starts the move that often, so its count is cleared first.
    """
    return f"systemctl --user reset-failed {unit(folder)}; systemctl --user restart {unit(folder)}"


def nothing_beside(machine: BaseMachine, name: str) -> None:
    """Find no kept folder, half copy, mark or split beside `name`.

    Each is looked for alone, since ls fails when any one of several is missing.
    """
    machine.fail(f"ls -d {HOME}/{name}.before-subvolume-*")
    machine.fail(f"test -e {HOME}/{name}.subvolume-new")
    machine.fail(f"test -e {HOME}/{name}.subvolume-new.keep")
    machine.fail(f"ls -d {HOME}/{name}.split-*")


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
    nothing_beside(moved, "held")


def test_a_folder_only_mapped_stays_and_says_why(moved: BaseMachine) -> None:
    """A folder a process of the account's maps, with nothing in it open, is left where it is."""
    assert inode(moved, f"{HOME}/mapped") != "256"
    assert "maps " in said(moved, "mapped")
    nothing_beside(moved, "mapped")


def test_a_folder_whose_writer_runs_stays_though_nothing_is_open(moved: BaseMachine) -> None:
    """A writer running between writes holds nothing open, so its running keeps its folder."""
    assert inode(moved, f"{HOME}/busy") != "256"
    assert "busy.service is running, so" in said(moved, "busy")
    nothing_beside(moved, "busy")
    # Nothing holds it, so nothing would say when to move it, and no waiter ever started.
    started = as_account(moved, f"systemctl --user show -P InvocationID {waiter('busy')}")
    started = started.strip()
    assert started == "", f"a waiter started for a writer alone, as {started}"


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
    nothing_beside(moved, "unreadable")
    assert "couldn't copy" in said(moved, "unreadable")
    as_account(moved, "systemctl --user start writer.service")
    assert as_account(moved, "systemctl --user is-active writer.service").strip() == "active"


def test_a_folder_a_program_opens_as_it_is_copied_stays_whole(moved: BaseMachine) -> None:
    """A program that starts as its folder is copied keeps the folder, whole, where it was."""
    assert inode(moved, f"{HOME}/raced") != "256", "the folder moved under the racer"
    assert as_account(moved, "systemctl --user is-active racer").strip() == "active"
    told = said(moved, "raced")
    # Seen by the look before the exchange or the one after it, by the file it wrote or the
    # one it holds; either way the waiter is started for the file it holds.
    seen = ", as it was copied" in told or ", as it moved" in told
    assert seen, f"the move didn't say the racer reached it as it moved: {told}"
    assert "raced/f1 open" in told, f"the move didn't say the racer held it: {told}"
    assert "until that quits" in told, f"the move started no waiter for the racer: {told}"
    assert moved.succeed(f"ls {HOME}/raced | wc -l").strip() == "8000"
    moved.succeed(f"grep -qx opened {HOME}/raced/f2")
    nothing_beside(moved, "raced")


def test_a_folder_moves_once_the_process_holding_it_quits(moved: BaseMachine) -> None:
    """A folder left for a process holding it moves by itself, whole, once that one quits."""
    # The racer starts once, but its trigger would go on firing at each copy.
    as_account(moved, "systemctl --user stop racer.path")
    for folder, holder in (("quitting", "quitter"), ("raced", "racer")):
        assert inode(moved, f"{HOME}/{folder}") != "256", f"{folder} moved under {holder}"
        as_account(moved, f"systemctl --user stop {holder}")
        moves = f"test $(stat -c %i {HOME}/{folder}) = 256"
        as_account(moved, f"timeout 60 sh -c 'until {moves}; do sleep 0.5; done'")
        told = waited(moved, folder)
        assert "so this waits for it to quit" in told, f"{folder}'s waiter said: {told}"
    assert moved.succeed(f"cat {HOME}/quitting/file").strip() == "held"
    assert moved.succeed(f"ls {HOME}/raced | wc -l").strip() == "8000"
    # What the racer wrote as the first move copied went with the folder.
    moved.succeed(f"grep -qx opened {HOME}/raced/f2")


def test_a_second_run_leaves_a_subvolume_as_it_is(moved: BaseMachine) -> None:
    """A subvolume's root isn't copied again, and folders freed since move at the next run."""
    before = moved.succeed(f"ls -d {HOME}/moved.before-subvolume-*")
    as_account(moved, "chmod 600 unreadable/file")
    # The held folders' waiters would start the same moves as their holders stop.
    as_account(moved, f"systemctl --user stop {waiter('held')} {waiter('mapped')}")
    as_account(moved, "systemctl --user stop holder mapper writer busy")
    freed = ("moved", "held", "mapped", "unreadable", "busy")
    as_account(moved, "systemctl --user restart " + " ".join(unit(f) for f in freed))
    assert moved.succeed(f"ls -d {HOME}/moved.before-subvolume-*") == before
    assert inode(moved, f"{HOME}/held") == "256"
    assert inode(moved, f"{HOME}/mapped") == "256"
    assert inode(moved, f"{HOME}/unreadable") == "256"
    assert inode(moved, f"{HOME}/busy") == "256"
    assert moved.succeed(f"cat {HOME}/busy/file").strip() == "written"


def test_a_run_stopped_after_its_exchange_keeps_the_folder_as_it_was(moved: BaseMachine) -> None:
    """Keep the folder as it was when a run stopped just after its exchange.

    What it leaves where the copy was made is that folder, so the next run keeps it
    beside the subvolume rather than clearing it.
    """
    as_account(moved, "mkdir moved.subvolume-new && echo older > moved.subvolume-new/file")
    as_account(moved, f"systemctl --user restart {unit('moved')}")
    moved.fail(f"test -e {HOME}/moved.subvolume-new")
    kept = moved.succeed(f"grep -l older {HOME}/moved.before-subvolume-*/file").split()
    assert len(kept) == 1, f"the folder left by the stopped run is kept once, not {kept}"
    assert inode(moved, f"{HOME}/moved") == "256"


def test_a_marked_copy_beside_a_subvolume_goes_only_once_nothing_reached_it(
    moved: BaseMachine,
) -> None:
    """A mark alone is cleared, and the folder as it was goes aside only if unchanged since it.

    The mark bears the time its copy began, so a file changed after it is a write the subvolume
    may lack, and each run fails until Joel has dealt with it.
    """
    restart = again("moved")
    as_account(moved, "touch moved.subvolume-new.keep")
    as_account(moved, restart)
    moved.fail(f"test -e {HOME}/moved.subvolume-new.keep")
    # Stopped as it looked, with nothing written since the copy began.
    as_account(moved, "mkdir moved.subvolume-new && echo calm > moved.subvolume-new/file")
    as_account(moved, "touch moved.subvolume-new.keep")
    # Held, it may still be written in, so it stays until its holder quits.
    as_account(
        moved,
        "systemd-run --user --unit lingerer"
        f" bash -c 'exec 3< {HOME}/moved.subvolume-new/file; sleep infinity'",
    )
    holds = "test -e /proc/$(systemctl --user show -P MainPID lingerer)/fd/3"
    as_account(moved, f"timeout 30 sh -c 'until {holds}; do sleep 0.2; done'")
    as_account(moved, restart, succeed=False)
    moved.succeed(f"test -e {HOME}/moved.subvolume-new.keep")
    told = said(moved, "moved")
    assert "changed as it moved (/proc/" in told, f"the held run said: {told}"
    as_account(moved, "systemctl --user stop lingerer")
    as_account(moved, restart)
    moved.fail(f"test -e {HOME}/moved.subvolume-new")
    moved.fail(f"test -e {HOME}/moved.subvolume-new.keep")
    moved.succeed(f"grep -l calm {HOME}/moved.before-subvolume-*/file")
    assert "held nothing more" in said(moved, "moved")
    # Written in after the copy began, which each run then says until it's dealt with.
    as_account(moved, "mkdir moved.subvolume-new")
    as_account(moved, "touch -d '1 minute ago' moved.subvolume-new.keep")
    as_account(moved, "echo late > moved.subvolume-new/file")
    for _ in range(2):
        as_account(moved, restart, succeed=False)
        moved.succeed(f"grep -qx late {HOME}/moved.subvolume-new/file")
        moved.succeed(f"test -e {HOME}/moved.subvolume-new.keep")
    told = said(moved, "moved")
    assert "changed as it moved" in told, f"the run didn't say what changed: {told}"
    as_account(moved, "rm -r moved.subvolume-new moved.subvolume-new.keep")
    as_account(moved, restart)


def test_a_folder_with_late_writes_set_aside_stays_until_they_are_dealt_with(
    moved: BaseMachine,
) -> None:
    """A marked copy or a split beside a folder keeps it, and says so though something holds it."""
    restart = again("missing")
    as_account(moved, "mkdir missing && echo here > missing/file")
    as_account(moved, "mkdir missing.subvolume-new && touch missing.subvolume-new.keep")
    as_account(
        moved,
        f"systemd-run --user --unit keeper bash -c 'exec 3< {HOME}/missing/file; sleep infinity'",
    )
    holds = "test -e /proc/$(systemctl --user show -P MainPID keeper)/fd/3"
    as_account(moved, f"timeout 30 sh -c 'until {holds}; do sleep 0.2; done'")
    as_account(moved, restart, succeed=False)
    told = said(moved, "missing")
    assert "a copy with writes in it is kept" in told, f"the held run said: {told}"
    as_account(moved, "rm -r missing.subvolume-new missing.subvolume-new.keep")
    as_account(moved, "mkdir missing.split-1")
    as_account(moved, restart, succeed=False)
    assert inode(moved, f"{HOME}/missing") != "256", "the folder moved past its split"
    told = said(moved, "missing")
    assert "missing.split-1, so it stays" in told, f"the held run said: {told}"
    as_account(moved, "systemctl --user stop keeper")
    as_account(moved, "rmdir missing.split-1")
    as_account(moved, restart)
    assert inode(moved, f"{HOME}/missing") == "256"

"""A crash's core stays root's alone, its stack trace stays its owner's, and a node's goes later.

cores-kept-from-their-owner.nix, beside this, runs this through pytest inside the driver
(tests/lib/vm_pytest.py). An account aborts two of its own processes, a program named node
standing in for a pi and sleep itself, as any program running as it could abort a pi.
It can't open or dump either core, though the journal still shows it, in wheel as Joel's
account is, each one's stack trace. Once six hours have passed, the module's removal takes
the node's core and leaves the other's.
"""

from __future__ import annotations

import datetime as dt
import shlex
from typing import TYPE_CHECKING

import pytest

if TYPE_CHECKING:
    from test_driver.machine import BaseMachine

ACCOUNT = "someone"
CORES = "/var/lib/systemd/coredump"
# The journal's id for systemd-coredump's message that a process dumped core.
DUMPED = "fc2e22bc6ee647b6b90729ab34a250b1"
# How long systemd-coredump may take to keep a core and say so in the journal.
KEEPING = dt.timedelta(seconds=60)
# Each crash's pid and the path of its core, by the program that crashed.
Cores = dict[str, tuple[str, str]]


def mode(machine: BaseMachine) -> str:
    """Give the cores' folder's mode and owners."""
    return machine.succeed(f"stat -c '%a %U:%G' {CORES}").strip()


def as_account(command: str) -> str:
    """Give a command that runs as the account, with no limit on its cores."""
    inner = f"ulimit -c unlimited; {command}"
    return f"runuser -u {ACCOUNT} -- sh -c {shlex.quote(inner)}"


def stack_trace(pid: str) -> str:
    """Give a command that prints, as the account, the journal's message for a pid's core."""
    return as_account(f"journalctl --no-pager -q -o cat MESSAGE_ID={DUMPED} COREDUMP_PID={pid}")


def crash(machine: BaseMachine, program: str) -> tuple[str, str]:
    """Start a program as the account, abort it as the account, and give its pid and core."""
    # Started and aborted in one shell, since one ending with its runuser
    # takes what it left in the background with it.
    started = f"{program} 600 >/dev/null 2>&1 & pid=$!; sleep 1; kill -ABRT $pid; echo $pid"
    pid = machine.succeed(as_account(started)).strip()
    name = program.rsplit("/", 1)[-1]
    pattern = f"{CORES}/core.{name}.1000.*.{pid}.*"
    machine.wait_until_succeeds(f"ls {pattern}", timeout=KEEPING)
    core = machine.succeed(f"ls {pattern}").strip()
    return pid, core


@pytest.fixture(scope="module")
def cores(machine: BaseMachine) -> Cores:
    """Boot, then crash the stand-in host and sleep as the account, giving their pids and cores."""
    machine.wait_for_unit("multi-user.target")
    booted = mode(machine)
    assert booted == "700 root:root", f"the folder is {booted} before any crash"
    return {
        "node": crash(machine, "node"),
        "sleep": crash(machine, "sleep"),
    }


@pytest.mark.usefixtures("cores")
def test_only_root_enters_the_cores_folder(machine: BaseMachine) -> None:
    """The cores' folder is root's alone, whatever a core's own access list grants."""
    crashed = mode(machine)
    assert crashed == "700 root:root", f"the folder is {crashed} after the crashes"


def test_the_owner_can_neither_open_nor_dump_a_core(machine: BaseMachine, cores: Cores) -> None:
    """The account whose process crashed can't read its core, by its path or through coredumpctl."""
    pid, core = cores["node"]
    status, said = machine.execute(as_account(f"cat {shlex.quote(core)} 2>&1 >/dev/null"))
    assert status != 0, f"the account read its core: {said}"
    assert "Permission denied" in said, f"the core failed some other way: {said}"
    status, said = machine.execute(
        as_account(f"coredumpctl --no-pager dump {pid} --output=/tmp/core 2>&1")
    )
    assert status != 0, f"coredumpctl dumped the core for the account: {said}"
    assert "Permission denied" in said, f"coredumpctl failed some other way: {said}"


def test_the_owner_still_reads_the_stack_trace(machine: BaseMachine, cores: Cores) -> None:
    """The journal still shows the account, in wheel, each crash's stack trace."""
    for pid, _ in cores.values():
        traced = f"{stack_trace(pid)} | grep -q 'Stack trace of thread'"
        machine.wait_until_succeeds(traced, timeout=KEEPING)
        said = machine.succeed(stack_trace(pid))
        assert "Stack trace of thread" in said, f"no stack trace for {pid}: {said}"


def test_a_nodes_core_goes_six_hours_on_and_no_other(machine: BaseMachine, cores: Cores) -> None:
    """Six hours after it was kept, the removal takes a node's core and leaves another's."""
    machine.succeed("systemctl is-active coredump-forget-node.timer")
    _, node = cores["node"]
    _, other = cores["sleep"]
    both = f"{shlex.quote(node)} {shlex.quote(other)}"
    machine.succeed(f"touch -d '5 hours ago' {both}")
    machine.succeed("systemctl start coredump-forget-node.service")
    machine.succeed(f"test -e {shlex.quote(node)}")
    machine.succeed(f"touch -d '7 hours ago' {both}")
    machine.succeed("systemctl start coredump-forget-node.service")
    machine.fail(f"test -e {shlex.quote(node)}")
    machine.succeed(f"test -e {shlex.quote(other)}")

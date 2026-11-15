"""Where a pi host's busy command goes, in a virtual machine.

hosts.nix, beside this, runs this through pytest inside the driver (tests/lib/vm_pytest.py). A
stand-in host's unit in agents.slice starts a command that spins, and that command starts another
in a session of its own, as Nix starts ssh to a builder. The sorter moves the command whole to
jobs, where the kernel ends it first when memory runs out, while the host stays in its unit and a
stand-in session beside it, in agents.slice, keeps waking on time. The programs the units run are
hosts.nix's, given as the `values` fixture.
"""

from __future__ import annotations

import datetime as dt
from typing import TYPE_CHECKING, Any

import pytest

if TYPE_CHECKING:
    from test_driver.machine import BaseMachine

# How long a unit or a move may take to appear.
APPEARING = dt.timedelta(seconds=30)
# The scopes the sorter holds below jobs.slice for work an agent writes there.
WORK_SCOPE = "jobs.slice/jobs-work.slice/work-.*\\.scope$"
# The adjustment the sorter gives agents' work, so the kernel's OOM killer ends it first.
FIRST = 1000
# How many tenths of a second the stand-in session waits through, and how late it may wake,
# in microseconds, as slices-vm-test.py holds the foreground to.
WAKES = 150
LATEST_WAKE = 100_000
# The seconds the command's own child sleeps, which name it here.
CHILD = 1041
# Where the stand-in session writes how late it woke, in alice's own home.
LATE = "/home/alice/session-late"


def user(command: str) -> str:
    """Give the command as one run in alice's account's manager."""
    return f"systemctl -M alice@ --user {command}"


def read(machine: BaseMachine, path: str) -> str:
    """Give what the kernel's file holds, without the trailing newline."""
    return machine.succeed(f"cat {path}").strip()


@pytest.fixture(scope="module")
def up(machine: BaseMachine) -> BaseMachine:
    """Give the machine once alice's sorter runs."""
    machine.wait_for_unit("user@1000.service")
    machine.wait_until_succeeds(user("is-active jobs-sorter.service"), timeout=APPEARING)
    return machine


def test_a_hosts_busy_command_goes_to_jobs_whole_and_its_sessions_wake_on_time(
    up: BaseMachine, values: dict[str, Any]
) -> None:
    """A host's runaway goes to jobs with what it started, the host stays, its session wakes."""
    bash, sleep, setsid = values["bash"], values["sleep"], values["setsid"]
    up.succeed(
        "systemd-run -M alice@ --user --slice=agents.slice --unit=mycelium-pi-test"
        f" {bash} {values['wakes']} {sleep} {WAKES} {LATE}"
    )
    up.succeed(
        "systemd-run -M alice@ --user --slice=agents.slice --unit=pi-host-test"
        f" --expand-environment=no {bash} -c"
        f" '{setsid} {bash} -c \"{setsid} {sleep} {CHILD} & while :; do :; done\" & wait'"
    )
    child = f"$(pgrep -xf '{sleep} {CHILD}')"
    up.wait_until_succeeds(f"grep -q '{WORK_SCOPE}' /proc/{child}/cgroup", timeout=APPEARING)
    host = up.succeed(user("show -P MainPID pi-host-test.service")).strip()
    here = read(up, f"/proc/{host}/cgroup")
    assert here.endswith("agents.slice/pi-host-test.service"), f"the host left its unit: {here}"
    # The sorter ranks work for the OOM killer on a look of its own, after the move.
    up.wait_until_succeeds(f"grep -qx {FIRST} /proc/{child}/oom_score_adj", timeout=APPEARING)
    host_adj = int(read(up, f"/proc/{host}/oom_score_adj"))
    assert host_adj < FIRST, f"the host is as first for the OOM killer as its command: {host_adj}"
    up.wait_until_succeeds(f"test -s {LATE}", timeout=dt.timedelta(seconds=WAKES / 10) + APPEARING)
    late = int(read(up, LATE))
    up.succeed(f"kill -- -{child} $(ps -o pgid= -p $(ps -o ppid= -p {child}) | tr -d ' ')")
    # The host ends of itself once the command it waits on does.
    up.wait_until_fails(user("is-active pi-host-test.service"), timeout=APPEARING)
    up.succeed(f"rm {LATE}")
    assert late < LATEST_WAKE, f"the session beside the host woke {late} us late"

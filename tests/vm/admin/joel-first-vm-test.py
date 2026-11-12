"""Joel's shell keeps drawing on time while agents' work bursts for memory.

joel-first.nix, beside this, runs this through pytest inside the driver (tests/lib/vm_pytest.py).
It replays 12 November's 16:33 freeze on a machine of an eighth of optimus's memory. Agents'
sessions, their work, the work other sessions wait on and Joel's apps first hold memory, each in
its slice, the sessions and the build using theirs over and over. A stand-in shell then draws a
frame every 16 ms in session.slice, and a few seconds in, a fetch in agents.slice and a grep and
three smaller commands in jobs.slice burst for more, as they did. As then, every slice stays
under its own ceiling, and together they pass memory by a fifth.
"""

from __future__ import annotations

import json
from typing import TYPE_CHECKING, Any

import pytest

if TYPE_CHECKING:
    from test_driver.machine import BaseMachine

ACCOUNT = "/sys/fs/cgroup/user.slice/user-1000.slice/user@1000.service"
RUNTIME = "/run/user/1000"
FILES = "/home/alice"

# What each slice holds, in MiB. Joel's apps and the shell hold what they
# held on optimus at 17:06 divided by eight, 5.9 GiB and 2.5. The rest is
# sized as 16:33 was: agents' sessions, with the fetch's 3.5 GiB, half read
# and half its own, and jobs, with the grep's reading and three commands of
# 1.4 to 1.8 GiB, each just under its slice's ceiling of today, and shared's
# file cache under its own, so that only together do they pass memory.
SESSIONS = 20
SESSION = 150
JOBS_HELD = 1536
SHARED_READ = 1000
APPS = 768
SCENE = 320
FETCH_HELD = 450
FETCH_READ = 450
GREP_READ = 1000
COMMANDS = 3
COMMAND = 350
# How often each walks what it holds, in seconds: a session's collector now
# and then, a build and the burst's work all the time.
SESSION_WALK = 3
WORK_WALK = 1

# How long the shell draws, and how far in the burst comes.
DRAWING = 40
BURST_AFTER = 5
# The latest a frame may come, in milliseconds: a frame's time, past which
# it misses the screen's next refresh and is dropped. On today's slices of
# 12 November the worst came 108 ms and 34 ms late in two runs; with the
# floors and ceilings fitted, 5.6 ms.
LATEST = 16

READS = {
    "pushed": SHARED_READ,
    "fetched": FETCH_READ,
    "grepped": GREP_READ,
}


class Account:
    """Alice's account, where every stand-in runs as a unit of her manager's."""

    def __init__(self, machine: BaseMachine, values: dict[str, Any]) -> None:
        """Act on alice's account on the machine, running the values' programs."""
        self.machine = machine
        self.python: str = values["python"]
        self.frames: str = values["frames"]
        self.heap: str = values["heap"]
        self.work: str = values["work"]

    def run(self, unit: str, slice_: str, command: str) -> None:
        """Run the command as a unit of alice's in the slice."""
        self.machine.succeed(
            f"systemd-run -M alice@ --user --slice={slice_} --unit={unit} {command}"
        )

    def hold(self, unit: str, slice_: str, mib: int) -> None:
        """Hold that many MiB of alice's own in the slice, untouched, and wait until it's held."""
        said = f"{RUNTIME}/{unit}.pid"
        self.run(unit, slice_, f"{self.python} {self.heap} {mib} {said}")
        self.machine.wait_for_file(said, timeout=120)

    def start_working(self, unit: str, slice_: str, mib: int, every: float) -> str:
        """Start using that many MiB in the slice every so often, giving where it says it's full."""
        said = f"{RUNTIME}/{unit}.pid"
        self.run(unit, slice_, f"{self.python} {self.work} {mib} {every} {said}")
        return said

    def work_with(self, unit: str, slice_: str, mib: int, every: float) -> None:
        """Use that many MiB in the slice every so often, once it's all held."""
        self.machine.wait_for_file(self.start_working(unit, slice_, mib, every), timeout=120)

    def read(self, name: str, slice_: str) -> None:
        """Read the named file in the slice, so its pages are the slice's cache."""
        self.run(
            name, slice_, f"/run/current-system/sw/bin/dd if={FILES}/{name} of=/dev/null bs=1M"
        )

    def used(self, slice_: str) -> int:
        """Give the slice's memory, in MiB."""
        return int(self.machine.succeed(f"cat {ACCOUNT}/{slice_}/memory.current")) >> 20


def stalls(machine: BaseMachine) -> int:
    """Give how many times a program has had to reclaim memory itself."""
    return int(machine.succeed("awk '/^allocstall_/ { n += $2 } END { print n + 0 }' /proc/vmstat"))


@pytest.fixture(scope="module")
def loaded(machine: BaseMachine, values: dict[str, Any]) -> Account:
    """Give alice's account holding what optimus's slices held before the burst."""
    machine.wait_for_unit("user@1000.service")
    # Written by root, the files' pages would be root's; out of the cache,
    # each is charged to the slice that reads it first.
    for name, mib in READS.items():
        machine.succeed(f"head -c {mib}M /dev/urandom > {FILES}/{name}")
    machine.succeed(f"chown alice {FILES}/* && sync && echo 3 > /proc/sys/vm/drop_caches")
    account = Account(machine, values)
    account.hold("apps", "app.slice", APPS)
    for n in range(SESSIONS):
        account.work_with(f"session-{n}", "agents.slice", SESSION, SESSION_WALK)
    account.work_with("build", "jobs.slice", JOBS_HELD, WORK_WALK)
    account.read("pushed", "shared.slice")
    return account


def test_the_shell_draws_on_time_through_a_burst_of_work(loaded: Account) -> None:
    """Joel's shell drops no frame while agents' work bursts."""
    machine = loaded.machine
    result = f"{RUNTIME}/frames.json"
    before = stalls(machine)
    loaded.run(
        "shell",
        "session.slice",
        f"{loaded.python} {loaded.frames} {SCENE} {DRAWING} {result}",
    )
    machine.sleep(BURST_AFTER)
    loaded.start_working("fetch", "agents.slice", FETCH_HELD, WORK_WALK)
    loaded.read("fetched", "agents.slice")
    loaded.read("grepped", "jobs.slice")
    for n in range(COMMANDS):
        loaded.start_working(f"command-{n}", "jobs.slice", COMMAND, WORK_WALK)
    machine.wait_for_file(result, timeout=DRAWING * 4)
    frames = json.loads(machine.succeed(f"cat {result}"))
    held = {
        slice_: loaded.used(slice_)
        for slice_ in ("session.slice", "app.slice", "agents.slice", "jobs.slice")
    }
    said = f"{frames}, {stalls(machine) - before} reclaims by programs themselves, {held} MiB"
    print(f"the shell's frames: {said}")
    assert frames["frames"] >= DRAWING * 50, f"the shell drew too few frames: {said}"
    assert frames["worst"] <= LATEST, f"a frame came past {LATEST} ms: {said}"

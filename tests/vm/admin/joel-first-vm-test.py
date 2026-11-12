"""Joel's shell keeps drawing on time while agents' work bursts for memory.

joel-first.nix, beside this, runs this through pytest inside the driver (tests/lib/vm_pytest.py).
It replays 12 November's 16:33 freeze at an eighth of optimus's size. Agents' sessions, their
work, the work other sessions wait on and Joel's apps first hold what they held then, each in its
slice. A stand-in shell then draws a frame every 16 ms in session.slice, and a few seconds in, a
fetch in agents.slice and a grep and three smaller commands in jobs.slice want a quarter more of
memory within seconds, as they did. Every size is optimus's divided by eight.
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

# What each slice held on optimus at 17:06, in MiB, divided by eight: agents'
# sessions 20 GiB in 80 windows, kept here as 20 heaps; jobs 13.7 GiB, most
# of it a build's and a grep's; shared's file cache 4.7 GiB; Joel's apps 5.9.
SESSIONS = 20
SESSION = 128
JOBS_HELD = 1024
JOBS_READ = 512
SHARED_READ = 512
APPS = 768
# The shell's scene, session.slice's 2.5 GiB divided by eight.
SCENE = 320
# The burst: the fetch's 3.5 GiB, half read and half its own, the grep's
# 7.1 GiB of reading, and three commands of 1.4 to 1.8 GiB beside them.
FETCH_HELD = 224
FETCH_READ = 224
GREP_READ = 896
COMMANDS = 3
COMMAND = 200

# How long the shell draws, and how far in the burst comes.
DRAWING = 40
BURST_AFTER = 5
# The latest a frame may come, in milliseconds: three frames, past which a
# key's echo is a visible stall.
LATEST = 50


class Account:
    """Alice's account, where every stand-in runs as a unit of her manager's."""

    def __init__(self, machine: BaseMachine, values: dict[str, Any]) -> None:
        """Act on alice's account on the machine, running the values' programs."""
        self.machine = machine
        self.python: str = values["python"]
        self.frames: str = values["frames"]
        self.heap: str = values["heap"]

    def run(self, unit: str, slice_: str, command: str) -> None:
        """Run the command as a unit of alice's in the slice."""
        self.machine.succeed(
            f"systemd-run -M alice@ --user --slice={slice_} --unit={unit} {command}"
        )

    def start_holding(self, unit: str, slice_: str, mib: int) -> str:
        """Start holding that many MiB of alice's own in the slice, giving where it says so."""
        said = f"{RUNTIME}/{unit}.pid"
        self.run(unit, slice_, f"{self.python} {self.heap} {mib} {said}")
        return said

    def hold(self, unit: str, slice_: str, mib: int) -> None:
        """Hold that many MiB of alice's own in the slice, and wait until it's held."""
        self.machine.wait_for_file(self.start_holding(unit, slice_, mib), timeout=120)

    def read(self, unit: str, slice_: str, mib: int) -> None:
        """Read a file of that many MiB in the slice, so its pages are the slice's cache."""
        path = f"{FILES}/{unit}"
        self.machine.succeed(f"fallocate -l {mib}M {path} && chown alice {path}")
        self.run(unit, slice_, f"/run/current-system/sw/bin/dd if={path} of=/dev/null bs=1M")

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
    account = Account(machine, values)
    account.hold("apps", "app.slice", APPS)
    for n in range(SESSIONS):
        account.hold(f"session-{n}", "agents.slice", SESSION)
    account.hold("build", "jobs.slice", JOBS_HELD)
    account.read("built", "jobs.slice", JOBS_READ)
    account.read("pushed", "shared.slice", SHARED_READ)
    return account


def test_the_shell_draws_on_time_through_a_burst_of_work(loaded: Account) -> None:
    """Joel's shell draws every frame within three of its time while agents' work bursts."""
    machine = loaded.machine
    result = f"{RUNTIME}/frames.json"
    before = stalls(machine)
    loaded.run(
        "shell",
        "session.slice",
        f"{loaded.python} {loaded.frames} {SCENE} {DRAWING} {result}",
    )
    machine.sleep(BURST_AFTER)
    loaded.start_holding("fetch", "agents.slice", FETCH_HELD)
    loaded.read("fetched", "agents.slice", FETCH_READ)
    loaded.read("grepped", "jobs.slice", GREP_READ)
    for n in range(COMMANDS):
        loaded.start_holding(f"command-{n}", "jobs.slice", COMMAND)
    machine.wait_for_file(result, timeout=DRAWING * 4)
    frames = json.loads(machine.succeed(f"cat {result}"))
    held = {slice_: loaded.used(slice_) for slice_ in ("agents.slice", "jobs.slice")}
    said = f"{frames}, {stalls(machine) - before} reclaims by programs themselves, {held} MiB"
    assert frames["frames"] >= DRAWING * 50, f"the shell drew too few frames: {said}"
    assert frames["worst"] <= LATEST, f"a frame came past {LATEST} ms: {said}"

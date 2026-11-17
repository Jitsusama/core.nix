"""Where a pi host's busy command goes, in a virtual machine.

hosts.nix, beside this, runs this through pytest inside the driver (tests/lib/vm_pytest.py). A
stand-in host's unit in agents.slice starts a command that spins, and that command starts another
in a session of its own, as Nix starts ssh to a builder. The sorter moves the command whole to
jobs, where the kernel ends it first when memory runs out, while the host stays in its unit and a
stand-in session beside it, in agents.slice, keeps waking on time. Through a call, which the sorter
hears in the account's PipeWire, builds and jobs are held to a quarter of the processors, held
again when something clears that mid-call, and let go once it ends. The programs the units run are
hosts.nix's, given as the `values` fixture.
"""

from __future__ import annotations

import datetime as dt
import math
import shlex
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
# The slices a call brakes: jobs in alice's account, builds in the system's.
JOBS = "/sys/fs/cgroup/user.slice/user-1000.slice/user@1000.service/jobs.slice"
BUILDS = "/sys/fs/cgroup/build.slice"
# The period systemd writes in cpu.max beside a quota, or beside none, in microseconds.
PERIOD = 100_000
# How long the call program's stream may take to start running, as calls-vm-test.py gives it.
RECORDS_WITHIN = dt.timedelta(seconds=30)
# How long the sorter may take to brake once the stream runs, which is a call after two seconds,
# or to let go once it stops, five seconds after.
BRAKING = dt.timedelta(seconds=30)
# How long PipeWire runs with nothing recording before that's taken as no call: past the two
# seconds a recording takes to count as one.
NO_CALL_FOR = 5
# The call program: PulseAudio's recorder, copied under a call program's name, which PipeWire's
# PulseAudio server names its stream for.
CALLER = "/home/alice/zoom"
# The call program's stream, running, as jq finds it in pw-dump's graph.
RECORDING = (
    '.[] | select(.info.props["application.process.binary"] == "zoom" and .info.state == "running")'
)
# The slices' names, as the sorter says them.
BRAKED = ("jobs.slice", "build.slice")
# What the sorter says once it finds a slice it braked let go mid-call and brakes it again.
BRAKED_AGAIN = "Found {} let go mid-call, and braked it again."
# What the sorter says once a call has ended and it has let go of both slices.
LET_GO = "Let go of builds and jobs, no call on."
# The sorter's journal, read as root.
SORTER = "journalctl -b _SYSTEMD_USER_UNIT=jobs-sorter.service -o cat --no-pager"


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
    try:
        busy_host_goes_to_jobs(up, values)
    finally:
        # The command and its spinners, which a test failing partway leaves running. The bracket
        # keeps the pattern from matching the shell that runs pkill.
        up.execute(f"pkill -f 'bin/[s]leep {CHILD}'")


def busy_host_goes_to_jobs(up: BaseMachine, values: dict[str, Any]) -> None:
    """Run the host's runaway beside the session, and check where each went and how it woke."""
    bash, sleep, setsid = values["bash"], values["sleep"], values["setsid"]
    up.succeed(
        "systemd-run -M alice@ --user --slice=agents.slice --unit=mycelium-pi-test"
        f" {bash} {values['wakes']} {sleep} {WAKES} {LATE}"
    )
    up.succeed(
        "systemd-run -M alice@ --user --slice=agents.slice --unit=pi-host-test"
        f" --expand-environment=no {bash} -c"
        # A spinner on each of the machine's processors, so the session has none to spare.
        f" '{setsid} {bash} -c \"{setsid} {sleep} {CHILD} &"
        f" for _ in {{1..{values['processors']}}}; do while :; do :; done & done; wait\" & wait'"
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


def said(machine: BaseMachine) -> str:
    """Give the sorter's journal, with each slice's cpu.max and the call's own journal."""
    return "\n".join(
        machine.execute(command)[1]
        for command in (
            f"head {JOBS}/cpu.max {BUILDS}/cpu.max",
            SORTER,
            "journalctl -b _SYSTEMD_USER_UNIT=call.service -o cat --no-pager",
        )
    )


def held_to(machine: BaseMachine, cpu_max: str) -> None:
    """Wait until both slices' cpu.max read cpu_max, saying what the sorter heard if they don't."""
    try:
        for folder in (JOBS, BUILDS):
            machine.wait_until_succeeds(f"grep -qx '{cpu_max}' {folder}/cpu.max", timeout=BRAKING)
    except Exception:
        print(said(machine))
        raise


def times_said(machine: BaseMachine, line: str) -> int:
    """Give how many times the sorter has said the line since the machine booted."""
    return int(machine.succeed(f"{SORTER} | grep -cF {shlex.quote(line)} || true").strip() or 0)


def says_again(machine: BaseMachine, line: str, before: int) -> None:
    """Wait until the sorter has said the line more times than before."""
    try:
        machine.wait_until_succeeds(
            f"[ $({SORTER} | grep -cF {shlex.quote(line)}) -gt {before} ]", timeout=BRAKING
        )
    except Exception:
        print(said(machine))
        raise


def test_a_call_keeps_builds_and_jobs_braked_till_it_ends(
    up: BaseMachine, values: dict[str, Any]
) -> None:
    """Through a call builds and jobs hold a quarter of the processors, held again if cleared."""
    sleep = values["sleep"]
    sorter = up.succeed(user("show -P InvocationID jobs-sorter.service")).strip()
    try:
        # Work in each slice, so each is there to read, as builds' is while a build runs.
        up.succeed(f"systemd-run --slice=build.slice --unit=a-build {sleep} 1042")
        up.succeed(f"systemd-run -M alice@ --user --slice=jobs.slice --unit=a-job {sleep} 1043")
        processors = int(up.succeed("nproc"))
        quota = math.ceil(processors / 4) * PERIOD
        assert quota < processors * PERIOD, f"a quarter of {processors} processors is all of them"
        braked, free = f"{quota} {PERIOD}", f"max {PERIOD}"
        try:
            up.succeed(user("start pipewire.service pipewire-pulse.service"))
            up.wait_for_unit("wireplumber.service", "alice")
        except Exception:
            print(up.execute("journalctl -b --user-unit=pipewire.service --no-pager")[1])
            raise
        # PipeWire running with nothing recording is no call, however long it runs.
        up.sleep(NO_CALL_FOR)
        held_to(up, free)
        up.succeed(f'install -o alice -m 755 "$(readlink -f {values["pacat"]})" {CALLER}')
        up.succeed(
            "systemd-run -M alice@ --user --unit=call --collect"
            f" {CALLER} --record --device=silent-microphone /dev/null"
        )
        try:
            up.wait_until_succeeds(
                "su - alice -c"
                f" 'XDG_RUNTIME_DIR=/run/user/1000 {values['pwDump']} --no-colors'"
                f" | {values['jq']} -e '{RECORDING}'",
                timeout=RECORDS_WITHIN,
            )
        except Exception:
            print(said(up))
            print(up.execute("journalctl -b --user-unit=wireplumber.service --no-pager")[1])
            dump = f"XDG_RUNTIME_DIR=/run/user/1000 {values['pwDump']} --no-colors"
            print(up.execute(f"su - alice -c '{dump}'")[1])
            raise
        held_to(up, braked)
        # Something clears both brakes mid-call, as a set-property naming no quota does.
        before = {name: times_said(up, BRAKED_AGAIN.format(name)) for name in BRAKED}
        up.succeed(user("set-property --runtime jobs.slice CPUQuota="))
        up.succeed("systemctl set-property --runtime build.slice CPUQuota=")
        for name, times in before.items():
            says_again(up, BRAKED_AGAIN.format(name), times)
        held_to(up, braked)
        let_go = times_said(up, LET_GO)
        up.succeed(user("stop call.service"))
        says_again(up, LET_GO, let_go)
        held_to(up, free)
        # The sorter let go itself, the one that braked, not by ending and having its stop clear
        # both, nor a restarted one finding no call.
        still = up.succeed(user("show -P InvocationID jobs-sorter.service")).strip()
        assert still == sorter, f"the sorter was started again mid-test: {sorter} then {still}"
    finally:
        up.execute(user("stop call.service"))
        up.execute("systemctl stop a-build.service")
        up.execute(user("stop a-job.service"))

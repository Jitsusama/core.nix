"""What the kernel gives each slice, the account's and the system's, in a virtual machine.

tests/slices.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), and its
opening comment says why the kernel is asked rather than the module's settings read back. Each
test asks for the state it starts from as a fixture: the account's slices held open on the
machine naming no disk, its jobs' holders running, or the measured machine with its own slices
held open. A test that starts work names it by its own seconds, so any test runs alone with
`-k`, and the one that stops the measured disk's sharing comes last. The programs the units run
are slices.nix's, given as the `values` fixture.
"""

from __future__ import annotations

import datetime as dt
import json
from typing import TYPE_CHECKING, Any

import pytest

if TYPE_CHECKING:
    from test_driver.driver import Driver
    from test_driver.machine import BaseMachine

ACCOUNT = "/sys/fs/cgroup/user.slice/user-1000.slice/user@1000.service"
JOBS = f"{ACCOUNT}/jobs.slice"
TIMED = f"{JOBS}/jobs-timed.slice"
WORK = f"{JOBS}/jobs-work.slice"
MIB = 2**20
# The memory shares the module keeps for apps, and for every slice above them.
APPS_SHARE = 0.13
ABOVE_SHARE = 0.16
SESSION_FLOOR = 1536 * MIB
# The share of memory past which work in jobs is slowed.
JOBS_HIGH = 0.5
# How much of a CPU the light quest must have, alone, over three seconds.
ALONE_USEC = 3_000_000 * 0.8
# How far from four to one the heavy quest's share against the light one's may go on a
# virtual CPU.
LEAST_RATIO = 2.5
MOST_RATIO = 6.5
# How long a unit, a scope or a weight may take to appear.
APPEARING = dt.timedelta(seconds=30)
# The scopes held below jobs.slice by work an agent writes there, as the sorter names them.
WORK_SCOPE = "jobs-work.slice/work-.*\\.scope$"


class Account:
    """Alice's account on a machine, where the programs its units run are the values'."""

    def __init__(self, machine: BaseMachine, values: dict[str, Any]) -> None:
        """Act on alice's account on that machine, running the values' programs."""
        self.machine = machine
        self.sleep: str = values["sleep"]
        self.bash: str = values["bash"]
        self.chrt: str = values["chrt"]
        self.taskset: str = values["taskset"]

    def hold(self, name: str) -> None:
        """Hold the account's slice open, since its cgroup exists only while something runs."""
        self.machine.succeed(
            f"systemd-run -M alice@ --user --slice={name}.slice --unit=in-{name}"
            f" {self.sleep} infinity"
        )
        self.machine.wait_until_succeeds(
            f"systemctl -M alice@ --user is-active in-{name}.service", timeout=APPEARING
        )

    def read(self, path: str) -> str:
        """Give what the kernel's file holds, without the trailing newline."""
        return self.machine.succeed(f"cat {path}").strip()

    def write_into_jobs(
        self, unit: str, cgroup: str, command: str, quest: str | None = None
    ) -> None:
        """Write a shell into the cgroup as pi does, to run the command once it's sorted."""
        named = f" -E PI_QUEST={quest}" if quest else ""
        self.machine.succeed(
            f"systemd-run -M alice@ --user --slice=jobs.slice --unit={unit}{named}"
            f" --expand-environment=no -p KillMode=process {self.bash} -c"
            f" 'echo $$ > {cgroup}/cgroup.procs;"
            " until grep -q /work- /proc/$$/cgroup; do sleep 0.1; done;"
            f" {command} & wait'"
        )

    def sleeping(self, seconds: int) -> str:
        """Give the process id of the sleep that waits the given seconds."""
        return f"$(pgrep -xf '{self.sleep} {seconds}')"

    def cgroup_of(self, seconds: int) -> str:
        """Give the cgroup of the sleep that waits the given seconds."""
        return self.read(f"/proc/{self.sleeping(seconds)}/cgroup")

    def wait_until_in(self, seconds: int, pattern: str) -> None:
        """Wait until the sleep that waits the given seconds is in a cgroup matching it."""
        self.machine.wait_until_succeeds(
            f"grep -q '{pattern}' /proc/{self.sleeping(seconds)}/cgroup", timeout=APPEARING
        )

    def weigh_quests(self, weights: dict[str, int]) -> None:
        """Write each quest's weight where pi's sessions keep it."""
        self.machine.succeed("install -d -o alice -m 700 /run/user/1000/jobs")
        self.machine.succeed(
            f"echo '{json.dumps(weights)}' > /run/user/1000/jobs/quests.json.tmp"
            " && mv /run/user/1000/jobs/quests.json.tmp /run/user/1000/jobs/quests.json"
        )

    def floor(self, path: str) -> int:
        """Give the memory, in bytes, the kernel keeps for the cgroup whatever else needs it."""
        return int(self.read(f"{path}/memory.min"))

    def memory(self) -> int:
        """Give the memory the kernel reports, in bytes, which systemd turns a share into."""
        return int(self.machine.succeed("awk '/MemTotal/ {print $2}' /proc/meminfo")) * 1024


@pytest.fixture(scope="module")
def started(driver: Driver) -> None:
    """Boot both machines at once, so a slow host waits for one boot rather than two."""
    driver.start_all()


@pytest.fixture(scope="module")
def account(started: None, machine: BaseMachine, values: dict[str, Any]) -> Account:
    """Hold the account's slices and machine.slice open on the machine naming no disk."""
    del started
    machine.wait_for_unit("user@1000.service")
    held = Account(machine, values)
    for name in ["app", "session", "background"]:
        held.hold(name)
    machine.succeed(f"systemd-run --slice=machine.slice --unit=in-machine {held.sleep} infinity")
    machine.wait_for_unit("in-machine.service")
    return held


@pytest.fixture(scope="module")
def drive(account: Account) -> str:
    """Give the major and minor number of the drive behind /home, the partition's parent."""
    source = account.machine.succeed("findmnt -nvo SOURCE -T /home").strip()
    parent = account.machine.succeed(f"lsblk -dno PKNAME {source}").strip()
    return account.read(f"/sys/class/block/{parent or source.removeprefix('/dev/')}/dev")


@pytest.fixture(scope="module")
def jobs(account: Account) -> Account:
    """Wait for the holders of timed and awaited work to run, as their cgroups need."""
    for holder in ["timed", "awaited"]:
        account.machine.wait_until_succeeds(
            f"systemctl -M alice@ --user is-active {holder}.service", timeout=APPEARING
        )
    return account


@pytest.fixture(scope="module")
def measured(started: None, machines: dict[str, BaseMachine], values: dict[str, Any]) -> Account:
    """Hold app.slice and session.slice open on the machine whose disk has a model."""
    del started
    ordered = machines["ordered"]
    ordered.wait_for_unit("user@1000.service")
    held = Account(ordered, values)
    for name in ["app", "session"]:
        held.hold(name)
    ordered.wait_for_unit("disk-order-vda.service")
    return held


def latency(account: Account, name: str, under: str = ACCOUNT) -> str:
    """Give the latency target the kernel holds for the slice, under its holder's cgroup."""
    return account.read(f"{under}/{name}.slice/io.latency")


def cost(measured: Account, name: str) -> list[str]:
    """Give the settings io.cost holds for the measured disk, from its model or qos file."""
    vda = measured.read("/sys/class/block/vda/dev")
    lines = measured.read(f"/sys/fs/cgroup/io.cost.{name}").splitlines()
    return next(line.split()[1:] for line in lines if line.startswith(f"{vda} "))


def test_what_joel_uses_has_a_millisecond(account: Account, drive: str) -> None:
    """What Joel uses has a millisecond on the drive behind /home."""
    assert latency(account, "app") == f"{drive} target=1000"
    assert latency(account, "session") == f"{drive} target=1000"


def test_the_work_it_starts_has_none(account: Account) -> None:
    """The work it starts has none, so it gives way."""
    assert latency(account, "jobs") == ""
    assert latency(account, "background") == ""


def test_every_account_has_it_and_machines_none(account: Account, drive: str) -> None:
    """Among the system's, every account has it, and machines none."""
    assert latency(account, "user", "/sys/fs/cgroup") == f"{drive} target=1000"
    assert latency(account, "machine", "/sys/fs/cgroup") == ""
    assert latency(account, "system", "/sys/fs/cgroup") == ""


def test_apps_keep_a_hard_share_of_memory(account: Account) -> None:
    """Apps keep a hard share of memory, and every slice above more."""
    total = account.memory()
    for path, share in [
        (f"{ACCOUNT}/app.slice", APPS_SHARE),
        (ACCOUNT, ABOVE_SHARE),
        ("/sys/fs/cgroup/user.slice/user-1000.slice", ABOVE_SHARE),
        ("/sys/fs/cgroup/user.slice", ABOVE_SHARE),
    ]:
        assert abs(account.floor(path) - total * share) <= MIB, path
    assert account.floor(f"{ACCOUNT}/session.slice") == SESSION_FLOOR
    assert account.floor(JOBS) == 0


def test_work_in_jobs_is_slowed_never_ended(account: Account) -> None:
    """Work in jobs is slowed past half of memory, and never ended."""
    high = int(account.read(f"{JOBS}/memory.high"))
    assert abs(high - account.memory() * JOBS_HIGH) <= MIB
    assert account.read(f"{JOBS}/memory.max") == "max"


def test_timed_work_is_inside_idle_jobs(jobs: Account) -> None:
    """Timed work is inside idle jobs, far above its holder."""
    assert jobs.read(f"{JOBS}/cpu.idle") == "1"
    assert jobs.read(f"{TIMED}/cpu.weight") == "10000"
    assert jobs.read(f"{JOBS}/jobs-awaited.slice/cpu.weight") == "1000"
    assert jobs.read(f"{JOBS}/jobs.service/cpu.weight") == "100"


def test_work_in_jobs_moves_itself_in(jobs: Account) -> None:
    """Work in jobs moves itself in, as pi's waiting shell does."""
    jobs.machine.succeed(
        "systemd-run -M alice@ --user --slice=jobs.slice --unit=mover"
        f" --expand-environment=no -p KillMode=process {jobs.bash} -c"
        f" 'echo $$ > {TIMED}/timed.service/cgroup.procs;"
        " cat /proc/self/cgroup > /tmp/moved; exec sleep infinity'"
    )
    jobs.machine.wait_until_succeeds("grep -q timed.service /tmp/moved", timeout=APPEARING)


def test_a_scope_from_jobs_goes_back_unless_it_sets_limits(jobs: Account) -> None:
    """A scope that work in jobs starts goes back, unless it sets limits."""
    # An agent's shell in jobs starts two scopes with no slice, which systemd puts in
    # app.slice: one with a memory ceiling, then, once that one runs, one without, so the
    # sorter has looked at both by the time it moves the second.
    jobs.machine.succeed(
        "systemd-run -M alice@ --user --slice=jobs.slice --unit=agent"
        f" --expand-environment=no -p KillMode=process {jobs.bash} -c"
        f" 'echo $$ > {JOBS}/jobs.service/cgroup.procs;"
        f" systemd-run --user --scope --unit=kept -p MemoryMax=1G {jobs.sleep} 1002 &"
        " until systemctl --user is-active --quiet kept.scope; do sleep 0.1; done;"
        f" systemd-run --user --scope --unit=stray {jobs.sleep} 1001 & wait'"
    )
    # The shell itself was given a scope of its own in jobs by then, and the stray goes back
    # to it.
    jobs.wait_until_in(1001, WORK_SCOPE)
    assert "app.slice/kept.scope" in jobs.cgroup_of(1002)


def test_each_piece_of_work_gets_a_scope_of_its_own(jobs: Account) -> None:
    """Each piece of work written into jobs gets a scope of its own."""
    # Two commands written into jobs.service as pi writes its shells, one starting a child
    # once its own scope is there.
    for seconds in [1003, 1004]:
        jobs.write_into_jobs(f"piece-{seconds}", f"{JOBS}/jobs.service", f"{jobs.sleep} {seconds}")
    for seconds in [1003, 1004]:
        jobs.wait_until_in(seconds, f"jobs.slice/{WORK_SCOPE}")
    first, second = jobs.cgroup_of(1003), jobs.cgroup_of(1004)
    assert first != second
    # Together they weigh as jobs.service did, below a terminal's work.
    assert jobs.read(f"{WORK}/cpu.weight") == "100"
    # oomd reads each one's swap, as it does any leaf's.
    scope = first.removeprefix("0::").removeprefix("/user.slice/user-1000.slice/user@1000.service")
    jobs.machine.succeed(f"test -e {ACCOUNT}{scope}/memory.swap.current")
    # A scope that fails, as one does when its command exits before it's moved, is unloaded
    # rather than left in the failed list.
    unit = scope.rsplit("/", 1)[-1]
    collect = jobs.machine.succeed(f"systemctl -M alice@ --user show -P CollectMode {unit}")
    assert collect.strip() == "inactive-or-failed"
    # The holder stays, so the next write still lands.
    holder = jobs.machine.succeed("systemctl -M alice@ --user show -P MainPID jobs.service")
    assert holder.strip() in jobs.read(f"{JOBS}/jobs.service/cgroup.procs").split()


def test_work_for_a_quest_goes_in_its_slice(jobs: Account) -> None:
    """An agent's work for a quest goes in its quest's slice, at its weight."""
    jobs.weigh_quests({"QEST-20261024-DRIVEN": 400, "QEST-20261024-BENCHD": 50})
    into = f"{JOBS}/jobs.service"
    jobs.write_into_jobs("quest-1011", into, f"{jobs.sleep} 1011", "QEST-20261024-DRIVEN")
    jobs.write_into_jobs("quest-1012", into, f"{jobs.sleep} 1012")
    jobs.write_into_jobs("quest-1013", into, f"{jobs.sleep} 1013", "QEST-20260101-UNLIST")
    jobs.wait_until_in(1011, "jobs-work.slice/jobs-work-qest20261024driven.slice/work-.*\\.scope$")
    driven = f"{WORK}/jobs-work-qest20261024driven.slice"
    jobs.machine.wait_until_succeeds(f"grep -qx 400 {driven}/cpu.weight", timeout=APPEARING)
    assert jobs.read(f"{driven}/io.weight") == "default 400"
    # Work naming no quest, or one pi's sessions don't list, stays as it was.
    for seconds in [1012, 1013]:
        jobs.wait_until_in(seconds, WORK_SCOPE)
    # The quests' weights rank only agents' work: jobs-work.slice keeps its own.
    assert jobs.read(f"{WORK}/cpu.weight") == "100"


def test_a_quests_slice_follows_its_weight(jobs: Account) -> None:
    """A quest's slice follows its weight when pi's sessions change it."""
    followed = "jobs-work-qest20261024follow.slice"
    jobs.weigh_quests({"QEST-20261024-FOLLOW": 400})
    jobs.write_into_jobs(
        "quest-1014", f"{JOBS}/jobs.service", f"{jobs.sleep} 1014", "QEST-20261024-FOLLOW"
    )
    jobs.machine.wait_until_succeeds(
        f"grep -qx 400 {WORK}/{followed}/cpu.weight", timeout=APPEARING
    )
    jobs.weigh_quests({"QEST-20261024-FOLLOW": 50})
    jobs.machine.wait_until_succeeds(f"grep -qx 50 {WORK}/{followed}/cpu.weight", timeout=APPEARING)
    jobs.machine.succeed(f"systemctl -M alice@ --user stop {followed}")


def test_quests_share_the_cpu_by_weight(jobs: Account) -> None:
    """Quests share the CPU by weight, and one alone takes all of it."""

    def used(quest_slice: str) -> int:
        """Give the CPU, in microseconds, a quest's slice has used."""
        stat = jobs.read(f"{WORK}/{quest_slice}/cpu.stat")
        return int(
            next(line.split()[1] for line in stat.splitlines() if line.startswith("usage_usec"))
        )

    def spin(unit: str, quest: str) -> None:
        """Keep the first CPU busy for a quest, as a build would."""
        jobs.write_into_jobs(
            unit,
            f"{JOBS}/jobs.service",
            f'exec {jobs.taskset} -c 0 {jobs.bash} -c "while :; do :; done"',
            quest,
        )

    jobs.weigh_quests({"QEST-20261024-HEAVY0": 400, "QEST-20261024-LIGHT0": 100})
    heavy, light = "jobs-work-qest20261024heavy0.slice", "jobs-work-qest20261024light0.slice"
    spin("spin-light", "QEST-20261024-LIGHT0")
    jobs.machine.wait_until_succeeds(f"grep -qx 100 {WORK}/{light}/cpu.weight", timeout=APPEARING)
    # Alone, the light quest has the whole CPU, nobody else wanting it.
    before = used(light)
    jobs.machine.succeed("sleep 3")
    assert used(light) - before > ALONE_USEC
    spin("spin-heavy", "QEST-20261024-HEAVY0")
    jobs.machine.wait_until_succeeds(f"grep -qx 400 {WORK}/{heavy}/cpu.weight", timeout=APPEARING)
    jobs.machine.succeed("sleep 1")
    was = used(heavy), used(light)
    jobs.machine.succeed("sleep 5")
    ratio = (used(heavy) - was[0]) / max(used(light) - was[1], 1)
    # Four to one by weight, within what a virtual CPU's scheduling allows.
    assert LEAST_RATIO < ratio < MOST_RATIO, f"heavy against light: {ratio:.2f}"
    jobs.machine.succeed(f"systemctl -M alice@ --user stop {heavy} {light}")


def test_agents_lose_real_time_and_joel_keeps_it(jobs: Account) -> None:
    """Work from agents loses real time, and Joel's own work keeps it."""

    def real_time(seconds: int, cgroup: str) -> None:
        """Write a shell into the cgroup to sleep in SCHED_RR once it has a scope of its own."""
        jobs.machine.succeed(
            f"systemd-run -M alice@ --user --slice=jobs.slice --unit=rt-{seconds}"
            f" --expand-environment=no -p KillMode=process {jobs.bash} -c"
            f" 'echo $$ > {cgroup}/cgroup.procs;"
            " until grep -q /work- /proc/$$/cgroup; do sleep 0.1; done;"
            f" exec {jobs.chrt} --rr 1 {jobs.sleep} {seconds}'"
        )

    def rt_limit(seconds: int) -> list[str]:
        """Give the real-time priority limit of the sleep that waits the given seconds."""
        limits = jobs.machine.succeed(
            f"grep 'Max realtime priority' /proc/{jobs.sleeping(seconds)}/limits"
        )
        return limits.split()[3:5]

    # Joel's first: once an agent's has lost it, the sorter has looked at Joel's in the same
    # pass, while it ran in real time.
    real_time(1006, f"{JOBS}/jobs-awaited.slice/awaited.service")
    jobs.machine.wait_until_succeeds(
        f"{jobs.chrt} -p {jobs.sleeping(1006)} | grep -q SCHED_RR", timeout=APPEARING
    )
    real_time(1005, f"{JOBS}/jobs.service")
    jobs.machine.wait_until_succeeds(
        f"{jobs.chrt} -p {jobs.sleeping(1005)} | grep -q SCHED_OTHER", timeout=APPEARING
    )
    # Nor can it, or what it starts, take real time back.
    assert rt_limit(1005) == ["0", "0"]
    assert "SCHED_RR" in jobs.machine.succeed(f"{jobs.chrt} -p {jobs.sleeping(1006)}")
    assert rt_limit(1006) == ["10", "10"]


def test_on_the_disk_slices_weigh_as_on_the_cpu(jobs: Account) -> None:
    """On the disk, slices weigh as they do on the CPU."""
    for path, weight in [
        (f"{ACCOUNT}/session.slice", 1000),
        (f"{ACCOUNT}/app.slice", 100),
        (JOBS, 1),
        (TIMED, 10000),
        (f"{JOBS}/jobs-awaited.slice", 1000),
        (f"{ACCOUNT}/background.slice", 1),
        ("/sys/fs/cgroup/user.slice", 1000),
        ("/sys/fs/cgroup/system.slice", 100),
    ]:
        assert jobs.read(f"{path}/io.weight") == f"default {weight}", path


def test_a_measured_disk_is_shared_by_its_model(measured: Account) -> None:
    """A measured disk is shared by weight, by the model it was given."""
    model = cost(measured, "model")
    for setting in ["ctrl=user", "rbps=1000000000", "rrandiops=50000", "wseqiops=80000"]:
        assert setting in model
    qos = cost(measured, "qos")
    for setting in ["enable=1", "ctrl=user", "rlat=5000", "wlat=5000"]:
        assert setting in qos


def test_with_it_no_slice_holds_a_latency_target(measured: Account) -> None:
    """With it, no slice holds a latency target."""
    for path in [f"{ACCOUNT}/app.slice", f"{ACCOUNT}/session.slice", "/sys/fs/cgroup/user.slice"]:
        assert measured.read(f"{path}/io.latency") == "", path


def test_old_latency_targets_go_as_it_turns_on(measured: Account) -> None:
    """Latency targets left from before the disk was measured go as it turns on."""
    # A switch from a machine that had targets keeps them: systemd doesn't write over a
    # target it no longer manages, so they stay in the kernel until a reboot.
    vda = measured.read("/sys/class/block/vda/dev")
    held = [f"{ACCOUNT}/app.slice", f"{ACCOUNT}/session.slice", "/sys/fs/cgroup/user.slice"]
    for path in held:
        measured.machine.succeed(f"echo '{vda} target=1000' > {path}/io.latency")
    measured.machine.succeed("systemctl restart disk-order-vda.service")
    for path in held:
        assert measured.read(f"{path}/io.latency") == "", path


def test_stopping_it_hands_the_disk_back(measured: Account) -> None:
    """Stopping it hands the disk back to first come, first served.

    It turns the measured disk's sharing off, so it comes last.
    """
    measured.machine.succeed("systemctl stop disk-order-vda.service")
    assert "enable=0" in cost(measured, "qos")

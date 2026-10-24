"""A check's VM goes ahead of the builds beside it, each build in a cgroup of its own.

The daemon moves into a cgroup beside its builds, and each build gets one,
handed the controllers for CPU, disk, memory and pids. A build running a
process named as qemu is has every thread of it put to the normal policy at
nice -10, a thread it starts afterwards included, and its cgroup weighed at
1000 for CPU and disk, while a compiler's build beside it stays as the
daemon set it, at 100. A container's build, with no qemu in it, goes ahead
the same way, a service in it named nothing in particular too. A build
asked for from a quest's slice, as an agent's command asks, takes that
slice's weight, and follows it when it changes.
The daemon's own cgroup is marked for systemd-oomd to spare while a build
can go instead. Restarting the daemon, ten times over with vm-first looking
every 20 ms, leaves the builds running, and the new daemon starts and
builds again. A build making cgroups of its own, as a container does, is
refused handing them controllers, so the daemon restarts beside it too.
A build that has gone is said in vm-first's journal, with
what its client ran, its quest, what it wrote and read, and what it left
in memory still to write. Stopping a build leaves nothing of it. A build
asking for a range of user ids gets 65,536. A build asking for its
filesystem to be synced is told the call isn't implemented, while sync
itself still runs, there and outside the daemon.

tests/vm-first.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py). Each
test asks for the state it starts from as a fixture: the daemon started, a VM's and a
compiler's builds running beside it, or a quest's build beside those. The test that stops the
builds comes last, as it ends what the others look at.
"""

from __future__ import annotations

import datetime as dt
import re
from dataclasses import dataclass
from typing import TYPE_CHECKING

import pytest

if TYPE_CHECKING:
    from test_driver.machine import BaseMachine

# What a thread of the VM is put to, and what a build is left at: nice and
# scheduling class, as ps writes them.
AHEAD = "-10 TS"
AS_BUILT = "0 B"
# The ids a build that asks for a range of them is given.
RANGE = "65536"
DAEMON = "/sys/fs/cgroup/build.slice/nix-daemon.service"
# What the writer's build puts through to the disk, in MiB.
WRITTEN_MIB = 64
# A quest's slice, named as jobs-sorter names one under jobs-work.slice.
QUEST = "jobs-work-qestabc.slice"
# Through the daemon, as every agent's build goes: root alone may build in
# its own process, which puts the build in the caller's cgroup instead.
NIX_BUILD = "nix-build --store daemon --no-out-link"
# How many times the daemon is restarted under its builds.
RESTARTS = 10
# How long a build may take to start its process, or to be gone once stopped.
STARTING = dt.timedelta(minutes=1)
# How long vm-first or the daemon may take to act on what it watches.
ACTING = dt.timedelta(seconds=10)
# How long the VM may take to start the threads it runs with.
THREADING = dt.timedelta(seconds=20)
# How long a refusal may take to reach the journal through the daemon and its client.
REFUSING = dt.timedelta(seconds=30)


class Builds:
    """The machine's daemon, the builds it runs, and what vm-first says of them."""

    def __init__(self, machine: BaseMachine) -> None:
        """Build on that machine."""
        self.machine = machine

    def start(self, name: str) -> None:
        """Start a build of one of the machine's derivations, left running."""
        self.machine.succeed(f"systemd-run --unit={name} {NIX_BUILD} /etc/{name}.nix")

    def found(self, comm: str) -> str:
        """Wait for the process the kernel names so, and give its id."""
        self.machine.wait_until_succeeds(f"pgrep -x {comm}", timeout=STARTING)
        return self.machine.succeed(f"pgrep -x {comm}").strip()

    def cgroup_of(self, pid: str) -> str:
        """Give the process's cgroup, as a path under the cgroup file system."""
        return "/sys/fs/cgroup" + self.machine.succeed(f"cut -d: -f3 /proc/{pid}/cgroup").strip()

    def scheduling(self, pid: str) -> str:
        """Say each nice and class the process's threads run at, one a line."""
        return self.machine.succeed(
            f"ps -L -o ni=,cls= -p {pid} | awk '{{print $1, $2}}' | sort -u",
        ).strip()

    def wait_until_ahead(self, pid: str) -> None:
        """Wait for every thread of the process to go ahead."""
        self.machine.wait_until_succeeds(
            f"[ \"$(ps -L -o ni=,cls= -p {pid} | awk '{{print $1, $2}}' | sort -u)\" = '{AHEAD}' ]",
            timeout=ACTING,
        )

    def wait_for_the_daemon_aside(self) -> None:
        """Wait for the daemon to move itself into a cgroup beside its builds.

        systemd starts it in the service's own cgroup, and it moves a moment
        later, so a single look can land before the move.
        """
        self.machine.wait_until_succeeds(
            "grep -qx '0::/build.slice/nix-daemon.service/nix-daemon' "
            "/proc/$(systemctl show -P MainPID nix-daemon.service)/cgroup",
            timeout=ACTING,
        )

    def wait_for_the_controllers(self) -> None:
        """Wait for vm-first to hand the builds every controller."""
        self.machine.wait_until_succeeds(
            f"[ \"$(cat {DAEMON}/cgroup.subtree_control)\" = 'cpu io memory pids' ]",
            timeout=ACTING,
        )

    def restart_the_daemon(self) -> None:
        """Restart the daemon, forgetting earlier starts, and wait for it to move aside."""
        # Each restart forgets the ones before, which would otherwise pass
        # systemd's limit on starts.
        self.machine.succeed("systemctl reset-failed nix-daemon.service")
        self.machine.succeed("systemctl restart nix-daemon.service")
        self.machine.wait_for_unit("nix-daemon.service")
        self.wait_for_the_daemon_aside()

    def told(self) -> str:
        """Give everything vm-first has said."""
        return self.machine.succeed("journalctl -u vm-first.service -o cat")

    def told_of(self, etc: str) -> str:
        """Run the build in /etc/<etc> to its end, and read what vm-first said of it."""
        self.machine.succeed(f"{NIX_BUILD} /etc/{etc}")
        start = f"A build for `{NIX_BUILD} /etc/{etc}`, for no quest"
        self.machine.wait_until_succeeds(
            f"journalctl -u vm-first.service -o cat | grep -qF '{start}'", timeout=ACTING
        )
        (line,) = [line for line in self.told().splitlines() if line.startswith(start)]
        return line


@dataclass(frozen=True)
class Running:
    """A VM's build and a compiler's beside it, by the processes they run."""

    vm: str
    compiler: str


def mib(said: str, before: str) -> float:
    """Read the size a line says just after `before`, in MiB, or 0 when it says none."""
    size = re.search(rf"{before} ([\d.]+) (B|KiB|MiB|GiB)", said)
    if size is None:
        return 0
    scale = {"B": 1 / 1024**2, "KiB": 1 / 1024, "MiB": 1, "GiB": 1024}
    return float(size.group(1)) * scale[size.group(2)]


@pytest.fixture(scope="module")
def builds(machine: BaseMachine) -> Builds:
    """Boot the machine until vm-first watches, and start the daemon."""
    machine.wait_for_unit("vm-first.service")
    machine.succeed("systemctl start nix-daemon.service")
    return Builds(machine)


@pytest.fixture(scope="module")
def running(builds: Builds) -> Running:
    """Start a VM's build and a compiler's, and give the processes each runs."""
    builds.start("vm-build")
    builds.start("compiler-build")
    return Running(vm=builds.found("qemu-system-x86"), compiler=builds.found("cc1"))


@pytest.fixture(scope="module")
def quests(builds: Builds, running: Running) -> str:
    """Start a build from a quest's slice weighed at 400, and give its cgroup."""
    # The slice an agent's command for a quest runs in, as jobs-sorter
    # names it, weighed once the build's client brings it into being.
    builds.machine.succeed(
        f"systemd-run --unit=quest-build --slice={QUEST} {NIX_BUILD} /etc/quest-build.nix"
    )
    builds.machine.succeed(f"systemctl set-property --runtime {QUEST} CPUWeight=400")
    builds.machine.wait_until_succeeds(
        f"[ $(ls -d {DAEMON}/nix-build-* | wc -l) -eq 3 ]", timeout=STARTING
    )
    others = {builds.cgroup_of(running.vm), builds.cgroup_of(running.compiler)}
    every = builds.machine.succeed(f"ls -d {DAEMON}/nix-build-*").split()
    (quest,) = [build for build in every if build not in others]
    return quest


def test_the_daemon_sits_beside_its_builds_each_in_a_cgroup_of_its_own(
    builds: Builds, running: Running
) -> None:
    """The daemon sits beside its builds, each in a cgroup of its own."""
    builds.wait_for_the_daemon_aside()
    for pid in [running.vm, running.compiler]:
        assert builds.cgroup_of(pid).startswith(f"{DAEMON}/nix-build-uid-"), builds.cgroup_of(pid)
    assert builds.cgroup_of(running.vm) != builds.cgroup_of(running.compiler)


@pytest.mark.usefixtures("running")
def test_the_builds_are_handed_the_controllers_to_weigh_and_count_them_by(builds: Builds) -> None:
    """The builds are handed the controllers to weigh and count them by."""
    builds.wait_for_the_controllers()


def test_systemd_oomd_is_asked_to_take_a_build_before_the_daemon(builds: Builds) -> None:
    """systemd-oomd is asked to take a build before the daemon."""
    builds.machine.wait_until_succeeds(
        f'[ "$(getfattr -n user.oomd_avoid --only-values {DAEMON}/nix-daemon)" = 1 ]',
        timeout=ACTING,
    )


def test_every_thread_of_the_vm_goes_ahead_one_started_later_too(
    builds: Builds, running: Running
) -> None:
    """Every thread of the VM goes ahead, one started later too."""
    builds.machine.wait_until_succeeds(
        f"[ $(ls /proc/{running.vm}/task | wc -l) -ge 5 ]", timeout=THREADING
    )
    builds.wait_until_ahead(running.vm)

    told = builds.told()

    assert "Brought qemu-system-x86" in told, told


def test_the_vms_build_is_weighed_above_the_builds_beside_it(
    builds: Builds, running: Running
) -> None:
    """The VM's build is weighed above the builds beside it."""
    vm = builds.cgroup_of(running.vm)
    builds.machine.wait_until_succeeds(f"grep -qx 1000 {vm}/cpu.weight", timeout=ACTING)
    assert builds.machine.succeed(f"cat {vm}/io.weight").strip() == "default 1000"


def test_a_compilers_build_stays_as_the_daemon_set_it(builds: Builds, running: Running) -> None:
    """A compiler's build stays as the daemon set it."""
    assert builds.scheduling(running.compiler) == AS_BUILT, builds.scheduling(running.compiler)
    compiler = builds.cgroup_of(running.compiler)
    assert builds.machine.succeed(f"cat {compiler}/cpu.weight").strip() == "100"


def test_a_quests_build_is_weighed_as_its_quest_and_follows_it(
    builds: Builds, running: Running, quests: str
) -> None:
    """A quest's build is weighed as its quest, and follows it."""
    builds.machine.wait_until_succeeds(f"grep -qx 400 {quests}/cpu.weight", timeout=ACTING)
    assert builds.machine.succeed(f"cat {quests}/io.weight").strip() == "default 400"

    builds.machine.succeed(f"systemctl set-property --runtime {QUEST} CPUWeight=50")

    builds.machine.wait_until_succeeds(f"grep -qx 50 {quests}/cpu.weight", timeout=ACTING)
    compiler = builds.cgroup_of(running.compiler)
    assert builds.machine.succeed(f"cat {compiler}/cpu.weight").strip() == "100"


def test_a_restarted_daemon_leaves_its_builds_running_and_builds_again(
    builds: Builds, running: Running
) -> None:
    """A restarted daemon leaves its builds running, and builds again."""
    # Between the old daemon going and the new one moving aside, the
    # service's cgroup must take no controllers, or systemd can't start
    # the new daemon in it. One restart can miss that window; ten, with
    # vm-first looking every 20 ms, don't.
    for _ in range(RESTARTS):
        builds.restart_the_daemon()

    builds.machine.succeed(f"kill -0 {running.vm} {running.compiler}")
    builds.wait_for_the_controllers()

    # A build asking for a range of user ids, as a container's check does.
    built = builds.machine.succeed(f"{NIX_BUILD} /etc/uid-range.nix").strip()

    given = builds.machine.succeed(f"cat {built}").strip()

    assert given == RANGE, f"the build was given {given} ids"


def test_a_build_cant_hand_controllers_on_itself_so_the_daemon_restarts_beside_it(
    builds: Builds,
) -> None:
    """A build can't hand controllers on itself, so the daemon restarts beside it."""
    machine = builds.machine
    # systemd takes the daemon's hand-off back before each start, which
    # would fail while a build's own cgroups held controllers, as a
    # container's would if Nix let its build hand them on.
    builds.start("nester-build")
    machine.wait_until_succeeds(f"ls -d {DAEMON}/nix-build-*/asked", timeout=STARTING)
    nester = machine.succeed(f"dirname {DAEMON}/nix-build-*/asked").strip()
    # The refusal reaches the journal through the daemon and its client,
    # so it can land just after the cgroup it was asked for.
    machine.wait_until_succeeds(
        "journalctl -u nester-build.service -o cat"
        " | grep -q 'cgroup.subtree_control: Permission denied'",
        timeout=REFUSING,
    )
    assert machine.succeed(f"cat {nester}/cgroup.subtree_control").strip() == ""

    builds.restart_the_daemon()

    machine.succeed(f"grep -q . {nester}/inner/cgroup.procs")
    machine.succeed("systemctl stop nester-build")


def test_a_containers_build_goes_ahead_as_a_vms_does_every_process_in_it(
    builds: Builds, running: Running
) -> None:
    """A container's build goes ahead as a VM's does, every process in it."""
    builds.start("container-build")
    container = builds.cgroup_of(builds.found("systemd-nspawn"))
    service = builds.found("service")

    builds.machine.wait_until_succeeds(f"grep -qx 1000 {container}/cpu.weight", timeout=ACTING)
    builds.wait_until_ahead(service)
    told = builds.told()
    assert f"Weighed {container}/cpu.weight at 1000 for its test." in told, told
    assert builds.scheduling(running.compiler) == AS_BUILT, builds.scheduling(running.compiler)

    builds.machine.succeed("systemctl stop container-build")


def test_a_finished_build_says_what_it_wrote_and_who_asked_for_it(builds: Builds) -> None:
    """A finished build says what it wrote, and who asked for it."""
    line = builds.told_of("writer-build.nix")
    assert mib(line, "wrote") >= WRITTEN_MIB, line


def test_a_build_that_ends_before_the_disk_has_its_writes_says_what_it_left(
    builds: Builds,
) -> None:
    """A build that ends before the disk has its writes says what it left."""
    line = builds.told_of("lazy-writer-build.nix")
    assert mib(line, "wrote") + mib(line, "leaving up to") >= WRITTEN_MIB, line


def test_a_builds_syncfs_is_refused_as_not_implemented_and_sync_still_runs(
    builds: Builds,
) -> None:
    """A build's syncfs is refused as not implemented, and sync still runs."""
    built = builds.machine.succeed(f"{NIX_BUILD} /etc/syncer-build.nix").strip()

    answered = builds.machine.succeed(f"cat {built}")

    assert "Function not implemented" in answered, answered
    assert "sync ran" in answered, answered
    # Only the daemon and its builds are refused it.
    builds.machine.succeed("sync -f /")


@pytest.mark.usefixtures("running", "quests")
def test_a_stopped_build_leaves_nothing_running_nor_its_cgroup(builds: Builds) -> None:
    """A stopped build leaves nothing running, nor its cgroup."""
    builds.machine.succeed("systemctl stop vm-build compiler-build quest-build")

    builds.machine.wait_until_fails("pgrep -x qemu-system-x86 || pgrep -x cc1", timeout=STARTING)
    builds.machine.wait_until_fails(f"ls -d {DAEMON}/nix-build-*", timeout=STARTING)
    told = builds.told()
    assert "/etc/quest-build.nix`, for quest qestabc, wrote" in told, told

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
"""

import re
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from driver_names import machine, subtest

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


def scheduling(pid: str) -> str:
    """Say each nice and class the process's threads run at, one a line."""
    return machine.succeed(
        f"ps -L -o ni=,cls= -p {pid} | awk '{{print $1, $2}}' | sort -u",
    ).strip()


# Through the daemon, as every agent's build goes: root alone may build in
# its own process, which puts the build in the caller's cgroup instead.
NIX_BUILD = "nix-build --store daemon --no-out-link"


def build(name: str) -> None:
    """Start a build of one of the machine's derivations, left running."""
    machine.succeed(f"systemd-run --unit={name} {NIX_BUILD} /etc/{name}.nix")


def found(comm: str) -> str:
    """Wait for the process the kernel names so, and give its id."""
    machine.wait_until_succeeds(f"pgrep -x {comm}", timeout=60)
    return machine.succeed(f"pgrep -x {comm}").strip()


def cgroup_of(pid: str) -> str:
    """Give the process's cgroup, as a path under the cgroup file system."""
    return "/sys/fs/cgroup" + machine.succeed(f"cut -d: -f3 /proc/{pid}/cgroup").strip()


def wait_for_the_daemon_aside() -> None:
    """Wait for the daemon to move itself into a cgroup beside its builds.

    systemd starts it in the service's own cgroup, and it moves a moment
    later, so a single look can land before the move.
    """
    machine.wait_until_succeeds(
        "grep -qx '0::/build.slice/nix-daemon.service/nix-daemon' "
        "/proc/$(systemctl show -P MainPID nix-daemon.service)/cgroup",
        timeout=10,
    )


def wait_for_the_controllers() -> None:
    """Wait for vm-first to hand the builds every controller."""
    machine.wait_until_succeeds(
        f"[ \"$(cat {DAEMON}/cgroup.subtree_control)\" = 'cpu io memory pids' ]", timeout=10
    )


machine.wait_for_unit("vm-first.service")
machine.succeed("systemctl start nix-daemon.service")
build("vm-build")
build("compiler-build")
vm = found("qemu-system-x86")
compiler = found("cc1")

with subtest("the daemon sits beside its builds, each in a cgroup of its own"):
    wait_for_the_daemon_aside()
    for pid in [vm, compiler]:
        assert cgroup_of(pid).startswith(f"{DAEMON}/nix-build-uid-"), cgroup_of(pid)
    assert cgroup_of(vm) != cgroup_of(compiler)

with subtest("the builds are handed the controllers to weigh and count them by"):
    wait_for_the_controllers()

with subtest("systemd-oomd is asked to take a build before the daemon"):
    machine.wait_until_succeeds(
        f'[ "$(getfattr -n user.oomd_avoid --only-values {DAEMON}/nix-daemon)" = 1 ]',
        timeout=10,
    )

with subtest("every thread of the VM goes ahead, one started later too"):
    machine.wait_until_succeeds(f"[ $(ls /proc/{vm}/task | wc -l) -ge 5 ]", timeout=20)
    machine.wait_until_succeeds(
        f"[ \"$(ps -L -o ni=,cls= -p {vm} | awk '{{print $1, $2}}' | sort -u)\" = '{AHEAD}' ]",
        timeout=10,
    )

    told = machine.succeed("journalctl -u vm-first.service -o cat")

    assert "Brought qemu-system-x86" in told, told

with subtest("the VM's build is weighed above the builds beside it"):
    machine.wait_until_succeeds(f"grep -qx 1000 {cgroup_of(vm)}/cpu.weight", timeout=10)
    assert machine.succeed(f"cat {cgroup_of(vm)}/io.weight").strip() == "default 1000"

with subtest("a compiler's build stays as the daemon set it"):
    assert scheduling(compiler) == AS_BUILT, scheduling(compiler)
    assert machine.succeed(f"cat {cgroup_of(compiler)}/cpu.weight").strip() == "100"

with subtest("a quest's build is weighed as its quest, and follows it"):
    # The slice an agent's command for a quest runs in, as jobs-sorter
    # names it, weighed once the build's client brings it into being.
    machine.succeed(
        f"systemd-run --unit=quest-build --slice={QUEST} {NIX_BUILD} /etc/quest-build.nix"
    )
    machine.succeed(f"systemctl set-property --runtime {QUEST} CPUWeight=400")
    machine.wait_until_succeeds(f"[ $(ls -d {DAEMON}/nix-build-* | wc -l) -eq 3 ]", timeout=60)
    others = {cgroup_of(vm), cgroup_of(compiler)}
    builds = machine.succeed(f"ls -d {DAEMON}/nix-build-*").split()
    (quests,) = [build for build in builds if build not in others]

    machine.wait_until_succeeds(f"grep -qx 400 {quests}/cpu.weight", timeout=10)
    assert machine.succeed(f"cat {quests}/io.weight").strip() == "default 400"

    machine.succeed(f"systemctl set-property --runtime {QUEST} CPUWeight=50")

    machine.wait_until_succeeds(f"grep -qx 50 {quests}/cpu.weight", timeout=10)
    assert machine.succeed(f"cat {cgroup_of(compiler)}/cpu.weight").strip() == "100"

with subtest("a restarted daemon leaves its builds running, and builds again"):
    # Between the old daemon going and the new one moving aside, the
    # service's cgroup must take no controllers, or systemd can't start
    # the new daemon in it. One restart can miss that window; ten, with
    # vm-first looking every 20 ms, don't. Each restart forgets the ones
    # before, which would otherwise pass systemd's limit on starts.
    for _ in range(10):
        machine.succeed("systemctl reset-failed nix-daemon.service")
        machine.succeed("systemctl restart nix-daemon.service")
        machine.wait_for_unit("nix-daemon.service")
        wait_for_the_daemon_aside()

    machine.succeed(f"kill -0 {vm} {compiler}")
    wait_for_the_controllers()

    # A build asking for a range of user ids, as a container's check does.
    built = machine.succeed(f"{NIX_BUILD} /etc/uid-range.nix").strip()

    given = machine.succeed(f"cat {built}").strip()

    assert given == RANGE, f"the build was given {given} ids"

with subtest("a build can't hand controllers on itself, so the daemon restarts beside it"):
    # systemd takes the daemon's hand-off back before each start, which
    # would fail while a build's own cgroups held controllers, as a
    # container's would if Nix let its build hand them on.
    build("nester-build")
    machine.wait_until_succeeds(f"ls -d {DAEMON}/nix-build-*/asked", timeout=60)
    nester = machine.succeed(f"dirname {DAEMON}/nix-build-*/asked").strip()
    # The refusal reaches the journal through the daemon and its client,
    # so it can land just after the cgroup it was asked for.
    machine.wait_until_succeeds(
        "journalctl -u nester-build.service -o cat"
        " | grep -q 'cgroup.subtree_control: Permission denied'",
        timeout=30,
    )
    assert machine.succeed(f"cat {nester}/cgroup.subtree_control").strip() == ""

    machine.succeed("systemctl reset-failed nix-daemon.service")
    machine.succeed("systemctl restart nix-daemon.service")
    machine.wait_for_unit("nix-daemon.service")
    wait_for_the_daemon_aside()

    machine.succeed(f"grep -q . {nester}/inner/cgroup.procs")
    machine.succeed("systemctl stop nester-build")


def told_of(etc: str) -> str:
    """Run the build in /etc/<etc> to its end, and read what vm-first said of it."""
    machine.succeed(f"{NIX_BUILD} /etc/{etc}")
    start = f"A build for `{NIX_BUILD} /etc/{etc}`, for no quest"
    machine.wait_until_succeeds(
        f"journalctl -u vm-first.service -o cat | grep -qF '{start}'", timeout=10
    )
    told = machine.succeed("journalctl -u vm-first.service -o cat")
    (line,) = [line for line in told.splitlines() if line.startswith(start)]
    return line


def mib(said: str, before: str) -> float:
    """Read the size a line says just after `before`, in MiB, or 0 when it says none."""
    found = re.search(rf"{before} ([\d.]+) (B|KiB|MiB|GiB)", said)
    if found is None:
        return 0
    scale = {"B": 1 / 1024**2, "KiB": 1 / 1024, "MiB": 1, "GiB": 1024}
    return float(found.group(1)) * scale[found.group(2)]


with subtest("a container's build goes ahead as a VM's does, every process in it"):
    build("container-build")
    container = found("systemd-nspawn")
    service = found("service")

    machine.wait_until_succeeds(f"grep -qx 1000 {cgroup_of(container)}/cpu.weight", timeout=10)
    machine.wait_until_succeeds(
        f"[ \"$(ps -L -o ni=,cls= -p {service} | awk '{{print $1, $2}}' | sort -u)\" = '{AHEAD}' ]",
        timeout=10,
    )
    told = machine.succeed("journalctl -u vm-first.service -o cat")
    assert f"Weighed {cgroup_of(container)}/cpu.weight at 1000 for its test." in told, told
    assert scheduling(compiler) == AS_BUILT, scheduling(compiler)

    machine.succeed("systemctl stop container-build")

with subtest("a finished build says what it wrote, and who asked for it"):
    line = told_of("writer-build.nix")
    assert mib(line, "wrote") >= WRITTEN_MIB, line

with subtest("a build that ends before the disk has its writes says what it left"):
    line = told_of("lazy-writer-build.nix")
    assert mib(line, "wrote") + mib(line, "leaving up to") >= WRITTEN_MIB, line

with subtest("a build's syncfs is refused as not implemented, and sync still runs"):
    built = machine.succeed(f"{NIX_BUILD} /etc/syncer-build.nix").strip()

    answered = machine.succeed(f"cat {built}")

    assert "Function not implemented" in answered, answered
    assert "sync ran" in answered, answered
    # Only the daemon and its builds are refused it.
    machine.succeed("sync -f /")

with subtest("a stopped build leaves nothing running, nor its cgroup"):
    machine.succeed("systemctl stop vm-build compiler-build quest-build")

    machine.wait_until_fails("pgrep -x qemu-system-x86 || pgrep -x cc1", timeout=60)
    machine.wait_until_fails(f"ls -d {DAEMON}/nix-build-*", timeout=60)
    told = machine.succeed("journalctl -u vm-first.service -o cat")
    assert "/etc/quest-build.nix`, for quest qestabc, wrote" in told, told

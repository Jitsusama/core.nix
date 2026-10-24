"""What the kernel gives each slice, the account's and the system's, in a virtual machine.

tests/slices.nix runs this, and its opening comment says why the kernel is asked rather than the
module's settings read back. run-subtests cuts the script into subtests at each top-level
`with subtest(`, so every subtest stays at the top level under its own name, and what sits
outside them, the boot, the units holding each slice's cgroup open and the helpers, runs
whichever subtests are chosen. The programs it runs in those units come in one JSON file, whose
path slices.nix writes in place of the marker below.
"""

import json
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from driver_names import machine, ordered, start_all, subtest, t

# coreutils' sleep and bash, by their paths in the store.
values = json.loads(Path("@values@").read_text())
sleep: str = values["sleep"]
bash: str = values["bash"]
chrt: str = values["chrt"]

account = "/sys/fs/cgroup/user.slice/user-1000.slice/user@1000.service"
# Both machines boot at once, rather than the measured one only once the first is done with, so
# a slow host waits for one boot instead of two in a row.
start_all()
machine.wait_for_unit("user@1000.service")

# A slice's cgroup exists only while something runs in it.
for name in ["app", "session", "background"]:
    machine.succeed(
        f"systemd-run -M alice@ --user --slice={name}.slice --unit=in-{name} {sleep} infinity"
    )
    machine.wait_until_succeeds(
        f"systemctl -M alice@ --user is-active in-{name}.service", timeout=30
    )

# The drive behind /home: the partition's parent, or the disk itself.
source = machine.succeed("findmnt -nvo SOURCE -T /home").strip()
parent = machine.succeed(f"lsblk -dno PKNAME {source}").strip()
drive = parent or source.removeprefix("/dev/")
number = machine.succeed(f"cat /sys/class/block/{drive}/dev").strip()

machine.succeed(f"systemd-run --slice=machine.slice --unit=in-machine {sleep} infinity")
machine.wait_for_unit("in-machine.service")


def latency(name: str, under: str = account) -> str:
    """Give the latency target the kernel holds for the slice, under its holder's cgroup."""
    return machine.succeed(f"cat {under}/{name}.slice/io.latency").strip()


with subtest("what Joel uses has a millisecond on the drive behind /home"):
    t.assertEqual(latency("app"), f"{number} target=1000")
    t.assertEqual(latency("session"), f"{number} target=1000")

with subtest("the work it starts has none, so it gives way"):
    t.assertEqual(latency("jobs"), "")
    t.assertEqual(latency("background"), "")

with subtest("among the system's, every account has it, and machines none"):
    t.assertEqual(latency("user", "/sys/fs/cgroup"), f"{number} target=1000")
    t.assertEqual(latency("machine", "/sys/fs/cgroup"), "")
    t.assertEqual(latency("system", "/sys/fs/cgroup"), "")

# systemd turns a share into bytes of the memory the kernel reports.
total = int(machine.succeed("awk '/MemTotal/ {print $2}' /proc/meminfo")) * 1024


def floor(path: str) -> int:
    """Give the memory, in bytes, the kernel keeps for the cgroup whatever else needs it."""
    return int(machine.succeed(f"cat {path}/memory.min"))


def near(path: str, share: float) -> None:
    """Check the cgroup keeps that share of the memory, to within a mebibyte."""
    t.assertAlmostEqual(floor(path), total * share, delta=2**20, msg=path)


with subtest("apps keep a hard share of memory, and every slice above more"):
    near(f"{account}/app.slice", 0.13)
    t.assertEqual(floor(f"{account}/session.slice"), 1536 * 2**20)
    near(account, 0.16)
    near("/sys/fs/cgroup/user.slice/user-1000.slice", 0.16)
    near("/sys/fs/cgroup/user.slice", 0.16)
    t.assertEqual(floor(f"{account}/jobs.slice"), 0)


with subtest("work in jobs is slowed past half of memory, and never ended"):
    high = int(machine.succeed(f"cat {account}/jobs.slice/memory.high"))
    t.assertAlmostEqual(high, total * 0.5, delta=2**20)
    t.assertEqual(machine.succeed(f"cat {account}/jobs.slice/memory.max").strip(), "max")

jobs = f"{account}/jobs.slice"
timed = f"{jobs}/jobs-timed.slice"

with subtest("timed work is inside idle jobs, far above its holder"):
    machine.wait_until_succeeds("systemctl -M alice@ --user is-active timed.service", timeout=30)
    t.assertEqual(machine.succeed(f"cat {jobs}/cpu.idle").strip(), "1")
    t.assertEqual(machine.succeed(f"cat {timed}/cpu.weight").strip(), "10000")
    # awaited.service holds jobs-awaited.slice open, so the slice's cgroup
    # is there to read only once the service runs.
    machine.wait_until_succeeds("systemctl -M alice@ --user is-active awaited.service", timeout=30)
    t.assertEqual(machine.succeed(f"cat {jobs}/jobs-awaited.slice/cpu.weight").strip(), "1000")
    t.assertEqual(machine.succeed(f"cat {jobs}/jobs.service/cpu.weight").strip(), "100")

with subtest("work in jobs moves itself in, as pi's waiting shell does"):
    machine.succeed(
        "systemd-run -M alice@ --user --slice=jobs.slice --unit=mover"
        f" --expand-environment=no -p KillMode=process {bash} -c"
        f" 'echo $$ > {timed}/timed.service/cgroup.procs;"
        " cat /proc/self/cgroup > /tmp/moved; exec sleep infinity'"
    )
    machine.wait_until_succeeds("grep -q timed.service /tmp/moved", timeout=30)


def cgroup_of(seconds: int) -> str:
    """Give the cgroup of the sleep that waits the given seconds."""
    return machine.succeed(f"cat /proc/$(pgrep -xf '{sleep} {seconds}')/cgroup").strip()


with subtest("a scope that work in jobs starts goes back, unless it sets limits"):
    # An agent's shell in jobs starts two scopes with no slice, which
    # systemd puts in app.slice: one with a memory ceiling, then, once
    # that one runs, one without, so the sorter has looked at both by the
    # time it moves the second.
    machine.succeed(
        "systemd-run -M alice@ --user --slice=jobs.slice --unit=agent"
        f" --expand-environment=no -p KillMode=process {bash} -c"
        f" 'echo $$ > {jobs}/jobs.service/cgroup.procs;"
        f" systemd-run --user --scope --unit=kept -p MemoryMax=1G {sleep} 1002 &"
        " until systemctl --user is-active --quiet kept.scope; do sleep 0.1; done;"
        f" systemd-run --user --scope --unit=stray {sleep} 1001 & wait'"
    )
    # The shell itself was given a scope of its own in jobs by then, and
    # the stray goes back to it.
    machine.wait_until_succeeds(
        f"grep -q 'jobs-work.slice/work-.*\\.scope$' /proc/$(pgrep -xf '{sleep} 1001')/cgroup",
        timeout=30,
    )
    t.assertIn("app.slice/kept.scope", cgroup_of(1002))

with subtest("each piece of work written into jobs gets a scope of its own"):
    # Two commands written into jobs.service as pi writes its shells, one
    # starting a child once its own scope is there.
    for seconds in [1003, 1004]:
        machine.succeed(
            f"systemd-run -M alice@ --user --slice=jobs.slice --unit=piece-{seconds}"
            f" --expand-environment=no -p KillMode=process {bash} -c"
            f" 'echo $$ > {jobs}/jobs.service/cgroup.procs;"
            " until grep -q /work- /proc/$$/cgroup; do sleep 0.1; done;"
            f" {sleep} {seconds} & wait'"
        )
    for seconds in [1003, 1004]:
        machine.wait_until_succeeds(
            f"grep -q 'jobs.slice/jobs-work.slice/work-.*\\.scope$'"
            f" /proc/$(pgrep -xf '{sleep} {seconds}')/cgroup",
            timeout=30,
        )
    first, second = cgroup_of(1003), cgroup_of(1004)
    t.assertNotEqual(first, second)
    # Together they weigh as jobs.service did, below a terminal's work.
    t.assertEqual(machine.succeed(f"cat {jobs}/jobs-work.slice/cpu.weight").strip(), "100")
    # oomd reads each one's swap, as it does any leaf's.
    scope = first.removeprefix("0::").removeprefix("/user.slice/user-1000.slice/user@1000.service")
    machine.succeed(f"test -e {account}{scope}/memory.swap.current")
    # The holder stays, so the next write still lands.
    holder = machine.succeed("systemctl -M alice@ --user show -P MainPID jobs.service").strip()
    t.assertIn(holder, machine.succeed(f"cat {jobs}/jobs.service/cgroup.procs").split())


def real_time(seconds: int, cgroup: str) -> None:
    """Write a shell into the cgroup as pi or a terminal does, to sleep in SCHED_RR.

    It takes real time only once it has a scope of its own, then sleeps the given seconds.
    """
    machine.succeed(
        f"systemd-run -M alice@ --user --slice=jobs.slice --unit=rt-{seconds}"
        f" --expand-environment=no -p KillMode=process {bash} -c"
        f" 'echo $$ > {cgroup}/cgroup.procs;"
        " until grep -q /work- /proc/$$/cgroup; do sleep 0.1; done;"
        f" exec {chrt} --rr 1 {sleep} {seconds}'"
    )


def policy(seconds: int) -> str:
    """Give the scheduling policy of the sleep that waits the given seconds."""
    return machine.succeed(f"{chrt} -p $(pgrep -xf '{sleep} {seconds}')")


def rt_limit(seconds: int) -> list[str]:
    """Give the real-time priority limit of the sleep that waits the given seconds."""
    return machine.succeed(
        f"grep 'Max realtime priority' /proc/$(pgrep -xf '{sleep} {seconds}')/limits"
    ).split()[3:5]


with subtest("work from agents loses real time, and Joel's own work keeps it"):
    # Joel's first: once an agent's has lost it, the sorter has looked at
    # Joel's in the same pass, while it ran in real time.
    real_time(1006, f"{jobs}/jobs-awaited.slice/awaited.service")
    machine.wait_until_succeeds(
        f"{chrt} -p $(pgrep -xf '{sleep} 1006') | grep -q SCHED_RR", timeout=30
    )
    real_time(1005, f"{jobs}/jobs.service")
    machine.wait_until_succeeds(
        f"{chrt} -p $(pgrep -xf '{sleep} 1005') | grep -q SCHED_OTHER", timeout=30
    )
    # Nor can it, or what it starts, take real time back.
    t.assertEqual(rt_limit(1005), ["0", "0"])
    t.assertIn("SCHED_RR", policy(1006))
    t.assertEqual(rt_limit(1006), ["10", "10"])


def weight(path: str) -> str:
    """Give the weight the cgroup has on every disk shared by weight."""
    return machine.succeed(f"cat {path}/io.weight").strip()


with subtest("on the disk, slices weigh as they do on the CPU"):
    t.assertEqual(weight(f"{account}/session.slice"), "default 1000")
    t.assertEqual(weight(f"{account}/app.slice"), "default 100")
    t.assertEqual(weight(jobs), "default 1")
    t.assertEqual(weight(timed), "default 10000")
    t.assertEqual(weight(f"{jobs}/jobs-awaited.slice"), "default 1000")
    t.assertEqual(weight(f"{account}/background.slice"), "default 1")
    t.assertEqual(weight("/sys/fs/cgroup/user.slice"), "default 1000")
    t.assertEqual(weight("/sys/fs/cgroup/system.slice"), "default 100")

# The measured machine: its disk, and units holding app.slice and
# session.slice open.
ordered.wait_for_unit("user@1000.service")
for name in ["app", "session"]:
    ordered.succeed(
        f"systemd-run -M alice@ --user --slice={name}.slice --unit=in-{name} {sleep} infinity"
    )
    ordered.wait_until_succeeds(
        f"systemctl -M alice@ --user is-active in-{name}.service", timeout=30
    )
vda = ordered.succeed("cat /sys/class/block/vda/dev").strip()


def cost(name: str) -> list[str]:
    """Give the settings io.cost holds for the measured disk, from its model or qos file."""
    lines = ordered.succeed(f"cat /sys/fs/cgroup/io.cost.{name}").splitlines()
    return next(line.split()[1:] for line in lines if line.startswith(f"{vda} "))


with subtest("a measured disk is shared by weight, by the model it was given"):
    ordered.wait_for_unit("disk-order-vda.service")
    model = cost("model")
    for setting in ["ctrl=user", "rbps=1000000000", "rrandiops=50000", "wseqiops=80000"]:
        t.assertIn(setting, model)
    qos = cost("qos")
    for setting in ["enable=1", "ctrl=user", "rlat=5000", "wlat=5000"]:
        t.assertIn(setting, qos)

with subtest("with it, no slice holds a latency target"):
    for path in [f"{account}/app.slice", f"{account}/session.slice", "/sys/fs/cgroup/user.slice"]:
        t.assertEqual(ordered.succeed(f"cat {path}/io.latency").strip(), "", msg=path)

with subtest("latency targets left from before the disk was measured go as it turns on"):
    # A switch from a machine that had targets keeps them: systemd doesn't write over a
    # target it no longer manages, so they stay in the kernel until a reboot.
    held = [f"{account}/app.slice", f"{account}/session.slice", "/sys/fs/cgroup/user.slice"]
    for path in held:
        ordered.succeed(f"echo '{vda} target=1000' > {path}/io.latency")
    ordered.succeed("systemctl restart disk-order-vda.service")
    for path in held:
        t.assertEqual(ordered.succeed(f"cat {path}/io.latency").strip(), "", msg=path)

with subtest("stopping it hands the disk back to first come, first served"):
    ordered.succeed("systemctl stop disk-order-vda.service")
    t.assertIn("enable=0", cost("qos"))

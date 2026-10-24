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
    from driver_names import machine, subtest, t

# coreutils' sleep and bash, by their paths in the store.
values = json.loads(Path("@values@").read_text())
sleep: str = values["sleep"]
bash: str = values["bash"]

account = "/sys/fs/cgroup/user.slice/user-1000.slice/user@1000.service"
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
    machine.wait_until_succeeds(
        f"grep -q jobs.slice/jobs.service /proc/$(pgrep -xf '{sleep} 1001')/cgroup", timeout=30
    )
    t.assertIn("app.slice/kept.scope", cgroup_of(1002))

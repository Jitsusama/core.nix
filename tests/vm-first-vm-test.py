"""A check's VM goes ahead of the builds beside it; a container's build gets ids.

A process named as qemu is, started as a build user in the Nix daemon's
cgroup as batch at nice 5, has every thread put to the normal policy at
nice -10, a thread it starts afterwards included, while a compiler beside
it stays as it was. A build asking for a range of user ids gets 65,536.
"""

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from driver_names import machine, subtest

# What a thread of the VM is put to, and what a build is left at: nice and
# scheduling class, as ps writes them.
AHEAD = "-10 TS"
AS_BUILT = "5 B"
# The ids a build that asks for a range of them is given.
RANGE = "65536"


def scheduling(pid: str) -> str:
    """Say each nice and class the process's threads run at, one a line."""
    return machine.succeed(
        f"ps -L -o ni=,cls= -p {pid} | awk '{{print $1, $2}}' | sort -u",
    ).strip()


machine.wait_for_unit("vm-first.service")
machine.succeed("systemctl start nix-daemon.service")
vm = machine.succeed("as-build fake-vm").strip()
compiler = machine.succeed("as-build fake-compiler").strip()

with subtest("every thread of the VM goes ahead, one started later too"):
    machine.wait_until_succeeds(f"[ $(ls /proc/{vm}/task | wc -l) -ge 5 ]", timeout=20)
    machine.wait_until_succeeds(
        f"[ \"$(ps -L -o ni=,cls= -p {vm} | awk '{{print $1, $2}}' | sort -u)\" = '{AHEAD}' ]",
        timeout=10,
    )

    told = machine.succeed("journalctl -u vm-first.service -o cat")

    assert "Brought qemu-system-x86" in told, told

with subtest("a compiler beside it stays as the daemon set it"):
    assert scheduling(compiler) == AS_BUILT, scheduling(compiler)

with subtest("a build asking for a range of user ids gets one"):
    built = machine.succeed("nix-build --no-out-link /etc/uid-range.nix").strip()

    given = machine.succeed(f"cat {built}").strip()

    assert given == RANGE, f"the build was given {given} ids"

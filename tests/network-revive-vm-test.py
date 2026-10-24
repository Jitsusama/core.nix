"""network-revive followed in a virtual machine, on an account whose services need the network.

tests/network-revive.nix runs this, and its opening comment says what the test proves.
run-subtests cuts the script into subtests at each top-level `with subtest(`, so every subtest
stays at the top level under its own name, and what sits outside them, the boot and the
helpers, runs whichever subtests are chosen. The script needs no value from Nix: the account,
its stand-in services and the uplink are all named in the machine network-revive.nix builds.
"""

import re
import shlex
from typing import TYPE_CHECKING

from vm_helpers import as_account

if TYPE_CHECKING:
    from driver_names import machine, subtest, t

# What network-revive has said in joel's journal, read as joel from a root shell.
JOURNAL = "su - joel -c 'XDG_RUNTIME_DIR=/run/user/1000 journalctl --user -u network-revive -o cat'"


def as_joel(command: str) -> str:
    """Run the command as joel, in his home, with his systemd at hand, and give its output."""
    return as_account(machine, command)


def state(unit: str) -> list[str]:
    """Give the unit's active state, its sub-state and its result."""
    return as_joel(f"systemctl --user show -P ActiveState,SubState,Result {unit}").split()


def wait_for_state(unit: str, *expected: str) -> None:
    """Wait until the unit's active state and sub-state are those expected."""
    machine.wait_until_succeeds(
        "su - joel -c "
        + shlex.quote(
            "export XDG_RUNTIME_DIR=/run/user/1000; "
            f"test \"$(systemctl --user show -P ActiveState,SubState {unit} | paste -sd ' ')\""
            f" = {shlex.quote(' '.join(expected))}"
        ),
        timeout=120,
    )


def said() -> str:
    """Give everything network-revive has said in the journal."""
    return as_joel("journalctl --user -u network-revive -o cat")


def times_said(pattern: str) -> int:
    """Count the lines network-revive has said that match the pattern whole."""
    return len([line for line in said().splitlines() if re.fullmatch(pattern, line)])


def wait_for_more(pattern: str, than: int) -> None:
    """Wait for the daemon to say a line matching the pattern more than it had."""
    machine.wait_until_succeeds(
        f"test $({JOURNAL} | grep -Ecx {shlex.quote(pattern)}) -gt {than}", timeout=60
    )


def reloads() -> list[str]:
    """Give the servers resolved asked at each reload, one reload a line."""
    return as_joel("cat ~/reloads 2>/dev/null || true").splitlines()


def wait_for_reloads(count: int) -> list[str]:
    """Wait for at least that many reloads, and give what each saw."""
    machine.wait_until_succeeds(f"test $(wc -l < /home/joel/reloads) -ge {count}", timeout=60)
    return reloads()


machine.wait_for_unit("multi-user.target")
machine.wait_for_unit("user@1000.service")
machine.wait_for_unit("network-revive.service", "joel")

with subtest("without the network, each service is let down as the network leaves it"):
    as_joel("systemctl --user start --no-block needs-network waits-long never-ready backs-up")
    wait_for_state("needs-network", "failed", "failed")
    t.assertEqual(state("needs-network")[2], "start-limit-hit")
    wait_for_state("waits-long", "activating", "auto-restart")
    wait_for_state("never-ready", "failed", "failed")
    wait_for_state("backs-up", "failed", "failed")

with subtest("once the network comes, what it let down starts at once"):
    machine.succeed("networkctl up uplink")
    wait_for_state("needs-network", "active", "running")
    wait_for_state("waits-long", "active", "running")
    wait_for_state("backs-up", "inactive", "dead")
    t.assertEqual(state("backs-up")[2], "success")
    lines = said()
    t.assertEqual(
        lines.count(
            "started needs-network.service again, which had failed, as the network changed"
        ),
        1,
        lines,
    )
    t.assertIn(
        "started waits-long.service now, not at its next try, as the network changed",
        lines,
    )

with subtest("a service whose check never passes stays down, and the journal says why"):
    machine.wait_until_succeeds(
        f"{JOURNAL}"
        " | grep -qx 'left never-ready.service down: its check still failed 31 s"
        " after the network changed: never ready'",
        timeout=120,
    )
    t.assertEqual(state("never-ready")[:2], ["failed", "failed"])

with subtest("a running service reloads once for each change, and a stopped one stays stopped"):
    as_joel("systemctl --user start follows-names stopped")
    as_joel("systemctl --user stop stopped")
    machine.succeed("resolvectl dns uplink 10.0.0.54")
    t.assertEqual(wait_for_reloads(1), ["10.0.0.54"])
    # Ending where it began, a burst changes nothing.
    unchanged = times_said("the machine reaches what it did")
    machine.succeed("resolvectl dns uplink 10.0.0.55 && resolvectl dns uplink 10.0.0.54")
    wait_for_more("the machine reaches what it did", unchanged)
    # Ending elsewhere, however long, it's one change.
    machine.succeed("for server in 56 57 58 59 60; do resolvectl dns uplink 10.0.0.$server; done")
    machine.wait_until_succeeds("grep -q 10.0.0.60 /home/joel/reloads", timeout=60)
    t.assertEqual(reloads(), ["10.0.0.54", "10.0.0.60"])
    t.assertEqual(state("stopped")[:2], ["inactive", "dead"])
    t.assertEqual(state("needs-network")[:2], ["active", "running"])

with subtest("it follows networkd through a restart"):
    looked = times_said("the machine reaches .*")
    machine.succeed("systemctl restart systemd-networkd")
    machine.wait_until_succeeds("networkctl status uplink | grep -q routable", timeout=60)
    wait_for_more("the machine reaches .*", looked)
    machine.succeed("resolvectl dns uplink 10.0.0.61")
    machine.wait_until_succeeds("grep -q 10.0.0.61 /home/joel/reloads", timeout=60)

with subtest("it hears resolved come back asking other servers"):
    # Nothing changes for networkd to hand over, so resolved coming
    # back is all there is to hear.
    machine.succeed(
        "mkdir -p /run/systemd/resolved.conf.d"
        " && printf '[Resolve]\\nDNS=10.0.0.99\\n' > /run/systemd/resolved.conf.d/other.conf"
        " && systemctl restart systemd-resolved"
    )
    wait_for_more("the machine reaches .*10[.]0[.]0[.]99%0.*", 0)

with subtest("starting, it starts what it finds down"):
    as_joel("systemctl --user stop network-revive")
    machine.succeed("networkctl down uplink")
    as_joel("systemctl --user restart --no-block needs-network")
    wait_for_state("needs-network", "failed", "failed")
    machine.succeed("networkctl up uplink")
    machine.wait_until_succeeds("ip -o route get 10.0.0.53 | grep -q ' dev uplink '", timeout=60)
    as_joel("systemctl --user start network-revive")
    wait_for_state("needs-network", "active", "running")
    t.assertIn("started needs-network.service again, which had failed\n", said() + "\n")

with subtest("a start asked as systemd executes itself again is asked again"):
    # Heard going, so its coming back is a change of its own rather than
    # the end of a burst that changed nothing.
    looked = times_said("the machine reaches .*")
    machine.succeed("networkctl down uplink")
    wait_for_more("the machine reaches .*", looked)
    as_joel("systemctl --user start --no-block meets-reexec")
    wait_for_state("meets-reexec", "failed", "failed")
    machine.succeed("networkctl up uplink")
    wait_for_state("meets-reexec", "active", "running")
    machine.succeed("test -e /home/joel/reexecuted")
    # The start was asked while systemd was held away, and it let go.
    machine.succeed("test -e /home/joel/away")
    machine.fail("test -e /home/joel/hold")
    machine.succeed("journalctl _UID=1000 _COMM=systemd -o cat | grep -qx Reexecuting.")
    t.assertIn(
        "started meets-reexec.service again, which had failed, as the network changed", said()
    )

with subtest("systemd refused nothing it was asked for"):
    t.assertEqual(times_said("couldn't .*"), 0, said())

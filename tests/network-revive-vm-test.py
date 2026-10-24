"""network-revive followed in a virtual machine, on an account whose services need the network.

tests/network-revive.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py),
and its opening comment says what the test proves. Each test asks for the state it starts
from as a fixture: the machine booted, the services let down with no network, the network
back, a service following resolved's servers. A fixture's work is done once a run, however
many tests ask for it, so any test runs alone with `-k`, and all of them run in the order
they're written, each after what it needs.
"""

from __future__ import annotations

import datetime as dt
import re
import shlex
from typing import TYPE_CHECKING

import pytest
from vm_helpers import FIRST_RUNTIME, as_account

if TYPE_CHECKING:
    from test_driver.machine import BaseMachine

# How long systemd may take to bring a unit to a state: long enough for one service's five
# tries a second apart and a start limit, with room for a busy host.
SETTLING = dt.timedelta(minutes=2)
# How long network-revive may take to say something, or a reload to land.
SAYING = dt.timedelta(minutes=1)
# What network-revive has said in joel's journal, read as joel from a root shell.
JOURNAL = "su - joel -c " + shlex.quote(
    f"XDG_RUNTIME_DIR={FIRST_RUNTIME} journalctl --user -u network-revive -o cat"
)


class Joel:
    """The account whose services need the network, and what network-revive tells him."""

    def __init__(self, machine: BaseMachine) -> None:
        """Act on joel's account on that machine."""
        self.machine = machine

    def run(self, command: str) -> str:
        """Run the command as joel, in his home, with his systemd at hand, and give its output."""
        return as_account(self.machine, command)

    def state(self, unit: str) -> list[str]:
        """Give the unit's active state, its sub-state and its result."""
        return self.run(f"systemctl --user show -P ActiveState,SubState,Result {unit}").split()

    def wait_for_state(self, unit: str, active: str, sub: str) -> None:
        """Wait until the unit's active state and sub-state are those."""
        shown = f"systemctl --user show -P ActiveState,SubState {unit} | paste -sd ' '"
        self.machine.wait_until_succeeds(
            "su - joel -c "
            + shlex.quote(
                f"export XDG_RUNTIME_DIR={FIRST_RUNTIME}; "
                f'test "$({shown})" = {shlex.quote(f"{active} {sub}")}'
            ),
            timeout=SETTLING,
        )

    def said(self) -> str:
        """Give everything network-revive has said."""
        return self.run("journalctl --user -u network-revive -o cat")

    def times_said(self, pattern: str) -> int:
        """Count the lines network-revive has said that match the pattern whole."""
        return len([line for line in self.said().splitlines() if re.fullmatch(pattern, line)])

    def wait_until_said(
        self, pattern: str, more_than: int = 0, timeout: dt.timedelta = SAYING
    ) -> None:
        """Wait until network-revive has said a line matching the pattern more than it had."""
        self.machine.wait_until_succeeds(
            f"test $({JOURNAL} | grep -Ecx {shlex.quote(pattern)}) -gt {more_than}",
            timeout=timeout,
        )

    def reloads(self) -> list[str]:
        """Give the servers resolved asked at each reload, one reload a line."""
        return self.run("cat ~/reloads 2>/dev/null || true").splitlines()

    def wait_for_reload_to(self, server: str) -> None:
        """Wait until a reload has seen resolved ask that server, its dots read as dots."""
        self.machine.wait_until_succeeds(
            f"grep -qF {shlex.quote(server)} /home/joel/reloads", timeout=SAYING
        )


@pytest.fixture(scope="module")
def joel(machine: BaseMachine) -> Joel:
    """Boot the machine until joel's network-revive runs, and give his account."""
    machine.wait_for_unit("multi-user.target")
    machine.wait_for_unit("user@1000.service")
    machine.wait_for_unit("network-revive.service", "joel")
    return Joel(machine)


# Each state the account is brought to is a fixture giving the account in it, so one that
# builds on another takes it, and a test asks for the state it starts from.


@pytest.fixture(scope="module")
def let_down(joel: Joel) -> Joel:
    """Start the services that need the network while it's gone, and wait for each to give up."""
    joel.run("systemctl --user start --no-block needs-network waits-long never-ready backs-up")
    joel.wait_for_state("needs-network", "failed", "failed")
    joel.wait_for_state("waits-long", "activating", "auto-restart")
    joel.wait_for_state("never-ready", "failed", "failed")
    joel.wait_for_state("backs-up", "failed", "failed")
    return joel


@pytest.fixture(scope="module")
def network_back(let_down: Joel) -> Joel:
    """Bring the network up under the services it let down, and wait for those that can start."""
    joel = let_down
    joel.machine.succeed("networkctl up uplink")
    joel.wait_for_state("needs-network", "active", "running")
    joel.wait_for_state("waits-long", "active", "running")
    joel.wait_for_state("backs-up", "inactive", "dead")
    return joel


@pytest.fixture(scope="module")
def following_names(network_back: Joel) -> Joel:
    """Start the service that reloads when resolved's servers change."""
    joel = network_back
    joel.run("systemctl --user start follows-names")
    return joel


@pytest.mark.usefixtures("let_down")
def test_without_the_network_each_service_is_let_down(joel: Joel) -> None:
    """Without the network, each service is let down as the network leaves it."""
    # waits-long keeps trying, so its state is the one let_down waited for, not read again.
    assert joel.state("needs-network") == ["failed", "failed", "start-limit-hit"]
    assert joel.state("never-ready")[:2] == ["failed", "failed"]
    assert joel.state("backs-up")[:2] == ["failed", "failed"]


@pytest.mark.usefixtures("network_back")
def test_once_the_network_comes_what_it_let_down_starts_at_once(joel: Joel) -> None:
    """Once the network comes, what it let down starts at once."""
    assert joel.state("backs-up") == ["inactive", "dead", "success"]
    said = joel.said()
    restarted = "started needs-network.service again, which had failed, as the network changed"
    assert said.count(restarted) == 1, said
    assert "started waits-long.service now, not at its next try, as the network changed" in said


@pytest.mark.usefixtures("network_back")
def test_a_service_whose_check_never_passes_stays_down(joel: Joel) -> None:
    """A service whose check never passes stays down, and the journal says why."""
    joel.wait_until_said(
        "left never-ready.service down: its check still failed 31 s after the network changed:"
        " never ready",
        timeout=SETTLING,
    )
    assert joel.state("never-ready")[:2] == ["failed", "failed"]


@pytest.mark.usefixtures("following_names")
def test_a_running_service_reloads_once_for_each_change(joel: Joel, machine: BaseMachine) -> None:
    """A running service reloads once for each change, and a stopped one stays stopped."""
    joel.run("systemctl --user start stopped && systemctl --user stop stopped")
    machine.succeed("resolvectl dns uplink 10.0.0.54")
    joel.wait_for_reload_to("10.0.0.54")
    assert joel.reloads() == ["10.0.0.54"]
    # Ending where it began, a burst changes nothing.
    unchanged = joel.times_said("the machine reaches what it did")
    machine.succeed("resolvectl dns uplink 10.0.0.55 && resolvectl dns uplink 10.0.0.54")
    joel.wait_until_said("the machine reaches what it did", more_than=unchanged)
    # Ending elsewhere, however long, it's one change.
    machine.succeed("for server in 56 57 58 59 60; do resolvectl dns uplink 10.0.0.$server; done")
    joel.wait_for_reload_to("10.0.0.60")
    assert joel.reloads() == ["10.0.0.54", "10.0.0.60"]
    assert joel.state("stopped")[:2] == ["inactive", "dead"]
    assert joel.state("needs-network")[:2] == ["active", "running"]


@pytest.mark.usefixtures("following_names")
def test_it_follows_networkd_through_a_restart(joel: Joel, machine: BaseMachine) -> None:
    """It follows networkd through a restart."""
    looked = joel.times_said("the machine reaches .*")
    machine.succeed("systemctl restart systemd-networkd")
    machine.wait_until_succeeds("networkctl status uplink | grep -q routable", timeout=SAYING)
    joel.wait_until_said("the machine reaches .*", more_than=looked)
    machine.succeed("resolvectl dns uplink 10.0.0.61")
    joel.wait_for_reload_to("10.0.0.61")


@pytest.mark.usefixtures("network_back")
def test_it_hears_resolved_come_back_asking_other_servers(joel: Joel, machine: BaseMachine) -> None:
    """It hears resolved come back asking other servers."""
    # Nothing changes for networkd to hand over, so resolved coming back is all there is to
    # hear.
    machine.succeed(
        "mkdir -p /run/systemd/resolved.conf.d"
        " && printf '[Resolve]\\nDNS=10.0.0.99\\n' > /run/systemd/resolved.conf.d/other.conf"
        " && systemctl restart systemd-resolved"
    )
    joel.wait_until_said("the machine reaches .*10[.]0[.]0[.]99%0.*")


@pytest.mark.usefixtures("network_back")
def test_starting_it_starts_what_it_finds_down(joel: Joel, machine: BaseMachine) -> None:
    """Starting, it starts what it finds down."""
    joel.run("systemctl --user stop network-revive")
    machine.succeed("networkctl down uplink")
    joel.run("systemctl --user restart --no-block needs-network")
    joel.wait_for_state("needs-network", "failed", "failed")
    machine.succeed("networkctl up uplink")
    machine.wait_until_succeeds(
        "ip -o route get 10.0.0.53 | grep -q ' dev uplink '", timeout=SAYING
    )
    joel.run("systemctl --user start network-revive")
    joel.wait_for_state("needs-network", "active", "running")
    assert "started needs-network.service again, which had failed" in joel.said().splitlines()


@pytest.mark.usefixtures("network_back")
def test_a_start_asked_as_systemd_executes_itself_again_is_asked_again(
    joel: Joel, machine: BaseMachine
) -> None:
    """A start asked as systemd executes itself again is asked again."""
    # Heard going, so its coming back is a change of its own rather than the end of a burst
    # that changed nothing.
    looked = joel.times_said("the machine reaches .*")
    machine.succeed("networkctl down uplink")
    joel.wait_until_said("the machine reaches .*", more_than=looked)
    joel.run("systemctl --user start --no-block meets-reexec")
    joel.wait_for_state("meets-reexec", "failed", "failed")
    machine.succeed("networkctl up uplink")
    joel.wait_for_state("meets-reexec", "active", "running")
    machine.succeed("test -e /home/joel/reexecuted")
    # The start was asked while systemd was held away, and it let go.
    machine.succeed("test -e /home/joel/away")
    machine.fail("test -e /home/joel/hold")
    machine.succeed("journalctl _UID=1000 _COMM=systemd -o cat | grep -qx Reexecuting.")
    assert (
        "started meets-reexec.service again, which had failed, as the network changed"
        in joel.said()
    )


def test_systemd_refused_nothing_it_was_asked_for(joel: Joel) -> None:
    """Systemd refused nothing it was asked for."""
    # It reads what was said in whichever tests ran before it, so it's written last.
    assert joel.times_said("couldn't .*") == 0, joel.said()

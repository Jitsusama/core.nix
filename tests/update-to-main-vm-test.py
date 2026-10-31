"""A machine updates itself to signed main when its asker asks, and nothing else moves it.

tests/update-to-main.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py).
The repository is bare, at /srv/machines.git, and a commit's flake gives one of the machine's
systems as main's: its first, or a specialisation that adds /etc/switched, or one that adds
/etc/unhealthy, which the machine's health check refuses. Each test commits the main it needs
and reads what the machine runs afterwards.
"""

from __future__ import annotations

import json
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any

import pytest

if TYPE_CHECKING:
    from test_driver.machine import BaseMachine

WORK = "/srv/work"

# What `update-to-main ask` exits with for each ending (src/main.rs).
FAILED = 1
REFUSED = 2
UNASKED = 3

# A flake whose system is a link to one already on the machine, built with the sandbox off
# so the link's builder is the machine's own shell.
FLAKE = """{{
  outputs = _: {{
    system = derivation {{
      name = "system";
      system = "x86_64-linux";
      builder = "/run/current-system/sw/bin/sh";
      args = [ "-c" "/run/current-system/sw/bin/ln -s {target} $out" ];
    }};
  }};
}}
"""


@dataclass(frozen=True)
class Machine:
    """The machine, its systems, and the repository its main is read from."""

    machine: BaseMachine
    first: str
    switched: str
    unhealthy: str

    def commit(self, target: str, *, signed: bool = True) -> str:
        """Commit a main whose system is target, signed by the trusted key or not."""
        self.machine.succeed(f"cat > {WORK}/flake.nix <<'EOF'\n{FLAKE.format(target=target)}EOF")
        how = "-c gpg.format=ssh -c user.signingkey=/root/key" if signed else ""
        flag = "-S" if signed else ""
        self.machine.succeed(
            f"cd {WORK} && git add flake.nix"
            f" && git {how} commit --quiet --allow-empty {flag} -m main"
            " && git push --quiet origin HEAD:main"
        )
        return self.machine.succeed(f"git -C {WORK} rev-parse HEAD").strip()

    def ask(self, account: str = "someone") -> tuple[int, list[dict[str, Any]], str]:
        """Ask for an update as an account; give its status, its lines and all it said."""
        status, said = self.machine.execute(f"runuser -u {account} -- update-to-main ask 2>&1")
        lines = [json.loads(line) for line in said.splitlines() if line.startswith("{")]
        return status, lines, said

    def running(self) -> str:
        """Give the system the machine is running."""
        return self.machine.succeed("readlink -f /run/current-system").strip()

    def booting(self) -> str:
        """Give the system the machine boots next, through main's link."""
        return self.machine.succeed("readlink -f /nix/var/nix/profiles/system").strip()


@pytest.fixture(scope="module")
def machine_(machine: BaseMachine, values: dict[str, str]) -> Machine:
    """Give the machine up, with an empty repository and the key that signs its main."""
    machine.wait_for_unit("update-to-main.service")
    machine.wait_for_file("/run/update-to-main/ask.sock")
    first = machine.succeed("readlink -f /run/current-system").strip()
    systems = {
        name: machine.succeed(f"readlink -f {first}/specialisation/{name}").strip()
        for name in ("switched", "unhealthy")
    }
    machine.succeed(
        f"install -m 0600 {values['key']} /root/key",
        "git init --quiet --bare --initial-branch=main /srv/machines.git",
        f"git init --quiet --initial-branch=main {WORK}",
        f"git -C {WORK} remote add origin /srv/machines.git",
        "git config --global user.name Tester",
        "git config --global user.email tester@test",
    )
    return Machine(machine, first, systems["switched"], systems["unhealthy"])


@pytest.fixture
def on_first(machine_: Machine) -> Machine:
    """Give the machine on its first system, main signed and pointing there."""
    if machine_.running() != machine_.first:
        machine_.commit(machine_.first)
        status, _, said = machine_.ask()
        assert status == 0, said
    assert machine_.running() == machine_.first
    return machine_


def test_an_asker_updates_the_machine_to_signed_main(on_first: Machine) -> None:
    """An asker updates the machine to signed main, which it runs and boots next."""
    head = on_first.commit(on_first.switched)
    status, lines, said = on_first.ask()
    assert status == 0, said
    assert [line["step"] for line in lines] == ["fetching", "building", "switching", "done"]
    assert lines[-1]["commit"] == head
    assert lines[-1]["applied"] == "now"
    on_first.machine.succeed("test -e /etc/switched")
    assert on_first.running() == on_first.switched
    assert on_first.booting() == on_first.switched


def test_main_where_it_was_builds_nothing(on_first: Machine) -> None:
    """Asked again with main where it was, it answers unchanged and builds nothing."""
    head = on_first.commit(on_first.switched)
    status, _, said = on_first.ask()
    assert status == 0, said
    status, lines, said = on_first.ask()
    assert status == 0, said
    assert lines == [{"step": "fetching"}, {"step": "unchanged", "commit": head}]


def test_an_account_outside_the_group_cant_ask(on_first: Machine) -> None:
    """An account outside update-ask can't reach the socket, and nothing moves."""
    on_first.commit(on_first.switched)
    status, _, said = on_first.ask("other")
    assert status == UNASKED, said
    assert "Permission denied" in said
    assert on_first.running() == on_first.first


def test_an_unsigned_main_is_refused(on_first: Machine) -> None:
    """An unsigned main is refused, and the machine stays on what it ran."""
    on_first.commit(on_first.switched, signed=False)
    status, lines, said = on_first.ask()
    assert status == REFUSED, said
    assert lines[-1]["step"] == "refused"
    assert "isn't signed by a key this machine trusts" in lines[-1]["why"]
    assert on_first.running() == on_first.first


def test_a_machine_not_whole_goes_back(on_first: Machine) -> None:
    """A main the health check refuses is switched to, then left for what the machine ran."""
    on_first.commit(on_first.unhealthy)
    status, lines, said = on_first.ask()
    assert status == FAILED, said
    assert lines[-1]["step"] == "failed"
    assert "went back to what it ran" in lines[-1]["why"]
    on_first.machine.fail("test -e /etc/unhealthy")
    assert on_first.running() == on_first.first
    assert on_first.booting() == on_first.first

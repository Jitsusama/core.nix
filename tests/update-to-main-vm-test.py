"""A machine updates itself to signed main when its asker asks, and nothing else moves it.

tests/update-to-main.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py).
The repository is bare, at /srv/machines.git, and a commit's flake gives one of the machine's
systems as main's: its first, or a specialisation that adds /etc/switched, or one that adds
/etc/unhealthy, which the machine's health check refuses. A third, moved, holds a copy of
update-to-main at another path, for a switch made by hand. Each test commits the main it needs
and reads what the machine runs afterwards.

hosted trusts its repository's word for main rather than a signature, so its commits are
unsigned, their flake in the repository's machine folder. Its objects are read from someone's
clone at /home/someone/machines, and its token comes from a stand-in that writes who ran it
to that account's home. It has no builders and builds elsewhere, so its systems are marked
light unless a test says otherwise.

following's flake reads the system from a file in its core input, whose repository is
/srv/core.git, so a commit to core's main alone moves the machine.
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

# A flake whose system is a link to one already on the machine, made in the sandbox by the
# busybox the machine puts there, and marked light when asked.
FLAKE = """{{
  outputs = _: {{
    system = derivation {{
      name = "system";
      system = "x86_64-linux";
      builder = "/bin/sh";
      args = [ "-c" "/bin/busybox ln -s {target} $out" ];
      {light}
    }};
  }};
}}
"""
LIGHT = "preferLocalBuild = true;"

# following's flake: the system is the one its core input's target file names.
FOLLOWING_FLAKE = """{
  inputs.core = { url = "git+file:///srv/core.git"; flake = false; };
  outputs = { core, ... }: {
    system = derivation {
      name = "system";
      system = "x86_64-linux";
      builder = "/bin/sh";
      args = [ "-c" "/bin/busybox ln -s ${builtins.readFile "${core}/target"} $out" ];
    };
  };
}
"""
CORE = "/srv/core"
SIGN = "-c gpg.format=ssh -c user.signingkey=/root/key"


@dataclass(frozen=True)
class Machine:
    """The machine, its systems, and the repository its main is read from."""

    machine: BaseMachine
    first: str
    switched: str
    unhealthy: str
    folder: str = WORK
    light: bool = False

    def commit(self, target: str, *, signed: bool = True, light: bool | None = None) -> str:
        """Commit a main whose system is target, signed by the trusted key or not."""
        light = self.light if light is None else light
        flake = FLAKE.format(target=target, light=LIGHT if light else "")
        self.machine.succeed(
            f"mkdir -p {self.folder} && cat > {self.folder}/flake.nix <<'EOF'\n{flake}EOF"
        )
        how = SIGN if signed else ""
        flag = "-S" if signed else ""
        self.machine.succeed(
            f"cd {WORK} && git add -A"
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


def core_commit(machine: BaseMachine, target: str, *, signed: bool = True) -> str:
    """Commit to core's main a target naming the system, signed by the trusted key or not."""
    how, flag = (SIGN, "-S") if signed else ("", "")
    machine.succeed(
        f"printf %s {target} > {CORE}/target && cd {CORE} && git add -A"
        f" && git {how} commit --quiet --allow-empty {flag} -m core"
        " && git push --quiet origin HEAD:main"
    )
    return machine.succeed(f"git -C {CORE} rev-parse HEAD").strip()


def up(machine: BaseMachine, folder: str = WORK, *, light: bool = False) -> Machine:
    """Give a machine up, with an empty repository to commit its main to."""
    machine.wait_for_unit("update-to-main.service")
    machine.wait_for_file("/run/update-to-main/ask.sock")
    first = machine.succeed("readlink -f /run/current-system").strip()
    systems = {
        name: machine.succeed(f"readlink -f {first}/specialisation/{name}").strip()
        for name in ("switched", "unhealthy")
    }
    machine.succeed(
        "git init --quiet --bare --initial-branch=main /srv/machines.git",
        f"git init --quiet --initial-branch=main {WORK}",
        f"git -C {WORK} remote add origin /srv/machines.git",
        "git config --global user.name Tester",
        "git config --global user.email tester@test",
    )
    return Machine(machine, first, systems["switched"], systems["unhealthy"], folder, light)


@pytest.fixture(scope="module")
def machine_(machine: BaseMachine, values: dict[str, str]) -> Machine:
    """Give the machine up, with an empty repository and the key that signs its main."""
    machine_ = up(machine)
    machine.succeed(f"install -m 0600 {values['key']} /root/key")
    return machine_


@pytest.fixture(scope="module")
def hosted_(machines: dict[str, BaseMachine]) -> Machine:
    """Give the hosted machine up, with someone's clone of its repository."""
    hosted = up(machines["hosted"], f"{WORK}/machine", light=True)
    hosted.machine.succeed(
        f"git -C {WORK} commit --quiet --allow-empty -m start",
        f"git -C {WORK} push --quiet origin HEAD:main",
        # The repository is root's, which someone's git won't read unless told it may.
        "runuser -l someone -c 'git config --global safe.directory \"*\"'",
        "runuser -l someone -c 'git clone --quiet /srv/machines.git machines'",
    )
    return hosted


@pytest.fixture
def hosted_on_first(hosted_: Machine) -> Machine:
    """Give the hosted machine on its first system, with a token to give."""
    hosted_.machine.succeed("rm -f /etc/no-token")
    if hosted_.running() != hosted_.first:
        hosted_.commit(hosted_.first, signed=False)
        status, _, said = hosted_.ask()
        assert status == 0, said
    assert hosted_.running() == hosted_.first
    return hosted_


@pytest.fixture(scope="module")
def following_(machines: dict[str, BaseMachine], values: dict[str, str]) -> Machine:
    """Give the following machine up, main's flake locked to core's first commit."""
    following = up(machines["following"])
    machine = following.machine
    machine.succeed(
        f"install -m 0600 {values['key']} /root/key",
        "git init --quiet --bare --initial-branch=main /srv/core.git",
        f"git init --quiet --initial-branch=main {CORE}",
        f"git -C {CORE} remote add origin /srv/core.git",
    )
    core_commit(machine, following.first)
    machine.succeed(
        f"cat > {WORK}/flake.nix <<'EOF'\n{FOLLOWING_FLAKE}EOF",
        f"cd {WORK} && git add -A && nix flake lock && git add -A"
        f" && git {SIGN} commit --quiet -S -m main && git push --quiet origin HEAD:main",
    )
    return following


@pytest.fixture
def following_on_first(following_: Machine) -> Machine:
    """Give the following machine on its first system, core's main signed and naming it."""
    if following_.running() != following_.first:
        core_commit(following_.machine, following_.first)
        status, _, said = following_.ask()
        assert status == 0, said
    assert following_.running() == following_.first
    return following_


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
    # How it ended is kept, past the restart its switch brought, for the
    # group to read and no one else.
    kept = on_first.machine.succeed("runuser -u someone -- cat /run/update-to-main/last.json")
    assert json.loads(kept) == lines[-1]
    on_first.machine.fail("runuser -u other -- cat /run/update-to-main/last.json")


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


def test_a_hosted_main_is_built_unsigned_from_the_accounts_checkout(
    hosted_on_first: Machine,
) -> None:
    """A hosted main is built unsigned, its objects fetched as someone and read by root."""
    hosted = hosted_on_first
    head = hosted.commit(hosted.switched, signed=False)
    status, lines, said = hosted.ask()
    assert status == 0, said
    assert lines[-1]["commit"] == head
    hosted.machine.succeed("test -e /etc/switched")
    assert hosted.running() == hosted.switched
    # The token came from someone, with someone's groups and none of root's.
    asker, *groups = hosted.machine.succeed("cat /home/someone/token-asker").split()
    assert asker == "someone", groups
    assert "root" not in groups, groups
    # main was fetched into someone's checkout, as someone, and root reads it from there.
    hosted.machine.succeed(f"runuser -l someone -c 'git -C machines cat-file -e {head}'")
    borrowed = "/var/lib/update-to-main/repository.git/objects/info/alternates"
    assert hosted.machine.succeed(f"cat {borrowed}").strip() == (
        "/home/someone/machines/.git/objects"
    )


def test_a_hosted_main_with_a_heavy_piece_and_no_builder_builds_nothing(
    hosted_on_first: Machine,
) -> None:
    """Building elsewhere with no builder, a system not marked light isn't built here."""
    hosted = hosted_on_first
    hosted.commit(hosted.switched, signed=False, light=False)
    status, lines, said = hosted.ask()
    assert status == FAILED, said
    assert "on the machine's builders" in lines[-1]["why"]
    assert any("local builds are disabled" in line for line in lines[-1]["lines"]), said
    assert hosted.running() == hosted.first


def test_a_followed_input_moves_the_machine_with_main_where_it_was(
    following_on_first: Machine,
) -> None:
    """A signed commit to core's main alone switches the machine to the system it names."""
    following = following_on_first
    main = following.machine.succeed(f"git -C {WORK} rev-parse HEAD").strip()
    core = core_commit(following.machine, following.switched)
    status, lines, said = following.ask()
    assert status == 0, said
    assert lines[-1]["commit"] == main
    assert lines[-1]["followed"] == {"core": core}
    following.machine.succeed("test -e /etc/switched")
    assert following.running() == following.switched


def test_a_followed_input_unsigned_is_refused(following_on_first: Machine) -> None:
    """An unsigned commit to core's main is refused, and the machine stays where it was."""
    following = following_on_first
    core_commit(following.machine, following.switched, signed=False)
    status, lines, said = following.ask()
    assert status == REFUSED, said
    assert lines[-1]["why"].startswith("core's main: ")
    assert following.running() == following.first


def test_a_hosted_main_with_no_token_builds_nothing(hosted_on_first: Machine) -> None:
    """With no token to ask its host with, the hosted machine stays on what it ran."""
    hosted = hosted_on_first
    hosted.commit(hosted.switched, signed=False)
    hosted.machine.succeed("touch /etc/no-token")
    status, lines, said = hosted.ask()
    hosted.machine.succeed("rm /etc/no-token")
    assert status == FAILED, said
    assert "asking for a token failed" in lines[-1]["why"]
    assert lines[-1]["lines"] == ["no login"]
    assert hosted.running() == hosted.first


def test_a_switch_by_hand_brings_the_program_it_switched_to(on_first: Machine) -> None:
    """A switch made without an ask still brings its update-to-main, which serves the next ask."""
    machine = on_first.machine
    moved = machine.succeed(f"readlink -f {on_first.first}/specialisation/moved").strip()
    program = machine.succeed(f"readlink -f {moved}/sw/bin/update-to-main").strip()
    serving = "readlink -f /proc/$(systemctl show -p MainPID --value update-to-main)/exe"
    assert machine.succeed(serving).strip() != program
    machine.succeed(f"{moved}/bin/switch-to-configuration test")
    machine.wait_until_succeeds(f'test "$({serving})" = {program}', timeout=30)
    status, _, said = on_first.ask()
    assert status == 0, said


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

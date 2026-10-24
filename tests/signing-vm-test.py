"""Commits signed in a virtual machine with keys made in its TPM, as Joel and his agents commit.

tests/signing.nix runs this through pytest inside the driver (tests/lib/vm_pytest.py), and its
opening comment says what the test proves. Each test asks for the state it starts from as a
fixture: Joel's account up with the askpass answering for him, then both keys made in the TPM.
A fixture's work is done once a run, however many tests ask for it, and each test reads only
the prompts its own commit asked, so any test runs alone with `-k`. The PIN, the askpasses
and the signer the module lists are signing.nix's, given as the `values` fixture.
"""

from __future__ import annotations

import shlex
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any

import pytest
from vm_helpers import as_account

if TYPE_CHECKING:
    from collections.abc import Iterator

    from test_driver.machine import BaseMachine

# Where Joel's key and the agent's key are made, as docs/installing.md makes them.
JOEL_KEY = "~/.local/share/signing/joel-signing"
AGENT_KEY = "~/.ssh/agent-signing"


def at_terminal(command: str) -> str:
    """Give a command run at a terminal, as Joel types one."""
    return "script -qec " + shlex.quote(command) + " /dev/null"


class Joel:
    """Joel's account, whose agent signs his commits, and the askpass answering for him."""

    def __init__(self, machine: BaseMachine, runtime: str) -> None:
        """Act on Joel's account on that machine, his runtime folder that one."""
        self.machine = machine
        self.runtime = runtime

    def run(self, command: str, *, succeed: bool = True) -> str:
        """Run a command as Joel in his home, giving its output; unless succeed, it must fail."""
        return as_account(self.machine, command, runtime=self.runtime, succeed=succeed)

    def asked_while(self, command: str, *, succeed: bool = True) -> list[str]:
        """Run a command as Joel and give every prompt the askpass was asked while it ran."""
        self.run(f"rm -f {self.runtime}/asked")
        self.run(command, succeed=succeed)
        return self.run(f"cat {self.runtime}/asked 2>/dev/null || true").split()

    def signer_of_last_commit(self) -> list[str]:
        """Give whether git trusts the last commit's signature, and the key that made it."""
        return self.run(
            "git -C repo -c gpg.ssh.allowedSignersFile=$HOME/signers log -1 --format='%G? %GK'"
        ).split()

    def fingerprint(self, public_key: str) -> str:
        """Give the fingerprint of the public key in the file."""
        return self.run(f"ssh-keygen -lf {public_key}").split()[1]

    def head(self) -> str:
        """Give the commit the repository is at, or nothing before its first."""
        return self.run("git -C repo rev-parse -q --verify HEAD || true")


@dataclass(frozen=True)
class Keys:
    """The fingerprints of the two keys made in the TPM."""

    joel: str
    agent: str


@pytest.fixture(scope="module")
def joel(machine: BaseMachine, values: dict[str, Any]) -> Joel:
    """Give Joel's account, his agent listening and the askpass answering its prompts."""
    account = Joel(machine, values["runtime"])
    machine.wait_for_unit("multi-user.target")
    machine.succeed("loginctl enable-linger joel")
    machine.wait_for_unit("user@1000.service")
    machine.wait_for_unit("ssh-tpm-agent.socket", "joel")
    account.run(
        f"systemctl --user set-environment SSH_ASKPASS={values['askpass']}"
        " SSH_ASKPASS_REQUIRE=force"
    )
    return account


@pytest.fixture(scope="module")
def keys(joel: Joel, values: dict[str, Any]) -> Keys:
    """Give Joel's key, behind his PIN, and the agent's, without one, both made in the TPM.

    Both are trusted in ~/signers, which the tests verify against, and the agent holds them
    once it starts again. A repository waits for commits.
    """
    joel.run("mkdir -p ~/.ssh ~/.local/share/signing")
    joel.run(f"ssh-tpm-keygen -N {values['pin']} -C joel@test -f {JOEL_KEY}")
    # No PIN: an empty answer to both of the keygen's prompts.
    joel.run(
        f"SSH_ASKPASS_REQUIRE=force SSH_ASKPASS={values['noPin']}"
        f" ssh-tpm-keygen -C agent@test -f {AGENT_KEY}"
    )
    joel.run(
        f"for key in {JOEL_KEY}.pub {AGENT_KEY}.pub;"
        ' do echo "joel@test namespaces=\\"git\\" $(cat $key)"; done > signers'
    )
    joel.run("systemctl --user restart ssh-tpm-agent")
    joel.machine.wait_for_unit("signing-key.service", "joel")
    joel.run("git init -q repo")
    return Keys(
        joel=joel.fingerprint(f"{JOEL_KEY}.pub"), agent=joel.fingerprint(f"{AGENT_KEY}.pub")
    )


@pytest.fixture
def refusing(joel: Joel) -> Iterator[None]:
    """Have the askpass refuse every confirmation until the test ends."""
    joel.run(f"touch {joel.runtime}/refuse")
    yield
    joel.run(f"rm {joel.runtime}/refuse")


def test_both_keys_are_made_in_the_tpm_and_the_agent_holds_them(joel: Joel, keys: Keys) -> None:
    """Both keys are made in the TPM and the agent holds them."""
    listed = joel.run(f"SSH_AUTH_SOCK={joel.runtime}/ssh-tpm-agent.sock ssh-add -l")
    assert keys.joel in listed
    assert keys.agent in listed


def test_git_trusts_the_signers_the_module_lists(joel: Joel, values: dict[str, Any]) -> None:
    """Git trusts the signers the module lists."""
    signers = joel.run('cat "$(git config gpg.ssh.allowedSignersFile)"')
    assert signers.strip() == values["otherSigner"]


def test_a_commit_over_pipes_is_the_agents_and_asks_nothing(joel: Joel, keys: Keys) -> None:
    """A commit over pipes is the agent's and asks nothing."""
    asked = joel.asked_while("git -C repo commit -q --allow-empty -m 'from a harness' < /dev/null")
    assert joel.signer_of_last_commit() == ["G", keys.agent]
    assert asked == []


def test_a_commit_at_a_terminal_is_joels_after_his_pin_and_a_confirmation(
    joel: Joel, keys: Keys
) -> None:
    """A commit at a terminal is Joel's, after his PIN and a confirmation."""
    asked = joel.asked_while(at_terminal("git -C repo commit -q --allow-empty -m 'from Joel'"))
    assert joel.signer_of_last_commit() == ["G", keys.joel]
    assert "confirm" in asked
    assert "pin" in asked


def test_sign_as_picks_the_key_outright(joel: Joel, keys: Keys) -> None:
    """SIGN_AS picks the key outright."""
    joel.run(at_terminal("SIGN_AS=agent git -C repo commit -q --allow-empty -m 'chosen'"))
    assert joel.signer_of_last_commit() == ["G", keys.agent]


@pytest.mark.usefixtures("keys", "refusing")
def test_a_refused_confirmation_leaves_no_commit(joel: Joel) -> None:
    """A refused confirmation leaves no commit."""
    head = joel.head()
    asked = joel.asked_while(
        at_terminal("git -C repo commit -q --allow-empty -m 'refused'"), succeed=False
    )
    assert "confirm" in asked
    assert joel.head() == head

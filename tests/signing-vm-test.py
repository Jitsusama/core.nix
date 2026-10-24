"""Commits signed in a virtual machine with keys made in its TPM, as Joel and his agents commit.

tests/signing.nix runs this, and its opening comment says what the test proves. run-subtests
cuts the script into subtests at each top-level `with subtest(`, so every subtest stays at
the top level under its own name, and what sits outside them, the boot and the helpers, runs
whichever subtests are chosen. The PIN, the askpasses that answer for Joel and the signer the
module lists are signing.nix's, so they come from the JSON file it writes, whose path it puts
in place of the marker below.
"""

import json
import shlex
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from driver_names import machine, subtest, t

# Joel's runtime folder, where the askpass writes down what it's asked; the PIN it answers
# with; it and one answering nothing, as programs in the store; and the other machine's
# signer the module lists, all as signing.nix has them.
values = json.loads(Path("@values@").read_text())
runtime: str = values["runtime"]
pin: str = values["pin"]
askpass: str = values["askpass"]
no_pin: str = values["noPin"]
other_signer: str = values["otherSigner"]


def as_joel(command: str, *, succeed: bool = True) -> str:
    """Run a command as Joel, in his home, and give what it said, failing as it's told to."""
    environment = f"export XDG_RUNTIME_DIR={runtime}; cd ~; "
    run = machine.succeed if succeed else machine.fail
    return run("su - joel -c " + shlex.quote(environment + command))


def at_terminal(command: str) -> str:
    """Give a command run at a terminal, as Joel types one."""
    return "script -qec " + shlex.quote(command) + " /dev/null"


def signer_of_last_commit() -> list[str]:
    """Give whether git trusts the last commit's signature, and the key that made it."""
    return as_joel(
        "git -C repo -c gpg.ssh.allowedSignersFile=$HOME/signers log -1 --format='%G? %GK'"
    ).split()


def fingerprint(public_key: str) -> str:
    """Give the fingerprint of the public key in the file."""
    return as_joel(f"ssh-keygen -lf {public_key}").split()[1]


def asked() -> list[str]:
    """Give every prompt the askpass has been asked."""
    return as_joel(f"cat {runtime}/asked 2>/dev/null || true").split()


machine.wait_for_unit("multi-user.target")
machine.succeed("loginctl enable-linger joel")
machine.wait_for_unit("user@1000.service")
machine.wait_for_unit("ssh-tpm-agent.socket", "joel")
as_joel(f"systemctl --user set-environment SSH_ASKPASS={askpass} SSH_ASKPASS_REQUIRE=force")

joel_key = "~/.local/share/signing/joel-signing"
agent_key = "~/.ssh/agent-signing"

with subtest("both keys are made in the TPM and the agent holds them"):
    as_joel("mkdir -p ~/.ssh ~/.local/share/signing")
    as_joel(f"ssh-tpm-keygen -N {pin} -C joel@test -f {joel_key}")
    # No PIN: an empty answer to both of the keygen's prompts.
    as_joel(
        f"SSH_ASKPASS_REQUIRE=force SSH_ASKPASS={no_pin}"
        f" ssh-tpm-keygen -C agent@test -f {agent_key}"
    )
    as_joel(
        f"for key in {joel_key}.pub {agent_key}.pub;"
        ' do echo "joel@test namespaces=\\"git\\" $(cat $key)"; done > signers'
    )
    as_joel("systemctl --user restart ssh-tpm-agent")
    machine.wait_for_unit("signing-key.service", "joel")
    listed = as_joel(f"SSH_AUTH_SOCK={runtime}/ssh-tpm-agent.sock ssh-add -l")
    assert "joel@test" in listed, listed
    assert "agent@test" in listed, listed
    as_joel("git init -q repo")

with subtest("git trusts the signers the module lists"):
    signers = as_joel('cat "$(git config gpg.ssh.allowedSignersFile)"')
    t.assertEqual(signers.strip(), other_signer)

with subtest("a commit over pipes is the agent's and asks nothing"):
    as_joel("git -C repo commit -q --allow-empty -m 'from a harness' < /dev/null")
    t.assertEqual(signer_of_last_commit(), ["G", fingerprint(f"{agent_key}.pub")])
    t.assertEqual(asked(), [])

with subtest("a commit at a terminal is Joel's, after his PIN and a confirmation"):
    as_joel(at_terminal("git -C repo commit -q --allow-empty -m 'from Joel'"))
    t.assertEqual(signer_of_last_commit(), ["G", fingerprint(f"{joel_key}.pub")])
    t.assertIn("confirm", asked())
    t.assertIn("pin", asked())

with subtest("SIGN_AS picks the key outright"):
    as_joel(at_terminal("SIGN_AS=agent git -C repo commit -q --allow-empty -m 'chosen'"))
    t.assertEqual(signer_of_last_commit(), ["G", fingerprint(f"{agent_key}.pub")])

with subtest("a refused confirmation leaves no commit"):
    head = as_joel("git -C repo rev-parse HEAD")
    as_joel(f"touch {runtime}/refuse")
    as_joel(at_terminal("git -C repo commit -q --allow-empty -m 'refused'"), succeed=False)
    t.assertEqual(as_joel("git -C repo rev-parse HEAD"), head)

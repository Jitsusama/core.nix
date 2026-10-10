# Makes Joel's two signing keys in a virtual machine's TPM, as
# docs/installing.md does, and commits the ways he and his harnesses do. A
# commit made over pipes is signed with the agent's key and asks nothing; one
# made at a terminal is signed with Joel's key, after its PIN and a
# confirmation; a refused confirmation leaves no commit. git verifies each
# signature against the key that should have made it, and trusts the signers
# the module lists without losing how it signs.
{ self, pkgs }:
let
  pin = "2468";
  runtime = "/run/user/1000";
  # Answers the agent's prompts, as Joel does through the themed ones, and
  # writes down each one it was asked. It refuses confirmations once told to.
  askpass = pkgs.writeShellScript "askpass" ''
    echo "''${SSH_ASKPASS_PROMPT:-pin}" >> ${runtime}/asked
    if [ "''${SSH_ASKPASS_PROMPT:-}" = confirm ]; then
      [ ! -e ${runtime}/refuse ]
    else
      echo ${pin}
    fi
  '';
  noPin = pkgs.writeShellScript "no-pin" "echo";
  # Another machine's key. The keys under test are only made once the machine
  # runs, so the test verifies against its own file; this one is what a
  # machine lists, which must leave the rest of git's signing settings alone.
  otherSigner = ''someone@test namespaces="git" ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIPE3v5gInXpF9AF9ZJDHfv+dG7ISngMHkCkmp/EeRCjL'';
in
pkgs.testers.runNixOSTest {
  name = "signing";

  nodes.machine = {
    imports = [
      self.nixosModules.home-manager
      self.nixosModules.tpm
    ];

    virtualisation.tpm.enable = true;

    users.users.joel.isNormalUser = true;
    home-manager.users.joel = {
      imports = [ self.homeModules.signing ];
      jitsusama.identity.email = "joel@test";
      jitsusama.signing.allowedSigners = [ otherSigner ];
      home.stateVersion = "26.05";
    };
  };

  testScript = ''
    import shlex

    def as_joel(command, succeed=True):
        environment = "export XDG_RUNTIME_DIR=${runtime}; cd ~; "
        run = machine.succeed if succeed else machine.fail
        return run("su - joel -c " + shlex.quote(environment + command))

    # A command at a terminal, as Joel types one.
    def at_terminal(command):
        return "script -qec " + shlex.quote(command) + " /dev/null"

    def signer_of_last_commit():
        return as_joel(
            "git -C repo -c gpg.ssh.allowedSignersFile=$HOME/signers log -1 --format='%G? %GK'"
        ).split()

    def fingerprint(public_key):
        return as_joel(f"ssh-keygen -lf {public_key}").split()[1]

    def asked():
        return as_joel("cat ${runtime}/asked 2>/dev/null || true").split()

    machine.wait_for_unit("multi-user.target")
    machine.succeed("loginctl enable-linger joel")
    machine.wait_for_unit("user@1000.service")
    machine.wait_for_unit("ssh-tpm-agent.socket", "joel")
    as_joel("systemctl --user set-environment SSH_ASKPASS=${askpass} SSH_ASKPASS_REQUIRE=force")

    joel_key = "~/.local/share/signing/joel-signing"
    agent_key = "~/.ssh/agent-signing"

    with subtest("both keys are made in the TPM and the agent holds them"):
        as_joel("mkdir -p ~/.ssh ~/.local/share/signing")
        as_joel(f"ssh-tpm-keygen -N ${pin} -C joel@test -f {joel_key}")
        # No PIN: an empty answer to both of the keygen's prompts.
        as_joel(
            "SSH_ASKPASS_REQUIRE=force SSH_ASKPASS=${noPin}"
            f" ssh-tpm-keygen -C agent@test -f {agent_key}"
        )
        as_joel(
            f"for key in {joel_key}.pub {agent_key}.pub;"
            " do echo \"joel@test namespaces=\\\"git\\\" $(cat $key)\"; done > signers"
        )
        as_joel("systemctl --user restart ssh-tpm-agent")
        machine.wait_for_unit("signing-key.service", "joel")
        listed = as_joel("SSH_AUTH_SOCK=${runtime}/ssh-tpm-agent.sock ssh-add -l")
        assert "joel@test" in listed and "agent@test" in listed, listed
        as_joel("git init -q repo")

    with subtest("git trusts the signers the module lists"):
        signers = as_joel("cat \"$(git config gpg.ssh.allowedSignersFile)\"")
        t.assertEqual(signers.strip(), ${builtins.toJSON otherSigner})

    with subtest("a commit over pipes is the agent's and asks nothing"):
        as_joel("git -C repo commit -q --allow-empty -m 'from a harness' < /dev/null")
        t.assertEqual(signer_of_last_commit(), ["G", fingerprint(f"{agent_key}.pub")])
        t.assertEqual(asked(), [])

    with subtest("a commit at a terminal is Joel's, after his PIN and a confirmation"):
        as_joel(at_terminal("git -C repo commit -q --allow-empty -m 'from Joel'"))
        t.assertEqual(signer_of_last_commit(), ["G", fingerprint(f"{joel_key}.pub")])
        t.assertIn("confirm", asked())
        t.assertIn("pin", asked())

    with subtest("TPM_SIGNER picks the key outright"):
        as_joel(at_terminal("TPM_SIGNER=agent git -C repo commit -q --allow-empty -m 'chosen'"))
        t.assertEqual(signer_of_last_commit(), ["G", fingerprint(f"{agent_key}.pub")])

    with subtest("a refused confirmation leaves no commit"):
        head = as_joel("git -C repo rev-parse HEAD")
        as_joel("touch ${runtime}/refuse")
        as_joel(at_terminal("git -C repo commit -q --allow-empty -m 'refused'"), succeed=False)
        t.assertEqual(as_joel("git -C repo rev-parse HEAD"), head)
  '';
}

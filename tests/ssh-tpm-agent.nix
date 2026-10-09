# Makes an SSH key with a PIN in a virtual machine's TPM, as
# docs/installing.md does, then has the account's ssh-tpm-agent sign with it:
# the agent loads the key when it starts, asks for the PIN through SSH_ASKPASS, and
# the signature checks out. It fails when the account can't reach the TPM or
# the agent doesn't start.
{ self, pkgs }:
let
  pin = "2468";
  # Answers the agent's PIN prompt, as Joel does through the themed one.
  typePin = pkgs.writeShellScript "type-pin" "echo ${pin}";
in
pkgs.testers.runNixOSTest {
  name = "ssh-tpm-agent";

  nodes.machine = {
    imports = [
      self.nixosModules.home-manager
      self.nixosModules.tpm
    ];

    virtualisation.tpm.enable = true;

    users.users.joel.isNormalUser = true;
    home-manager.users.joel = {
      imports = [ self.homeModules.ssh-tpm-agent ];
      home.stateVersion = "26.05";
    };
  };

  testScript = ''
    import shlex

    def as_joel(command):
        environment = (
            "export XDG_RUNTIME_DIR=/run/user/$(id -u); "
            "export SSH_AUTH_SOCK=$XDG_RUNTIME_DIR/ssh-tpm-agent.sock; "
        )
        return machine.succeed("su - joel -c " + shlex.quote(environment + command))

    machine.wait_for_unit("multi-user.target")
    machine.succeed("loginctl enable-linger joel")
    machine.wait_for_unit("user@1000.service")
    machine.wait_for_unit("ssh-tpm-agent.socket", "joel")
    # The agent starts on its first connection, with these in its environment.
    as_joel("systemctl --user set-environment SSH_ASKPASS=${typePin} SSH_ASKPASS_REQUIRE=force")

    with subtest("the key is made in the TPM"):
        as_joel("mkdir -p ~/.ssh && ssh-tpm-keygen -N ${pin} -C joel@test -f ~/.ssh/id_ecdsa")
        as_joel("grep -q 'BEGIN TSS2 PRIVATE KEY' ~/.ssh/id_ecdsa.tpm")

    with subtest("the agent loads it when it starts"):
        as_joel("systemctl --user restart ssh-tpm-agent")
        listed = as_joel("ssh-add -l")
        assert "joel@test" in listed, listed

    with subtest("a signature from the agent checks out"):
        as_joel("echo commit > message")
        as_joel("ssh-keygen -Y sign -n git -f ~/.ssh/id_ecdsa.pub message")
        as_joel("ssh-keygen -Y check-novalidate -n git -s message.sig < message")
  '';
}

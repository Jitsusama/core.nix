# Makes Joel's two signing keys in a virtual machine's TPM, as
# docs/installing.md does, and commits the ways he and his harnesses do. A
# commit made over pipes is signed with the agent's key and asks nothing; one
# made at a terminal is signed with Joel's key, after its PIN and a
# confirmation; a refused confirmation leaves no commit, and one left
# unanswered holds up no harness's commit. git verifies each
# signature against the key that should have made it, and trusts the signers
# the module lists without losing how it signs. The tests are
# signing-vm-test.py, run under pytest.
{ self, pkgs }:
let
  pin = "2468";
  runtime = "/run/user/1000";
  # Answers the agent's prompts as Joel does, refusing or holding a
  # confirmation when told to (signing-askpass.sh).
  askpass = pkgs.writeShellScript "askpass" (
    builtins.replaceStrings [ "@runtime@" "@pin@" ] [ runtime pin ] (
      builtins.readFile ./signing-askpass.sh
    )
  );
  noPin = pkgs.writeShellScript "no-pin" "echo";
  # Another machine's key. The keys under test are only made once the machine
  # runs, so the test verifies against its own file; this one is what a
  # machine lists, which must leave the rest of git's signing settings alone.
  otherSigner = ''someone@test namespaces="git" ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIPE3v5gInXpF9AF9ZJDHfv+dG7ISngMHkCkmp/EeRCjL'';
  # What the tests need of the above, as their values fixture.
  values = pkgs.writeText "signing-values.json" (
    builtins.toJSON {
      inherit
        runtime
        pin
        askpass
        noPin
        otherSigner
        ;
    }
  );
in
import ./lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "signing";

  nodes.machine = {
    imports = [
      self.nixosModules.home-manager
      self.nixosModules.tpm
    ];

    virtualisation.tpm.enable = true;

    users.users.joel.isNormalUser = true;
    home-manager.users.joel = {
      imports = [ self.homeModules.tpm-signing ];
      jitsusama.identity.email = "joel@test";
      jitsusama.signing.allowedSigners = [ otherSigner ];
      home.stateVersion = "26.05";
    };
  };

  tests = ./signing-vm-test.py;
  inherit values;
}

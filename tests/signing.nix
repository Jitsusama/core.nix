# Makes Joel's two signing keys in a virtual machine's TPM, as
# docs/installing.md does, and commits the ways he and his harnesses do. A
# commit made over pipes is signed with the agent's key and asks nothing; one
# made at a terminal is signed with Joel's key, after its PIN and a
# confirmation; a refused confirmation leaves no commit. git verifies each
# signature against the key that should have made it, and trusts the signers
# the module lists without losing how it signs. The tests are
# signing-vm-test.py, run under pytest.
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
      imports = [ self.homeModules.tpm-signing ];
      jitsusama.identity.email = "joel@test";
      jitsusama.signing.allowedSigners = [ otherSigner ];
      home.stateVersion = "26.05";
    };
  };

  # What VM tests share (tests/lib), pytest and its runner among them.
  extraPythonPackages = python: [ (pkgs.callPackage ./lib/vm-helpers.nix { } python) ];

  # Run by hand, the driver takes pytest's options from PYTEST_ADDOPTS, so
  # PYTEST_ADDOPTS='-k terminal' runs only the commit made at a terminal.
  testScript = ''
    import vm_pytest
    vm_pytest.run(globals(), "${./signing-vm-test.py}", values="${values}")
  '';
  # nix fmt holds the tests and vm_pytest to mypy --strict against the
  # driver's own types, so the driver's check of this one line would add only
  # its time.
  skipTypeCheck = true;
}

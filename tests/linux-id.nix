# Makes a passkey and a security key in a virtual machine's TPM through
# linux-id, with Yubico's FIDO library in a browser's place (passkey.py), and
# signs in with each. Each asks the account's askpass for a yes in linux-id's
# own words, a refusal makes nothing, a site that wants to know who's there
# is refused, and the keys outlive linux-id restarting, since the TPM holds
# them. The tests are linux-id-vm-test.py, which pytest runs inside the
# driver (tests/lib/vm_pytest.py).
{ self, pkgs }:
let
  runtime = "/run/user/1000";
  # What the tests need of the above, their `values` fixture.
  values = pkgs.writeText "linux-id-values.json" (builtins.toJSON { inherit runtime; });
  # Answers as Joel does through the shell's prompt, and writes down each
  # question, until it's told to refuse.
  askpass = pkgs.writeShellScript "askpass" (
    builtins.replaceStrings [ "@runtime@" ] [ runtime ] (builtins.readFile ./linux-id-askpass.sh)
  );
  # ruff and mypy read it through `nix fmt`, so the writers' flake8 is off.
  passkey = pkgs.writers.writePython3Bin "passkey" {
    libraries = [ pkgs.python3Packages.fido2 ];
    doCheck = false;
  } (builtins.readFile ./passkey.py);
in
import ./lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "linux-id";

  nodes.machine = {
    imports = [
      self.nixosModules.home-manager
      self.nixosModules.linux-id
    ];

    virtualisation.tpm.enable = true;

    # Signed in at the seat, as Joel is at the desktop, which is what gives
    # the account the devices.
    users.users.joel.isNormalUser = true;
    services.getty.autologinUser = "joel";
    home-manager.users.joel = {
      systemd.user.sessionVariables.SSH_ASKPASS = "${askpass}";
      home.stateVersion = "26.05";
    };
    environment.systemPackages = [
      passkey
      pkgs.libfido2
    ];
  };

  tests = ./linux-id-vm-test.py;
  inherit values;
}

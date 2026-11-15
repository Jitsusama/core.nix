# Runs net's switch reader as the machine runs it, a system service under a
# user of its own, against a second machine standing in for the home's
# TP-Link switch (switch-stand-in.py), which serves the real switch's status
# pages only once signed in. Only a booted machine shows what systemd makes
# of the credential: that the reader starts signed out without one, signs
# in once it's written, and that Joel's account can read neither the
# password nor anything that holds it, while it reads the report. The
# tests are net-switch-vm-test.py, which pytest runs inside the driver
# (tests/lib/vm_pytest.py).
{ self, pkgs }:
let
  inherit (pkgs) lib;
  # The test's own password, made up, so each test can look for it.
  password = "stand-in switch password 7Q";
  counted = "/run/switch-stand-in/sign-ins";
  values = pkgs.writeText "net-switch-values.json" (builtins.toJSON { inherit password counted; });
in
import ../../lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "net-switch";

  nodes.switch = {
    networking.firewall.allowedTCPPorts = [ 80 ];
    systemd.services.switch-stand-in = {
      wantedBy = [ "multi-user.target" ];
      serviceConfig = {
        RuntimeDirectory = "switch-stand-in";
        ExecStart = lib.escapeShellArgs [
          (lib.getExe pkgs.python3)
          "${./switch-stand-in.py}"
          "${../../../modules/net/contract/switch}"
          password
          counted
        ];
      };
    };
  };

  nodes.machine =
    { nodes, ... }:
    {
      imports = [ self.nixosModules.net ];
      jitsusama.net.switch.address = nodes.switch.networking.primaryIPAddress;
      # Joel's account, as the laptop has it: uid 1000, in wheel.
      users.users.joel = {
        isNormalUser = true;
        uid = 1000;
        extraGroups = [ "wheel" ];
      };
    };

  tests = ./net-switch-vm-test.py;
  inherit values;
}

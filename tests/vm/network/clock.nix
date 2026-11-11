# Sets a machine's clock on a network that names a time server of its own
# that never answers, as a hotel's did. The machine's own servers are a second
# machine running chrony, which it reaches; it has to get there, and be told
# the network's server too, which it asks beside its own. networkd names the
# network's server once the link is up, and chrony is told it then, without
# being started again. The tests are clock-vm-test.py, which pytest runs
# inside the driver (tests/lib/vm_pytest.py).
{ self, pkgs }:
let
  silent = "192.168.1.99";
  # What the tests need from Nix, in one file, their `values` fixture.
  values = pkgs.writeText "clock-values.json" (builtins.toJSON { inherit silent; });
in
import ../../lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "clock";

  nodes.server = {
    services.chrony = {
      enable = true;
      servers = [ ];
      # Serves its own clock, to anyone on the test's network.
      extraConfig = ''
        allow
        local stratum 8
      '';
    };
    networking.firewall.allowedUDPPorts = [ 123 ];
  };

  nodes.machine = {
    imports = [ self.nixosModules.chrony ];
    networking.useNetworkd = true;
    networking.timeServers = [ "server" ];
    systemd.network.networks."40-eth1".ntp = [ silent ];
  };

  tests = ./clock-vm-test.py;
  inherit values;
}

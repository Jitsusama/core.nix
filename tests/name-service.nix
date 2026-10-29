# Runs two machines on one network, both answering mDNS there. The asker
# holds its IPv4 mDNS queries back for a second, so the answerer's reply
# over IPv6 always comes first, as it did on a laptop whose neighbours'
# IPv4 addresses then never resolved through the name service. The tests
# are name-service-vm-test.py, which pytest runs inside the driver
# (tests/lib/vm_pytest.py).
{ self, pkgs }:
let
  machine = extra: {
    imports = [
      self.nixosModules.systemd-networkd
      self.nixosModules.nftables
    ]
    ++ extra;
    # The test's own file for the test network, sorting before 90-ethernet.
    systemd.network.networks."40-eth1".networkConfig.MulticastDNS = "yes";
  };
in
import ./lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "name-service";
  # The name service patches nsncd through an overlay.
  node.pkgsReadOnly = false;

  nodes.asker = machine [ self.nixosModules.name-service ];
  nodes.answerer = machine [ ];

  tests = ./name-service-vm-test.py;
}

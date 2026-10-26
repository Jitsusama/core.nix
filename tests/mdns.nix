# Runs three machines with networkd and the nftables firewall a laptop has,
# on one network. Two trust it, as a saved network wifid trusts turns mDNS
# on in its own file; the third treats it as a stranger's, with mDNS off as
# 90-wifi and 90-ethernet leave it. A trusting machine finds the other by its .local
# name, and nobody finds the stranger. The tests are mdns-vm-test.py, which
# pytest runs inside the driver (tests/lib/vm_pytest.py).
{ self, pkgs }:
let
  machine = mdns: {
    imports = [
      self.nixosModules.systemd-networkd
      self.nixosModules.nftables
    ];
    # The test's own file for the test network, sorting before 90-ethernet.
    systemd.network.networks."40-eth1".networkConfig.MulticastDNS = mdns;
  };
in
import ./lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "mdns";

  nodes.home = machine "yes";
  nodes.neighbour = machine "yes";
  nodes.stranger = machine "no";

  tests = ./mdns-vm-test.py;
}

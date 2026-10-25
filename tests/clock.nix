# Sets a machine's clock on a network that names a time server of its own
# that never answers, as a hotel's did. The machine's own servers are a second
# machine running chrony, which it reaches; it has to get there, and not keep
# asking the network's. Nothing names the network's server until networkd has
# it, so timesyncd starts once it does: started sooner, it would find no server
# named and fall back to the machine's, which proves nothing. The tests are
# clock-vm-test.py, which pytest runs inside the driver (tests/lib/vm_pytest.py).
{ self, pkgs }:
let
  silent = "192.168.1.99";
  # What the tests need from Nix, in one file, their `values` fixture.
  values = pkgs.writeText "clock-values.json" (builtins.toJSON { inherit silent; });
in
pkgs.testers.runNixOSTest {
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

  nodes.machine =
    { lib, ... }:
    {
      imports = [ self.nixosModules.timesyncd ];
      networking.useNetworkd = true;
      networking.timeServers = [ "server" ];
      systemd.network.networks."40-eth1".ntp = [ silent ];
      # A virtual machine takes its time from KVM, so NixOS leaves timesyncd
      # off in one.
      services.timesyncd.enable = lib.mkForce true;
      systemd.services.systemd-timesyncd.wantedBy = lib.mkForce [ ];
    };

  testScript = ''
    import vm_pytest
    vm_pytest.run(globals(), "${./clock-vm-test.py}", values="${values}")
  '';
  extraPythonPackages = python: [ (pkgs.callPackage ./lib/vm-helpers.nix { } python) ];
  # nix fmt holds the tests and vm_pytest to mypy --strict against the
  # driver's own types, so the driver's check of these lines would add only
  # its time.
  skipTypeCheck = true;
}

# Sets a machine's clock on a network that names a time server of its own
# that never answers, as a hotel's did. The machine's own servers are a second
# machine running chrony, which it reaches; it has to get there, and not keep
# asking the network's. Nothing names the network's server until networkd has
# it, so timesyncd starts once it does: started sooner, it would find no server
# named and fall back to the machine's, which proves nothing.
{ self, pkgs }:
let
  silent = "192.168.1.99";
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
    start_all()
    server.wait_for_unit("chronyd.service")
    machine.wait_until_succeeds("networkctl status eth1 | grep -q 'NTP: ${silent}'", timeout=60)
    machine.succeed("systemctl start systemd-timesyncd.service")
    machine.succeed("timedatectl show-timesync -p LinkNTPServers --value | grep -qx ${silent}")
    machine.wait_until_succeeds("timedatectl show -p NTPSynchronized --value | grep -qx yes", timeout=120)
    machine.succeed("timedatectl show-timesync -p ServerName --value | grep -qx server")
  '';
}

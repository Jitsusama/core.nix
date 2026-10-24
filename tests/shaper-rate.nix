# Runs the shaper on a client whose line, at the router, is an 8 Mbit/s
# link 20 ms away with a second of buffer, as a cable modem's is, and loads
# it with the account's uploads. With the shaper, an app's ping stays near
# the line's own 20 ms; without it, the buffer fills and the ping waits a
# good part of a second. An app's upload takes the line from the jobs' while
# it runs, leaving them CAKE's sixteenth. When the line drops to 4 Mbit/s
# the shaper follows it down within seconds, and back up when it returns.
# It does so too when the uploads ride a second link to the same router, as
# Wi-Fi beside a cable does: the one line carries both links' traffic.
#
# The margins are wide, so a loaded host running the machines slower
# can't turn one into the other.
{ self, pkgs }:
let
  inherit (pkgs) lib;
  server = "192.168.2.3";
  line = rate: "tbf rate ${rate} burst 32kbit limit 1000000";
in
pkgs.testers.runNixOSTest {
  name = "shaper-rate";

  nodes.client = {
    imports = [
      self.nixosModules.slices
      self.nixosModules.shaper
    ];
    networking.useDHCP = false;
    networking.defaultGateway = {
      address = "192.168.1.2";
      interface = "eth1";
    };
    # A second link to the same router, as Wi-Fi is beside a cable: its own
    # address and a default route behind the first's. Naming an interface
    # empties vlans' default, so the first link's is named too. QEMU names
    # a card's hardware address by its network and machine alone, so the
    # second has its own, or the switch sends both links' frames to one.
    virtualisation.vlans = [ 1 ];
    virtualisation.interfaces.eth2.vlan = 1;
    networking.interfaces.eth2.macAddress = "52:54:00:12:01:11";
    networking.interfaces.eth2.ipv4 = {
      addresses = [
        {
          address = "192.168.1.11";
          prefixLength = 24;
        }
      ];
      routes = [
        {
          address = "0.0.0.0";
          prefixLength = 0;
          via = "192.168.1.2";
          options.metric = "600";
        }
      ];
    };
    # Each address answers on its own link, and the server's replies to the
    # second's may come in on either, so the reverse-path check is loose.
    boot.kernel.sysctl = {
      "net.ipv4.conf.all.arp_ignore" = 1;
      "net.ipv4.conf.all.arp_announce" = 2;
    };
    networking.firewall.checkReversePath = "loose";
    systemd.services.shaper.environment.SHAPER_HOSTS = server;
    users.users.alice = {
      isNormalUser = true;
      linger = true;
    };
    environment.systemPackages = [
      pkgs.jq
      pkgs.socat
    ];
  };

  nodes.router = {
    virtualisation.vlans = [
      1
      2
    ];
    networking.useDHCP = false;
    networking.firewall.enable = false;
    boot.kernel.sysctl."net.ipv4.ip_forward" = 1;
    # The line: 20 ms away, then 8 Mbit/s with a second of buffer.
    systemd.services.line = {
      wantedBy = [ "multi-user.target" ];
      after = [ "network-online.target" ];
      wants = [ "network-online.target" ];
      path = [ pkgs.iproute2 ];
      serviceConfig.Type = "oneshot";
      serviceConfig.RemainAfterExit = true;
      script = ''
        tc qdisc replace dev eth2 root handle 1: netem delay 20ms limit 100000
        tc qdisc replace dev eth2 parent 1: handle 2: ${line "8mbit"}
      '';
    };
  };

  nodes.server = {
    virtualisation.vlans = [ 2 ];
    networking.useDHCP = false;
    networking.firewall.enable = false;
    networking.interfaces.eth1.ipv4.routes = [
      {
        address = "192.168.1.0";
        prefixLength = 24;
        via = "192.168.2.2";
      }
    ];
    systemd.services.sink = {
      wantedBy = [ "multi-user.target" ];
      script = "exec ${lib.getExe pkgs.socat} -u TCP-LISTEN:5201,fork,reuseaddr OPEN:/dev/null";
    };
  };

  testScript = builtins.readFile ./shaper-rate-vm-test.py;
  # nix fmt holds the script to mypy --strict against the driver's own types,
  # through driver_names.pyi, which the driver's lighter check can't find.
  skipTypeCheck = true;
}

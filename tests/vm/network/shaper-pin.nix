# Runs the shaper on a client whose router sends UDP one of two ways to the
# server by its source port, as the line past a router can: ports 30030 and
# 30031 the near way, every other port a way 40 ms slower. They are the last
# two a round probes, so their probes' conntrack entries are the newest, and
# a pin that didn't wait them out would find both ports held. The server
# forwards to the VPN's far end, in a network namespace of its own, so the
# server is a hop past the split that answers, as on the real line. Once
# the shaper has probed the ports toward the VPN's end, the VPN's next
# socket leaves from one of the two near ports, and the one after it from
# the other, while the first's flow still holds its port. With both ports
# held, the pin lets go, and the VPN's next socket leaves from its own. The
# tests are shaper-pin-vm-test.py, run through pytest.
{ self, pkgs }:
import ../../lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "shaper-pin";

  nodes.client = {
    imports = [
      self.nixosModules.slices
      self.nixosModules.shaper
    ];
    virtualisation.vlans = [ 1 ];
    networking.useDHCP = false;
    networking.defaultGateway = {
      address = "192.168.1.2";
      interface = "eth1";
    };
    jitsusama.shaper.controller = "one-way";
    systemd.services.shaper.environment.SHAPER_HOSTS = "192.168.2.3";
    systemd.services.shaper-record.environment.SHAPER_REFLECTORS = "192.168.2.3";
    jitsusama.shaper.vpns = [ "wg0" ];
    # The client's VPN runs in userspace on a tun device, as a VPN's daemon
    # does. socat sends from a chosen port, to hold one toward the VPN's end.
    environment.systemPackages = [
      pkgs.socat
      pkgs.wireguard-go
      pkgs.wireguard-tools
    ];
  };

  nodes.router = {
    virtualisation.vlans = [
      1
      2
      4
    ];
    networking.useDHCP = false;
    networking.firewall.enable = false;
    boot.kernel.sysctl."net.ipv4.ip_forward" = 1;
    # The near way is eth2; the far way, eth3, holds each packet 40 ms.
    systemd.services.ways = {
      wantedBy = [ "multi-user.target" ];
      after = [ "network-online.target" ];
      wants = [ "network-online.target" ];
      path = [ pkgs.iproute2 ];
      serviceConfig.Type = "oneshot";
      serviceConfig.RemainAfterExit = true;
      script = ''
        tc qdisc replace dev eth3 root netem delay 40ms
        ip route replace 192.168.9.0/24 via 192.168.4.3 dev eth3
        ip route replace 192.168.9.0/24 via 192.168.2.3 dev eth2 table 100
        ip rule add ipproto udp sport 30030-30031 lookup 100
      '';
    };
  };

  nodes.server = {
    virtualisation.vlans = [
      2
      4
    ];
    networking.useDHCP = false;
    networking.firewall.enable = false;
    boot.kernel.sysctl = {
      "net.ipv4.ip_forward" = 1;
      # What comes the far way goes back the near one.
      "net.ipv4.conf.all.rp_filter" = 0;
      "net.ipv4.conf.default.rp_filter" = 0;
      "net.ipv4.conf.eth2.rp_filter" = 0;
    };
    networking.interfaces.eth1.ipv4.routes = [
      {
        address = "192.168.1.0";
        prefixLength = 24;
        via = "192.168.2.2";
      }
    ];
    environment.systemPackages = [ pkgs.wireguard-tools ];
    # The VPN's far end, a hop past the server.
    systemd.services.end = {
      wantedBy = [ "multi-user.target" ];
      after = [ "network-online.target" ];
      wants = [ "network-online.target" ];
      path = [ pkgs.iproute2 ];
      serviceConfig.Type = "oneshot";
      serviceConfig.RemainAfterExit = true;
      script = ''
        ip netns add end
        ip link add ve0 type veth peer name ve1 netns end
        ip addr add 192.168.9.1/24 dev ve0
        ip link set ve0 up
        ip -n end addr add 192.168.9.2/24 dev ve1
        ip -n end link set ve1 up
        ip -n end link set lo up
        ip -n end route add default via 192.168.9.1
      '';
    };
  };

  tests = ./shaper-pin-vm-test.py;
}

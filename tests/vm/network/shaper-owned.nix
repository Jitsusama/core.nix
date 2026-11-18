# Runs the shaper's one-way controller on a client whose router shares an
# 8 Mbit/s upload flow by flow under fq_codel, as the eero does, so no
# probe sees its queue however full it is. Beside 32 uploads from the
# account's jobs, an app's one upload gets most of the connection with the
# shaper, which takes the queue back under the router, and a sliver of it
# without, where the router gives it one share of 33. The tests are
# shaper-owned-vm-test.py, run through pytest.
{ self, pkgs }:
let
  inherit (pkgs) lib;
  # The server is past the router at a public address, a documentation
  # one, as the shaper counts only what waits to go past this network.
  server = "203.0.113.3";
  router = "203.0.113.2";
  public = address: {
    addresses = lib.mkForce [
      {
        inherit address;
        prefixLength = 24;
      }
    ];
  };
  sink = port: {
    wantedBy = [ "multi-user.target" ];
    script = "exec ${lib.getExe pkgs.socat} -u TCP-LISTEN:${toString port},fork,reuseaddr OPEN:/dev/null";
  };
in
import ../../lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "shaper-owned";

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
    systemd.services.shaper.environment.SHAPER_HOSTS = server;
    systemd.services.shaper-record.environment.SHAPER_REFLECTORS = server;
    users.users.alice = {
      isNormalUser = true;
      linger = true;
    };
  };

  nodes.router = {
    virtualisation.vlans = [
      1
      2
    ];
    networking.useDHCP = false;
    networking.firewall.enable = false;
    boot.kernel.sysctl."net.ipv4.ip_forward" = 1;
    networking.interfaces.eth2.ipv4 = public router;
    # The connection: 8 Mbit/s, shared flow by flow.
    systemd.services.connection = {
      wantedBy = [ "multi-user.target" ];
      after = [ "network-online.target" ];
      wants = [ "network-online.target" ];
      path = [ pkgs.iproute2 ];
      serviceConfig.Type = "oneshot";
      serviceConfig.RemainAfterExit = true;
      script = ''
        tc qdisc replace dev eth2 root handle 1: htb default 1
        tc class replace dev eth2 parent 1: classid 1:1 htb rate 8mbit
        tc qdisc replace dev eth2 parent 1:1 handle 10: fq_codel
      '';
    };
  };

  nodes.server = {
    virtualisation.vlans = [ 2 ];
    # nftables counts what each port takes.
    environment.systemPackages = [ pkgs.nftables ];
    networking.useDHCP = false;
    networking.firewall.enable = false;
    networking.interfaces.eth1.ipv4 = public server // {
      routes = [
        {
          address = "192.168.1.0";
          prefixLength = 24;
          via = router;
        }
      ];
    };
    # The jobs' uploads go to 5201, the app's to 5202.
    systemd.services.jobs-sink = sink 5201;
    systemd.services.app-sink = sink 5202;
  };

  tests = ./shaper-owned-vm-test.py;
}

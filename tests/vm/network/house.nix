# Runs the shaper on a client whose network names a house: another machine
# on the same line, which says each second what it sends up. The other
# machine runs house-report as a system service, as a Mac does under
# launchd, naming no interface, so it counts the one its route to the
# client leaves by, and sends 3 Mbit/s across the home network. That upload
# never reaches the connection, so the client's own reading of the line
# can't hold it there: only the reports can. While they come, the client's
# upload is held under what the house's 8 Mbit/s leaves, and still once the
# other machine's route moves to another of its addresses, as a Mac's does
# to its cable's; once they stop, it climbs back to the connection. Both
# machines hold one key, so only signed reports are taken: with the true
# reporter stopped, reports forged from the house's own address under
# another key, in step and saying what it sends, hold nothing, and the
# upload climbs as it does once the house goes quiet. The tests are
# house-vm-test.py, run through pytest.
{ self, pkgs }:
let
  inherit (pkgs) lib;
  # The server is past the router at a public address, a documentation
  # one, as the shaper counts only what waits to go past this network.
  server = "203.0.113.3";
  router = "203.0.113.2";
  # The house's key, a test's alone, 32 bytes as one is made, put in /etc
  # as root's file rather than named as a store path.
  key = "a key for the house test, 32 b!!";
  keyFile = "/etc/house.key";
  keyed = {
    environment.etc."house.key" = {
      text = key;
      mode = "0400";
    };
  };
  public = address: {
    addresses = lib.mkForce [
      {
        inherit address;
        prefixLength = 24;
      }
    ];
  };
in
import ../../lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "house";

  nodes.client = {
    imports = [
      self.nixosModules.slices
      self.nixosModules.shaper
      keyed
    ];
    virtualisation.vlans = [ 1 ];
    networking.useDHCP = false;
    networking.defaultGateway = {
      address = "192.168.1.3";
      interface = "eth1";
    };
    systemd.services.shaper.environment.SHAPER_HOSTS = server;
    systemd.services.shaper-record.environment.SHAPER_REFLECTORS = server;
    jitsusama.shaper.controller = "one-way";
    # The router's hardware address, as QEMU gives the third machine by
    # name its card on the first network, names the network; the house is
    # the second's, at the address the test network gives it, or the one
    # it moves to.
    jitsusama.shaper.networks."52:54:00:12:01:03" = {
      up = 8000;
      house = {
        most = 8000;
        inherit keyFile;
        from = [
          "192.168.1.2"
          "192.168.1.12"
        ];
      };
    };
    users.users.alice = {
      isNormalUser = true;
      linger = true;
    };
    environment.systemPackages = [ pkgs.jq ];
  };

  # The house's other machine and its reporter, and socat and openssl to
  # forge reports with.
  nodes.house = {
    imports = [
      self.nixosModules.house-report
      keyed
    ];
    jitsusama.house-report.keyFile = keyFile;
    environment.systemPackages = [
      pkgs.openssl
      pkgs.socat
    ];
    virtualisation.vlans = [ 1 ];
    networking.useDHCP = false;
    networking.firewall.enable = false;
    jitsusama.house-report.to = "192.168.1.1";
    # Its upload, held to 3 Mbit/s with a short queue, so a report waits
    # behind it no more than 50 ms.
    systemd.services.upload = {
      after = [ "network-online.target" ];
      wants = [ "network-online.target" ];
      path = [ pkgs.iproute2 ];
      serviceConfig.ExecStartPre =
        "${lib.getExe' pkgs.iproute2 "tc"} qdisc replace dev eth1 root tbf rate 3mbit"
        + " burst 32kbit latency 50ms";
      script = "while true; do cat /dev/zero > /dev/tcp/192.168.1.3/5202; done";
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
    # The connection: 20 ms away, then 8 Mbit/s with a second of buffer.
    systemd.services.connection = {
      wantedBy = [ "multi-user.target" ];
      after = [ "network-online.target" ];
      wants = [ "network-online.target" ];
      path = [ pkgs.iproute2 ];
      serviceConfig.Type = "oneshot";
      serviceConfig.RemainAfterExit = true;
      script = ''
        tc qdisc replace dev eth2 root handle 1: netem delay 20ms limit 100000
        tc qdisc replace dev eth2 parent 1: handle 2: tbf rate 8mbit burst 32kbit limit 1000000
      '';
    };
    # Where the house's upload goes, on the home network itself.
    systemd.services.sink = {
      wantedBy = [ "multi-user.target" ];
      script = "exec ${lib.getExe pkgs.socat} -u TCP-LISTEN:5202,fork,reuseaddr OPEN:/dev/null";
    };
  };

  nodes.server = {
    virtualisation.vlans = [ 2 ];
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
    systemd.services.sink = {
      wantedBy = [ "multi-user.target" ];
      script = "exec ${lib.getExe pkgs.socat} -u TCP-LISTEN:5201,fork,reuseaddr OPEN:/dev/null";
    };
  };

  tests = ./house-vm-test.py;
}

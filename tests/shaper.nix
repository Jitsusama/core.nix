# Runs the shaper on a client whose line goes through a router to a server,
# and reads from CAKE's own counts which tin each kind of traffic went to,
# both ways: the account's jobs, the work Joel waits on at a terminal, the
# system's builds, a guest it forwards for, an app's page and its call,
# Tuple's daemon, names and the shaper's pings. Traffic to the router,
# on the client's own network, goes by unslowed. When the default route
# moves to the client's other link, the queues move with it, and when the
# shaper stops, it takes them all away. Inside a WireGuard tunnel to the
# server, named as a machine names its VPN's, the same classes reach the
# tunnel's own CAKE. The script is shaper-vm-test.py.
{ self, pkgs }:
let
  inherit (pkgs) lib;
  server = "192.168.2.3";
in
pkgs.testers.runNixOSTest {
  name = "shaper";

  nodes.client = {
    imports = [
      self.nixosModules.slices
      self.nixosModules.shaper
    ];
    virtualisation.vlans = [
      1
      3
    ];
    networking.useDHCP = false;
    networking.defaultGateway = {
      address = "192.168.1.2";
      interface = "eth1";
    };
    # The test's only distant host is the server.
    systemd.services.shaper.environment.SHAPER_HOSTS = server;
    jitsusama.shaper.tunnels = [ "wg0" ];
    users.users.alice = {
      isNormalUser = true;
      linger = true;
    };
    environment.systemPackages = [
      pkgs.jq
      pkgs.wireguard-tools
    ];
  };

  nodes.router = {
    virtualisation.vlans = [
      1
      2
      3
    ];
    networking.useDHCP = false;
    networking.firewall.enable = false;
    boot.kernel.sysctl."net.ipv4.ip_forward" = 1;
  };

  nodes.server = {
    virtualisation.vlans = [ 2 ];
    environment.systemPackages = [ pkgs.wireguard-tools ];
    networking.useDHCP = false;
    networking.firewall.enable = false;
    networking.interfaces.eth1.ipv4.routes = [
      {
        address = "192.168.1.0";
        prefixLength = 24;
        via = "192.168.2.2";
      }
      {
        address = "192.168.3.0";
        prefixLength = 24;
        via = "192.168.2.2";
      }
    ];
    # Takes what's sent to 5201, and sends 10 MB to whoever connects to 5202.
    systemd.services.sink = {
      wantedBy = [ "multi-user.target" ];
      script = "exec ${lib.getExe pkgs.socat} -u TCP-LISTEN:5201,fork,reuseaddr OPEN:/dev/null";
    };
    systemd.services.source = {
      wantedBy = [ "multi-user.target" ];
      # The listener comes first, so each connection forks its own head;
      # socat opens its first address first, and with the command there
      # one head served the first download and every later one read EOF.
      script = ''
        exec ${lib.getExe pkgs.socat} -U TCP-LISTEN:5202,fork,reuseaddr \
          'EXEC:${pkgs.coreutils}/bin/head -c 10000000 /dev/zero'
      '';
    };
  };

  testScript = builtins.readFile ./shaper-vm-test.py;
  # nix fmt holds the script to mypy --strict against the driver's own types,
  # through driver_names.pyi, which the driver's lighter check can't find.
  skipTypeCheck = true;
}

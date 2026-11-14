# Runs the shaper's one-way controller on a client whose router shares a
# 90 Mbit/s download and a 15 Mbit/s upload flow by flow under fq_codel,
# as the eero does, and fills both: the account's jobs download and upload
# without end, and an app downloads beside them, as a page does. Meanwhile
# an app holds two calls both ways, one to a media server's port, which the
# shaper puts in the video tin, and one to 443, which it takes for a page's
# QUIC and sends best effort. While a call is on, the call to 443 goes in
# the video tin too. Each call loses at most a hundredth of its datagrams
# coming down, a bar the router meets alone with the shaper stopped. The
# tests are shaper-call-vm-test.py, run through pytest.
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
  serve = script: {
    wantedBy = [ "multi-user.target" ];
    inherit script;
  };
  # One iperf3 server per call, as each serves one test at a time.
  call = port: serve "exec ${lib.getExe pkgs.iperf3} --server --port ${toString port}";
  # Stands in for a-call-is-on, which reads PipeWire, which the client
  # hasn't got: a call is on while /run/call-is-on is there.
  callIsOn = pkgs.writeShellScriptBin "a-call-is-on" "test -e /run/call-is-on";
in
import ../../lib/pytest-vm-test.nix { inherit pkgs; } {
  name = "shaper-call";

  nodes.client = {
    imports = [
      self.nixosModules.slices
      self.nixosModules.shaper
    ];
    virtualisation.vlans = [ 1 ];
    # A full connection both ways is a hundred Mbit/s through CAKE, an IFB
    # and nftables: on one core the machine falls behind and drops what
    # arrives before any queue sees it, which a laptop never does.
    virtualisation.cores = 4;
    networking.useDHCP = false;
    networking.defaultGateway = {
      address = "192.168.1.2";
      interface = "eth1";
    };
    jitsusama.shaper.controller = "one-way";
    systemd.services.shaper.environment.SHAPER_HOSTS = server;
    systemd.services.shaper.path = lib.mkBefore [ callIsOn ];
    systemd.services.shaper-record.environment.SHAPER_REFLECTORS = server;
    users.users.alice = {
      isNormalUser = true;
      linger = true;
    };
    environment.systemPackages = [ pkgs.iperf3 ];
  };

  nodes.router = {
    virtualisation.vlans = [
      1
      2
    ];
    virtualisation.cores = 2;
    networking.useDHCP = false;
    networking.firewall.enable = false;
    boot.kernel.sysctl."net.ipv4.ip_forward" = 1;
    networking.interfaces.eth2.ipv4 = public router;
    # The connection: 90 Mbit/s down to the client on eth1, 15 up to the
    # server on eth2, each shared flow by flow.
    systemd.services.connection = {
      wantedBy = [ "multi-user.target" ];
      after = [ "network-online.target" ];
      wants = [ "network-online.target" ];
      path = [ pkgs.iproute2 ];
      serviceConfig.Type = "oneshot";
      serviceConfig.RemainAfterExit = true;
      script = ''
        line() {
          tc qdisc replace dev "$1" root handle 1: htb default 1
          tc class replace dev "$1" parent 1: classid 1:1 htb rate "$2"
          tc qdisc replace dev "$1" parent 1:1 handle 10: fq_codel
        }
        line eth1 90mbit
        line eth2 15mbit
      '';
    };
  };

  nodes.server = {
    virtualisation.vlans = [ 2 ];
    virtualisation.cores = 2;
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
    # Uploads go to 5201; whoever connects to 5202 takes zeros until they
    # hang up; and the calls are served on 5301 and 443.
    systemd.services.sink = serve ''
      exec ${lib.getExe pkgs.socat} -u TCP-LISTEN:5201,fork,reuseaddr OPEN:/dev/null
    '';
    systemd.services.source = serve ''
      exec ${lib.getExe pkgs.socat} -U TCP-LISTEN:5202,fork,reuseaddr \
        'EXEC:${pkgs.coreutils}/bin/cat /dev/zero'
    '';
    systemd.services.media-call = call 5301;
    systemd.services.page-call = call 443;
  };

  tests = ./shaper-call-vm-test.py;
}

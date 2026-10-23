# Runs the shaper on a client whose line, at the router, is an 8 Mbit/s
# link 20 ms away with a second of buffer, as a cable modem's is, and loads
# it with the account's uploads. With the shaper, an app's ping stays near
# the line's own 20 ms; without it, the buffer fills and the ping waits a
# good part of a second. When the line drops to 4 Mbit/s the shaper follows
# it down within seconds, and back up when it returns.
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
    systemd.services.shaper.environment.SHAPER_HOSTS = server;
    users.users.alice = {
      isNormalUser = true;
      linger = true;
    };
    environment.systemPackages = [ pkgs.jq ];
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

  testScript = ''
    import json, statistics

    start_all()
    server.wait_for_open_port(5201)
    router.wait_for_unit("line.service")
    client.wait_for_unit("user@1000.service")
    client.wait_until_succeeds("tc -j qdisc show dev eth1 | grep -q cake", timeout=30)

    def up():
        status = json.loads(client.succeed("cat /run/shaper/status.json"))
        return next(l for l in status if l["name"] == "eth1")["up"]

    def ping(name):
        """An app's pings to the server: the median round trip, in ms."""
        client.succeed(
            f"systemd-run -M alice@ --user --wait --collect --slice=app.slice"
            f" -p StandardOutput=file:/tmp/{name}.txt"
            f" ${pkgs.iputils}/bin/ping -n -c 40 -i 0.25 ${server}"
        )
        text = client.succeed(f"cat /tmp/{name}.txt")
        times = [float(w[5:]) for w in text.split() if w.startswith("time=")]
        t.assertGreater(len(times), 20, text)
        return statistics.median(times)

    # Four uploads from the account's jobs, for as long as the test runs.
    client.succeed(
        "systemd-run -M alice@ --user --slice=jobs.slice --unit=uploads"
        " ${pkgs.bash}/bin/bash -c 'for i in 1 2 3 4; do"
        " (while true; do cat /dev/zero > /dev/tcp/${server}/5201; done) & done; wait'"
    )

    with subtest("the shaper finds the line and an app's pings stay near it"):
        client.wait_until_succeeds(
            "jq -e '.[] | select(.name == \"eth1\") | .up < 8000' /run/shaper/status.json",
            timeout=60,
        )
        client.sleep(15)
        # It probes a little past the line and cuts at the queue that
        # builds, so any moment finds it within a tenth of the line.
        rate = up()
        t.assertGreater(rate, 5000)
        t.assertLess(rate, 8800)
        shaped = ping("shaped")
        t.assertLess(shaped, 80, f"shaped at {rate} kbit/s")

    with subtest("when the line drops, the shaper follows it down"):
        router.succeed("tc qdisc change dev eth2 parent 1: handle 2: ${line "4mbit"}")
        client.wait_until_succeeds(
            "jq -e '.[] | select(.name == \"eth1\") | .up < 4000' /run/shaper/status.json",
            timeout=30,
        )
        client.sleep(10)
        t.assertLess(ping("dropped"), 100)

    with subtest("and back up when it returns"):
        router.succeed("tc qdisc change dev eth2 parent 1: handle 2: ${line "8mbit"}")
        client.wait_until_succeeds(
            "jq -e '.[] | select(.name == \"eth1\") | .up > 6000' /run/shaper/status.json",
            timeout=120,
        )

    with subtest("without the shaper, the line's buffer fills"):
        client.succeed("systemctl stop shaper.service")
        client.sleep(10)
        unshaped = ping("unshaped")
        t.assertGreater(unshaped, 300)
        print(f"an app's median round trip: {shaped} ms shaped, {unshaped} ms not")
  '';
}

# Runs the shaper on a client whose line goes through a router to a server,
# and reads from CAKE's own counts which tin each kind of traffic went to,
# both ways: the account's jobs, the system's builds, an app's page and its
# call, Tuple's daemon, names and the shaper's pings. Traffic to the router,
# on the client's own network, goes by unslowed. When the default route
# moves to the client's other link, the queues move with it, and when the
# shaper stops, it takes them all away. Inside a WireGuard tunnel to the
# server, named as a machine names its VPN's, the same classes reach the
# tunnel's own CAKE.
#
# The counts are what CAKE says it sent, so a class that never reached its
# packets, a set left empty or a rule that matched the wrong depth, shows
# as packets in the wrong tin.
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
    # Takes what's sent to 5201, and sends 10 MB to whoever connects to 5202:
    # arriving packets reach the IFB merged by GRO, often 13 KB each, so a
    # download needs that much to count as many packets as an upload.
    systemd.services.sink = {
      wantedBy = [ "multi-user.target" ];
      script = "exec ${lib.getExe pkgs.socat} -u TCP-LISTEN:5201,fork,reuseaddr OPEN:/dev/null";
    };
    systemd.services.source = {
      wantedBy = [ "multi-user.target" ];
      script = ''
        exec ${lib.getExe pkgs.socat} -u \
          'EXEC:${pkgs.coreutils}/bin/head -c 10000000 /dev/zero' TCP-LISTEN:5202,fork,reuseaddr
      '';
    };
  };

  testScript = ''
    import json

    bash = "${pkgs.bash}/bin/bash"
    tins = ["bulk", "best effort", "video", "voice"]

    start_all()
    server.wait_for_unit("sink.service")
    server.wait_for_unit("source.service")
    server.wait_for_open_port(5201)
    server.wait_for_open_port(5202)
    client.wait_for_unit("user@1000.service")
    client.wait_for_unit("shaper.service")

    def qdiscs(dev):
        return json.loads(client.succeed(f"tc -s -j qdisc show dev {dev}"))

    def counts(dev):
        cake = next(q for q in qdiscs(dev) if q["kind"] == "cake")
        return [t["sent_packets"] for t in cake["tins"]]

    def ifb(link):
        index = int(client.succeed(f"cat /sys/class/net/{link}/ifindex"))
        return f"shaper{index:x}"

    def shaped(link):
        kinds = {(q["kind"], q.get("parent", "root")) for q in qdiscs(link)}
        return ("htb", "root") in kinds and ("cake", "1:2") in kinds

    client.wait_until_succeeds("tc -j qdisc show dev eth1 | grep -q cake", timeout=30)
    client.wait_until_succeeds("nft list set inet shaper uplinks | grep -q eth1", timeout=30)

    # Each slice is held open, as something always runs in them on a
    # machine in use; a slice made afresh is the subtest further on.
    def hold(slice, system=False):
        where = "" if system else "-M alice@ --user"
        unit = "hold-" + slice.removesuffix(".slice")
        client.succeed(
            f"systemctl {where} is-active {unit}.service"
            f" || systemd-run {where} --slice={slice} --unit={unit}"
            " ${pkgs.coreutils}/bin/sleep infinity"
        )

    for slice in ["jobs.slice", "background.slice", "app.slice"]:
        hold(slice)
    hold("build.slice", system=True)
    for name, slice in [("bulk4", "background.slice"), ("bulk1", "build.slice")]:
        client.wait_until_succeeds(
            f"nft list set inet shaper {name} | grep -q {slice}", timeout=10
        )

    def run(where, script):
        """Runs a bash script in a slice: an account's, or the system's."""
        if where.startswith("system:"):
            slice = where.removeprefix("system:")
            client.succeed(f"systemd-run --wait --collect --slice={slice} {bash} -c '{script}'")
        else:
            unit = f"--unit={where.removeprefix('unit:')}" if where.startswith("unit:") else ""
            slice = "app.slice" if unit else where
            client.succeed(
                f"systemd-run -M alice@ --user --wait --collect --slice={slice} {unit}"
                f" {bash} -c '{script}'"
            )

    upload = "head -c 1000000 /dev/zero > /dev/tcp/${server}/5201"
    download = "exec 3<>/dev/tcp/${server}/5202; cat <&3 > /dev/null"
    def datagrams(port, n=300):
        return f"for i in {{1..{n}}}; do echo x > /dev/udp/${server}/{port}; done"

    def lands(tin, where, script, dev="eth1", n=300):
        before = counts(dev)
        run(where, script)
        after = counts(dev)
        moved = dict(zip(tins, (a - b for a, b in zip(after, before))))
        t.assertGreaterEqual(moved[tin], n, f"{where} on {dev}: {moved}")
        for other in tins:
            # The shaper's own pings go on into voice meanwhile.
            if other not in (tin, "voice"):
                t.assertLess(moved[other], n // 4, f"{where} on {dev}: {moved}")

    with subtest("the account's work goes in bulk, both ways"):
        lands("bulk", "jobs.slice", upload)
        lands("bulk", "jobs.slice", download, dev=ifb("eth1"))
        lands("bulk", "background.slice", datagrams(9000))

    with subtest("the system's builds go in bulk"):
        lands("bulk", "system:build.slice", upload)

    with subtest("an app's pages are best effort, and its calls video"):
        lands("best effort", "app.slice", upload)
        lands("best effort", "app.slice", download, dev=ifb("eth1"))
        lands("best effort", "app.slice", datagrams(443))
        lands("video", "app.slice", datagrams(9000))

    with subtest("everything Tuple's daemon sends is video"):
        # The daemon runs well before a call, so its unit's set knows it.
        lands("video", "unit:tuple", f"sleep 1; {upload}")

    with subtest("names and time go first, from anything"):
        lands("voice", "jobs.slice", datagrams(53))

    with subtest("the shaper's pings go first, so they measure the line"):
        before = counts("eth1")[3]
        client.sleep(3)
        t.assertGreater(counts("eth1")[3] - before, 5)

    with subtest("a slice made again keeps its class"):
        account = "/sys/fs/cgroup/user.slice/user-1000.slice/user@1000.service"
        client.succeed("systemctl -M alice@ --user stop background.slice")
        client.wait_until_fails(f"test -d {account}/background.slice")
        hold("background.slice")
        client.sleep(1)
        lands("bulk", "background.slice", datagrams(9000))

    with subtest("traffic to the client's own network isn't slowed"):
        before = counts("eth1")
        run("jobs.slice", "for i in {1..1000}; do echo x > /dev/udp/192.168.1.2/9000; done")
        t.assertLess(sum(counts("eth1")) - sum(before), 100)
        local = next(q for q in qdiscs("eth1") if q.get("parent") == "1:1")
        t.assertGreater(local["packets"], 500)

    with subtest("inside a tunnel, the same classes reach its own queue"):
        keys = {}
        for node in (client, server):
            node.succeed("wg genkey > /root/key")
            keys[node.name] = node.succeed("wg pubkey < /root/key").strip()
        for node, me, peer, there in (
            (client, "10.9.0.1", server, "${server}"),
            (server, "10.9.0.2", client, "192.168.1.1"),
        ):
            node.succeed(
                "ip link add wg0 type wireguard",
                f"wg set wg0 private-key /root/key listen-port 51820 peer {keys[peer.name]}"
                f" endpoint {there}:51820 allowed-ips 10.9.0.0/24",
                f"ip addr add {me}/24 dev wg0",
                "ip link set wg0 up",
            )
        client.wait_until_succeeds("ping -c1 -W1 10.9.0.2", timeout=30)
        client.wait_until_succeeds("tc -j qdisc show dev wg0 | grep -q cake", timeout=15)
        client.wait_until_succeeds("nft list set inet shaper uplinks | grep -q wg0", timeout=15)
        inner = "head -c 1000000 /dev/zero > /dev/tcp/10.9.0.2/5201"
        lands("bulk", "jobs.slice", inner, dev="wg0")
        lands("best effort", "app.slice", inner, dev="wg0")

    with subtest("the queues follow the default route to another link"):
        client.succeed("ip route replace default via 192.168.3.2 dev eth2")
        client.wait_until_succeeds("tc -j qdisc show dev eth2 | grep -q cake", timeout=15)
        client.wait_until_succeeds("! tc -j qdisc show dev eth1 | grep -q cake", timeout=15)
        t.assertFalse(shaped("eth1"))
        client.succeed(f"! ip link show {ifb('eth1')}")
        client.wait_until_succeeds("nft list set inet shaper uplinks | grep -q eth2", timeout=15)
        lands("bulk", "jobs.slice", upload, dev="eth2")

    with subtest("its status says what it holds the link to"):
        status = client.succeed("shaper status")
        t.assertIn("eth2", status)
        t.assertIn("held under load", status)

    with subtest("stopping it leaves the links as they were"):
        eth2_ifb = ifb("eth2")
        client.succeed("systemctl stop shaper.service")
        t.assertFalse(shaped("eth2"))
        client.succeed(f"! ip link show {eth2_ifb}")
        client.succeed("! tc -j qdisc show dev eth2 | grep -q ingress")
        client.succeed("! tc -j qdisc show dev wg0 | grep -q cake")
  '';
}

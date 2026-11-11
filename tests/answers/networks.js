// The cases of networks for the launcher's answers, which ../answers.qml runs
// with every other place's; its opening comment says how.
.import "common.js" as Common
.import "../Numbers.js" as Numbers
.import "../Traffic.js" as Traffic
.import "../Interfaces.js" as Interfaces
.import "../Networkd.js" as Networkd

function run(failures, fromRoot) {
    let asked = 0;
    const Maths = fromRoot.maths;
    const quoted = Common.quoted;
    const machine = Common.checker(failures, () => asked++);
    // The network's figures, from /proc/net/dev and the shaper's file.
    const netdev = (wifi, warp) => "Inter-|   Receive |  Transmit\n"
        + " face |bytes    packets errs drop fifo frame compressed multicast|bytes ...\n"
        + "    lo: 500 5 0 0 0 0 0 0 500 5 0 0 0 0 0 0\n"
        + `wlp0s20f3: ${wifi[0]} 10 0 1 0 0 0 0 ${wifi[1]} 10 0 2 0 0 0 0\n`
        + `tailscale0: ${warp[0]} 4 0 0 0 0 0 0 ${warp[1]} 4 0 0 0 0 0 0\n`
        + "shaper2: 9000 9 0 0 0 0 0 0 0 0 0 0 0 0 0 0\n";
    const counted = Traffic.counters(netdev([1000, 200], [400, 100]));
    machine("an interface's counters", counted.wlp0s20f3, { in: 1000, out: 200, dropped: 3 });
    machine("every interface listed", Object.keys(counted),
        ["lo", "wlp0s20f3", "tailscale0", "shaper2"]);
    machine("what isn't /proc/net/dev", Traffic.counters("nothing here\n"), {});
    const cardKinds = { lo: "", wlp0s20f3: "wifi", tailscale0: "tunnel", shaper2: "" };
    const named = all => {
        const out = {};
        for (const name of Object.keys(all))
            out[name] = { kind: cardKinds[name], counters: all[name] };
        return out;
    };
    const recounted = Traffic.counters(netdev([3000, 600], [1400, 300]));
    const moved = Traffic.rates(named(recounted), named(counted), 2);
    machine("the cards' rates, without the tunnel over them",
        [moved.in, moved.out, moved.dropped], [1000, 200, 0]);
    machine("a tunnel's rates, listed apart",
        moved.each.find(one => one.name === "tailscale0").in, 500);
    machine("the loopback and mirrors left out", moved.each.map(one => one.name),
        ["wlp0s20f3", "tailscale0"]);
    const reset = Traffic.counters(netdev([100, 50], [0, 0]));
    machine("a counter that went back starts from nothing",
        Traffic.rates(named(reset), named(counted), 1).in, 100);
    machine("since the start, with no read before",
        Traffic.rates(named(counted), null, 10).in, 100);
    const number = (value, places) => value.toFixed(places);
    machine("a rate in megabits", Traffic.bits(1500000, number), "12 Mb/s");
    machine("a slow rate in kilobits", Traffic.bits(500, number), "4.0 kb/s");
    machine("nothing moving", Traffic.bits(0, number), "0 b/s");
    const status = [{
        name: "wlp0s20f3", ifb: "shaper2", up: 11830, down: 40000, state: "climbing",
        baselines: [["9.9.9.9", 51.1], ["1.1.1.1", 21.1], ["8.8.8.8", 20.6]], follows: null
    }];
    const link = Traffic.line(status, "wlp0s20f3");
    machine("what the line carries, in bytes a second", [link.in, link.out], [5000000, 1478750]);
    machine("the quickest server first", link.trips.map(trip => trip.server),
        ["8.8.8.8", "1.1.1.1", "9.9.9.9"]);
    machine("a line not shaped", Traffic.line(status, "enp1s0"), null);
    machine("no shaper at all", Traffic.line({}, "wlp0s20f3"), null);
    machine("the shaper's mirrors", Traffic.mirrors(status), ["shaper2"]);
    machine("the middle round trip", Traffic.typical(link.trips), 21.1);
    machine("the middle of an even count", Traffic.typical([{ ms: 10 }, { ms: 20 }]), 15);
    machine("no round trips", Traffic.typical([]), null);

    // The machine's connections, from ip, the routing tables and
    // resolv.conf, each by what it is.
    const networkListed = [{
        ifname: "lo", flags: ["LOOPBACK", "UP", "LOWER_UP"], mtu: 65536,
        link_type: "loopback", address: "00:00:00:00:00:00",
        addr_info: [{ family: "inet", local: "127.0.0.1", prefixlen: 8, scope: "host" }]
    }, {
        ifname: "wlp0s20f3", flags: ["BROADCAST", "MULTICAST", "UP", "LOWER_UP"], mtu: 1500,
        link_type: "ether", address: "02:00:00:00:01:00", parentbus: "pci",
        addr_info: [
            { family: "inet", local: "192.168.4.48", prefixlen: 22, scope: "global" },
            { family: "inet6", local: "fe80::ff:fe00:100", prefixlen: 64,
                scope: "link" }
        ]
    }, {
        ifname: "enp1s0", flags: ["NO-CARRIER", "BROADCAST", "UP"], mtu: 1500,
        link_type: "ether", address: "3c:22:fb:12:34:56", permaddr: "3c:22:fb:00:00:01",
        parentbus: "pci", addr_info: []
    }, {
        ifname: "virbr0", flags: ["NO-CARRIER", "BROADCAST", "MULTICAST", "UP"], mtu: 1500,
        link_type: "ether", address: "52:54:00:62:19:d2", linkinfo: { info_kind: "bridge" },
        addr_info: [{ family: "inet", local: "192.168.122.1", prefixlen: 24, scope: "global" }]
    }, {
        ifname: "tailscale0", flags: ["POINTOPOINT", "UP", "LOWER_UP"], mtu: 1280,
        link_type: "none", linkinfo: { info_kind: "tun" },
        addr_info: [
            { family: "inet", local: "100.101.102.103", prefixlen: 32, scope: "global" },
            { family: "inet6", local: "fd7a:115c:a1e0::1", prefixlen: 128, scope: "global" }
        ]
    }, {
        ifname: "ifb4wlp0s20f3", flags: ["UP"], mtu: 1500, link_type: "ether",
        linkinfo: { info_kind: "ifb" }, addr_info: []
    }, {
        ifname: "veth0", flags: ["UP"], mtu: 1500, link_type: "ether",
        linkinfo: { info_kind: "veth" }, addr_info: []
    }];
    const networkLinks = Interfaces.interfaces(networkListed, ["wlp0s20f3"]);
    const networkKinds = {};
    for (const each of networkLinks)
        networkKinds[each.name] = each.kind;
    machine("each connection by what it is", networkKinds, {
        wlp0s20f3: "wifi", enp1s0: "wired", virbr0: "bridge", tailscale0: "tunnel",
        veth0: "virtual"
    });
    // The network's speed names each the same from sysfs, as it reads it
    // without spawning ip, for these and the tunnels it couldn't tell
    // apart before: a WireGuard one, a VXLAN one, and a GRE one, which
    // sysfs names only by its type.
    const sturdySys = {
        lo: { uevent: "INTERFACE=lo\nIFINDEX=1\n", type: "772" },
        wlp0s20f3: { uevent: "DEVTYPE=wlan\nINTERFACE=wlp0s20f3\n", type: "1", card: true },
        enp1s0: { uevent: "INTERFACE=enp1s0\n", type: "1", card: true },
        virbr0: { uevent: "DEVTYPE=bridge\nINTERFACE=virbr0\n", type: "1" },
        tailscale0: { uevent: "INTERFACE=tailscale0\n", type: "65534", tun: true },
        ifb4wlp0s20f3: { uevent: "INTERFACE=ifb4wlp0s20f3\n", type: "1" },
        veth0: { uevent: "INTERFACE=veth0\n", type: "1" },
        wg0: { uevent: "DEVTYPE=wireguard\nINTERFACE=wg0\n", type: "65534" },
        vxlan0: { uevent: "DEVTYPE=vxlan\nINTERFACE=vxlan0\n", type: "1" },
        gre1: { uevent: "INTERFACE=gre1\n", type: "778" }
    };
    const sturdyIp = networkListed.concat([
        { ifname: "wg0", link_type: "none", linkinfo: { info_kind: "wireguard" } },
        { ifname: "vxlan0", link_type: "ether", linkinfo: { info_kind: "vxlan" } },
        { ifname: "gre1", link_type: "gre", linkinfo: { info_kind: "gre" } }
    ]);
    const sturdyKinds = (named) => {
        const out = {};
        for (const link of sturdyIp)
            out[link.ifname] = named(link);
        return out;
    };
    const sturdyAddresses = sturdyKinds(link => Interfaces.kind(link,
        link.ifname === "wlp0s20f3"));
    machine("the tunnels by what they are", [sturdyAddresses.wg0, sturdyAddresses.vxlan0,
        sturdyAddresses.gre1], ["tunnel", "tunnel", "tunnel"]);
    machine("each connection named alike from sysfs",
        sturdyKinds(link => Interfaces.sysKind(link.ifname, sturdySys[link.ifname])),
        sturdyAddresses);
    const networkLink = name => networkLinks.find(each => each.name === name);
    machine("a card's address, past its own link only", networkLink("wlp0s20f3").four,
        [{ address: "192.168.4.48", prefix: 22 }]);
    machine("no link-local IPv6 address", networkLink("wlp0s20f3").six, []);
    machine("a tunnel's IPv6 address", networkLink("tailscale0").six[0].address,
        "fd7a:115c:a1e0::1");
    machine("a tunnel has no hardware address", networkLink("tailscale0").mac, "");
    machine("a card on its own address", networkLink("wlp0s20f3").own, "");
    machine("a card on another address than its own", networkLink("enp1s0").own,
        "3c:22:fb:00:00:01");
    machine("how each stands", ["wlp0s20f3", "enp1s0"].map(name => networkLink(name).state),
        ["up", "no link"]);
    machine("a card turned off", Interfaces.state({ flags: ["BROADCAST"] }), "off");
    machine("what isn't ip's", Interfaces.interfaces({}, []), []);
    machine("where a hardware address came from", ["0", "1", "3", ""].map(Interfaces.assigned),
        ["own", "random", "set", ""]);
    const networkRoute = "Iface\tDestination\tGateway \tFlags\tRefCnt\tUse\tMetric\tMask\t"
        + "\tMTU\tWindow\tIRTT\n"
        + "wlp0s20f3\t00000000\t0104A8C0\t0003\t0\t0\t600\t00000000\t0\t0\t0\n"
        + "enp1s0\t00000000\t0100000A\t0003\t0\t0\t100\t00000000\t0\t0\t0\n"
        + "wlp0s20f3\t0004A8C0\t00000000\t0001\t0\t0\t600\t00FCFFFF\t0\t0\t0\n";
    const networkGateways = Interfaces.gateways(networkRoute);
    machine("each default gateway", networkGateways, {
        wlp0s20f3: { address: "192.168.4.1", metric: 600 },
        enp1s0: { address: "10.0.0.1", metric: 100 }
    });
    machine("the way out, costing least", Interfaces.wayOut(networkGateways), "enp1s0");
    machine("no way out", Interfaces.wayOut({}), "");
    const networkZeros = "00000000000000000000000000000000";
    const networkSix = `${networkZeros} 00 ${networkZeros} 00 `
        + "fe800000000000000000000000000001 00000400 00000001 00000000 00450003 wlp0s20f3\n"
        + "fd7a115ca1e000000000000000000000 30 00000000000000000000000000000000 00 "
        + "00000000000000000000000000000000 00000100 00000001 00000000 00000001 tailscale0\n";
    machine("an IPv6 default gateway", Interfaces.gateways6(networkSix), {
        wlp0s20f3: { address: "fe80:0000:0000:0000:0000:0000:0000:0001", metric: 1024 }
    });
    const networkResolv = "# written by resolved\nnameserver 127.0.2.2\nnameserver 127.0.2.3\n"
        + "search .\noptions edns0\n";
    machine("the name servers", Interfaces.servers(networkResolv), ["127.0.2.2", "127.0.2.3"]);
    machine("resolved's stub",
        Interfaces.stubbed(Interfaces.servers("nameserver 127.0.0.53\n")), true);
    machine("name servers of their own", Interfaces.stubbed(["127.0.2.2"]), false);

    // A cable's link as its folder in /sys/class/net says it, each
    // file's text as read, newline and all.
    const cableSys = {
        speed: "1000\n", duplex: "full\n", carrier: "1\n",
        carrier_down_count: "3\n", carrier_up_count: "4\n",
        "statistics/rx_errors": "12\n", "statistics/rx_crc_errors": "11\n",
        "statistics/rx_frame_errors": "1\n", "statistics/rx_length_errors": "0\n",
        "statistics/rx_over_errors": "0\n", "statistics/rx_missed_errors": "0\n",
        "statistics/rx_fifo_errors": "0\n", "statistics/tx_errors": "2\n",
        "statistics/tx_aborted_errors": "0\n", "statistics/tx_carrier_errors": "2\n",
        "statistics/tx_fifo_errors": "0\n", "statistics/tx_heartbeat_errors": "0\n",
        "statistics/tx_window_errors": "0\n", "statistics/collisions": "0\n",
        "statistics/rx_dropped": "40\n", "statistics/tx_dropped": "0\n"
    };
    const cable = Interfaces.cable(name => cableSys[name] ?? "");
    machine("a cable's speed and duplex", [cable.speed, cable.duplex], [1000, "full"]);
    machine("how a cable stands, from its carrier", [cable.state,
        Interfaces.cable(name => name === "carrier" ? "0\n" : "").state,
        Interfaces.cable(() => "").state], ["up", "no link", "off"]);
    machine("how often a cable's link was lost", cable.lost, 3);
    machine("a cable's errors each way", [cable.errors.in, cable.errors.out], [12, 2]);
    machine("only the kinds of error it had, in order",
        cable.errors.kinds.map(kind => [kind.id, kind.count]),
        [["damaged", 11], ["misaligned", 1], ["no-link", 2]]);
    machine("what a cable set aside each way", [cable.errors.droppedIn,
        cable.errors.droppedOut], [40, 0]);
    // A driver that doesn't know the speed says -1, and one that doesn't
    // know the duplex says so; a link that's down can't be read at all.
    const unknownCable = Interfaces.cable(name => ({ speed: "-1\n",
        duplex: "unknown\n" })[name] ?? "");
    machine("a speed and duplex nobody knows", [unknownCable.speed, unknownCable.duplex,
        unknownCable.lost, unknownCable.errors.in], [null, null, null, null]);
    machine("a half-duplex cable", Interfaces.cable(name => name === "duplex" ? "half" : "")
        .duplex, "half");
    const linkRates = [10, 100, 1000, 2500, 10000].map(mbps => Interfaces.linkRate(mbps,
        (value, places) => value.toFixed(places)));
    machine("a link's rate in its own units", linkRates,
        ["10 Mb/s", "100 Mb/s", "1 Gb/s", "2.5 Gb/s", "10 Gb/s"]);

    // A DHCP lease as networkd's Describe says it over varlink: its
    // times in microseconds since boot, the server's offer as options,
    // each its bytes in hex.
    const described = {
        Interfaces: [{
            Name: "cable0",
            DHCPv4Client: {
                Lease: {
                    LeaseTimestampUSec: 100000000, Timeout1USec: 43300000000,
                    Timeout2USec: 75700000000, Hostname: "desk",
                    Message: {
                        options: [{ tag: 1, data: "ffffff00" }, { tag: 3, data: "0a090002" },
                            { tag: 6, data: "0a0900030a090004" },
                            { tag: 15, data: "686f6d652e6172706100" },
                            { tag: 42, data: "0a090005" }, { tag: 51, data: "00015180" },
                            { tag: 54, data: "0a090002" }]
                    }
                }
            }
        }, { Name: "wlan0" }]
    };
    const leases = Networkd.leases(described);
    machine("a lease, by its connection", leases.cable0, {
        server: "10.9.0.2", lasts: 86400, got: 100, renews: 43300, rebinds: 75700,
        ends: 86500, nameServers: ["10.9.0.3", "10.9.0.4"], timeServers: ["10.9.0.5"],
        domain: "home.arpa", hostname: "desk"
    });
    machine("no lease where networkd holds none", "wlan0" in leases, false);
    // An older networkd says the times alone.
    const bare = Networkd.leases({ Interfaces: [{ Name: "eth0", DHCPv4Client: {
        Lease: { LeaseTimestampUSec: 5000000, Timeout1USec: 6000000 } } }] }).eth0;
    machine("a lease whose offer isn't said", [bare.server, bare.lasts, bare.got,
        bare.renews, bare.rebinds, bare.ends, bare.nameServers], [null, null, 5, 6, null,
        null, []]);
    machine("nothing from a reply that isn't one", Networkd.leases(null), {});

    // What's at a cable's other end, as networkd's GetLLDPNeighbors says.
    const neighbours = Networkd.neighbours({
        Neighbors: [{
            InterfaceIndex: 3, InterfaceName: "cable0",
            Neighbors: [{
                ChassisID: "52:54:00:aa:bb:cc", RawChassisID: [], PortID: "gi1/0/3",
                RawPortID: [], PortDescription: "desk drop 3", SystemName: "closet",
                SystemDescription: "a switch", EnabledCapabilities: 20, VlanID: 10
            }]
        }]
    });
    machine("a cable's neighbour", neighbours.cable0, [{
        system: "closet", description: "a switch", port: "gi1/0/3",
        portName: "desk drop 3", chassis: "52:54:00:aa:bb:cc", acts: ["bridge", "router"],
        vlan: 10, profile: null
    }]);
    machine("no neighbours from a reply that isn't one", Networkd.neighbours(undefined), {});
    machine("a neighbour's parts, joined and written, what it doesn't say left out",
        Networkd.neighbourParts(neighbours.cable0[0]).map(said => `${said.id}=${said.side}`), [
            "system=closet", "description=a switch", "port=gi1/0/3",
            "port-name=desk drop 3", "chassis=52:54:00:aa:bb:cc", "acts=bridge, router",
            "vlan=10"
        ]);
    machine("no acts said by a neighbour doing nothing",
        Networkd.neighbourParts({ system: "closet", acts: [] }).map(said => said.id),
        ["system"]);

    // How long a time lasted, and a signal's height, as Machine and Home
    // Network say them.
    machine("how long, in hours and minutes", Interfaces.lasted(11744), "3 h 15 min");
    machine("how long, in days", Interfaces.lasted(200000), "2 d 7 h");
    machine("how long, in whole hours", Interfaces.lasted(72000), "20 h");
    machine("how long, in whole days", Interfaces.lasted(172800), "2 d");
    machine("how long, only just", Interfaces.lasted(20), "under a minute");
    machine("a signal's height", [-90, -60, -30, -10].map(Interfaces.level), [0, 0.5, 1, 1]);
    machine("a whole reading takes the real minus and no decimals",
        [Numbers.wholeReading(-14), Numbers.wholeReading(-60.4), Numbers.wholeReading(3)],
        ["\u221214", "\u221260", "3"]);
    return asked;
}

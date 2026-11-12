// What the launcher's Home Network place says of the home network, run in
// Qt's own JavaScript engine, the one the shell runs it in, over the
// record the eero daemon's parsers are tested against
// (modules/eero/home-network.json). The `the-home-network-says-each-figure`
// check copies HomeNetwork.js, HomeNetworkDrawers.js and what they import
// into launcher/, as the shell lays them out, When.js and the record
// beside this file, and runs `qml home-network.qml` in UTC. Each case
// names what's asked and what has to come back: the rows' order, what
// each row says, and every figure the eero contract places in the
// internet's, a node's, a device's and a profile's drawer
// (modules/eero/contract/README.md).
import QtQuick
import "launcher/HomeNetwork.js" as Home
import "launcher/HomeNetworkDrawers.js" as Drawers
import "launcher/HomeNetworkHistory.js" as History

QtObject {
    // A moment a few minutes after eero last answered.
    readonly property real now: Date.parse("2026-11-07T23:00:00Z")

    function read(path) {
        const request = new XMLHttpRequest();
        request.open("GET", path, false);
        request.send();
        return JSON.parse(request.responseText);
    }

    // A drawer's facts by their part's id, or a radio's by its band.
    function byId(drawer) {
        const out = {};
        for (const fact of drawer.facts)
            if (fact.heading === undefined)
                out[fact.id === "port" ? `port ${fact.fill.n}` : fact.id ?? fact.label] = fact;
        return out;
    }

    Component.onCompleted: {
        const failures = [];
        let asked = 0;
        const expect = (what, got, wanted) => {
            asked++;
            const said = JSON.stringify(got);
            if (said !== JSON.stringify(wanted))
                failures.push(`${what} gave ${said}, not ${JSON.stringify(wanted)}`);
        };
        const model = read("home-network.json");
        const node = id => model.nodes.find(one => one.id === id);
        const device = name => model.devices.find(one => one.name === name);

        // The figures, each in its own unit, said one way, to three
        // significant digits (VKCF3Z, 2026-11-07).
        expect("a rate past a thousand Mb/s", Home.rate(1200.9), "1.20 Gb/s");
        expect("a rate to three digits", Home.rate(369.86), "370 Mb/s");
        expect("a rate's zero kept", Home.rate(48.0), "48.0 Mb/s");
        expect("a rate that rounds up a unit", Home.rate(999.96), "1.00 Gb/s");
        expect("a speed in Mb/s", Home.rate(11.452299), "11.5 Mb/s");
        expect("a slow speed", Home.rate(0.0239604), "24.0 kb/s");
        expect("a crawl", Home.rate(0.0000982), "98.2 b/s");
        expect("a link speed", Home.rate(1000), "1.00 Gb/s");
        expect("a figure under nothing takes the real minus", Home.signed(-48), "\u221248");
        expect("bytes", Home.bytes(153124674), "153 MB");
        expect("few bytes", Home.bytes(53741), "53.7 kB");
        expect("bytes under ten", Home.bytes(2400000), "2.40 MB");
        expect("no bytes", Home.bytes(0), "0 B");
        expect("an unknown size", Home.bytes(null), "");
        expect("the upper 5 GHz radio", Home.radioBand("5.8GHz"), "5 GHz high");
        expect("the lower 5 GHz radio", Home.radioBand("5.2GHz"), "5 GHz low");
        expect("a radio eero hasn't named before", Home.radioBand("3.5GHz"), "3.5 GHz");
        expect("eero's word", Home.worded("laptop_computer"), "laptop computer");
        expect("a time today", History.when("2026-11-07T07:22:42.035Z", now), "07:22");
        expect("yesterday", History.when("2026-11-06T16:21:33Z", now), "yesterday 16:21");
        expect("a day this week by its weekday", History.when("2026-11-05T10:52:15Z", now),
            "Thursday 10:52");
        expect("a day this year", History.when("2026-09-24T09:53:00Z", now), "24 Sep 09:53");
        expect("a year before", History.when("2020-11-13T21:47:15Z", now), "13 Nov 2020");
        expect("a moment ago", Home.ago("2026-11-07T22:59:48Z", now), "12 s ago");
        expect("minutes ago", Home.ago("2026-11-07T22:54:06.658Z", now), "5 min ago");

        // The gateway node first, then each node after the one it hangs
        // from, siblings by location, each with its devices, strongest
        // first.
        const groups = Home.grouped(model);
        expect("the nodes' order", groups.map(group => group.node.location),
            ["Hall", "Den", "Study"]);
        expect("the gateway node's devices", groups[0].devices.map(one => one.name),
            ["generic 3", "phone 1"]);
        expect("a node's devices", groups[1].devices.map(one => one.name),
            ["generic 2", "laptop computer 2", "printer 1", "tablet 1"]);
        // This machine, the weakest under its node, comes first under it and
        // says so where every row shows it.
        expect("this machine first under its node", groups[2].devices.map(one => one.name)
            .slice(0, 2), ["laptop computer 1", "generic 4"]);
        expect("this machine's side", Home.deviceRow(device("laptop computer 1")).side,
            "this machine \u00b7 \u221275 dBm");
        const astray = Object.assign({}, model, {
            devices: model.devices.concat([Object.assign({}, device("phone 1"),
                { mac: "02:00:00:00:00:ff", node: "9999" })])
        });
        expect("a device on no node listed", Home.grouped(astray).slice(-1)[0].node, null);
        const orphan = Object.assign({}, node("2002"), {
            id: "2009",
            location: "Attic",
            backhaul: { kind: "wi-fi", upstream: "Shed", radio: "5.8GHz", bars: 2 }
        });
        expect("a node hanging from one not listed comes last",
            Home.nodesInOrder(model.nodes.concat([orphan])).map(one => one.location),
            ["Hall", "Den", "Study", "Attic"]);

        // The head: the internet, how it stands and everything at a glance.
        // Its day said as HomeNetwork.qml has it said, by When.js.
        const internetRow = (said) => Home.internetRow(said, now, undefined, History.dayOn);
        expect("the internet's row", internetRow(model), {
            name: "Internet",
            side: "connected \u00b7 370 Mb/s down",
            note: "11.9 Mb/s up \u00b7 on Thursday \u00b7 conferencing and gaming on \u00b7 "
                + "1 outage in 30 days \u00b7 3 nodes online \u00b7 15 devices on now"
        });
        // The last speed test's day, as every day the launcher shows.
        const testedAt = (at) => internetRow(Object.assign({}, model, {
            internet: Object.assign({}, model.internet, {
                speed_test: Object.assign({}, model.internet.speed_test, { at: at })
            })
        })).note.split(" \u00b7 ")[1];
        expect("a speed test today", testedAt("2026-11-07T10:00:00Z"), "today");
        expect("a speed test yesterday", testedAt("2026-11-06T10:00:00Z"), "yesterday");
        expect("a speed test past six days", testedAt("2026-10-28T10:00:00Z"), "on 28 Oct");
        expect("a speed test another year", testedAt("2025-11-13T10:00:00Z"),
            "on 13 Nov 2025");
        const unkept = Object.assign({}, model, { internet: Object.assign({}, model.internet,
            { history: undefined }) });
        expect("eero's outages where no history is kept", internetRow(unkept).note
            .split(" \u00b7 ")[3], "no outages");
        expect("the home network's state where it isn't connected",
            internetRow(Object.assign({}, model, { internet: Object.assign({},
                model.internet, { mesh: "degraded" }) })).note.split(" \u00b7 ")[0],
            "home network degraded");
        expect("the internet unreachable", internetRow(Object.assign({}, model,
            { cloud: { state: "unreachable", since: model.read } })).side,
            "eero unreachable");
        expect("the place's holding", Home.holding(model), "3 nodes \u00b7 15 devices");

        // A node: its devices, its backhaul, and how it is.
        expect("the gateway node's row", Home.nodeRow(node("2003")), {
            name: "Hall",
            side: "2 devices",
            note: "gateway node \u00b7 Ethernet \u00b7 eero Pro 6 \u00b7 v7.17.4-5"
        });
        expect("a node over Wi-Fi", Home.nodeRow(node("2002")), {
            name: "Study",
            side: "9 devices",
            note: "Wi-Fi from Hall over 5 GHz high \u00b7 5 of 5 bars \u00b7 eero Pro 6 \u00b7 "
                + "v7.17.4-5"
        });
        expect("a node offline", Home.nodeRow(Object.assign({}, node("2001"),
            { state: "OFFLINE" })).note.split(" \u00b7 ")[0], "offline");

        // A device: its signal, band and channel, rate each way, speed now.
        expect("a quiet device", Home.deviceRow(device("generic 2")), {
            name: "generic 2",
            side: "\u221237 dBm",
            note: "5 GHz \u00b7 channel 52 \u00b7 1.20 Gb/s down \u00b7 1.08 Gb/s up \u00b7 idle \u00b7 "
                + "10.0.0.45",
            mark: "wireless"
        });
        expect("a busy computer", Home.deviceRow(device("unknown computer 1")).note,
            "5 GHz \u00b7 channel 149 \u00b7 1.20 Gb/s down \u00b7 1.20 Gb/s up \u00b7 "
            + "11.5 Mb/s down \u00b7 9.86 Mb/s up \u00b7 10.0.0.20");
        expect("a computer's mark", Home.deviceRow(device("unknown computer 1")).mark, "computer");
        expect("a device rated one way", Home.deviceRow(device("printer 1")).note,
            "2.4 GHz \u00b7 channel 6 \u00b7 72.2 Mb/s down \u00b7 idle \u00b7 10.0.0.31");
        expect("a trickle", Home.deviceRow(device("generic 3")).note,
            "2.4 GHz \u00b7 channel 6 \u00b7 72.2 Mb/s down \u00b7 98.2 b/s down \u00b7 154 b/s up "
            + "\u00b7 10.0.0.99");
        expect("a device in a profile", Home.deviceRow(Object.assign({}, device("printer 1"),
            { profile: "3002" }), {}, model.profiles).note.split(" \u00b7 ").slice(-1)[0],
            "profile 2");
        const cabled = Object.assign({}, device("generic 3"), {
            ethernet: true,
            band: null,
            channel: null,
            signal_dbm: null,
            rx: null,
            tx: null,
            port: { name: "2", mbps: 1000 }
        });
        expect("a device over Ethernet", Home.deviceRow(cabled), {
            name: "generic 3",
            side: "Ethernet",
            note: "Ethernet \u00b7 98.2 b/s down \u00b7 154 b/s up \u00b7 10.0.0.99",
            mark: "wired"
        });
        expect("a device named only by its address", Home.nameOf({ mac: "02:00:00:00:00:99" }),
            "02:00:00:00:00:99");

        // The internet's drawer: every figure the contract places there,
        // but how it stands, which its row's side says.
        const internet = byId(Drawers.internet(model, now));
        const value = (facts, id) => facts[id] ? facts[id].value : null;
        expect("the internet's eyebrow, where it comes from", [Drawers.internet(model, now)
            .eyebrow, Drawers.internet(model, now).names], ["via Example ISP", true]);
        expect("no state the row says", internet.state, undefined);
        for (const [id, wanted] of [
            ["mesh", "connected"],
            ["isp", "Example ISP \u00b7 AS64496"],
            ["public", "203.0.113.1"],
            ["lease", "DHCP \u00b7 203.0.113.1/23 \u00b7 router 203.0.113.2"],
            ["gateway", "10.0.0.1"],
            ["mode", "NAT \u00b7 addresses automatic"],
            ["dns", "automatic \u00b7 203.0.113.3 \u00b7 203.0.113.4 \u00b7 caching off \u00b7 "
                + "IPv6 automatic"],
            ["ipv6", "off"],
            ["main-subnet", "10.0.0.0/22 \u00b7 DHCP 10.0.0.20 to 10.0.0.254"],
            ["guest-subnet", "off \u00b7 10.0.0.0/21 \u00b7 DHCP 10.0.0.20 to 10.0.0.254"],
            ["forwards", "none"],
            ["pinholes", "none"],
            ["speed-test", "370 Mb/s down \u00b7 11.9 Mb/s up \u00b7 Thursday 10:52"],
            ["down", "370 Mb/s"],
            ["up", "11.9 Mb/s"],
            ["kept", "15 speed tests since 24 Sep 09:53"],
            ["sqm", "optimized"],
            ["upnp", "on"],
            ["randomization", "off"],
            ["steering", "on"],
            ["wpa3", "off"],
            ["security", "WPA2"],
            ["wifi-mode", "default"],
            ["ac-compatibility", "off"],
            ["name-clash", "none seen"],
            ["mlo", "off"],
            ["thread", "off \u00b7 name 56 \u00b7 channel 25"],
            ["guest", "off \u00b7 name 49"],
            ["ddns", "off"],
            ["backup", "on"],
            ["power-saving", "off"],
            ["outages", "1 in 30 days"],
            ["looked", "22:54 \u00b7 from home"],
            ["to-gateway", "2.00 ms"],
            ["to-internet", "11.9 ms"],
            ["lookup", "14.1 ms"],
            ["nodes", "3 nodes online"],
            ["firmware", "v7.17.4-5 \u00b7 up to date"],
            ["updates", "at 02:00 \u00b7 last 07:19"],
            ["traffic", "167 GB down \u00b7 72.7 GB up"],
            ["read", "22:54 \u00b7 answering"]
        ])
            expect(`the internet's ${id}`, value(internet, id), wanted);
        // What the head says leads, as it's what a narrow drawer has room
        // for before its more; the addresses and switches follow, each
        // under a heading of its own.
        const internetFacts = Drawers.internet(model, now).facts;
        expect("the internet's drawer leads with what its row says", internetFacts.slice(0, 6)
            .map(fact => fact.id), ["speed-test", "nodes", "outages", "isp", "mesh",
            "firmware"]);
        expect("the internet's groups", internetFacts.filter(fact => fact.heading !== undefined)
            .map(fact => fact.heading), ["probe", "outage-list", "speed-tests", "addressing",
            "settings", "events"]);

        // The internet's drawer holds only the internet: no device, on now
        // or not, is named in it, by its name or its hardware address, and
        // no list of devices is under it, since each device's drawer says
        // its own and each node's who came and went (Joel, 2026-11-11).
        const plain = text => String(text ?? "").replace(/\u00a0/g, " ").replace(/\u200b/g, "")
            .toLowerCase();
        const internetSays = internetFacts.map(fact => plain(`${fact.label ?? ""} `
            + `${fact.value ?? ""}`)).join("\n");
        expect("no device under the internet", model.devices.concat(model.devices_off)
            .filter(one => [Home.nameOf(one), one.mac].some(said => internetSays
                .includes(plain(said)))).map(one => Home.nameOf(one)), []);
        expect("no list of devices under the internet", internetFacts.filter(fact => ["devices",
            "reserved", "devices-off", "joins-and-leaves"].includes(fact.id ?? fact.heading))
            .map(fact => fact.id ?? fact.heading), []);
        expect("eero's outages counted where no history is kept", value(byId(Drawers
            .internet(unkept, now)), "outages"), "none recorded");

        // This machine's last look at the internet, each target's time
        // sparked over the last hour's looks.
        expect("the gateway node's time sparked, scaled to its most",
            internet["to-gateway"].figure.values.map(share => Math.round(share * 24) / 10),
            [2.1, 1.8, 2.4, 1.9, 2.2, 2]);
        expect("each target sparked", [internet["to-internet"].figure.shape,
            internet.lookup.figure.values.length], ["spark", 6]);
        const looks = model.internet.history.probes;
        const away = byId(Drawers.internet(Object.assign({}, model, {
            internet: Object.assign({}, model.internet, {
                probe: Object.assign({}, model.internet.probe, {
                    home: false, gateway_ms: null, internet_ms: null, reachable: false
                }),
                history: Object.assign({}, model.internet.history, {
                    probes: Object.assign({}, looks, {
                        internet_ms: [11.8, null, 13.1, 11.6, 12.0, null]
                    })
                })
            })
        }), now));
        expect("a look from away, the internet not reached", [value(away, "looked"),
            away["to-gateway"], away["to-internet"]], ["22:54 \u00b7 away from home \u00b7 "
            + "internet not reached", undefined, undefined]);
        expect("a look unanswered is a gap in its spark", Drawers.internet(Object.assign({},
            model, { internet: Object.assign({}, model.internet, {
                history: { probes: Object.assign({}, looks, {
                    internet_ms: [13.1, null, 13.1]
                }), outages: [] }
            }) }), now).facts.find(fact => fact.id === "to-internet").figure.values,
            [1, null, 1]);

        // Each outage of 30 days, newest first: when, how long, who saw it.
        // What's under a heading runs to the next, or to the drawer's end.
        const under = (facts, heading) => {
            const from = facts.findIndex(fact => fact.heading === heading) + 1;
            const to = facts.findIndex((fact, at) => at >= from && fact.heading !== undefined);
            return facts.slice(from, to < 0 ? facts.length : to);
        };
        const listed = heading => under(Drawers.internet(model, now).facts, heading);
        expect("an outage this machine saw", listed("outage-list"), [
            { label: "Tuesday 03:12", value: "1 min \u00b7 seen by this machine" }
        ]);
        const outageList = Drawers.internet(Object.assign({}, model, {
            internet: Object.assign({}, model.internet, {
                history: Object.assign({}, model.internet.history, { outages: [
                    { seen_by: "probe", start: "2026-11-07T22:50:00Z", end: null, eero: null },
                    { seen_by: "eero", start: null, end: null, eero: { kind: "lte" } }
                ] })
            })
        }), now).facts;
        expect("an outage going on, and one of eero's", outageList.slice(outageList.findIndex(
            fact => fact.heading === "outage-list") + 1).slice(0, 2), [
            { label: "22:50", value: "10 min so far \u00b7 seen by this machine" },
            { label: "time not given", value: "seen by eero" }
        ]);

        // The address kept for a device is in its own drawer.
        const addresses = model.internet.addresses;
        const keeping = (reservations, subnets) => Object.assign({}, model, {
            internet: Object.assign({}, model.internet, {
                addresses: Object.assign({}, addresses, { reservations,
                    subnets: subnets ?? addresses.subnets })
            })
        });
        const printer = device("printer 1");
        const reserving = keeping(addresses.reservations.concat([{ mac: printer.mac.toUpperCase(),
            description: "office printer", address: "10.0.0.30" }]));
        const odd = keeping([], [Object.assign({}, addresses.subnets[1], { kind: "iot",
            name: "things", mask: "255.0.255.0", on: true, internet: false, open: true })]);
        expect("a subnet of a kind eero hasn't named, by its name, its mask whole",
            byId(Drawers.internet(odd, now)).things.value, "no internet \u00b7 open "
            + "\u00b7 10.0.0.0 \u00b7 mask 255.0.255.0 \u00b7 DHCP 10.0.0.20 to 10.0.0.254");
        expect("a device's reserved address, not the one it has", ["address", "reserved"]
            .map(id => value(byId(Drawers.device(reserving, printer, now)), id)),
            ["10.0.0.31", "10.0.0.30"]);
        const keptAsIs = keeping([{ mac: printer.mac, description: "", address: printer.address }]);
        expect("the address it has, said reserved and said once", [
            value(byId(Drawers.device(keptAsIs, printer, now)), "address"),
            byId(Drawers.device(keptAsIs, printer, now)).reserved], ["10.0.0.31 \u00b7 reserved",
            undefined]);
        expect("bands that differ, band by band", value(byId(Drawers.internet(Object.assign({},
            model, { internet: Object.assign({}, model.internet, { security: [
                { band: "2.4 GHz", security: "wpa2" }, { band: "5 GHz", security: "wpa3" }] })
        }), now)), "security"), "2.4 GHz WPA2 \u00b7 5 GHz WPA3");
        expect("none for a device with none", byId(Drawers.device(model, printer, now))
            .reserved, undefined);
        expect("a name clash", value(byId(Drawers.internet(Object.assign({}, model, {
            internet: Object.assign({}, model.internet, { name_clash: true })
        }), now)), "name-clash"), "another network has its name");

        // The devices that joined or left a node over the last day, newest
        // first, in that node's drawer.
        const onNode = (id, heading, given) => under(Drawers.node(given ?? model, node(id), now)
            .facts, heading);
        expect("who left a node", onNode("2001", "joins-and-leaves"), [
            { label: "22:49", value: "phone\u00a01\u200b left \u00b7 5 GHz" }
        ]);
        expect("who joined a node", onNode("2002", "joins-and-leaves"), [
            { label: "22:39", value: "laptop\u00a0computer\u00a01\u200b joined \u00b7 5 GHz" }
        ]);
        // The devices eero knows that aren't on now, the last on first,
        // each in the drawer of the node it was last on, how and when.
        const off = ["2001", "2002", "2003"].map(id => onNode(id, "devices-off"));
        expect("each device not on now, under its node", [off.map(list => list.length),
            off[1][0], off[2][0]], [[5, 3, 1],
            { label: "game console 2", value: "5 GHz \u00b7 21:38" },
            { label: "generic 5", value: "Ethernet \u00b7 13:15" }]);
        expect("one eero names nothing, by its hardware address", off[0].some(fact =>
            fact.label === "02:00:00:00:00:1b"), true);
        expect("none off, no heading", Drawers.node(Object.assign({}, model,
            { devices_off: [] }), node("2001"), now).facts.some(fact =>
            fact.heading === "devices-off"), false);
        expect("none joined or left, no heading", Drawers.node(model, node("2003"), now).facts
            .some(fact => fact.heading === "joins-and-leaves"), false);
        expect("the speed tests sparked", internet.down.figure.shape, "spark");
        expect("a spark's newest last, scaled to the most", internet.up.figure.values.slice(-1)[0]
            <= 1 && internet.up.figure.values.length, 15);
        expect("the day's traffic sparked by the hour", internet.traffic.figure.values.length, 20);

        // A node's drawer: which, how it stands, each radio and port.
        const hall = byId(Drawers.node(model, node("2003"), now));
        for (const [id, wanted] of [
            ["model", "eero Pro 6 \u00b7 K010011"],
            ["serial", "SERIAL0003"],
            ["heard", "6 min ago"],
            ["backhaul", "Ethernet \u00b7 5 of 5 bars"],
            ["devices", "all on Wi-Fi"],
            ["2.4 GHz", "02:00:00:00:00:0c"],
            ["port 1", "1.00 Gb/s of 1.00 Gb/s \u00b7 internet \u00b7 backhaul \u00b7 0 B in \u00b7 "
                + "153 MB out"],
            ["port 2", "1.00 Gb/s of 1.00 Gb/s \u00b7 0 B in \u00b7 53.7 kB out"],
            ["firmware", "v7.17.4-5 \u00b7 up to date"],
            ["up", "15 h 31 min \u00b7 since 07:22"],
            ["cloud", "15 h 6 min"],
            ["added", "13 Nov 2020"],
            ["address", "10.0.0.1"],
            ["ipv6", "fe80::7/64 \u00b7 fe80::8/64"],
            ["mac", "02:00:00:00:00:21"],
            ["light", "on \u00b7 100%"],
            ["power", "USB"],
            ["traffic", "150 MB down \u00b7 10.1 MB up"]
        ])
            expect(`the gateway node's ${id}`, value(hall, id), wanted);
        expect("no support end eero didn't give", hall.support, undefined);
        expect("no state the row says", hall.state, undefined);
        expect("a node's drawer leads with what its row lacks", Drawers.node(model, node("2003"),
            now).facts.slice(0, 5).map(fact => fact.id), ["heard", "backhaul", "devices",
            "model", "serial"]);
        expect("a node offline says so first", Drawers.node(model, Object.assign({}, node("2003"),
            { state: "OFFLINE" }), now).facts[0], { id: "state", value: "offline" });
        const hallFacts = Drawers.node(model, node("2003"), now).facts;
        expect("each radio by band", under(hallFacts, "radios").map(fact => [fact.label,
            fact.value]), [
            ["2.4 GHz", "channel 6 \u00b7 20 MHz \u00b7 1 device"],
            ["5 GHz high", "channel 149 \u00b7 80 MHz \u00b7 1 device"],
            ["5 GHz low", "channel 52 \u00b7 80 MHz \u00b7 0 devices"]
        ]);
        const busy = under(hallFacts, "busy");
        expect("how busy each radio is", busy.map(fact => [fact.label, fact.value]), [
            ["2.4 GHz", "8%"], ["5 GHz high", "41%"], ["5 GHz low", "5%"]
        ]);
        expect("how busy, sparked as a share of all its time", busy[1].figure, {
            shape: "spark", values: [0.45, 0.38, 0.5, 0.43, 0.39, 0.41], warn: false
        });
        const power = under(hallFacts, "radio-power");
        expect("how strongly each radio sends", power.map(fact => [fact.label, fact.value]), [
            ["2.4 GHz", "30 dBm"], ["5 GHz high", "28 dBm"], ["5 GHz low", "22 dBm"]
        ]);
        expect("a steady power, sparked to its most", power[0].figure.values,
            [1, 1, 1, 1, 1, 1]);
        const unrecorded = Object.assign({}, node("2003"), {
            history: {},
            radios: node("2003").radios.map(radio => Object.assign({}, radio, { history: {} }))
        });
        const unrecordedFacts = Drawers.node(model, unrecorded, now).facts;
        expect("how busy as a stroke, where no history is kept",
            under(unrecordedFacts, "busy")[1].figure, {
                shape: "stroke", values: [0.41], warn: false
            });
        expect("no power spark where no history is kept",
            under(unrecordedFacts, "radio-power")[1].figure, null);
        const study = byId(Drawers.node(model, node("2002"), now));
        expect("a node's backhaul over Wi-Fi", value(study, "backhaul"),
            "Wi-Fi from Hall over 5 GHz high \u00b7 5 of 5 bars");
        expect("its bars sparked over the last hour, of five", study.backhaul.figure.values,
            [1, 0.8, 1, 1, 1, 1]);
        expect("a backhaul's bars as a stroke, where no history is kept", byId(Drawers.node(
            model, Object.assign({}, node("2002"), { history: {} }), now)).backhaul.figure,
            { shape: "stroke", values: [1], warn: false });
        expect("no bars over Ethernet", hall.backhaul.figure, null);
        expect("a port not linked", value(study, "port 1"), "not linked \u00b7 up to 1.00 Gb/s");
        const onHall = Drawers.node(model, node("2003"), now).facts;
        expect("the devices on a node, strongest first", under(onHall, "on-it"), [
            { label: "generic 3", value: "\u221248 dBm \u00b7 2.4 GHz \u00b7 channel 6" },
            { label: "phone 1", value: "\u221252 dBm \u00b7 5 GHz \u00b7 channel 149" }
        ]);
        expect("the eyebrow of a node, what it is then its kind", [Drawers.node(model,
            node("2002"), now).eyebrow, Drawers.node(model, node("2002"), now).names],
            ["on home \u00b7 node", true]);
        expect("a node's devices on both", byId(Drawers.node(model, Object.assign({},
            node("2002"), { devices: { all: 9, wifi: 6, ethernet: 3 } }), now)).devices.value,
            "6 on Wi-Fi \u00b7 3 on Ethernet");
        expect("a node with no devices", byId(Drawers.node(model, Object.assign({},
            node("2002"), { devices: { all: 0, wifi: 0, ethernet: 0 } }), now)).devices,
            undefined);

        // A device's drawer: each way's rate and how it's made, and the rest.
        const generic = Drawers.device(model, device("generic 2"), now);
        const facts = byId(generic);
        for (const [id, wanted] of [
            ["kind", "generic"],
            ["node", "Den \u00b7 eero Pro 6"],
            ["signal", "\u221237 dBm"],
            ["snr", "71 dB"],
            ["score", "5 of 5 bars"],
            ["band", "5 GHz \u00b7 channel 52 \u00b7 5,260 MHz"],
            ["bssid", "02:00:00:00:00:02"],
            ["rx", "1.20 Gb/s \u00b7 MCS 11 \u00b7 2 streams \u00b7 80 MHz \u00b7 800 ns guard "
                + "\u00b7 Wi-Fi 6"],
            ["tx", "1.08 Gb/s \u00b7 MCS 10 \u00b7 2 streams \u00b7 80 MHz \u00b7 800 ns guard "
                + "\u00b7 Wi-Fi 6"],
            ["speed", "idle"],
            ["retries", "0% \u00b7 quiet for 540 ms"],
            ["packets", "52,846 down \u00b7 94,913 up \u00b7 147,759 in all"],
            ["resent", "735 \u00b7 0 per million"],
            ["failed", "5,494 per million"],
            ["dropped", "0 \u00b7 0 per million"],
            ["network", "home \u00b7 WPA2 \u00b7 main"],
            ["address", "10.0.0.45"],
            ["addresses", "fe80::6"],
            ["mac", "02:00:00:00:00:0a \u00b7 private"],
            ["first-seen", "22 Oct 16:54"],
            ["on-since", "12:43"],
            ["last-active", "22:49"]
        ])
            expect(`a device's ${id}`, value(facts, id), wanted);
        expect("a device's eyebrow", [generic.eyebrow, generic.names], ["on Den \u00b7 Wi-Fi",
            true]);
        // Its title is what its row is called, as the eero daemon serves it, so
        // a device only its maker names reads "Intel device" in both.
        const unnamed = Object.assign({}, device("generic 2"), { name: null, hostname: null });
        const called = Drawers.device(model, unnamed, now, { called: "Intel device" });
        expect("a device's title is its row's name", called.title, "Intel device");
        expect("a device's title, without a row's name", Drawers.device(model, unnamed, now).title,
            unnamed.mac);
        expect("no hostname that only says its name again", facts.hostname, undefined);
        expect("a device's drawer leads with how well it's heard", generic.facts.slice(0, 7)
            .map(fact => fact.id), ["signal", "snr", "score", "band", "rx", "tx", "speed"]);
        expect("no profile it isn't in", facts.profile, undefined);

        // What moves sparks the device's last hour, as the daemon keeps it.
        expect("its signal sparked, from the faintest heard to the strongest",
            facts.signal.figure.values.map(level => Math.round(level * 60 - 90)),
            [-40, -38, -41, -39, -36, -37]);
        expect("its SNR sparked", facts.snr.figure.values, [1, 1, 1, 1, 1, 1]);
        expect("each way's rate sparked to its most", [facts.rx.figure.values.map(share =>
            Math.round(share * 12009) / 10), facts.tx.figure.values],
            [[1200.9, 1080.8, 900.7, 1200.9, 1200.9, 1200.9], [1, 1, 1, 1, 1, 1]]);
        expect("an idle hour sparks as nothing moving", facts.speed.figure.values,
            [0, 0, 0, 0, 0, 0]);
        expect("its speed sparked both ways together", byId(Drawers.device(model,
            device("unknown computer 1"), now)).speed.figure.values.indexOf(1), 1);
        const tabletFacts = byId(Drawers.device(model, device("tablet 1"), now));
        expect("no SNR nor rate up eero never gave", [tabletFacts.snr, tabletFacts.tx],
            [undefined, undefined]);
        const gapped = Object.assign({}, device("generic 2"), { history: {
            signal_dbm: [-60, null, -60], rx_mbps: Array(60).fill(10).concat(Array(60).fill(20))
        } });
        const gappedFacts = byId(Drawers.device(model, gapped, now));
        expect("a read eero gave nothing at is a gap", gappedFacts.signal.figure.values
            .map(level => level === null ? null : Math.round(level * 60 - 90)), [-60, null, -60]);
        expect("an hour read every 30 seconds fits twenty places", gappedFacts.rx.figure.values,
            Array(10).fill(0.5).concat(Array(10).fill(1)));

        // Where it moved over the last day, newest first, by when.
        const moved = facts => facts.slice(facts.findIndex(fact => fact.heading === "roams") + 1,
            facts.findIndex(fact => fact.heading === "events"));
        expect("a move to another node and band", moved(Drawers.device(model, device("tablet 1"),
            now).facts), [{ label: "22:44", value: "Study to Den \u00b7 2.4 GHz to 5 GHz" }]);
        expect("a move to another band on one node", moved(Drawers.device(model, Object.assign({},
            device("tablet 1"), { roams: [{ at: "2026-11-07T21:10:00Z", from_node: "2001",
                node: "2001", from_band: "2.4 GHz", band: "5 GHz" }] }), now).facts),
            [{ label: "21:10", value: "Den \u00b7 2.4 GHz to 5 GHz" }]);
        expect("no moves, no heading", generic.facts.some(fact => fact.heading === "roams"),
            false);
        const older = byId(Drawers.device(model, device("printer 1"), now));
        expect("a Wi-Fi 4 device's rate", value(older, "rx"),
            "72.2 Mb/s \u00b7 MCS 7 \u00b7 1 stream \u00b7 20 MHz \u00b7 400 ns guard \u00b7 Wi-Fi 4");
        expect("no rate up it never gave", older.tx, undefined);
        expect("eero's score as a share where it gives no bars", byId(Drawers.device(model,
            Object.assign({}, model.devices.find(one => one.mac === "02:00:00:00:00:05"),
            { bars: null }), now)).score.value, "78%");
        const inProfile = byId(Drawers.device(Object.assign({}, model, {
            profiles: [{ id: "3001", name: "profile 1", paused: true, devices: [] }]
        }), Object.assign({}, device("phone 1"), { profile: "3001" }), now));
        expect("a device's profile", value(inProfile, "profile"), "profile 1 \u00b7 paused");

        // What happened to the internet, newest first, in its drawer, with
        // each notice the daemon told set among eero's events; what
        // happened to a device is in the device's own.
        const events = Drawers.internet(model, now).facts;
        const timeline = events.slice(events.findIndex(fact => fact.heading === "events") + 1);
        // A name in what eero said keeps its words on one line, a name
        // inside a longer one kept as part of it.
        const named = Drawers.device(Object.assign({}, model, { told: [], events: [{
            at: "2026-11-07T22:50:00Z", text: "laptop computer 1 joined, then phone 1." }] }),
            device("laptop computer 1"), now).facts;
        expect("each name in an event kept whole",
            named[named.findIndex(fact => fact.heading === "events") + 1].value,
            "laptop\u00a0computer\u00a01\u200b joined, then phone\u00a01.");
        // laptop computer 1 joining is told in its own drawer.
        expect("every notice but a device's, and no device's event", timeline.length, 4);
        // Each is prose, set in Words, and eero's figures are set as ours.
        expect("a notice by when it was told, then what it said", timeline.slice(0, 2), [
            { label: "07:24",
                value: "The nodes updated to v7.17.4-5. They were on v7.17.3-9.", words: true },
            { label: "yesterday 16:21",
                value: "A firmware update is waiting. "
                    + "eero updates the nodes to v7.17.4-5 at 02:00.", words: true }
        ]);
        expect("the oldest last", timeline[3], { label: "Tuesday 03:12",
            value: "The internet is out. Nothing past the home network answers this machine.",
            words: true });
        const quiet = Drawers.internet(Object.assign({}, model, { events: [], told: [{
            at: "2026-11-07T07:24:13Z", summary: "The internet is back", body: "" }] }), now)
            .facts;
        expect("a notice with nothing more to say is its summary",
            quiet[quiet.findIndex(fact => fact.heading === "events") + 1].value,
            "The internet is back");
        const tablet = Drawers.device(model, device("tablet 1"), now).facts;
        expect("a device's own events", tablet.slice(tablet.findIndex(fact =>
            fact.heading === "events") + 1).map(fact => fact.value), [
            "tablet\u00a01\u200b connected to Den eero Pro 6 on 5.2\u00a0GHz channel 52.",
            "tablet\u00a01\u200b disconnected from Study eero Pro 6."
        ]);
        expect("a notice told of a device, in its drawer", under(Drawers.device(model,
            device("laptop computer 1"), now).facts, "events").some(fact => String(fact.value)
            .includes("joined the home network")), true);
        expect("none of another's", Drawers.device(model, device("generic 2"), now).facts
            .some(fact => fact.heading === "events"), false);

        // Each device's readings kept as records come, the newest last, so its
        // drawer sparks its signal and its speed.
        const once = Home.remembered({}, model, 3);
        expect("a first reading", once["02:00:00:00:00:0a"], { signal: [-37], speed: [0] });
        const later = Object.assign({}, model, {
            devices: [Object.assign({}, device("generic 2"), {
                signal_dbm: -41,
                speed: { down_mbps: 2, up_mbps: 0.5 }
            })]
        });
        let kept = once;
        for (let read = 0; read < 3; read++)
            kept = Home.remembered(kept, later, 3);
        expect("readings kept to the most", kept["02:00:00:00:00:0a"], {
            signal: [-41, -41, -41], speed: [2.5, 2.5, 2.5]
        });
        expect("a device gone is let go", Object.keys(kept).length, 1);
        // Those readings spark its drawer only where the record keeps no
        // history, as one from an older daemon wouldn't.
        const unkeptDevice = Object.assign({}, device("generic 2"), { history: {} });
        const sparked = byId(Drawers.device(model, unkeptDevice, now, {
            readings: Home.remembered(once, model, 20)["02:00:00:00:00:0a"]
        }));
        expect("a signal read twice, sparked", sparked.signal.figure.values.map(level =>
            Math.round(level * 60 - 90)), [-37, -37]);
        expect("a speed read twice, sparked", sparked.speed.figure.shape, "spark");
        expect("a signal read once, as a stroke", byId(Drawers.device(model, unkeptDevice, now))
            .signal.figure.shape, "stroke");

        // A profile: how it stands. Its drawer is the eero daemon's
        // (modules/eero's drawers_tests.rs).
        expect("a profile's row", Home.profileRow(model, model.profiles[0]), {
            name: "profile 1",
            side: "active",
            note: "10 devices \u00b7 none on now"
        });
        const held = Object.assign({}, model.profiles[1], {
            paused: true,
            devices: ["02:00:00:00:00:0a", "02:00:00:00:00:35"]
        });
        expect("a profile held off", Home.profileRow(model, held).side, "paused");

        // Said as warnings, since Qt keeps ordinary logging quiet.
        for (const failure of failures)
            console.warn(failure);
        console.warn(failures.length > 0 ? `${failures.length} went wrong of ${asked}`
            : `all ${asked} held`);
        Qt.exit(failures.length > 0 ? 1 : 0);
    }
}
